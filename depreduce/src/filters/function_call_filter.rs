use std::collections::{HashMap, HashSet};

use rustpython_parser::{
    Parse,
    ast::{self, Ranged},
};

use crate::{
    editors::split_location,
    filters::Filterable,
    graph::{
        DependencyGraph, NodeId,
        bazel_xml_parser::{Query, SkyValue},
    },
};

fn has_called(e: &ast::Expr, func_id: &str) -> bool {
    match e {
        ast::Expr::Call(call) => {
            call.func
                .as_name_expr()
                .map(|name| name.id.as_str() == func_id)
                .unwrap_or(false)
                || call
                    .keywords
                    .iter()
                    .any(|kw| has_called(&kw.value, func_id))
                || call.args.iter().any(|arg| has_called(&arg, func_id))
                || has_called(&call.func, func_id)
        }
        ast::Expr::BoolOp(expr_bool_op) => {
            expr_bool_op.values.iter().any(|v| has_called(v, func_id))
        }
        ast::Expr::NamedExpr(expr_named_expr) => has_called(&expr_named_expr.value, func_id),
        ast::Expr::BinOp(expr_bin_op) => {
            has_called(&expr_bin_op.left, func_id) || has_called(&expr_bin_op.right, func_id)
        }
        ast::Expr::UnaryOp(expr_unary_op) => has_called(&expr_unary_op.operand, func_id),
        ast::Expr::Lambda(expr_lambda) => has_called(&expr_lambda.body, func_id),
        ast::Expr::IfExp(expr_if_exp) => {
            has_called(&expr_if_exp.test, func_id)
                || has_called(&expr_if_exp.body, func_id)
                || has_called(&expr_if_exp.orelse, func_id)
        }
        ast::Expr::Dict(expr_dict) => {
            expr_dict
                .keys
                .iter()
                .flatten()
                .any(|k| has_called(k, func_id))
                || expr_dict.values.iter().any(|v| has_called(v, func_id))
        }
        ast::Expr::Set(expr_set) => expr_set.elts.iter().any(|e| has_called(e, func_id)),
        ast::Expr::ListComp(expr_list_comp) => {
            has_called(&expr_list_comp.elt, func_id)
                || expr_list_comp.generators.iter().any(|g| {
                    has_called(&g.iter, func_id)
                        || g.ifs.iter().any(|if_expr| has_called(if_expr, func_id))
                })
        }
        ast::Expr::SetComp(expr_set_comp) => {
            has_called(&expr_set_comp.elt, func_id)
                || expr_set_comp.generators.iter().any(|g| {
                    has_called(&g.iter, func_id)
                        || g.ifs.iter().any(|if_expr| has_called(if_expr, func_id))
                })
        }
        ast::Expr::DictComp(expr_dict_comp) => {
            has_called(&expr_dict_comp.key, func_id)
                || has_called(&expr_dict_comp.value, func_id)
                || expr_dict_comp.generators.iter().any(|g| {
                    has_called(&g.iter, func_id)
                        || g.ifs.iter().any(|if_expr| has_called(if_expr, func_id))
                })
        }
        ast::Expr::GeneratorExp(expr_generator_exp) => {
            has_called(&expr_generator_exp.elt, func_id)
                || expr_generator_exp.generators.iter().any(|g| {
                    has_called(&g.iter, func_id)
                        || g.ifs.iter().any(|if_expr| has_called(if_expr, func_id))
                })
        }
        ast::Expr::Await(expr_await) => has_called(&expr_await.value, func_id),
        ast::Expr::Yield(expr_yield) => expr_yield
            .value
            .as_ref()
            .map_or(false, |v| has_called(v, func_id)),
        ast::Expr::YieldFrom(expr_yield_from) => has_called(&expr_yield_from.value, func_id),
        ast::Expr::Compare(expr_compare) => {
            has_called(&expr_compare.left, func_id)
                || expr_compare
                    .comparators
                    .iter()
                    .any(|c| has_called(c, func_id))
        }
        ast::Expr::FormattedValue(expr_formatted_value) => {
            has_called(&expr_formatted_value.value, func_id)
        }
        ast::Expr::JoinedStr(expr_joined_str) => expr_joined_str
            .values
            .iter()
            .any(|v| has_called(v, func_id)),
        ast::Expr::Attribute(expr_attribute) => has_called(&expr_attribute.value, func_id),
        ast::Expr::Subscript(expr_subscript) => {
            has_called(&expr_subscript.value, func_id) || has_called(&expr_subscript.slice, func_id)
        }
        ast::Expr::Starred(expr_starred) => has_called(&expr_starred.value, func_id),
        ast::Expr::List(expr_list) => expr_list.elts.iter().any(|e| has_called(e, func_id)),
        ast::Expr::Tuple(expr_tuple) => expr_tuple.elts.iter().any(|e| has_called(e, func_id)),
        ast::Expr::Slice(expr_slice) => {
            expr_slice
                .lower
                .as_ref()
                .map_or(false, |v| has_called(v, func_id))
                || expr_slice
                    .upper
                    .as_ref()
                    .map_or(false, |v| has_called(v, func_id))
                || expr_slice
                    .step
                    .as_ref()
                    .map_or(false, |v| has_called(v, func_id))
        }
        ast::Expr::Name(expr_name) => false,
        ast::Expr::Constant(_) => false,
    }
}

fn get_targets_containing_select(
    query: &Query,
    graph: &DependencyGraph,
    func_id: &str,
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
                            if has_called(&e.value, func_id) {
                                if let Some(id) = graph.get_node_id(&label) {
                                    res.insert(id);
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

pub struct FunctionCallFilter {
    pub func_id: String,
}

impl Filterable for FunctionCallFilter {
    fn filter(&self, graph: &DependencyGraph, query: &Query) -> HashSet<NodeId> {
        get_targets_containing_select(query, graph, &self.func_id)
    }
}

#[cfg(test)]
mod tests {
    use utils::{get_test_data_path, read_test_data};

    use crate::graph::bazel_xml_parser::parse_bazel_xml;

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
            func_id: "select".to_string(),
        };
        let res = filter.filter(&graph, &query);
        assert_eq!(res.len(), 1);
        assert_eq!(
            graph.nodes[*res.iter().next().unwrap()].label,
            "//trpc/overload_control:overload_control_defs"
        );
    }
}
