use std::collections::HashSet;


use crate::editors::DepEditor;
use crate::graph::NodeId;
use crate::reducers::reduce_context::{ReduceContext, ReduceSettings};

pub struct TopSortReducer {}

impl TopSortReducer {
    /// Attempts to lift dependencies for the given node.
    ///
    /// It will add the node as a dependency to dependents of the candidates.
    fn try_lift_deps(
        &self,
        ctx: &mut ReduceContext,
        node_id: NodeId,
        dependent_node_id: NodeId,
    ) -> bool {
        let label = ctx.settings.graph.nodes[node_id].label.clone();
        let dependent_label = ctx.settings.graph.nodes[dependent_node_id].label.clone();

        ctx.log(&format!(
            "  Trying to lift dependency node {} to {}\n",
            label, dependent_label
        ));

        let mut lifted_edges: Vec<(NodeId, NodeId)> = Vec::new();
        if let Some(in_edges) = ctx.settings.graph.node2in_edges.get(&dependent_node_id) {
            let mut dependent_of_dependents: Vec<_> = in_edges
                .iter()
                .map(|(dependent_of_dependent, _)| dependent_of_dependent)
                .collect();
            dependent_of_dependents.sort();

            for &dependent_of_dependent in dependent_of_dependents {
                let dependent_of_dependent_label = ctx.settings.graph.nodes[dependent_of_dependent]
                    .label
                    .clone();

                if let Some(edges) = ctx
                    .settings
                    .graph
                    .node2out_edges
                    .get(&dependent_of_dependent)
                {
                    if edges.contains_key(&node_id) {
                        ctx.log(&format!(
                            "  Skipping {} -> {} (already exists)\n",
                            dependent_of_dependent_label, label
                        ));
                        continue;
                    }
                }

                match ctx
                    .settings
                    .editor
                    .add(&dependent_of_dependent_label, &label)
                {
                    Ok(edit) => {
                        lifted_edges.push((dependent_of_dependent, node_id));
                        ctx.backup(&edit);
                        ctx.apply(&edit);
                        ctx.log(&format!(
                            "    Lifted {} -> {}\n",
                            dependent_of_dependent_label, label
                        ));
                    }
                    Err(e) => {
                        ctx.log(&format!(
                            "    Failed to lift {} -> {}: {}\n",
                            dependent_of_dependent_label, label, e
                        ));
                    }
                }
            }
        } else {
            ctx.log(&format!(
                "  No in-edges for {} -> {}, skipping lift\n",
                dependent_label, label
            ));
            return false;
        }

        if lifted_edges.is_empty() {
            ctx.log("  No changes made, skipping build\n");
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                for (from, to) in lifted_edges {
                    ctx.add_dependent(to, from);
                }
                ctx.log(&format!("  Committed changes: {}\n", status));
                true
            }
            Err(e) => {
                ctx.log(&format!("  {}\n", e));
                false
            }
        }
    }

    /// Attempts to add transitive dependencies for the given node.
    ///
    /// It will add dependencies of the node to the candidates.
    fn try_flatten_deps(
        &self,
        ctx: &mut ReduceContext,
        node_id: NodeId,
        dependent_node_id: NodeId,
    ) -> bool {
        let label = ctx.settings.graph.nodes[node_id].label.clone();
        ctx.log(&format!(
            "  Trying to flatten dependencies for node {}\n",
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

        let dependent_label = ctx.settings.graph.nodes[dependent_node_id].label.clone();
        for (transitive_dep_id, transitive_dep_label) in &transitive_deps {
            if let Some(edges) = ctx.settings.graph.node2out_edges.get(&dependent_node_id) {
                if edges.contains_key(transitive_dep_id) {
                    ctx.log(&format!(
                        "  Skipping {} -> {} (already exists)\n",
                        dependent_label, transitive_dep_label
                    ));
                    continue;
                }
            }

            match ctx.settings.graph.nodes[*transitive_dep_id].props.t {
                crate::graph::NodeType::Target(_) => {}
                _ => {
                    ctx.log(&format!(
                        "  Skipping {} -> {} (non-target)\n",
                        dependent_label, transitive_dep_label
                    ));
                    continue;
                }
            }

            match ctx
                .settings
                .editor
                .add(&dependent_label, transitive_dep_label)
            {
                Ok(edit) => {
                    ctx.backup(&edit);
                    ctx.apply(&edit);
                    ctx.log(&format!(
                        "  Added {} -> {}\n",
                        dependent_label, transitive_dep_label
                    ));
                    added_edges.push((dependent_node_id, *transitive_dep_id));
                }
                Err(e) => {
                    ctx.log(&format!(
                        "Failed to add {} -> {}: {}\n",
                        dependent_label, transitive_dep_label, e
                    ));
                    return false;
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

    fn try_remove_dep(
        &self,
        ctx: &mut ReduceContext,
        node_id: NodeId,
        dependent_node_id: NodeId,
    ) -> bool {
        ctx.log("  Trying a new candidate set\n");

        let mut removed = false;

        let label = ctx.settings.graph.nodes[node_id].label.clone();
        let dependent_label = ctx.settings.graph.nodes[dependent_node_id].label.clone();

        if ctx.get_indegree(dependent_node_id) <= 0 {
            ctx.log(
                format!(
                    "    Only consider deps for {} -> {} (because in-degree = {})\n",
                    dependent_label,
                    label,
                    ctx.get_indegree(dependent_node_id),
                )
                .as_str(),
            );
        }

        match ctx.settings.editor.remove(
            &dependent_label,
            &label,
            ctx.get_indegree(dependent_node_id) <= 0,
        ) {
            Ok(edit) => {
                removed = true;
                ctx.backup(&edit);
                ctx.apply(&edit);
                ctx.log(&format!("    Removed {} -> {}\n", dependent_label, label));
            }
            Err(e) => {
                ctx.log(&format!(
                    "    Failed to remove {} -> {}: {}\n",
                    dependent_label, label, e
                ));
            }
        }

        if !removed {
            ctx.log("  No changes made, skipping build\n");
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                ctx.remove_dependent(node_id, dependent_node_id);
                ctx.commit_changes();
                ctx.log(&format!("  Committed changes: {}\n\n", status));
                true
            }
            Err(e) => {
                ctx.log(&format!("  {}\n\n", e));
                if !ctx.settings.disable_dependency_lifting
                    && self.try_lift_deps(ctx, node_id, dependent_node_id)
                {
                    ctx.remove_dependent(node_id, dependent_node_id);
                    ctx.commit_changes();
                    true
                } else if !ctx.settings.disable_dependency_flattening
                    && self.try_flatten_deps(ctx, node_id, dependent_node_id)
                {
                    ctx.remove_dependent(node_id, dependent_node_id);
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

        let sorted_nodes: Vec<NodeId> = if !settings.disable_topological_sorting {
            graph.topsort()
        } else {
            ctx.log("Topological sorting is disabled, using original order.\n");
            (0..graph.nodes.len()).collect()
        };

        ctx.init_node2topsort_index(&sorted_nodes);

        ctx.log("Nodes:\n");
        for (i, node_id) in sorted_nodes.iter().enumerate() {
            ctx.log(&format!(
                "  {}: \t{}\n",
                i,
                graph.nodes.get(*node_id).unwrap().label
            ));
        }

        for (i, &node_id) in sorted_nodes.iter().enumerate() {
            ctx.log(&format!(
                "Processing node: {} ({}/{})\n",
                graph.nodes.get(node_id).unwrap().label,
                i + 1,
                sorted_nodes.len()
            ));

            ctx.generate_reduction_candidates(node_id);

            while let Some(dependent_node_id) = ctx.next_attempt(None) {
                self.try_remove_dep(&mut ctx, node_id, dependent_node_id);
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
            graph: &graph,
            build_command: get_test_data_path!(build_script)
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!(project_dir)
                .to_string_lossy()
                .to_string(),
            save_build_log: false,
            disable_dependency_flattening: false,
            disable_dependency_lifting: false,
            disable_topological_sorting: false,
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

    #[test]
    fn test_lifting_deps() {
        run_reducer_test(
            "test-lifting-deps-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/test-lifting-deps",
            "../../../examples/test-lifting-deps",
            "build.sh",
            "reducers/test-lifting-deps",
        );
    }
}
