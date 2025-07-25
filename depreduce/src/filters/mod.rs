use std::collections::HashSet;

use crate::graph::{DependencyGraph, NodeId, bazel_xml_parser::Query};

pub trait Filterable {
    fn filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId>;
}

mod alias_like_filter;
mod function_call_filter;
mod rule_based_filter;

pub use alias_like_filter::AliasLikeFilter;
pub use function_call_filter::FunctionCallFilter;
pub use rule_based_filter::RuleBasedFilter;
