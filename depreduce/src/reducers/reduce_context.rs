use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use utils::indent_all_lines;

use crate::editors::FileEdit;
use crate::graph::{DependencyGraph, NodeId};
use crate::reducers::candidate_generators::{
    ReductionCandidateGenerator, ReductionCandidateGeneratorFactory,
};

pub const INDENT_SIZE_FOR_STDOUT: usize = 8;

pub struct ReduceSettings<'a> {
    pub reduction_candidate_generator_factory: &'a dyn ReductionCandidateGeneratorFactory,
    pub graph: &'a DependencyGraph,
    pub build_command: String,
    pub cwd: String,
    pub save_build_log: bool,
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
    pub dependents: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReductionAttempt {
    candidates: GeneratedCandidates,
    ops: Vec<Operation>,
}

pub struct ReduceContext<'a> {
    pub settings: &'a ReduceSettings<'a>,

    history: HashMap<String, String>,
    in_degrees: Vec<usize>,
    added_node2in_nodes: Vec<HashSet<NodeId>>,
    removed_node2in_nodes: Vec<HashSet<NodeId>>,
    attempts: Vec<ReductionAttempt>,
}

impl<'a> ReduceContext<'a> {
    pub fn new(settings: &'a ReduceSettings<'a>) -> Self {
        Self {
            settings,
            history: HashMap::new(),
            in_degrees: settings
                .graph
                .nodes
                .iter()
                .map(|node| {
                    settings
                        .graph
                        .node2in_edges
                        .get(&node.id)
                        .map_or(0, |edges| edges.len())
                })
                .collect(),
            added_node2in_nodes: vec![HashSet::new(); settings.graph.nodes.len()],
            removed_node2in_nodes: vec![HashSet::new(); settings.graph.nodes.len()],
            attempts: Vec::new(),
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

        self.added_node2in_nodes[node_id].insert(dependent_node_id);
        self.removed_node2in_nodes[node_id].remove(&dependent_node_id);
        self.in_degrees[node_id] += 1;
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
        assert!(
            self.in_degrees[node_id] > 0,
            "Node {} has no incoming edges, cannot remove from node {}",
            self.settings.graph.nodes[node_id].label,
            self.settings.graph.nodes[dependent_node_id].label
        );

        self.added_node2in_nodes[node_id].remove(&dependent_node_id);
        self.removed_node2in_nodes[node_id].insert(dependent_node_id);
        self.in_degrees[node_id] -= 1;
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Remove(RemoveOperation {
                node_id,
                dependent_node_id,
            }));
    }

    pub fn get_indegree(&self, node_id: NodeId) -> usize {
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
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Restore(RestoreOperation {
                paths: self.history.keys().cloned().collect(),
            }));
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
        self.attempts
            .last_mut()
            .unwrap()
            .ops
            .push(Operation::Commit(CommitOperation {
                paths: self.history.keys().cloned().collect(),
            }));
        self.history.clear();
    }

    pub fn start_attempt(&mut self, node_id: NodeId, dependents_vec: Vec<NodeId>) {
        self.attempts.push(ReductionAttempt {
            candidates: GeneratedCandidates {
                node_id,
                dependents: dependents_vec,
            },
            ops: Vec::new(),
        });
    }

    pub fn generate_reduction_candidates(
        &mut self,
        node_id: NodeId,
    ) -> Box<dyn ReductionCandidateGenerator> {
        let mut dependents_vec = Vec::new();

        if let Some(dependents) = self.settings.graph.node2in_edges.get(&node_id) {
            dependents_vec = dependents
                .iter()
                .filter(|(dependent_node_id, _)| {
                    !self.removed_node2in_nodes[node_id].contains(dependent_node_id)
                })
                .map(|(dependent_node_id, _)| *dependent_node_id)
                .collect();
        }

        dependents_vec.extend(self.added_node2in_nodes[node_id].iter());
        dependents_vec.sort();

        self.settings
            .reduction_candidate_generator_factory
            .create(dependents_vec)
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
