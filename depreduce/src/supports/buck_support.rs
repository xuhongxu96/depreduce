use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader},
    process::Command,
};

use serde::Deserialize;

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::{BazelDepEditor, DepEditor},
    filters::BuildSystemSpecificInfo,
    graph::DependencyGraph,
    supports::BuildSystemSupport,
};

#[derive(Deserialize, Debug)]
struct BuckQueryOutput {
    #[serde(rename = "buck.type")]
    type_name: String,

    #[serde(rename = "buck.package")]
    package: String,

    #[serde(default)]
    srcs: Vec<String>,

    #[serde(default)]
    deps: Vec<String>,
}

fn get_buck_query(
    buck_path: &str,
    workspace: &str,
    target: &str,
) -> HashMap<String, BuckQueryOutput> {
    let mut p = Command::new(buck_path)
        .arg("query")
        .arg(target)
        .arg("--output-attribute")
        .arg("^(deps|srcs|buck.type|buck.package)$")
        .arg("--output-format")
        .arg("json")
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run buck query");

    let mut query_res = String::new();
    let stdout = p.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for (i, line) in stdout_lines.enumerate() {
        let line = line.expect("Failed to read line from buck query output");

        query_res.push_str(&line);
        if i % 1000 == 0 {
            eprintln!("Read {} lines from buck query output...", i);
        }
    }

    p.wait().expect("Buck query did not finish successfully");

    let map: HashMap<String, BuckQueryOutput> =
        serde_json::from_str(&query_res).expect("Failed to parse buck query output as JSON");
    map
}

pub struct BuckSupport {
    graph: DependencyGraph,
}

impl BuckSupport {
    pub fn new(workspace: &str, target: &str, config: &ReduceConfig) -> Self {
        Self {
            graph: todo!("Implement BuckSupport::new"),
        }
    }

    fn get_info(&self) -> BuildSystemSpecificInfo {
        BuildSystemSpecificInfo::Buck()
    }
}

impl BuildSystemSupport for BuckSupport {
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
        let keywords_for_deps_insertion = HashSet::from(["deps".to_string()]);
        let keywords_for_deps_removal = HashSet::from(["deps".to_string()]);

        Box::new(BazelDepEditor::new_with_label2location(
            todo!(),
            workspace_root,
            keywords_for_deps_insertion,
            keywords_for_deps_removal,
        ))
    }
}

#[cfg(test)]
mod tests {
    use utils::get_test_data_path;

    use super::*;

    #[test]
    fn test_buck_query() {
        let workspace = get_test_data_path!("../../../examples/buck-rust");
        let target = "//...";

        let output = get_buck_query("buck2", workspace.to_str().unwrap(), target);

        println!("Buck query output: {:#?}", output);
    }
}
