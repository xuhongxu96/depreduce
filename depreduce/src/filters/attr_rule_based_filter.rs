use std::collections::HashSet;

use regex::Regex;
use serde::Deserialize;

use crate::{
    filters::{
        CommonFilterOptions, Filterable, InternalFilterable, executable_rules::ExecutableRules,
    },
    graph::{
        DependencyGraph, NodeId,
        bazel_xml_parser::{Query, SkyValue, VariantProp},
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct AttrRuleBasedFilter {
    #[serde(default)]
    pub attrs: HashSet<String>,

    #[serde(default)]
    pub rules: Vec<String>,

    #[serde(flatten)]
    pub options: CommonFilterOptions,
}

impl InternalFilterable for AttrRuleBasedFilter {
    fn internal_filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId> {
        let mut res = HashSet::new();
        let rules = ExecutableRules::parse(&self.rules);

        'outer_loop: for value in &query.values {
            match value {
                SkyValue::Rule(rule) => {
                    let node_id = graph.get_node_id(&rule.name).unwrap();
                    if let Some(props) = &rule.props {
                        for prop in props {
                            match prop {
                                VariantProp::List(list)
                                    if list
                                        .name
                                        .as_ref()
                                        .map_or(false, |name| self.attrs.contains(name)) =>
                                {
                                    if let Some(items) = &list.items {
                                        for item in items {
                                            if let VariantProp::Label(label) = item {
                                                let dep_name = label.value.as_ref().unwrap();
                                                if rules.is_match(dep_name) {
                                                    res.insert(node_id);
                                                    continue 'outer_loop;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
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
    use crate::graph::bazel_xml_parser::parse_bazel_xml;

    use super::*;

    #[test]
    fn test_parse() {
        let filter = AttrRuleBasedFilter {
            attrs: HashSet::from_iter(["srcs".to_string()]),
            rules: vec!["regex:\\.d\\.ts$".to_string()],
            options: CommonFilterOptions::default(),
        };

        let xml = r#"
        <query version="2">
            <rule class="ts_project" location="BUILD" name="//a">
                <string name="name" value="a"/>
                <list name="srcs">
                    <label value="//src:a.ts"/>
                </list>
            </rule>
            <rule class="ts_project" location="BUILD" name="//a_type">
                <string name="name" value="a_type"/>
                <list name="srcs">
                    <label value="//src:a.d.ts"/>
                </list>
            </rule>
        </query>
        "#;

        let query = parse_bazel_xml(xml).unwrap();
        let graph = query.to_dep_graph(false, &HashSet::new()).unwrap();

        let res = filter.filter(&graph, &query);

        assert!(!res.contains(&graph.get_node_id("//a").unwrap()));
        assert!(res.contains(&graph.get_node_id("//a_type").unwrap()));
    }
}
