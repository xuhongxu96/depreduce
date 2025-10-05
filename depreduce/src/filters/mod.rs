use serde::Deserialize;
use std::collections::HashSet;

use crate::graph::{
    DependencyGraph, NodeId, bazel_xml_parser::BazelQuery, buck_json_parser::BuckQuery,
};

#[derive(Debug, Deserialize, Default)]
pub enum FilterOperationScope {
    #[default]
    All,
    Add,
    Remove,
}

#[derive(Debug, Deserialize, Default)]
pub struct CommonFilterOptions {
    #[serde(default)]
    pub scope: FilterOperationScope,

    #[serde(default)]
    pub transitive_level: i32,
}

pub enum BuildSystemSpecificInfo<'a> {
    Bazel(&'a BazelQuery),
    Buck(&'a BuckQuery),
    Cargo(),
}

trait InternalFilterable {
    fn internal_filter(
        &self,
        graph: &DependencyGraph,
        info: &BuildSystemSpecificInfo,
    ) -> HashSet<NodeId>;
    fn options(&self) -> &CommonFilterOptions;
}

pub trait Filterable {
    fn filter(&self, graph: &DependencyGraph, info: &BuildSystemSpecificInfo) -> HashSet<NodeId>;
    fn get_op_type(&self) -> &FilterOperationScope;
}

impl<T: InternalFilterable> Filterable for T {
    fn get_op_type(&self) -> &FilterOperationScope {
        &self.options().scope
    }

    fn filter(&self, graph: &DependencyGraph, info: &BuildSystemSpecificInfo) -> HashSet<NodeId> {
        let mut nodes = self::InternalFilterable::internal_filter(self, graph, info);

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

mod executable_rules;

mod alias_like_filter;
mod attr_rule_based_filter;
mod function_call_filter;
mod rule_based_filter;

pub use alias_like_filter::AliasLikeFilter;
pub use attr_rule_based_filter::AttrRuleBasedFilter;
pub use function_call_filter::FunctionCallFilter;
pub use rule_based_filter::RuleBasedFilter;
