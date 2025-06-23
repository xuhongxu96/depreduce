use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use std::fmt::Write;
use utils::DependencyMap;

type NodeId = usize;
type EdgeId = usize;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Unknown,
    Source,
    Target,
    GeneratedFile,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NodeProps {
    pub t: NodeType,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub props: NodeProps,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProps {}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub props: EdgeProps,
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,

    #[serde(skip)]
    name2node: HashMap<String, NodeId>,

    #[serde(skip)]
    node2out_edges: HashMap<NodeId, HashMap<NodeId, EdgeId>>,

    #[serde(skip)]
    node2in_edges: HashMap<NodeId, HashMap<NodeId, EdgeId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            name2node: HashMap::new(),
            node2out_edges: HashMap::new(),
            node2in_edges: HashMap::new(),
        }
    }

    pub fn build(&mut self) {
        // Rebuild name2node
        self.name2node.clear();
        for node in &self.nodes {
            self.name2node.insert(node.label.clone(), node.id);
        }

        // Rebuild node2out_edges and node2in_edges
        self.node2out_edges.clear();
        self.node2in_edges.clear();
        for edge in &self.edges {
            self.node2out_edges
                .entry(edge.from)
                .or_default()
                .insert(edge.to, edge.id);
            self.node2in_edges
                .entry(edge.to)
                .or_default()
                .insert(edge.from, edge.id);
        }
    }

    pub fn add_node(&mut self, label: String, props: NodeProps) -> Result<NodeId, String> {
        if let Some(&id) = self.name2node.get(&label) {
            return Err(format!(
                "Node with label '{}' already exists with id {}",
                label, id
            ));
        }

        let id = self.nodes.len();
        let node = Node {
            id,
            label: label.clone(),
            props,
        };
        self.nodes.push(node);
        self.name2node.insert(label, id);
        Ok(id)
    }

    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        props: EdgeProps,
    ) -> Result<EdgeId, String> {
        if from == to {
            return Err("Cannot create an edge from a node to itself".to_string());
        }
        if from >= self.nodes.len() || to >= self.nodes.len() {
            return Err(format!(
                "Invalid node ids: from {} or to {} does not exist",
                from, to
            ));
        }
        if self
            .node2out_edges
            .get(&from)
            .map_or(false, |edges| edges.contains_key(&to))
        {
            return Err(format!(
                "Edge from node {} (Label: {}) to node {} (Label: {}) already exists",
                from, self.nodes[from].label, to, self.nodes[to].label
            ));
        }

        let id = self.edges.len();
        let edge = Edge {
            id,
            from,
            to,
            props,
        };
        self.edges.push(edge);

        self.node2out_edges.entry(from).or_default().insert(to, id);
        self.node2in_edges.entry(to).or_default().insert(from, id);
        Ok(id)
    }

    pub fn get_node_id(&self, label: &str) -> Option<NodeId> {
        self.name2node.get(label).cloned()
    }

    pub fn get_edge_id(&self, from: NodeId, to: NodeId) -> Option<EdgeId> {
        self.node2out_edges
            .get(&from)
            .and_then(|edges| edges.get(&to).cloned())
    }

    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        writeln!(dot, "digraph DependencyGraph {{").unwrap();
        for node in &self.nodes {
            writeln!(
                dot,
                "    {} [label=\"{} ({:?})\"]",
                node.id, node.label, node.props.t
            )
            .unwrap();
        }
        for edge in &self.edges {
            writeln!(
                dot,
                "    {} -> {} [label=\"{}\"]",
                edge.from, edge.to, edge.id
            )
            .unwrap();
        }
        writeln!(dot, "}}").unwrap();
        dot
    }

    pub fn to_dependency_map(&self) -> DependencyMap {
        let mut dep_map = DependencyMap::new();
        for node in &self.nodes {
            let deps = self
                .node2out_edges
                .get(&node.id)
                .map(|edges| {
                    edges
                        .keys()
                        .map(|&id| self.nodes[id].label.clone())
                        .collect()
                })
                .unwrap_or_default();
            dep_map.deps.insert(node.label.clone(), deps);
        }
        dep_map
    }
}

#[cfg(test)]
mod tests {
    use utils::read_or_create_test_data;

    use super::*;

