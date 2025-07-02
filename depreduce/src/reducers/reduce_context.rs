use std::collections::{BinaryHeap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use utils::indent_all_lines;

use crate::editors::{DepEditor, FileEdit};
use crate::graph::{DependencyGraph, NodeId, NodeProps, NodeType, TargetType};

pub const INDENT_SIZE_FOR_STDOUT: usize = 8;

pub struct ReduceSettings<'a> {
    pub editor: &'a dyn DepEditor,
    pub graph: &'a DependencyGraph,
    pub build_command: String,
    pub cwd: String,
    pub save_build_log: bool,

    // settings
    pub disable_dependency_flattening: bool,
    pub disable_dependency_lifting: bool,
    pub disable_topological_sorting: bool,
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
    pub path: String,
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
    pub settings: &'a ReduceSettings<'a>,

    history: HashMap<String, String>,
    in_degrees: Vec<i32>,

    added_node2in_nodes: Vec<HashSet<NodeId>>,
    removed_node2in_nodes: Vec<HashSet<NodeId>>,

    current_node_id: Option<NodeId>,
    dependent_candidates: BinaryHeap<(usize, NodeId)>,
    attempts: Vec<ReductionAttempt>,

    node2topsort_index: Vec<usize>,
}

fn calculate_in_degrees(graph: &DependencyGraph) -> Vec<i32> {
    graph
        .nodes
        .iter()
        .map(|node| {
            graph
                .node2in_edges
                .get(&node.id)
                .map(|edges| {
                    edges.iter().filter(|(src_node_id, _)| {
                        !graph.nodes[**src_node_id].props.t.is_alias_target()
                    })
                })
                .map_or(0, |edges| edges.count().try_into().unwrap())
        })
        .collect()
}

impl<'a> ReduceContext<'a> {
    pub fn new(settings: &'a ReduceSettings<'a>) -> Self {
        assert!(
            !settings.disable_topological_sorting
                || (settings.disable_dependency_flattening && settings.disable_dependency_lifting),
            "disable_topological_sorting can only be set when disable_dependency_flattening and disable_dependency_lifting are both set"
        );

        Self {
            settings,
            history: HashMap::new(),
            in_degrees: calculate_in_degrees(settings.graph),
            added_node2in_nodes: vec![HashSet::new(); settings.graph.nodes.len()],
            removed_node2in_nodes: vec![HashSet::new(); settings.graph.nodes.len()],
            current_node_id: None,
            dependent_candidates: BinaryHeap::new(),
            attempts: Vec::new(),
            node2topsort_index: vec![0; settings.graph.nodes.len()],
        }
    }

    pub fn init_node2topsort_index(&mut self, sorted_nodes: &[NodeId]) {
        for (index, &node_id) in sorted_nodes.iter().enumerate() {
            self.node2topsort_index[node_id] = index;
        }
    }

    pub fn get_removed_dependents(&self, node_id: NodeId) -> &HashSet<NodeId> {
        &self.removed_node2in_nodes[node_id]
    }

    pub fn get_added_dependents(&self, node_id: NodeId) -> &HashSet<NodeId> {
        &self.added_node2in_nodes[node_id]
    }

    pub fn add_dependent(&mut self, node_id: NodeId, dependent_node_id: NodeId) {
        assert!(
            !self.added_node2in_nodes[node_id].contains(&dependent_node_id),
            "Node {} is already added to node {}",
            self.settings.graph.nodes[node_id].label,
            self.settings.graph.nodes[dependent_node_id].label
        );

        if self.current_node_id.map_or(false, |id| id == node_id) {
            self.log(&format!(
                "  Adding {} as a dependent candidate of {}\n",
                self.settings.graph.nodes[dependent_node_id].label,
                self.settings.graph.nodes[node_id].label
            ));
            self.dependent_candidates.push((
                self.node2topsort_index[dependent_node_id],
                dependent_node_id,
            ));
        }

        self.added_node2in_nodes[node_id].insert(dependent_node_id);
        self.removed_node2in_nodes[node_id].remove(&dependent_node_id);
        self.in_degrees[node_id] += 1;
        self.log(
            format!(
                "  In-degree of {} is now {}\n",
                self.settings.graph.nodes[node_id].label, self.in_degrees[node_id]
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
            !self.removed_node2in_nodes[node_id].contains(&dependent_node_id),
            "Node {} is already removed from node {}",
            self.settings.graph.nodes[node_id].label,
            self.settings.graph.nodes[dependent_node_id].label
        );

        self.added_node2in_nodes[node_id].remove(&dependent_node_id);
        self.removed_node2in_nodes[node_id].insert(dependent_node_id);
        self.in_degrees[node_id] -= 1;
        self.log(
            format!(
                "  In-degree of {} is now {}\n",
                self.settings.graph.nodes[node_id].label, self.in_degrees[node_id]
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
        self.in_degrees[node_id]
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

    pub fn apply(&mut self, edit: &FileEdit) {
        std::fs::write(&edit.path, &edit.content)
            .unwrap_or_else(|err| panic!("Failed to write file {}: {}", edit.path, err));
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Apply(ApplyOperation {
                path: edit.path.clone(),
            }));
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

        if let Some(dependents) = self.settings.graph.node2in_edges.get(&node_id) {
            self.dependent_candidates.extend(
                dependents
                    .iter()
                    .filter(|(dependent_node_id, _)| {
                        !self.removed_node2in_nodes[node_id].contains(dependent_node_id)
                    })
                    .map(|(dependent_node_id, _)| {
                        (
                            self.node2topsort_index[*dependent_node_id],
                            *dependent_node_id,
                        )
                    }),
            )
        }

        self.dependent_candidates
            .extend(
                self.added_node2in_nodes[node_id]
                    .iter()
                    .map(|&dependent_node_id| {
                        (
                            self.node2topsort_index[dependent_node_id],
                            dependent_node_id,
                        )
                    }),
            );
    }

    pub fn log(&mut self, message: &str) {
        if !cfg!(test) {
            print!("{}", message);
        }
    }

    pub fn get_attempts(&self) -> &[ReductionAttempt] {
        &self.attempts
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::bazel_xml_parser::{convert_query_to_dep_graph, parse_bazel_xml};

    use super::*;
    use utils::*;

    #[test]
    fn test_calculate_in_degrees() {
        let xml = read_test_data!("perses.xml");
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();

        let in_degrees = calculate_in_degrees(&graph)
            .iter()
            .enumerate()
            .map(|(i, &d)| (i, graph.nodes[i].label.to_string(), d))
            .collect::<Vec<_>>();

        let res = to_json_lines(&in_degrees);
        assert_eq!(
            res,
            read_or_create_test_data!("reducers/perses-in-degrees.jsonl", res)
        );
    }
}
