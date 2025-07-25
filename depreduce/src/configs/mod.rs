use std::collections::HashSet;

use regex::Regex;
use serde::Deserialize;

use crate::{
    filters::*,
    graph::{DependencyGraph, bazel_xml_parser::Query},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum FilterType {
    Rule(RuleBasedFilter),
    FunctionCall(FunctionCallFilter),
    Alias(AliasLikeFilter),
}

#[derive(Debug, Deserialize)]
pub struct FilterSpecification {
    #[serde(default)]
    pub filters: Vec<FilterType>,
}

#[derive(Debug, Deserialize)]
pub struct ReduceConfig {
    pub from: FilterSpecification,
    pub to: FilterSpecification,
}

impl FilterType {
    fn to_filterable<'a>(&'a self) -> &'a dyn Filterable {
        match &self {
            FilterType::Rule(f) => f,
            FilterType::FunctionCall(f) => f,
            FilterType::Alias(f) => f,
        }
    }
}

impl ReduceConfig {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

pub struct NodeInfo<'a, 'b> {
    pub rule_class: &'a str,
    pub target: &'b str,
}

impl FilterSpecification {
    pub fn get_skip_nodes<'a, 'b, 'c: 'b>(
        &self,
        graph: &'c DependencyGraph,
        query: &Query,
    ) -> HashSet<&'b str> {
        let mut skip_nodes = HashSet::new();

        for filter in &self.filters {
            let nodes = filter.to_filterable().filter(graph, query);
            skip_nodes.extend(
                nodes
                    .iter()
                    .map(|&id| graph.nodes[id].label.as_str())
                    .collect::<HashSet<_>>(),
            );
        }

        skip_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let content = r#"
[[from.filters]]
type = "FunctionCall"
func = "select"
keys = ["deps"]

[[from.filters]]
type = "FunctionCall"
func = "select"
keys = ["defines"]

[[to.filters]]
type = "FunctionCall"
func = "select"
keys = ["defines"]
        "#;
        let cfg: ReduceConfig = toml::from_str(content).unwrap();
        assert_eq!(cfg.from.filters.len(), 2);
        assert_eq!(cfg.to.filters.len(), 1);
    }
}
