use std::collections::HashSet;

use crate::graph::{DependencyGraph, NodeId};

pub struct RebuildCostCalculator<'a> {
    graph: &'a DependencyGraph,

    nodes_to_rebuild_cache: Vec<Option<HashSet<NodeId>>>,
}

impl<'a> RebuildCostCalculator<'a> {
    pub fn new(graph: &'a DependencyGraph) -> Self {
        RebuildCostCalculator {
            graph,
            nodes_to_rebuild_cache: vec![None; graph.nodes.len()],
        }
    }

    fn cache_nodes_to_rebuild(&mut self, node_id: NodeId, visited_nodes: &mut HashSet<NodeId>) {
        let mut nodes_to_rebuild = HashSet::new();
        visited_nodes.insert(node_id);

        if self.nodes_to_rebuild_cache[node_id].is_none() {
            if let Some(dependencies) = self.graph.get_in_edges(node_id) {
                for (dependent_node_id, _) in dependencies {
                    if visited_nodes.contains(dependent_node_id) {
                        continue;
                    }
                    nodes_to_rebuild.insert(*dependent_node_id);
                    self.cache_nodes_to_rebuild(*dependent_node_id, visited_nodes);
                    nodes_to_rebuild.extend(
                        self.nodes_to_rebuild_cache[*dependent_node_id]
                            .as_ref()
                            .unwrap()
                            .iter(),
                    );
                }
            }
            self.nodes_to_rebuild_cache[node_id] = Some(nodes_to_rebuild);
        }

        visited_nodes.remove(&node_id);
    }

    pub fn compute_rebuild_set(&mut self, node_id: NodeId) -> &HashSet<NodeId> {
        self.cache_nodes_to_rebuild(node_id, &mut HashSet::new());

        self.nodes_to_rebuild_cache[node_id].as_ref().unwrap()
    }

    pub fn calculate_rebuild_cost(&mut self, node_id: NodeId) -> usize {
        self.compute_rebuild_set(node_id).len()
    }

    pub fn calculate_rebuild_cost_sum(&mut self) -> usize {
        let mut total_cost = 0;

        let sorted_nodes = self.graph.topsort();
        for node_id in sorted_nodes {
            total_cost += self.calculate_rebuild_cost(node_id);
        }

        total_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::bazel_xml_parser::parse_bazel_xml_query;
    use utils::*;

    #[test]
    fn test_rebuild_cost_calculator_cxx() {
        let xml = read_test_data!("cxx-deps.xml");
        let query = parse_bazel_xml_query(&xml).unwrap();
        let graph = query.to_dep_graph(&HashSet::new()).unwrap();
        let mut calculator = RebuildCostCalculator::new(&graph);
        let a_cpp_node_id = graph.get_node_id("//liba:liba").unwrap();
        assert_eq!(2, calculator.calculate_rebuild_cost(a_cpp_node_id));

        let mut rebuild_nodes: Vec<_> = calculator.nodes_to_rebuild_cache[a_cpp_node_id]
            .as_ref()
            .unwrap()
            .iter()
            .copied()
            .collect();
        rebuild_nodes.sort();
        let mut expected_nodes = vec![
            graph.get_node_id("//libb:libb").unwrap(),
            graph.get_node_id("//main:main").unwrap(),
        ];
        expected_nodes.sort();
        assert_eq!(rebuild_nodes, expected_nodes);
        println!(
            "{:?}",
            calculator
                .nodes_to_rebuild_cache
                .iter()
                .enumerate()
                .map(|(src_node_id, opt_set)| {
                    opt_set
                        .as_ref()
                        .map(|set| {
                            set.iter()
                                .map(|node_id| {
                                    (
                                        graph.nodes[*node_id].label.clone(),
                                        graph.nodes[src_node_id].label.clone(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        );

        assert_eq!(6, calculator.calculate_rebuild_cost_sum());
    }

    #[test]
    fn test_rebuild_cost_calculator_cxx_optimized() {
        let xml = read_test_data!("cxx-deps-optimized.xml");
        let query = parse_bazel_xml_query(&xml).unwrap();
        let graph = query.to_dep_graph(&HashSet::new()).unwrap();
        let mut calculator = RebuildCostCalculator::new(&graph);
        let a_cpp_node_id = graph.get_node_id("//liba:liba").unwrap();
        assert_eq!(1, calculator.calculate_rebuild_cost(a_cpp_node_id));

        let mut rebuild_nodes: Vec<_> = calculator.nodes_to_rebuild_cache[a_cpp_node_id]
            .as_ref()
            .unwrap()
            .iter()
            .copied()
            .collect();
        rebuild_nodes.sort();
        let mut expected_nodes = vec![graph.get_node_id("//main:main").unwrap()];
        expected_nodes.sort();
        assert_eq!(rebuild_nodes, expected_nodes);

        println!(
            "{:?}",
            calculator
                .nodes_to_rebuild_cache
                .iter()
                .enumerate()
                .map(|(src_node_id, opt_set)| {
                    opt_set
                        .as_ref()
                        .map(|set| {
                            set.iter()
                                .map(|node_id| {
                                    (
                                        graph.nodes[*node_id].label.clone(),
                                        graph.nodes[src_node_id].label.clone(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        );

        assert_eq!(4, calculator.calculate_rebuild_cost_sum());
    }
}
