use serde::Deserialize;
use std::collections::HashSet;

use crate::graph::{DependencyGraph, NodeId, bazel_xml_parser::Query};

pub enum FilterOperationType {
    All,
    Add,
    Remove,
}

#[derive(Debug, Deserialize, Default)]
struct CommonFilterOptions {
    #[serde(default)]
    pub add_only: bool,

    #[serde(default)]
    pub remove_only: bool,

    #[serde(default)]
    pub transitive_level: i32,
}

trait InternalFilterable {
    fn internal_filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId>;
    fn options(&self) -> &CommonFilterOptions;
}

pub trait Filterable {
    fn filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId>;
    fn get_op_type(&self) -> FilterOperationType;
}

impl<T: InternalFilterable> Filterable for T {
    fn get_op_type(&self) -> FilterOperationType {
        if self.options().add_only {
            FilterOperationType::Add
        } else if self.options().remove_only {
            FilterOperationType::Remove
        } else {
            FilterOperationType::All
        }
    }

    fn filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId> {
        let mut nodes = self::InternalFilterable::internal_filter(self, graph, query);

        if self.options().transitive_level > 0 {
            let mut visited = HashSet::new();
            for _level in 0..self.options().transitive_level {
                let mut next_nodes = HashSet::new();
                for &node in &nodes {
                    if visited.contains(&node) {
                        continue;
                    }
                    visited.insert(node);
                    graph.node2in_edges.get(&node).map(|edges| {
                        for (from, _) in edges {
                            next_nodes.insert(from);
                        }
                    });
                }
                nodes.extend(next_nodes);
            }
        }

        nodes
    }
}

mod alias_like_filter;
mod function_call_filter;
mod rule_based_filter;

pub use alias_like_filter::AliasLikeFilter;
pub use function_call_filter::FunctionCallFilter;
pub use rule_based_filter::RuleBasedFilter;