    #[test]
    fn test_add_node_success() {
        let mut graph = DependencyGraph::new();
        let id = graph
            .add_node(
                "src/main.rs".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        assert_eq!(id, 0);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].label, "src/main.rs");
        assert_eq!(graph.nodes[0].props.t, NodeType::Source);
    }

    #[test]
    fn test_add_node_duplicate_label() {
        let mut graph = DependencyGraph::new();
        graph
            .add_node(
                "src/lib.rs".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let result = graph.add_node(
            "src/lib.rs".to_string(),
            NodeProps {
                t: NodeType::Target,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_edge_success() {
        let mut graph = DependencyGraph::new();
        let id1 = graph
            .add_node(
                "a".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id2 = graph
            .add_node(
                "b".to_string(),
                NodeProps {
                    t: NodeType::Target,
                },
            )
            .unwrap();
        let edge_id = graph
            .add_edge(id1, id2, EdgeProps {})
            .expect("Should add edge");
        assert_eq!(edge_id, 0);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, id1);
        assert_eq!(graph.edges[0].to, id2);
    }

    #[test]
    fn test_add_edge_self_loop() {
        let mut graph = DependencyGraph::new();
        let id = graph
            .add_node(
                "self".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let result = graph.add_edge(id, id, EdgeProps {});
        assert!(result.is_err());
    }

    #[test]
    fn test_add_edge_invalid_node() {
        let mut graph = DependencyGraph::new();
        let id = graph
            .add_node(
                "only".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let result = graph.add_edge(id, id + 1, EdgeProps {});
        assert!(result.is_err());
    }

    #[test]
    fn test_add_edge_duplicate() {
        let mut graph = DependencyGraph::new();
        let id1 = graph
            .add_node(
                "a".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id2 = graph
            .add_node(
                "b".to_string(),
                NodeProps {
                    t: NodeType::Target,
                },
            )
            .unwrap();
        graph.add_edge(id1, id2, EdgeProps {}).unwrap();
        // Try to add the same edge again
        let result = graph.add_edge(id1, id2, EdgeProps {});
        assert!(result.is_err());
    }

    #[test]
    fn test_get_node_id() {
        let mut graph = DependencyGraph::new();
        graph
            .add_node(
                "foo".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id = graph.get_node_id("foo");
        assert_eq!(id, Some(0));
        let none = graph.get_node_id("bar");
        assert_eq!(none, None);
    }

    #[test]
    fn test_build_rebuilds_indices() {
        let mut graph = DependencyGraph::new();
        let id1 = graph
            .add_node(
                "x".into(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id2 = graph
            .add_node(
                "y".into(),
                NodeProps {
                    t: NodeType::Target,
                },
            )
            .unwrap();
        graph.add_edge(id1, id2, EdgeProps {}).unwrap();

        // Simulate deserialization by clearing indices
        graph.name2node.clear();
        graph.node2out_edges.clear();
        graph.node2in_edges.clear();

        graph.build();

        assert_eq!(graph.get_node_id("x"), Some(id1));
        assert_eq!(graph.get_node_id("y"), Some(id2));
        assert_eq!(
            graph
                .node2out_edges
                .get(&id1)
                .map(|m| m.keys().cloned().collect::<Vec<_>>()),
            Some(vec![id2])
        );
        assert_eq!(
            graph
                .node2in_edges
                .get(&id2)
                .map(|m| m.keys().cloned().collect::<Vec<_>>()),
            Some(vec![id1])
        );
    }

    #[test]
    fn test_to_dot() {
        let mut graph = DependencyGraph::new();
        let id_foo = graph
            .add_node(
                "foo".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id_bar = graph
            .add_node(
                "bar".to_string(),
                NodeProps {
                    t: NodeType::Target,
                },
            )
            .unwrap();
        let id_baz = graph
            .add_node(
                "baz".to_string(),
                NodeProps {
                    t: NodeType::GeneratedFile,
                },
            )
            .unwrap();

        graph.add_edge(id_foo, id_bar, EdgeProps {}).unwrap();
        graph.add_edge(id_bar, id_baz, EdgeProps {}).unwrap();
        graph.add_edge(id_foo, id_baz, EdgeProps {}).unwrap();

        let res = graph.to_dot();
        assert_eq!(
            res,
            read_or_create_test_data!("dep_graph/graph/test_graph.dot", res)
        );
    }
}
