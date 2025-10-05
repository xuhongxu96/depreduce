use std::collections::HashSet;

use serde::Deserialize;

use crate::{
    filters::{
        BuildSystemSpecificInfo, CommonFilterOptions, InternalFilterable,
        executable_rules::ExecutableRules,
    },
    graph::{DependencyGraph, NodeId},
};

struct ExecutableFilterRules {
    rule_class_rules: ExecutableRules,
    target_name_rules: ExecutableRules,
}

#[derive(Debug, Deserialize, Default)]
pub struct RuleSpecification {
    #[serde(default)]
    pub rule_classes: Vec<String>,
    #[serde(default)]
    pub target_names: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RuleBasedFilter {
    #[serde(default)]
    pub allow: RuleSpecification,

    #[serde(default)]
    pub block: RuleSpecification,

    #[serde(flatten)]
    pub options: CommonFilterOptions,
}

impl ExecutableFilterRules {
    pub fn is_match(&self, rule_class: &str, target: &str) -> bool {
        self.rule_class_rules.is_match_names(rule_class)
            || self.target_name_rules.is_match_names(target)
            || self.rule_class_rules.is_match_regexes(rule_class)
            || self.target_name_rules.is_match_regexes(target)
    }
}

impl RuleBasedFilter {
    fn to_executable_filter(
        &self,
    ) -> (Option<ExecutableFilterRules>, Option<ExecutableFilterRules>) {
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

        (allow, block)
    }
}

impl InternalFilterable for RuleBasedFilter {
    fn internal_filter(
        &self,
        graph: &DependencyGraph,
        info: &BuildSystemSpecificInfo,
    ) -> HashSet<NodeId> {
        let node_and_rule_class = match info {
            &BuildSystemSpecificInfo::Bazel(q) => q.to_node_and_rule_class(),
            &BuildSystemSpecificInfo::Buck(q) => q.to_node_and_rule_class(),
            _ => panic!("RuleBasedFilter only supports Bazel"),
        };
        let (allow, block) = self.to_executable_filter();

        let mut res = HashSet::new();

        if allow.is_none() && block.is_none() {
            return res;
        }

        for node in node_and_rule_class {
            if let Some(allow) = &allow {
                if !allow.is_match(&node.1, &node.0) {
                    res.insert(graph.get_node_id(&node.0).unwrap());
                }
            }

            if let Some(block) = &block {
                if block.is_match(&node.1, &node.0) {
                    res.insert(graph.get_node_id(&node.0).unwrap());
                }
            }
        }

        res
    }

    fn options(&self) -> &super::CommonFilterOptions {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use crate::{filters::Filterable, graph::bazel_xml_parser::parse_bazel_xml_query};

    use super::*;

    #[test]
    fn test_parse() {
        let filter = RuleBasedFilter {
            allow: RuleSpecification::default(),
            block: RuleSpecification {
                rule_classes: vec!["regex:^javadoc_".to_string(), "py_library".to_string()],
                target_names: vec!["regex:test$".to_string(), "//test:a".to_string()],
            },
            options: CommonFilterOptions::default(),
        };

        let xml = r#"
        <query version="2">
            <rule class="javadoc_library" location="x" name="//test:a" />
            <rule class="py_library" location="y" name="//test:b" />
            <rule class="java_library" location="z" name="//test:c" />
        </query>
        "#;

        let query = parse_bazel_xml_query(xml).unwrap();
        let graph = query.to_dep_graph(&HashSet::new()).unwrap();

        let info = BuildSystemSpecificInfo::Bazel(&query);
        let res = filter.filter(&graph, &info);

        assert!(res.contains(&graph.get_node_id("//test:a").unwrap()));
        assert!(res.contains(&graph.get_node_id("//test:b").unwrap()));
        assert!(!res.contains(&graph.get_node_id("//test:c").unwrap()));
    }
}
