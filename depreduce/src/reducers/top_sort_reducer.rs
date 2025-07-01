use std::collections::HashSet;

use clap::builder::Str;

use crate::editors::DepEditor;
use crate::graph::NodeId;
use crate::reducers::reduce_context::{ReduceContext, ReduceSettings};

pub struct TopSortReducer {}

impl TopSortReducer {
    fn try_add_transitive_deps(&self, ctx: &mut ReduceContext, node_id: NodeId) -> bool {
        let label = ctx.settings.graph.nodes[node_id].label.clone();
        ctx.log(&format!(
            "  Trying to add transitive dependencies for node {}\n",
            label
        ));

        let mut transitive_deps: HashSet<(NodeId, String)> = HashSet::new();
        if let Some(tgt2edge) = ctx.settings.graph.node2out_edges.get(&node_id) {
            tgt2edge.keys().for_each(|dep_node| {
                transitive_deps.insert((
                    *dep_node,
                    ctx.settings
                        .graph
                        .nodes
                        .get(*dep_node)
                        .unwrap()
                        .label
                        .clone(),
                ));
            });
        }

        let mut transitive_deps: Vec<_> = transitive_deps
            .into_iter()
            .map(|(id, label)| (id, label))
            .collect();
        transitive_deps.sort();

        let mut added_edges: Vec<(NodeId, NodeId)> = Vec::new();

        for dep_node in ctx.get_current_candidates().clone() {
            let dep_node_label = ctx.settings.graph.nodes[dep_node].label.clone();
            for (transitive_dep_id, transitive_dep_label) in &transitive_deps {
                if let Some(edges) = ctx.settings.graph.node2out_edges.get(&dep_node) {
                    if edges.contains_key(transitive_dep_id) {
                        ctx.log(&format!(
                            "  Skipping {} -> {} (already exists)\n",
                            dep_node_label, transitive_dep_label
                        ));
                        continue;
                    }
                }

                match ctx.settings.graph.nodes[*transitive_dep_id].props.t {
                    crate::graph::NodeType::Target(_) => {}
                    _ => {
                        ctx.log(&format!(
                            "  Skipping {} -> {} (non-target)\n",
                            dep_node_label, transitive_dep_label
                        ));
                        continue;
                    }
                }

                match ctx
                    .settings
                    .editor
                    .add(&dep_node_label, transitive_dep_label)
                {
                    Ok(edit) => {
                        ctx.backup(&edit);
                        ctx.apply(&edit);
                        ctx.log(&format!(
                            "  Added {} -> {}\n",
                            dep_node_label, transitive_dep_label
                        ));
                        added_edges.push((dep_node, *transitive_dep_id));
                    }
                    Err(e) => {
                        ctx.log(&format!(
                            "Failed to add {} -> {}: {}\n",
                            dep_node_label, transitive_dep_label, e
                        ));
                        return false;
                    }
                }
            }
        }

        if added_edges.is_empty() {
            ctx.log("  No changes made, skipping build\n");
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                for (from, to) in added_edges {
                    ctx.add_dependent(to, from);
                }
                ctx.log(&format!("  Committed changes: {}\n", status));
                return true;
            }
            Err(e) => {
                ctx.log(&format!("  {}\n", e));
                return false;
            }
        }
    }

    fn try_remove_dep(&self, ctx: &mut ReduceContext, node_id: NodeId) -> bool {
        ctx.log("  Trying a new candidate set\n");

        let mut removed_edges: Vec<(NodeId, NodeId)> = Vec::new();

        let label = ctx.settings.graph.nodes[node_id].label.clone();
        for dep_node in ctx.get_current_candidates().clone() {
            if ctx.get_indegree(dep_node) <= 0 {
                ctx.log(
                    format!(
                        "    Only consider deps for {} -> {} (because in-degree = {})\n",
                        label,
                        ctx.settings.graph.nodes[dep_node].label,
                        ctx.get_indegree(dep_node)
                    )
                    .as_str(),
                );
            }

            let dep_node_label = ctx.settings.graph.nodes[dep_node].label.clone();
            match ctx.settings.editor.remove(
                &dep_node_label,
                &label,
                ctx.get_indegree(dep_node) <= 0,
            ) {
                Ok(edit) => {
                    removed_edges.push((dep_node, node_id));
                    ctx.backup(&edit);
                    ctx.apply(&edit);
                    ctx.log(&format!("    Removed {} -> {}\n", dep_node_label, label));
                }
                Err(e) => {
                    ctx.log(&format!(
                        "    Failed to remove {} -> {}: {}\n",
                        dep_node_label, label, e
                    ));
                    continue;
                }
            }
        }

        if removed_edges.is_empty() {
            ctx.log("  No changes made, skipping build\n");
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                for (from, to) in removed_edges {
                    ctx.remove_dependent(to, from);
                }

                ctx.commit_changes();
                ctx.log(&format!("  Committed changes: {}\n\n", status));
                true
            }
            Err(e) => {
                ctx.log(&format!("  {}\n\n", e));
                if self.try_add_transitive_deps(ctx, node_id) {
                    for (from, to) in removed_edges {
                        ctx.remove_dependent(to, from);
                    }
                    ctx.commit_changes();
                    true
                } else {
                    ctx.restore_backup();
                    false
                }
            }
        }
    }

    pub fn reduce<'a>(&self, settings: &'a ReduceSettings) -> Result<ReduceContext<'a>, String> {
        let &ReduceSettings { graph, .. } = settings;

        let mut ctx = ReduceContext::new(settings);
        let sorted_nodes = graph.topsort();

        ctx.log("Sorted nodes in topological order:\n");
        for node_id in sorted_nodes.iter() {
            ctx.log(&format!("  {}\n", graph.nodes.get(*node_id).unwrap().label));
        }

        for (i, &node_id) in sorted_nodes.iter().enumerate() {
            ctx.log(&format!(
                "Processing node: {} ({}/{})\n",
                graph.nodes.get(node_id).unwrap().label,
                i + 1,
                sorted_nodes.len()
            ));

            let mut generator = ctx.generate_reduction_candidates(node_id);

            while let Some(candidates) = generator.next() {
                ctx.start_attempt(node_id, candidates.clone(), None);
                let res = self.try_remove_dep(&mut ctx, node_id);
                generator.report_result(res);
            }
        }

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use utils::*;

    use crate::{
        editors::BazelDepEditor,
        graph::bazel_xml_parser::{Query, convert_query_to_dep_graph, parse_bazel_xml},
        reducers::candidate_generators::NaiveReductionCandidateGeneratorFactory,
    };

    use super::*;

    fn run_reducer_test(
        xml_file: &str,
        workspace_root: &str,
        project_dir: &str,
        build_script: &str,
        expected_out: &str,
    ) {
        let xml = read_test_data!(xml_file);
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(&query, workspace_root.to_string());

        let reducer = TopSortReducer {};
        let settings = ReduceSettings {
            editor: &editor,
            reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
            graph: &graph,
            build_command: get_test_data_path!(build_script)
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!(project_dir)
                .to_string_lossy()
                .to_string(),
            save_build_log: false,
        };
        let ctx = reducer.reduce(&settings).unwrap();
        let attempts = ctx.get_attempts();
        let res = to_json_lines(attempts);
        assert_eq!(
            res,
            read_or_create_test_data!(format!("{}{}", expected_out, ".ops.jsonl"), res)
        );

        let graph_json = serde_json::to_string(&graph).expect("Failed to serialize graph to JSON");
        assert_eq!(
            graph_json,
            read_or_create_test_data!(format!("{}{}", expected_out, ".graph.json"), graph_json)
        );
    }

    #[test]
    fn test_cxx() {
        run_reducer_test(
            "cxx-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
            "../../../examples/simple-cxx-project",
            "build.sh",
            "reducers/cxx",
        );
    }

    #[test]
    fn test_java() {
        run_reducer_test(
            "java-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project",
            "../../../examples/simple-java-project",
            "build.sh",
            "reducers/java",
        );
    }

    #[test]
    fn test_kotlin() {
        run_reducer_test(
            "kt-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-kotlin-project",
            "../../../examples/simple-kotlin-project",
            "build.sh",
            "reducers/kt",
        );
    }

    #[test]
    fn test_kotlin_transitive() {
        // What we want to test it whether the reducer can add deps of deps correctly, and
        // it also needs to deduplicate the added deps because main already depends on a.
        // See examples/kotlin-transitive/README.md for details.
        run_reducer_test(
            "kt-transitive-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/kotlin-transitive",
            "../../../examples/kotlin-transitive",
            "build.sh",
            "reducers/kt-transitive",
        );
    }
}
