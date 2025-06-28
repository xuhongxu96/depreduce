use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

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
}

pub struct ReduceContext<'a> {
    pub settings: &'a ReduceSettings<'a>,

    history: HashMap<String, String>,
    in_degrees: Vec<usize>,
    added_node2in_nodes: Vec<HashSet<NodeId>>,
    removed_node2in_nodes: Vec<HashSet<NodeId>>,
    logs: String,
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
            logs: String::new(),
        }
    }

    pub fn add_dependents(&mut self, node_id: NodeId, dependent_node_id: NodeId) {
        assert!(
            !self.added_node2in_nodes[node_id].contains(&dependent_node_id),
            "Node {} is already added to node {}",
            self.settings.graph.nodes[node_id].label,
            self.settings.graph.nodes[dependent_node_id].label
        );

        self.added_node2in_nodes[node_id].insert(dependent_node_id);
        self.in_degrees[node_id] += 1;
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

        self.removed_node2in_nodes[node_id].insert(dependent_node_id);
        self.in_degrees[node_id] -= 1;
    }

    pub fn get_indegree(&self, node_id: NodeId) -> usize {
        self.in_degrees[node_id]
    }

    pub fn get_logs(&self) -> &str {
        &self.logs
    }

    pub fn backup(&mut self, edit: &FileEdit) {
        let backup_content = std::fs::read_to_string(&edit.path)
            .unwrap_or_else(|err| panic!("Failed to read file {}: {}", edit.path, err));

        if !self.history.contains_key(&edit.path) {
            self.history.insert(edit.path.clone(), backup_content);
        }
    }

    pub fn apply(&self, edit: &FileEdit) {
        std::fs::write(&edit.path, &edit.content)
            .unwrap_or_else(|err| panic!("Failed to write file {}: {}", edit.path, err));
    }

    pub fn restore_backup(&mut self) {
        let mut log = String::new();
        log.push_str("  Restoring backups:\n");
        for (path, content) in &self.history {
            std::fs::write(path, content)
                .unwrap_or_else(|err| panic!("Failed to restore file {}: {}", path, err));
            log.push_str(&format!("    {}\n", path));
        }
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

        for line in stderr_lines {
            let line = line.expect("Failed to read line from bazel query output");
            self.log(&indent_all_lines(&line, INDENT_SIZE_FOR_STDOUT));
            self.log("\n");
        }

        let exit = process.wait()?;

        process.stdout.take().map(|mut stdout| {
            let mut output = String::new();
            stdout.read_to_string(&mut output).unwrap();
            if !output.is_empty() {
                self.log(&indent_all_lines("--- stdout ---", INDENT_SIZE_FOR_STDOUT));
                self.log(&indent_all_lines(&output, INDENT_SIZE_FOR_STDOUT));
            }
        });

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
        self.history.clear();
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
        self.logs.push_str(message);
        if !cfg!(test) {
            print!("{}", message);
        }
    }
}
