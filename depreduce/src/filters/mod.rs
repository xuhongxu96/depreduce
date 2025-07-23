use std::collections::HashSet;

use crate::graph::{DependencyGraph, NodeId, bazel_xml_parser::Query};

pub trait Filterable {
    fn filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId>;
}

mod function_call_filter;
pub use function_call_filter::FunctionCallFilter;
