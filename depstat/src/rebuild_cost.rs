use std::collections::{HashMap, HashSet};

use depreduce::graph::{DependencyGraph, NodeId};

struct RebuildCostCalculator<'a> {
    graph: &'a DependencyGraph,

    nodes_to_rebuild_cache: HashMap<NodeId, HashSet<NodeId>>,
}

impl<'a> RebuildCostCalculator<'a> {
    pub fn new(graph: &'a DependencyGraph) -> Self {
        RebuildCostCalculator {
            graph,
            nodes_to_rebuild_cache: HashMap::new(),
        }
    }

    fn cache_nodes_to_rebuild(&mut self, node_id: NodeId) {
        let mut nodes_to_rebuild = HashSet::new();

        if !nodes_to_rebuild.contains(&node_id) {
            if let Some(dependencies) = self.graph.get_in_edges(node_id) {
                for (dependent_node_id, _) in dependencies {
                    nodes_to_rebuild.insert(*dependent_node_id);
                    self.cache_nodes_to_rebuild(*dependent_node_id);
                    nodes_to_rebuild.extend(
                        self.nodes_to_rebuild_cache
                            .get(dependent_node_id)
                            .unwrap()
                            .iter(),
                    );
                }
            }
        }

        self.nodes_to_rebuild_cache
            .insert(node_id, nodes_to_rebuild);
    }

    pub fn calculate_rebuild_cost(&mut self, node_id: NodeId) -> usize {
        self.cache_nodes_to_rebuild(node_id);

        let nodes_to_rebuild = self.nodes_to_rebuild_cache.get(&node_id).unwrap();
        nodes_to_rebuild.len()
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
    use depreduce::graph::bazel_xml_parser::{convert_query_to_dep_graph, parse_bazel_xml};
    use utils::*;

    #[test]
    fn test_rebuild_cost_calculator_cxx() {
        let xml = read_test_data!("cxx-deps.xml");
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query, false).unwrap();
        let mut calculator = RebuildCostCalculator::new(&graph);
        let a_cpp_node_id = graph.get_node_id("//liba:a.cpp").unwrap();
        assert_eq!(5, calculator.calculate_rebuild_cost(a_cpp_node_id));

        let mut rebuild_nodes: Vec<_> = calculator
            .nodes_to_rebuild_cache
            .get(&a_cpp_node_id)
            .unwrap()
            .iter()
            .copied()
            .collect();
        rebuild_nodes.sort();
        let mut expected_nodes = vec![
            graph.get_node_id("//liba:liba").unwrap(),
            graph.get_node_id("//libb:libb").unwrap(),
            graph.get_node_id("//main:main").unwrap(),
            graph.get_node_id("//main:main.stripped").unwrap(),
            graph.get_node_id("//main:main.dwp").unwrap(),
        ];
        expected_nodes.sort();
        assert_eq!(rebuild_nodes, expected_nodes);

        assert_eq!(37, calculator.calculate_rebuild_cost_sum());
    }

    #[test]
    fn test_rebuild_cost_calculator_cxx_optimized() {
        let xml = read_test_data!("cxx-deps-optimized.xml");
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query, false).unwrap();
        let mut calculator = RebuildCostCalculator::new(&graph);
        let a_cpp_node_id = graph.get_node_id("//liba:a.cpp").unwrap();
        assert_eq!(4, calculator.calculate_rebuild_cost(a_cpp_node_id));

        let mut rebuild_nodes: Vec<_> = calculator
            .nodes_to_rebuild_cache
            .get(&a_cpp_node_id)
            .unwrap()
            .iter()
            .copied()
            .collect();
        rebuild_nodes.sort();
        let mut expected_nodes = vec![
            graph.get_node_id("//liba:liba").unwrap(),
            graph.get_node_id("//main:main").unwrap(),
            graph.get_node_id("//main:main.stripped").unwrap(),
            graph.get_node_id("//main:main.dwp").unwrap(),
        ];
        expected_nodes.sort();
        assert_eq!(rebuild_nodes, expected_nodes);

        assert_eq!(25, calculator.calculate_rebuild_cost_sum());
    }
}
