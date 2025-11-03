use std::collections::HashSet;

use serde::Deserialize;

use crate::{filters::*, graph::DependencyGraph};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum FilterType {
    Rule(RuleBasedFilter),
    FunctionCall(FunctionCallFilter),
    Alias(AliasLikeFilter),
    AttrRule(AttrRuleBasedFilter),
}

#[derive(Debug, Deserialize, Default)]
pub struct FilterSpecification {
    #[serde(default)]
    pub filters: Vec<FilterType>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ReduceConfig {
    #[serde(default)]
    pub from: FilterSpecification,

    #[serde(default)]
    pub to: FilterSpecification,

    #[serde(default)]
    pub timeout_seconds: u64,

    /// Bazel-specific
    #[serde(default = "HashSet::new")]
    pub readonly_deps_attrs: HashSet<String>,

    /// Rust-specific
    #[serde(default)]
    pub reduce_dev_deps: bool,
}

impl FilterType {
    fn to_filterable<'a>(&'a self) -> &'a dyn Filterable {
        match &self {
            FilterType::Rule(f) => f,
            FilterType::FunctionCall(f) => f,
            FilterType::Alias(f) => f,
            FilterType::AttrRule(f) => f,
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

pub struct SkipNodes<'a> {
    pub for_addition: HashSet<&'a str>,
    pub for_removal: HashSet<&'a str>,
}

impl FilterSpecification {
    pub fn get_skip_nodes<'a, 'b, 'c: 'b>(
        &self,
        graph: &'c DependencyGraph,
        info: &BuildSystemSpecificInfo,
    ) -> SkipNodes<'b> {
        let mut for_addition = HashSet::new();
        let mut for_removal = HashSet::new();

        for filter in &self.filters {
            let filter = filter.to_filterable();
            let nodes = filter.filter(graph, info);
            match filter.get_op_type() {
                FilterOperationScope::Add => {
                    for_addition.extend(nodes.iter().map(|&id| graph.nodes[id].label.as_str()));
                }
                FilterOperationScope::Remove => {
                    for_removal.extend(nodes.iter().map(|&id| graph.nodes[id].label.as_str()));
                }
                FilterOperationScope::All => {
                    for_addition.extend(nodes.iter().map(|&id| graph.nodes[id].label.as_str()));
                    for_removal.extend(nodes.iter().map(|&id| graph.nodes[id].label.as_str()));
                }
            }
        }

        SkipNodes {
            for_addition,
            for_removal,
        }
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
