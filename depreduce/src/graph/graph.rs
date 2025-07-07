use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use std::fmt::Write;
use utils::DependencyMap;

pub type NodeId = usize;
pub type EdgeId = usize;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TargetType {
    pub is_alias: bool,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Unknown,
    Source,
    Target(TargetType),
    GeneratedFile,
}

impl NodeType {
    pub fn is_alias_target(&self) -> bool {
        matches!(self, NodeType::Target(TargetType { is_alias: true }))
    }
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
    pub nodes: Vec<Node>,
    pub edges: Vec<Option<Edge>>,

    #[serde(skip)]
    pub name2node: HashMap<String, NodeId>,

    #[serde(skip)]
    pub node2out_edges: HashMap<NodeId, HashMap<NodeId, EdgeId>>,

    #[serde(skip)]
    pub node2in_edges: HashMap<NodeId, HashMap<NodeId, EdgeId>>,
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
            if let Some(edge) = edge {
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
        self.edges.push(Some(edge));

        self.node2out_edges.entry(from).or_default().insert(to, id);
        self.node2in_edges.entry(to).or_default().insert(from, id);
        Ok(id)
    }

    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), String> {
        if edge_id >= self.edges.len() {
            return Err(format!("Edge with id {} does not exist", edge_id));
        }

        let edge = self.edges[edge_id].as_ref().unwrap();
        if let Some(edges) = self.node2out_edges.get_mut(&edge.from) {
            edges.remove(&edge.to);
        }
        if let Some(edges) = self.node2in_edges.get_mut(&edge.to) {
            edges.remove(&edge.from);
        }

        self.edges[edge_id] = None;
        Ok(())
    }

    pub fn get_node_id(&self, label: &str) -> Option<NodeId> {
        self.name2node.get(label).cloned()
    }

    pub fn get_edge_id(&self, from: NodeId, to: NodeId) -> Option<EdgeId> {
        self.node2out_edges
            .get(&from)
            .and_then(|edges| edges.get(&to).cloned())
    }

    pub fn get_in_edges(&self, node_id: NodeId) -> Option<&HashMap<NodeId, EdgeId>> {
        self.node2in_edges.get(&node_id)
    }

    pub fn get_out_edges(&self, node_id: NodeId) -> Option<&HashMap<NodeId, EdgeId>> {
        self.node2out_edges.get(&node_id)
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
            if let Some(edge) = edge {
                writeln!(
                    dot,
                    "    {} -> {} [label=\"{}\"]",
                    edge.from, edge.to, edge.id
                )
                .unwrap();
            }
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

    pub fn topsort(&self) -> Vec<NodeId> {
        let mut visited = vec![false; self.nodes.len()];
        let mut stack = Vec::new();
        let mut result = Vec::new();

        fn visit(
            graph: &DependencyGraph,
            node_id: NodeId,
            visited: &mut [bool],
            stack: &mut Vec<NodeId>,
            result: &mut Vec<NodeId>,
        ) {
            if visited[node_id] {
                return;
            }
            visited[node_id] = true;

            if let Some(edges) = graph.node2out_edges.get(&node_id) {
                let mut edges: Vec<_> = edges.keys().collect();
                edges.sort();
                for &neighbor in edges {
                    visit(graph, neighbor, visited, stack, result);
                }
            }

            stack.push(node_id);
        }

        for node in &self.nodes {
            visit(self, node.id, &mut visited, &mut stack, &mut result);
        }

        while let Some(node_id) = stack.pop() {
            result.push(node_id);
        }

        result
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
                t: NodeType::Target(TargetType { is_alias: false }),
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
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
        let edge_id = graph
            .add_edge(id1, id2, EdgeProps {})
            .expect("Should add edge");
        assert_eq!(edge_id, 0);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].as_ref().unwrap().from, id1);
        assert_eq!(graph.edges[0].as_ref().unwrap().to, id2);
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
                    t: NodeType::Target(TargetType { is_alias: false }),
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
                    t: NodeType::Target(TargetType { is_alias: false }),
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
                    t: NodeType::Target(TargetType { is_alias: false }),
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

    #[test]
    fn test_topsort_linear_chain() {
        let mut graph = DependencyGraph::new();
        let id1 = graph
            .add_node(
                "n1".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id2 = graph
            .add_node(
                "n2".to_string(),
                NodeProps {
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
        let id3 = graph
            .add_node(
                "n3".to_string(),
                NodeProps {
                    t: NodeType::GeneratedFile,
                },
            )
            .unwrap();

        graph.add_edge(id1, id2, EdgeProps {}).unwrap();
        graph.add_edge(id2, id3, EdgeProps {}).unwrap();

        let order = graph.topsort();
        let pos = |id| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(id1) < pos(id2));
        assert!(pos(id2) < pos(id3));
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_topsort_disconnected_graph() {
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
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
        let id3 = graph
            .add_node(
                "c".to_string(),
                NodeProps {
                    t: NodeType::GeneratedFile,
                },
            )
            .unwrap();

        // No edges
        let order = graph.topsort();
        assert_eq!(order.len(), 3);
        assert!(order.contains(&id1));
        assert!(order.contains(&id2));
        assert!(order.contains(&id3));
    }

    #[test]
    fn test_topsort_branching() {
        let mut graph = DependencyGraph::new();
        let id_a = graph
            .add_node(
                "a".to_string(),
                NodeProps {
                    t: NodeType::Source,
                },
            )
            .unwrap();
        let id_b = graph
            .add_node(
                "b".to_string(),
                NodeProps {
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
        let id_c = graph
            .add_node(
                "c".to_string(),
                NodeProps {
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
        let id_d = graph
            .add_node(
                "d".to_string(),
                NodeProps {
                    t: NodeType::GeneratedFile,
                },
            )
            .unwrap();

        // a -> b, a -> c, b -> d, c -> d
        graph.add_edge(id_a, id_b, EdgeProps {}).unwrap();
        graph.add_edge(id_a, id_c, EdgeProps {}).unwrap();
        graph.add_edge(id_b, id_d, EdgeProps {}).unwrap();
        graph.add_edge(id_c, id_d, EdgeProps {}).unwrap();

        let order = graph.topsort();
        let pos = |id| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(id_a) < pos(id_b));
        assert!(pos(id_a) < pos(id_c));
        assert!(pos(id_b) < pos(id_d));
        assert!(pos(id_c) < pos(id_d));
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn test_topsort_single_node() {
        let mut graph = DependencyGraph::new();
        let id = graph
            .add_node(
                "only".to_string(),
                NodeProps {
                    t: NodeType::Unknown,
                },
            )
            .unwrap();
        let order = graph.topsort();
        assert_eq!(order, vec![id]);
    }

    #[test]
    fn test_topsort_no_nodes() {
        let graph = DependencyGraph::new();
        let order = graph.topsort();
        assert!(order.is_empty());
    }
}
