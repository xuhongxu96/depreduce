use std::collections::HashSet;

use serde::Deserialize;

use crate::{
    filters::Filterable,
    graph::{DependencyGraph, NodeId, bazel_xml_parser::Query},
};

#[derive(Debug, Deserialize, Default)]
pub struct AliasLikeFilter {}

impl Filterable for AliasLikeFilter {
    fn filter(&self, graph: &DependencyGraph, _: &Query) -> HashSet<NodeId> {
        let mut res = HashSet::new();
        for node in &graph.nodes {
            if node.props.t.is_alias_target() {
                res.insert(node.id);
            }
        }
        res
    }
}
