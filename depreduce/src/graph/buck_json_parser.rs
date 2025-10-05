use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::graph::{DependencyGraph, EdgeProps, NodeProps, NodeType, TargetType};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum BuckListOrMap {
    List(Vec<String>),
    Map(serde_json::Value),
}

impl Default for BuckListOrMap {
    fn default() -> Self {
        BuckListOrMap::List(vec![])
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuckQueryTarget {
    #[serde(rename = "buck.type")]
    pub type_name: String,

    #[serde(rename = "buck.package")]
    pub package: String,

    #[serde(default)]
    pub srcs: BuckListOrMap,

    #[serde(default)]
    pub deps: BuckListOrMap,

    #[serde(default)]
    pub exported_deps: BuckListOrMap,
}

#[derive(Serialize, Debug)]
pub struct BuckQuery {
    pub query: HashMap<String, BuckQueryTarget>,
}

pub fn parse_buck_json_query(s: &str) -> BuckQuery {
    BuckQuery {
        query: serde_json::from_str(s).expect("Failed to parse buck query output as JSON"),
    }
}

impl BuckQueryTarget {
    pub fn to_buck_path(&self) -> Result<String, String> {
        if !self.package.starts_with("root//") {
            return Err(format!(
                "Unexpected package format (Should start with root//): {}",
                self.package
            ));
        }
        let res = self.package.trim_start_matches("root//").replace(":", "/");
        if res.is_empty() {
            return Err("Empty package after trimming root//".to_string());
        }

        return Ok(res);
    }

    pub fn get_src_list(&self) -> Vec<String> {
        match &self.srcs {
            BuckListOrMap::List(list) => list.clone(),
            BuckListOrMap::Map(_) => {
                vec![]
            }
        }
    }

    pub fn get_target_list(&self) -> Vec<String> {
        match &self.deps {
            BuckListOrMap::List(list) => list.clone(),
            BuckListOrMap::Map(_) => {
                vec![]
            }
        }
    }
    pub fn get_exported_target_list(&self) -> Vec<String> {
        match &self.exported_deps {
            BuckListOrMap::List(list) => list.clone(),
            BuckListOrMap::Map(_) => {
                vec![]
            }
        }
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
                        is_alias: target.get_src_list().is_empty(),
                    }),
                },
            )?;
        }

        for (name, target) in &self.query {
            let from = res.get_node_id(name).unwrap();

            for dep in &target.get_target_list() {
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
                if res
                    .add_edge(from, to, EdgeProps { unremovable: false })
                    .is_err()
                {
                    eprintln!(
                        "Warning: failed to add exported dependency {} of target {} (may be duplicate)",
                        dep, name
                    );
                }
            }
        }

        for (name, target) in &self.query {
            let from = res.get_node_id(name).unwrap();

            for dep in &target.get_exported_target_list() {
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
                if res
                    .add_edge(from, to, EdgeProps { unremovable: true })
                    .is_err()
                {
                    eprintln!(
                        "Warning: failed to add exported dependency {} of target {} (may be duplicate)",
                        dep, name
                    );
                }
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
        assert_eq!(graph.edges[0].as_ref().unwrap().from, bar_id);
        assert_eq!(graph.edges[0].as_ref().unwrap().to, baz_id);
    }

    #[test]
    fn test_to_buck_path() {
        let target = BuckQueryTarget {
            type_name: "java_library".to_string(),
            package: "root//foo:bar".to_string(),
            srcs: BuckListOrMap::List(vec!["Bar.java".to_string()]),
            deps: BuckListOrMap::List(vec!["//foo:baz".to_string()]),
            exported_deps: BuckListOrMap::List(vec![]),
        };
        let path = target.to_buck_path().unwrap();
        assert_eq!(path, "foo/bar");
    }
}
