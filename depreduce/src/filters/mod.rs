use std::collections::HashMap;

use rustpython_parser::{
    Parse,
    ast::{self, Ranged},
};

use crate::{
    editors::split_location,
    graph::{
        DependencyGraph, NodeId,
        bazel_xml_parser::{Query, SkyValue},
    },
};

fn get_targets_containing_select(query: &Query, graph: &DependencyGraph) -> Vec<NodeId> {
    let mut res = vec![];

    let mut path2labels: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for value in &query.values {
        match value {
            SkyValue::Rule(rule) => {
                let (path, start_line, _end_col) = split_location(&rule.location);
                path2labels
                    .entry(path)
                    .or_default()
                    .push((rule.name.clone(), start_line));
            }
            _ => {}
        }
    }

    for (path, mut labels) in path2labels {
        if let Some(build_content) = std::fs::read_to_string(&path).ok() {
            let ast = rustpython_parser::ast::Suite::parse(&build_content, &path).unwrap();
            let mut ast_i = 0;
            labels.sort_by_key(|(_, start_line)| *start_line);
            for (label, start_line) in labels {
                let start_offset = build_content
                    .lines()
                    .take(start_line - 1)
                    .map(|s| s.len() + 1)
                    .sum::<usize>();

                while ast_i < ast.len() {
                    let stmt = &ast[ast_i];
                    if stmt.range().start().to_usize() < start_offset {
                        ast_i += 1;
                        continue;
                    }
                    match stmt {
                        ast::Stmt::Expr(e) => {
                            if format!("{:?}", e).contains("Identifier(\"select\")") {
                                if let Some(id) = graph.get_node_id(&label) {
                                    res.push(id);
                                }
                            }
                        }
                        _ => {}
                    }
                    ast_i += 1;
                    break;
                }
            }
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use utils::{get_test_data_path, read_test_data};

    use crate::graph::bazel_xml_parser::parse_bazel_xml;

    use super::*;

    #[test]
    fn test_get_targets_containing_select() {
        let xml = read_test_data!("filters/select.xml");
        let xml = xml.replace("/data/h445xu/", "DUMMY_PATH/").replace(
            "DUMMY_PATH/repo/trpc-cpp/trpc/overload_control/BUILD",
            get_test_data_path!("filters/select.BUILD")
                .to_str()
                .unwrap(),
        );
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = query.to_dep_graph(false).unwrap();

        let res = get_targets_containing_select(&query, &graph);
        assert_eq!(res.len(), 1);
        assert_eq!(
            graph.nodes[res[0]].label,
            "//trpc/overload_control:overload_control_defs"
        );
    }
}
