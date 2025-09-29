use std::{
    io::{BufRead, BufReader},
    process::Command,
};

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::{BazelDepEditor, DepEditor},
    filters::BuildSystemSpecificInfo,
    graph::{
        DependencyGraph,
        bazel_xml_parser::{Query, parse_bazel_xml},
    },
    supports::BuildSystemSupport,
};

fn get_bazel_query(workspace: &str, target: &str) -> String {
    let mut p = Command::new("bazel")
        .arg("query")
        .arg(format!(
            "deps({})",
            if target.is_empty() { "//..." } else { target }
        ))
        .arg("--keep_going")
        .arg("--notool_deps")
        .arg("--noimplicit_deps")
        .arg("--output")
        .arg("xml")
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run bazel query");

    let mut xml_str = String::new();
    let stdout = p.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for (i, line) in stdout_lines.enumerate() {
        let line = line.expect("Failed to read line from bazel query output");

        xml_str.push_str(&line);
        if i % 1000 == 0 {
            eprintln!("Read {} lines from bazel query output...", i);
        }
    }

    p.wait().expect("Bazel query did not finish successfully");

    xml_str
}

pub struct BazelSupport {
    query: Query,
    graph: DependencyGraph,
}

impl BazelSupport {
    pub fn new(workspace: &str, target: &str, config: &ReduceConfig) -> Self {
        let query_xml = get_bazel_query(workspace, target);
        let query = parse_bazel_xml(&query_xml).unwrap();
        let graph = query.to_dep_graph(&config.readonly_deps_attrs).unwrap();

        BazelSupport { query, graph }
    }

    fn get_info(&self) -> BuildSystemSpecificInfo {
        BuildSystemSpecificInfo::Bazel(&self.query)
    }
}

impl BuildSystemSupport for BazelSupport {
    fn get_graph(&self) -> &DependencyGraph {
        &self.graph
    }

    fn swap_graph(&mut self, out_graph: &mut DependencyGraph) {
        std::mem::swap(&mut self.graph, out_graph);
    }

    fn skip_from_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.from.get_skip_nodes(&self.graph, &self.get_info())
    }

    fn skip_to_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.to.get_skip_nodes(&self.graph, &self.get_info())
    }

    fn create_editor(&self, workspace_root: &str) -> Box<dyn DepEditor> {
        Box::new(BazelDepEditor::new(&self.query, workspace_root))
    }
}
