use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::graph::{DependencyGraph, EdgeProps, NodeProps, NodeType, TargetType};

#[derive(Serialize, Deserialize, Debug)]
pub struct BuckQueryTarget {
    #[serde(rename = "buck.type")]
    type_name: String,

    #[serde(rename = "buck.package")]
    package: String,

    #[serde(default)]
    srcs: Vec<String>,

    #[serde(default)]
    deps: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct BuckQuery {
    query: HashMap<String, BuckQueryTarget>,
}

pub fn parse_buck_json_query(s: &str) -> BuckQuery {
    BuckQuery {
        query: serde_json::from_str(s).expect("Failed to parse buck query output as JSON"),
    }
}

impl BuckQuery {
    pub fn to_dep_graph(&self) -> Result<DependencyGraph, String> {
        let mut res = DependencyGraph::new();
        for (name, target) in &self.query {
            res.add_node(
                name.clone(),
                NodeProps {
                    t: NodeType::Target(TargetType {
                        is_alias: target.srcs.is_empty(),
                    }),
                },
            )?;
        }

        for (name, target) in &self.query {
            let from = res.get_node_id(name).unwrap();

            for dep in &target.deps {
                let to = match res.get_node_id(dep) {
                    Some(id) => id,
                    None => {
                        eprintln!(
                            "Warning: dependency {} of target {} not found in graph",
                            dep, name
                        );
                        continue;
                    }
                };
                res.add_edge(from, to, EdgeProps { unremovable: false })?;
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_buck_json_query() {
        let json_str = r#"
        {
            "//foo:bar": {
                "buck.type": "java_library",
                "buck.package": "foo",
                "srcs": ["Bar.java"],
                "deps": ["//foo:baz"]
            },
            "//foo:baz": {
                "buck.type": "java_library",
                "buck.package": "foo",
                "srcs": ["Baz.java"],
                "deps": []
            }
        }
        "#;

        let query = parse_buck_json_query(json_str);
        assert_eq!(query.query.len(), 2);
        assert!(query.query.contains_key("//foo:bar"));
        assert!(query.query.contains_key("//foo:baz"));

        let graph = query.to_dep_graph().unwrap();
        assert_eq!(graph.nodes.len(), 2);
        let bar_id = graph.get_node_id("//foo:bar").unwrap();
        let baz_id = graph.get_node_id("//foo:baz").unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[bar_id].as_ref().unwrap().from, bar_id);
        assert_eq!(graph.edges[bar_id].as_ref().unwrap().to, baz_id);
    }
}
