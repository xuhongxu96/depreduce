use std::collections::{HashMap, HashSet};

use rustpython_parser::{
    Parse,
    ast::{self, Ranged},
};
use serde::Deserialize;

use crate::{
    editors::{BazelLabel, get_fn_name_and_rule_name, split_location},
    filters::{CommonFilterOptions, InternalFilterable},
    graph::{
        DependencyGraph, NodeId,
        bazel_xml_parser::{Query, SkyValue},
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct FunctionCallFilter {
    pub func: String,
    pub keys: HashSet<String>,

    #[serde(flatten)]
    pub options: CommonFilterOptions,
}

impl InternalFilterable for FunctionCallFilter {
    fn internal_filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId> {
        self.get_targets_containing_select(query, graph)
    }

    fn options(&self) -> &CommonFilterOptions {
        &self.options
    }
}

impl FunctionCallFilter {
    fn get_targets_containing_select(
        &self,
        query: &Query,
        graph: &DependencyGraph,
    ) -> HashSet<NodeId> {
        let mut res = HashSet::new();

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
                for (label, start_line) in labels.clone() {
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
                                if let Some((name, _)) = get_fn_name_and_rule_name(&e.value) {
                                    if name == BazelLabel::parse(&label).name {
                                        if self.has_called(&e.value) {
                                            if let Some(id) = graph.get_node_id(&label) {
                                                res.insert(id);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
        }

        res
    }

    fn has_called(&self, e: &ast::Expr) -> bool {
        match e {
            ast::Expr::Call(call) => {
                call.func
                    .as_name_expr()
                    .map(|name| name.id.as_str() == self.func)
                    .unwrap_or(false)
                    || call
                        .keywords
                        .iter()
                        .filter(|kw| {
                            if self.keys.is_empty() {
                                return true;
                            }

                            if let Some(key) = kw.arg.as_ref() {
                                self.keys.contains(key.as_str())
                            } else {
                                false
                            }
                        })
                        .any(|kw| self.has_called(&kw.value))
                    || call.args.iter().any(|arg| self.has_called(arg))
                    || self.has_called(&call.func)
            }
            ast::Expr::BoolOp(expr_bool_op) => {
                expr_bool_op.values.iter().any(|v| self.has_called(v))
            }
            ast::Expr::NamedExpr(expr_named_expr) => self.has_called(&expr_named_expr.value),
            ast::Expr::BinOp(expr_bin_op) => {
                self.has_called(&expr_bin_op.left) || self.has_called(&expr_bin_op.right)
            }
            ast::Expr::UnaryOp(expr_unary_op) => self.has_called(&expr_unary_op.operand),
            ast::Expr::Lambda(expr_lambda) => self.has_called(&expr_lambda.body),
            ast::Expr::IfExp(expr_if_exp) => {
                self.has_called(&expr_if_exp.test)
                    || self.has_called(&expr_if_exp.body)
                    || self.has_called(&expr_if_exp.orelse)
            }
            ast::Expr::Dict(expr_dict) => {
                expr_dict.keys.iter().flatten().any(|k| self.has_called(k))
                    || expr_dict.values.iter().any(|v| self.has_called(v))
            }
            ast::Expr::Set(expr_set) => expr_set.elts.iter().any(|e| self.has_called(e)),
            ast::Expr::ListComp(expr_list_comp) => {
                self.has_called(&expr_list_comp.elt)
                    || expr_list_comp.generators.iter().any(|g| {
                        self.has_called(&g.iter)
                            || g.ifs.iter().any(|if_expr| self.has_called(if_expr))
                    })
            }
            ast::Expr::SetComp(expr_set_comp) => {
                self.has_called(&expr_set_comp.elt)
                    || expr_set_comp.generators.iter().any(|g| {
                        self.has_called(&g.iter)
                            || g.ifs.iter().any(|if_expr| self.has_called(if_expr))
                    })
            }
            ast::Expr::DictComp(expr_dict_comp) => {
                self.has_called(&expr_dict_comp.key)
                    || self.has_called(&expr_dict_comp.value)
                    || expr_dict_comp.generators.iter().any(|g| {
                        self.has_called(&g.iter)
                            || g.ifs.iter().any(|if_expr| self.has_called(if_expr))
                    })
            }
            ast::Expr::GeneratorExp(expr_generator_exp) => {
                self.has_called(&expr_generator_exp.elt)
                    || expr_generator_exp.generators.iter().any(|g| {
                        self.has_called(&g.iter)
                            || g.ifs.iter().any(|if_expr| self.has_called(if_expr))
                    })
            }
            ast::Expr::Await(expr_await) => self.has_called(&expr_await.value),
            ast::Expr::Yield(expr_yield) => expr_yield
                .value
                .as_ref()
                .map_or(false, |v| self.has_called(v)),
            ast::Expr::YieldFrom(expr_yield_from) => self.has_called(&expr_yield_from.value),
            ast::Expr::Compare(expr_compare) => {
                self.has_called(&expr_compare.left)
                    || expr_compare.comparators.iter().any(|c| self.has_called(c))
            }
            ast::Expr::FormattedValue(expr_formatted_value) => {
                self.has_called(&expr_formatted_value.value)
            }
            ast::Expr::JoinedStr(expr_joined_str) => {
                expr_joined_str.values.iter().any(|v| self.has_called(v))
            }
            ast::Expr::Attribute(expr_attribute) => self.has_called(&expr_attribute.value),
            ast::Expr::Subscript(expr_subscript) => {
                self.has_called(&expr_subscript.value) || self.has_called(&expr_subscript.slice)
            }
            ast::Expr::Starred(expr_starred) => self.has_called(&expr_starred.value),
            ast::Expr::List(expr_list) => expr_list.elts.iter().any(|e| self.has_called(e)),
            ast::Expr::Tuple(expr_tuple) => expr_tuple.elts.iter().any(|e| self.has_called(e)),
            ast::Expr::Slice(expr_slice) => {
                expr_slice
                    .lower
                    .as_ref()
                    .map_or(false, |v| self.has_called(v))
                    || expr_slice
                        .upper
                        .as_ref()
                        .map_or(false, |v| self.has_called(v))
                    || expr_slice
                        .step
                        .as_ref()
                        .map_or(false, |v| self.has_called(v))
            }
            ast::Expr::Name(_) => false,
            ast::Expr::Constant(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use utils::{get_test_data_path, read_or_create_test_data, read_test_data};

    use crate::{
        filters::{FilterOperationScope, Filterable},
        graph::bazel_xml_parser::parse_bazel_xml,
    };

    use super::*;

    #[test]
    fn test_filter() {
        let xml = read_test_data!("filters/select.xml");
        let xml = xml.replace("/data/h445xu/", "DUMMY_PATH/").replace(
            "DUMMY_PATH/repo/trpc-cpp/trpc/overload_control/BUILD",
            get_test_data_path!("filters/select.BUILD")
                .to_str()
                .unwrap(),
        );
        let query = parse_bazel_xml(&xml).unwrap();
        let graph = query.to_dep_graph(false).unwrap();

        let filter = FunctionCallFilter {
            func: "select".to_string(),
            keys: HashSet::new(),
            options: CommonFilterOptions {
                scope: FilterOperationScope::default(),
                transitive_level: 0,
            },
        };
        let res = filter.filter(&graph, &query);
        assert_eq!(res.len(), 1);
        assert_eq!(
            graph.nodes[*res.iter().next().unwrap()].label,
            "//trpc/overload_control:overload_control_defs"
        );

        let filter = FunctionCallFilter {
            func: "select".to_string(),
            keys: HashSet::from_iter(vec!["defines".to_string()].iter().cloned()),
            options: CommonFilterOptions {
                scope: FilterOperationScope::default(),
                transitive_level: 1,
            },
        };
        let res = filter.filter(&graph, &query);
        let mut res = res
            .iter()
            .map(|id| graph.nodes[*id].label.clone())
            .collect::<Vec<_>>();
        res.sort();
        let res = res.join("\n");
        assert_eq!(
            res,
            read_or_create_test_data!("filters/transitive_results.txt", res)
        );

        let filter = FunctionCallFilter {
            func: "select".to_string(),
            keys: HashSet::from_iter(vec!["deps".to_string()].iter().cloned()),
            options: CommonFilterOptions {
                scope: FilterOperationScope::default(),
                transitive_level: 0,
            },
        };
        let res = filter.filter(&graph, &query);
        assert_eq!(res.len(), 0);
    }
}
