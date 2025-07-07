use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::Deserialize;

pub struct ExecutableRules {
    regexes: Vec<Regex>,
    names: HashSet<String>,
}

pub struct ExecutableFilterRules {
    rule_class_rules: ExecutableRules,
    target_name_rules: ExecutableRules,
}

pub struct ExecutableFilter {
    allow: Option<ExecutableFilterRules>,
    block: Option<ExecutableFilterRules>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RuleSpecification {
    #[serde(default)]
    pub rule_classes: Vec<String>,
    #[serde(default)]
    pub target_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct FilterSpecification {
    #[serde(default)]
    pub allow: RuleSpecification,
    #[serde(default)]
    pub block: RuleSpecification,
}

#[derive(Debug, Deserialize)]
pub struct ReduceConfig {
    pub from: FilterSpecification,
    pub to: FilterSpecification,
}

impl ReduceConfig {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

impl FilterSpecification {
    pub fn to_executable_filter(&self) -> ExecutableFilter {
        let allow = if !self.allow.rule_classes.is_empty() || !self.allow.target_names.is_empty() {
            Some(ExecutableFilterRules {
                rule_class_rules: ExecutableRules::parse(&self.allow.rule_classes),
                target_name_rules: ExecutableRules::parse(&self.allow.target_names),
            })
        } else {
            None
        };

        let block = if !self.block.rule_classes.is_empty() || !self.block.target_names.is_empty() {
            Some(ExecutableFilterRules {
                rule_class_rules: ExecutableRules::parse(&self.block.rule_classes),
                target_name_rules: ExecutableRules::parse(&self.block.target_names),
            })
        } else {
            None
        };

        ExecutableFilter { allow, block }
    }
}

impl ExecutableRules {
    fn parse(rules: &[String]) -> Self {
        let mut regexes = Vec::new();
        let mut names = HashSet::new();

        for rule in rules {
            if rule.starts_with("regex:") {
                if let Ok(regex) = Regex::new(&rule["regex:".len()..]) {
                    regexes.push(regex);
                }
            } else {
                names.insert(rule.clone());
            }
        }

        ExecutableRules { regexes, names }
    }
}

impl ExecutableFilterRules {
    pub fn is_match(&self, rule_class: &str, target: &str) -> bool {
        self.rule_class_rules.names.contains(rule_class)
            || self.target_name_rules.names.contains(target)
            || self
                .rule_class_rules
                .regexes
                .iter()
                .any(|rule| rule.is_match(rule_class))
            || self
                .target_name_rules
                .regexes
                .iter()
                .any(|rule| rule.is_match(target))
    }
}

pub struct NodeInfo<'a, 'b> {
    pub rule_class: &'a str,
    pub target: &'b str,
}

impl ExecutableFilter {
    pub fn get_skip_nodes<'a, 'b>(&self, nodes: &[NodeInfo<'a, 'b>]) -> HashSet<&'b str> {
        let mut skip_nodes = HashSet::new();

        if self.allow.is_none() && self.block.is_none() {
            return skip_nodes;
        }

        for node in nodes {
            if let Some(allow) = &self.allow {
                if !allow.is_match(node.rule_class, node.target) {
                    skip_nodes.insert(node.target);
                }
            }

            if let Some(block) = &self.block {
                if block.is_match(node.rule_class, node.target) {
                    skip_nodes.insert(node.target);
                }
            }
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
[from.block]
rule_classes = ['regex:^javadoc_', 'py_library']
target_names = ['regex:test$', '//test:a']

[to.block]
rule_classes = ['regex:^javadoc_', 'py_library']
target_names = ['regex:test$', '//test:a']
        "#;

        let cfg: ReduceConfig = toml::from_str(content).unwrap();
        for filter in [&cfg.from, &cfg.to] {
            let filter = filter.to_executable_filter();

            let res = filter.get_skip_nodes(&[
                NodeInfo {
                    rule_class: "javadoc_library",
                    target: "//test:a",
                },
                NodeInfo {
                    rule_class: "py_library",
                    target: "//test:b",
                },
                NodeInfo {
                    rule_class: "java_library",
                    target: "//test:c",
                },
            ]);

            assert!(res.contains("//test:a"));
            assert!(res.contains("//test:b"));
            assert!(!res.contains("//test:c"));
        }
    }
}
