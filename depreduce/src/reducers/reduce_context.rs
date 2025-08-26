use std::collections::{BinaryHeap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use utils::indent_all_lines;

use crate::editors::{DepEditor, FileEdit};
use crate::graph::{DependencyGraph, EdgeProps, NodeId};

pub const INDENT_SIZE_FOR_STDOUT: usize = 8;

pub struct ReduceSettings<'a> {
    pub editor: &'a dyn DepEditor,
    pub build_command: String,
    pub cwd: String,
    pub save_build_log: bool,

    // settings
    pub deps_only: bool,
    pub disable_dependency_flattening: bool,
    pub disable_dependency_flattening_for_alias_targets: bool,
    pub disable_dependency_lifting: bool,
    pub disable_topological_sorting: bool,
    pub disable_optimization_if_transitive_deps_exists: bool,
    pub timeout_seconds: u64,

    pub skip_from_node_ids_for_addition: HashSet<NodeId>,
    pub skip_to_node_ids_for_addition: HashSet<NodeId>,
    pub skip_from_node_ids_for_removal: HashSet<NodeId>,
    pub skip_to_node_ids_for_removal: HashSet<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddOperation {
    pub node_id: NodeId,
    pub dependent_node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveOperation {
    pub node_id: NodeId,
    pub dependent_node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildOperation {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupOperation {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyOperation {
    pub edit: FileEdit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOperation {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitOperation {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Add(AddOperation),
    Remove(RemoveOperation),
    Build(BuildOperation),
    Backup(BackupOperation),
    Restore(RestoreOperation),
    Apply(ApplyOperation),
    Commit(CommitOperation),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCandidates {
    pub node_id: NodeId,
    pub dependent: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReductionAttempt {
    pub candidates: GeneratedCandidates,
    pub ops: Vec<Operation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub remarks: Option<String>,
}

pub struct ReduceContext<'a> {
    pub graph: DependencyGraph,
    pub settings: &'a ReduceSettings<'a>,

    history: HashMap<String, String>,
    transitive_deps: Vec<HashSet<NodeId>>,
    transitive_without_direct_deps: Vec<HashSet<NodeId>>,
    original_deps: HashSet<(NodeId, NodeId)>, // src -> dep

    current_node_id: Option<NodeId>,
    dependent_candidates: BinaryHeap<(usize, NodeId)>,
    attempts: Vec<ReductionAttempt>,

    node2topsort_index: Vec<usize>,
}

impl<'a> ReduceContext<'a> {
    pub fn new(graph: DependencyGraph, settings: &'a ReduceSettings<'a>) -> Self {
        let graph_node_len = graph.nodes.len();
        let (transitive_deps, transitive_without_direct_deps) = graph.calculate_transitive_deps();
        let original_deps = graph
            .node2in_edges
            .iter()
            .flat_map(|(node_id, edges)| {
                edges
                    .iter()
                    .map(move |(dependent_node_id, _)| (*dependent_node_id, *node_id))
            })
            .collect::<HashSet<_>>();

        Self {
            graph,
            settings,
            history: HashMap::new(),
            transitive_deps,
            transitive_without_direct_deps,
            original_deps,
            current_node_id: None,
            dependent_candidates: BinaryHeap::new(),
            attempts: Vec::new(),
            node2topsort_index: vec![0; graph_node_len],
        }
    }

    pub fn init_node2topsort_index(&mut self, sorted_nodes: &[NodeId]) {
        for (index, &node_id) in sorted_nodes.iter().enumerate() {
            self.node2topsort_index[node_id] = index;
        }
    }

    pub fn check_remove_dependent(&mut self, node_id: NodeId, dependent_node_id: NodeId) -> bool {
        let label = || self.graph.nodes[node_id].label.clone();
        let dependent_label = || self.graph.nodes[dependent_node_id].label.clone();

        if self.is_added_dep(dependent_node_id, node_id) {
            // If the edge was added by us, we can always remove it.
            self.log(&format!(
                "  Allow removing {} -> {} (added by us)\n",
                dependent_label(),
                label()
            ));
            return true;
        }

        if self
            .settings
            .skip_from_node_ids_for_removal
            .contains(&dependent_node_id)
        {
            self.log(&format!(
                "  Skipping removing {} -> {} (skipped by `from` rules in config)\n",
                dependent_label(),
                label()
            ));
            return false;
        }

        if self
            .settings
            .skip_to_node_ids_for_removal
            .contains(&node_id)
        {
            self.log(&format!(
                "  Skipping removing {} -> {} (skipped by `to` rules in config)\n",
                dependent_label(),
                label()
            ));
            return false;
        }

        if let Some(edge_id) = self.graph.get_edge_id(dependent_node_id, node_id) {
            if self.graph.edges[edge_id]
                .as_ref()
                .unwrap()
                .props
                .unremovable
            {
                self.log(&format!(
                    "  Skipping removing {} -> {} (edge is unremovable)\n",
                    dependent_label(),
                    label()
                ));
                return false;
            }
        }

        true
    }

    pub fn check_add_dependent(&mut self, node_id: NodeId, dependent_node_id: NodeId) -> bool {
        let label = || self.graph.nodes[node_id].label.clone();
        let dependent_label = || self.graph.nodes[dependent_node_id].label.clone();

        if let Some(edges) = self.graph.node2out_edges.get(&dependent_node_id) {
            if edges.contains_key(&node_id) {
                self.log(&format!(
                    "  Skipping adding {} -> {} (already exists)\n",
                    dependent_label(),
                    label()
                ));
                return false;
            }
        }

        if self
            .settings
            .skip_from_node_ids_for_addition
            .contains(&dependent_node_id)
        {
            self.log(&format!(
                "  Skipping adding {} -> {} (skipped by `from` rules in config)\n",
                dependent_label(),
                label()
            ));
            return false;
        }

        if self
            .settings
            .skip_to_node_ids_for_addition
            .contains(&node_id)
        {
            self.log(&format!(
                "  Skipping adding {} -> {} (skipped by `to` rules in config)\n",
                dependent_label(),
                label()
            ));
            return false;
        }

        match self.graph.nodes[node_id].props.t {
            crate::graph::NodeType::Target(_) => {}
            _ => {
                self.log(&format!(
                    "  Skipping adding {} -> {} (non-target)\n",
                    dependent_label(),
                    label()
                ));
                return false;
            }
        }

        true
    }

    pub fn add_dependent(&mut self, node_id: NodeId, dependent_node_id: NodeId) {
        assert!(
            !self
                .graph
                .get_in_edges(node_id)
                .map_or(false, |edges| edges.contains_key(&dependent_node_id)),
            "Node {} is already a dependency of node {}",
            self.graph.nodes[node_id].label,
            self.graph.nodes[dependent_node_id].label
        );

        if self.current_node_id.map_or(false, |id| id == node_id) {
            self.log(&format!(
                "  Adding {} as a dependent candidate of {}\n",
                self.graph.nodes[dependent_node_id].label, self.graph.nodes[node_id].label
            ));
            self.dependent_candidates.push((
                self.node2topsort_index[dependent_node_id],
                dependent_node_id,
            ));
        }

        self.graph
            .add_edge(dependent_node_id, node_id, EdgeProps::default())
            .unwrap();
        (self.transitive_deps, self.transitive_without_direct_deps) =
            self.graph.calculate_transitive_deps();

        self.log(
            format!(
                "  In-degree of {} is now {}\n",
                self.graph.nodes[node_id].label,
                self.get_indegree(node_id)
            )
            .as_str(),
        );
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Add(AddOperation {
                node_id,
                dependent_node_id,
            }));
    }

    pub fn remove_dependent(&mut self, node_id: NodeId, dependent_node_id: NodeId) {
        assert!(
            self.graph
                .get_in_edges(node_id)
                .map_or(false, |edges| edges.contains_key(&dependent_node_id)),
            "Node {} is already removed from node {}",
            self.graph.nodes[node_id].label,
            self.graph.nodes[dependent_node_id].label
        );

        self.graph
            .remove_edge(self.graph.get_edge_id(dependent_node_id, node_id).unwrap())
            .unwrap();
        (self.transitive_deps, self.transitive_without_direct_deps) =
            self.graph.calculate_transitive_deps();

        self.log(
            format!(
                "  In-degree of {} is now {}\n",
                self.graph.nodes[node_id].label,
                self.get_indegree(node_id)
            )
            .as_str(),
        );
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Remove(RemoveOperation {
                node_id,
                dependent_node_id,
            }));
    }

    pub fn get_indegree(&self, node_id: NodeId) -> i32 {
        self.graph
            .node2in_edges
            .get(&node_id)
            .map(|edges| {
                edges.iter().filter(|&(src_node_id, edge_id)| {
                    !self.graph.nodes[*src_node_id].props.t.is_alias_target()
                        && !self.graph.edges[*edge_id]
                            .as_ref()
                            .unwrap()
                            .props
                            .unremovable
                })
            })
            .map_or(0, |edges| edges.count().try_into().unwrap())
    }

    pub fn backup(&mut self, edit: &FileEdit) {
        let backup_content = std::fs::read_to_string(&edit.path)
            .unwrap_or_else(|err| panic!("Failed to read file {}: {}", edit.path, err));

        if !self.history.contains_key(&edit.path) {
            self.history.insert(edit.path.clone(), backup_content);
            self.attempts
                .last_mut()
                .unwrap()
                .ops
                .push(Operation::Backup(BackupOperation {
                    path: edit.path.clone(),
                }));
        }
    }

    pub fn apply(&mut self, edit: FileEdit) {
        std::fs::write(&edit.path, &edit.content)
            .unwrap_or_else(|err| panic!("Failed to write file {}: {}", edit.path, err));
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Apply(ApplyOperation { edit }));
    }

    pub fn restore_backup(&mut self) {
        let mut log = String::new();
        log.push_str("  Restoring backups:\n");
        for (path, content) in &self.history {
            std::fs::write(path, content)
                .unwrap_or_else(|err| panic!("Failed to restore file {}: {}", path, err));
            log.push_str(&format!("    {}\n", path));
        }

        let mut paths: Vec<String> = self.history.keys().cloned().collect();
        paths.sort();
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Restore(RestoreOperation { paths }));
        self.log(&log);
        self.history.clear();
    }

    pub fn try_build(&mut self) -> Result<String, std::io::Error> {
        self.log(&format!(
            "  Running build command: {} (cwd: {})\n",
            self.settings.build_command, self.settings.cwd
        ));
        let start_time = std::time::Instant::now();
        let mut process = Command::new("/bin/bash")
            .arg(&self.settings.build_command)
            .current_dir(&self.settings.cwd)
            .stderr(Stdio::piped())
            .spawn()?;

        let stderr = process.stderr.as_mut().unwrap();
        let stderr_reader = BufReader::new(stderr);
        let stderr_lines = stderr_reader.lines();
        let mut stderr_str = String::new();

        for line in stderr_lines {
            let line = line.expect("Failed to read line from bazel query output");
            stderr_str.push_str(&line);
            stderr_str.push('\n');
            if line.is_empty() {
                continue;
            }
            self.log(&indent_all_lines(&line, INDENT_SIZE_FOR_STDOUT));
            self.log("\n");

            let elapsed = start_time.elapsed();
            if self.settings.timeout_seconds > 0
                && elapsed.as_secs() >= self.settings.timeout_seconds
            {
                self.log("  Build is taking too long, killing the process...\n");
                process.kill().ok();

                let mut process = Command::new("bazel")
                    .arg("shutdown")
                    .current_dir(&self.settings.cwd)
                    .stderr(Stdio::piped())
                    .spawn()?;

                let stderr = process.stderr.as_mut().unwrap();
                let stderr_reader = BufReader::new(stderr);
                let stderr_lines = stderr_reader.lines();

                for line in stderr_lines {
                    self.log(&indent_all_lines(
                        &line
                            .as_ref()
                            .unwrap_or(&"<failed to read line>".to_string()),
                        INDENT_SIZE_FOR_STDOUT,
                    ));
                    if line.as_ref().map_or(false, |l| {
                        l.contains("Waiting for it to complete on the server (server_pid=")
                    }) {
                        let server_pid = line
                            .unwrap()
                            .split("server_pid=")
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                            .and_then(|s| s.parse::<i32>().ok());
                        if let Some(pid) = server_pid {
                            self.log(&format!(
                                "  Killing bazel server process with PID {}\n",
                                pid
                            ));
                            Command::new("kill")
                                .arg("-9")
                                .arg(pid.to_string())
                                .output()
                                .ok();
                        }
                    }
                }

                process.wait().ok();

                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Build timed out",
                ));
            }
        }

        let exit = process.wait()?;

        let stdout = process.stdout.take().map(|mut stdout| {
            let mut output = String::new();
            stdout.read_to_string(&mut output).unwrap();
            let trimmed_output = output.trim();
            if !trimmed_output.is_empty() {
                self.log(&indent_all_lines("--- stdout ---", INDENT_SIZE_FOR_STDOUT));
                self.log(&indent_all_lines(&trimmed_output, INDENT_SIZE_FOR_STDOUT));
            }
            return output;
        });

        if !self.attempts.is_empty() {
            self.attempts
                .last_mut()
                .unwrap()
                .ops
                .push(Operation::Build(BuildOperation {
                    exit_code: exit.code().unwrap_or(-1),
                    stdout: if self.settings.save_build_log {
                        stdout.unwrap_or_default()
                    } else {
                        String::new()
                    },
                    stderr: if self.settings.save_build_log {
                        stderr_str
                    } else {
                        String::new()
                    },
                }));
        }

        if exit.success() {
            return Ok(format!("Build succeeded"));
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Build failed with exit code {}", exit.code().unwrap_or(-1),),
            ));
        }
    }

    pub fn commit_changes(&mut self) {
        let mut paths: Vec<String> = self.history.keys().cloned().collect();
        paths.sort();
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Commit(CommitOperation { paths }));
        self.history.clear();
    }

    pub fn start_attempt(
        &mut self,
        node_id: NodeId,
        dependent_node_id: NodeId,
        remarks: Option<String>,
    ) {
        self.attempts.push(ReductionAttempt {
            candidates: GeneratedCandidates {
                node_id,
                dependent: dependent_node_id,
            },
            ops: Vec::new(),
            remarks,
        });
    }

    pub fn next_attempt(&mut self, remarks: Option<String>) -> Option<NodeId> {
        if self.dependent_candidates.is_empty() {
            return None;
        }

        let dependent = self.dependent_candidates.pop().unwrap();

        self.log(&format!(
            "  Trying a new candidate. Remaining candidates: {}\n",
            self.dependent_candidates.len()
        ));

        self.attempts.push(ReductionAttempt {
            candidates: GeneratedCandidates {
                node_id: self.current_node_id.unwrap(),
                dependent: dependent.1,
            },
            ops: Vec::new(),
            remarks,
        });

        Some(dependent.1)
    }

    pub fn get_current_attempt(&self) -> Option<&ReductionAttempt> {
        self.attempts.last()
    }

    pub fn generate_reduction_candidates(&mut self, node_id: NodeId) {
        self.dependent_candidates.clear();
        self.current_node_id = Some(node_id);

        if let Some(dependents) = self.graph.node2in_edges.get(&node_id) {
            self.dependent_candidates.extend(
                dependents
                    .iter()
                    .filter(|&(_, edge_id)| {
                        !self.graph.edges[*edge_id]
                            .as_ref()
                            .unwrap()
                            .props
                            .unremovable
                    })
                    .map(|(dependent_node_id, _)| {
                        (
                            self.node2topsort_index[*dependent_node_id],
                            *dependent_node_id,
                        )
                    }),
            )
        }
    }

    pub fn log(&mut self, message: &str) {
        if !cfg!(test) {
            print!("{}", message);
        }
    }

    pub fn get_attempts(&self) -> &[ReductionAttempt] {
        &self.attempts
    }

    pub fn is_added_dep(&self, dependent_node_id: NodeId, node_id: NodeId) -> bool {
        !self.original_deps.contains(&(dependent_node_id, node_id))
    }

    pub fn has_transitive_deps(
        &self,
        dependent_node_id: NodeId,
        node_id: NodeId,
        consider_direct_deps: bool,
    ) -> bool {
        if consider_direct_deps {
            self.transitive_deps[dependent_node_id].contains(&node_id)
        } else {
            self.transitive_without_direct_deps[dependent_node_id].contains(&node_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        editors::BazelDepEditor,
        graph::bazel_xml_parser::{Query, parse_bazel_xml},
    };

    use super::*;
    use utils::*;

    #[test]
    fn test_calculate_in_degrees() {
        let xml = read_test_data!("perses.xml");
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = query.to_dep_graph(false, &HashSet::new()).unwrap();
        let editor = BazelDepEditor::new(
            &Query {
                values: vec![],
                version: 0,
            },
            "",
        );
        let settings = ReduceSettings {
            editor: &editor,
            build_command: "bazel build //...".to_string(),
            cwd: ".".to_string(),
            save_build_log: false,
            deps_only: false,
            timeout_seconds: 0,
            disable_dependency_flattening: false,
            disable_dependency_flattening_for_alias_targets: false,
            disable_dependency_lifting: false,
            disable_topological_sorting: false,
            disable_optimization_if_transitive_deps_exists: false,
            skip_from_node_ids_for_removal: HashSet::new(),
            skip_to_node_ids_for_removal: HashSet::new(),
            skip_from_node_ids_for_addition: HashSet::new(),
            skip_to_node_ids_for_addition: HashSet::new(),
        };

        let ctx = ReduceContext::new(graph, &settings);

        let in_degrees = ctx
            .graph
            .nodes
            .iter()
            .map(|node| ctx.get_indegree(node.id))
            .enumerate()
            .map(|(i, d)| (i, ctx.graph.nodes[i].label.to_string(), d))
            .collect::<Vec<_>>();

        let res = to_json_lines(&in_degrees);
        assert_eq!(
            res,
            read_or_create_test_data!("reducers/perses-in-degrees.jsonl", res)
        );
    }
}
