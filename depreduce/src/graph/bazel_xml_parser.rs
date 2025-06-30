use serde::Deserialize;

use crate::graph::graph::{DependencyGraph, EdgeProps, NodeProps, NodeType};

#[derive(Debug, Deserialize)]
pub struct VisibilityLabel {
    #[serde(rename = "@name")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SourceFile {
    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "visibility-label")]
    pub visibility: Option<Vec<VisibilityLabel>>,
}

#[derive(Debug, Deserialize)]
pub struct StringProp {
    #[serde(rename = "@name")]
    pub name: Option<String>,

    #[serde(rename = "@value")]
    pub value: Option<String>,
}

// #[derive(Debug, Deserialize)]
// pub enum ListItem {
//     #[serde(rename = "string")]
//     String(StringProp),

//     #[serde(rename = "label")]
//     Label(StringProp),

//     #[serde(rename = "output")]
//     Output(StringProp),
// }

#[derive(Debug, Deserialize)]
pub struct ListProp {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "$value")]
    pub items: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct Pair {
    #[serde(rename = "$value")]
    pub items: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct DictProp {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "pair")]
    pub pairs: Option<Vec<Pair>>,
}

#[derive(Debug, Deserialize)]
pub struct RuleIO {
    #[serde(rename = "@name")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub enum VariantProp {
    #[serde(rename = "string")]
    String(StringProp),

    #[serde(rename = "label")]
    Label(StringProp),

    #[serde(rename = "output")]
    Output(StringProp),

    #[serde(rename = "list")]
    List(ListProp),

    #[serde(rename = "dict")]
    Dict(DictProp),

    #[serde(rename = "boolean")]
    Boolean(StringProp),

    #[serde(rename = "int")]
    Int(StringProp),

    #[serde(rename = "tristate")]
    TriState(StringProp),

    #[serde(rename = "rule-input")]
    RuleInput(RuleIO),

    #[serde(rename = "rule-output")]
    RuleOutput(RuleIO),
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(rename = "@class")]
    pub class: String,

    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "$value")]
    pub props: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedFile {
    #[serde(rename = "@generating-rule")]
    pub generating_rule: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "@location")]
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct PackageGroup {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "list")]
    pub list_props: Option<Vec<ListProp>>,
}

#[derive(Debug, Deserialize)]
pub enum SkyValue {
    #[serde(rename = "source-file")]
    SourceFile(SourceFile),

    #[serde(rename = "rule")]
    Rule(Rule),

    #[serde(rename = "generated-file")]
    GeneratedFile(GeneratedFile),

    #[serde(rename = "package-group")]
    PackageGroup(PackageGroup),
}

#[derive(Debug, Deserialize)]
pub struct Query {
    #[serde(rename = "@version")]
    pub version: i32,

    #[serde(rename = "$value")]
    pub values: Vec<SkyValue>,
}

pub fn parse_bazel_xml(xml: &str) -> Result<Query, quick_xml::de::DeError> {
    quick_xml::de::from_str(xml)
}

fn is_alias_like_target(rule: &crate::graph::bazel_xml_parser::Rule) -> bool {
    use crate::graph::bazel_xml_parser::VariantProp;

    if let Some(props) = &rule.props {
        for prop in props {
            match prop {
                VariantProp::List(list_prop) => {
                    if list_prop.name == "srcs" {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    true
}

pub fn convert_query_to_dep_graph(query: &Query) -> Result<DependencyGraph, String> {
    let mut graph = DependencyGraph::new();

    for value in &query.values {
        match value {
            SkyValue::SourceFile(source_file) => {
                graph.add_node(
                    source_file.name.clone(),
                    NodeProps {
                        t: NodeType::Source,
                    },
                )?;
            }
            SkyValue::Rule(rule) => {
                graph.add_node(
                    rule.name.clone(),
                    NodeProps {
                        t: NodeType::Target(crate::graph::graph::TargetType {
                            is_alias: rule.class == "alias" || is_alias_like_target(rule),
                        }),
                    },
                )?;
            }
            SkyValue::GeneratedFile(generated_file) => {
                graph.add_node(
                    generated_file.name.clone(),
                    NodeProps {
                        t: NodeType::GeneratedFile,
                    },
                )?;
            }
            SkyValue::PackageGroup(_package_group) => {}
        }
    }

    for value in &query.values {
        match value {
            SkyValue::Rule(rule) => {
                let node_id = graph.get_node_id(&rule.name).unwrap();
                if let Some(props) = &rule.props {
                    for prop in props {
                        match prop {
                            VariantProp::RuleInput(rule_io) => {
                                let tgt = graph.get_node_id(&rule_io.name).unwrap_or_else(|| {
                                    graph
                                        .add_node(
                                            rule_io.name.clone(),
                                            NodeProps {
                                                t: NodeType::Unknown,
                                            },
                                        )
                                        .unwrap()
                                });

                                if graph.get_edge_id(node_id, tgt).is_none() {
                                    graph.add_edge(node_id, tgt, EdgeProps {})?;
                                }
                            }
                            VariantProp::RuleOutput(rule_io) => {
                                let src = graph.get_node_id(&rule_io.name).unwrap_or_else(|| {
                                    graph
                                        .add_node(
                                            rule_io.name.clone(),
                                            NodeProps {
                                                t: NodeType::Unknown,
                                            },
                                        )
                                        .unwrap()
                                });

                                if graph.get_edge_id(src, node_id).is_none() {
                                    graph.add_edge(src, node_id, EdgeProps {})?;
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

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::*;

    fn run_parse_test(input_path: &str, output_path: &str) {
        let xml = read_test_data!(input_path);

        let value: Query = parse_bazel_xml(&xml).unwrap();
        let res = format!("{:#?}", value);

        assert_eq!(res, read_or_create_test_data!(output_path, res));
    }

    #[test]
    fn test_parse_source_file() {
        run_parse_test("cxx-deps.xml", "dep_graph/bazel_xml_parser/cxx.out");
    }

    #[test]
    fn test_parse_source_file_java() {
        run_parse_test("java-deps.xml", "dep_graph/bazel_xml_parser/java.out");
    }

    #[test]
    fn test_parse_source_file_kt() {
        run_parse_test("kt-deps.xml", "dep_graph/bazel_xml_parser/kt.out");
    }

    #[test]
    fn test_parse_source_file_multi_lang() {
        run_parse_test(
            "multi-lang-deps.xml",
            "dep_graph/bazel_xml_parser/multi-lang.out",
        );
    }

    #[test]
    fn test_parse_source_file_multi_platform() {
        run_parse_test(
            "multi-platform-deps.xml",
            "dep_graph/bazel_xml_parser/multi-platform.out",
        );
    }

    #[test]
    fn test_parse_source_file_perses() {
        run_parse_test("perses.xml", "dep_graph/bazel_xml_parser/perses.out");
    }

    #[test]
    fn test_convert_query_to_dep_graph_cxx() {
        let xml = read_test_data!("cxx-deps.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();

        let res = graph.to_dot();
        assert_eq!(
            res,
            read_or_create_test_data!("dep_graph/bazel_xml_parser/cxx_graph.out", res)
        );
    }

    #[test]
    fn test_convert_query_to_dep_graph_perses() {
        let xml = read_test_data!("perses.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();

        let res = to_json_lines(&graph.to_dependency_map().to_sorted_vec());
        assert_eq!(
            res,
            read_or_create_test_data!("dep_graph/bazel_xml_parser/perses_graph.out", res)
        );
    }
}
