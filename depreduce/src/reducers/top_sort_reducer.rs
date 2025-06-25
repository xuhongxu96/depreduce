use crate::editors::DepEditor;
use crate::graph::DependencyGraph;
use crate::graph::bazel_xml_parser::Query;

struct TopSortReducer {
    graph: DependencyGraph,
    editor: Box<dyn DepEditor>,
}

impl TopSortReducer {
    pub fn new(graph: DependencyGraph, editor: Box<dyn DepEditor>) -> Self {
        Self { graph, editor }
    }

    pub fn reduce(&self) -> Result<String, String> {
        let mut logs = String::new();

        let mut sorted_nodes = self.graph.topsort();
        sorted_nodes.reverse();

        for node_id in sorted_nodes {
            let label = &self.graph.nodes[node_id].label;

            if let Some(dependents) = self.graph.node2in_edges.get(&node_id) {
                for (dep_node_id, edge_id) in dependents {
                    let dep_label = &self.graph.nodes[*dep_node_id].label;
                    match self.editor.remove(dep_label, label) {
                        Ok(file_edit) => {
                            logs.push_str(&format!(
                                "Removed dependency from {} to {}\n",
                                dep_label, label
                            ));

                            // TODO: Apply the file edit
                        }
                        Err(err) => {
                            logs.push_str(&format!(
                                "Failed to remove dependency from {} to {}: {}\n",
                                dep_label, label, err
                            ));
                        }
                    }
                }
            } else {
                logs.push_str(&format!("No dependent for node {}\n", label));
            }
        }

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use utils::*;

    use crate::{
        editors::BazelDepEditor,
        graph::bazel_xml_parser::{convert_query_to_dep_graph, parse_bazel_xml},
    };

    use super::*;

    #[test]
    fn test_cxx() {
        let xml = read_test_data!("cxx-deps.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(
            &query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project".to_string(),
        );

        let reducer = TopSortReducer::new(graph, Box::new(editor));
        let res = reducer.reduce();
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }
}
