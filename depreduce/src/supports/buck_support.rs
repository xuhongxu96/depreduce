use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader},
    process::Command,
};

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::{BazelDepEditor, DepEditor, generate_label2location_for_buck},
    filters::BuildSystemSpecificInfo,
    graph::{
        DependencyGraph,
        buck_json_parser::{BuckQuery, parse_buck_json_query},
    },
    supports::BuildSystemSupport,
};

fn get_buck_query(buck_path: &str, workspace: &str, target: &str) -> BuckQuery {
    let mut p = Command::new(buck_path)
        .arg("uquery")
        .arg(target)
        .arg("--output-attribute")
        .arg("^(deps|exported_deps|srcs|buck.type|buck.package)$")
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
        query_res.push('\n');
        if i % 1000 == 0 {
            eprintln!("Read {} lines from buck query output...", i);
        }
    }

    p.wait().expect("Buck query did not finish successfully");

    parse_buck_json_query(&query_res)
}

pub struct BuckSupport {
    query: BuckQuery,
    graph: DependencyGraph,
}

impl BuckSupport {
    pub fn new(workspace: &str, target: &str, config: &ReduceConfig) -> Self {
        if !config.readonly_deps_attrs.is_empty() {
            eprintln!(
                "Warning: readonly_deps_attrs is currently not supported for Buck. Ignoring it."
            );
        }

        let query = get_buck_query("buck2", workspace, target);
        let graph = query
            .to_dep_graph()
            .expect("Failed to convert buck query to graph");

        Self { query, graph }
    }

    fn get_info(&self) -> BuildSystemSpecificInfo {
        BuildSystemSpecificInfo::Buck(&self.query)
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

        Box::new(BazelDepEditor::new_with_buck_mode(
            generate_label2location_for_buck(&self.query, workspace_root),
            HashMap::new(),
            workspace_root,
            keywords_for_deps_insertion,
            keywords_for_deps_removal,
            true,
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
