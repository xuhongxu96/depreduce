use std::collections::HashSet;

use crate::reducers::reduce_context::Operation;
use crate::{
    graph::{NodeId, bazel_xml_parser::Query},
    reducers::reduce_context::{ReduceContext, ReduceSettings, ReductionAttempt},
};

pub struct AliasTargetPostprocessor<'a, 'b> {
    ctx: &'b mut ReduceContext<'a>,
}

#[derive(Debug)]
struct Candidate {
    node_id: NodeId,
    added_deps: Vec<NodeId>,
    removed_deps: Vec<NodeId>,
}

impl<'a, 'b> AliasTargetPostprocessor<'a, 'b> {
    pub fn new(ctx: &'b mut ReduceContext<'a>) -> Self {
        AliasTargetPostprocessor { ctx }
    }

    fn get_candidates(&self) -> Vec<Candidate> {
        let graph = self.ctx.settings.graph;
        let mut candidates: Vec<Candidate> = Vec::new();

        let mut prev_node_id: Option<NodeId> = None;
        let mut added_deps: Vec<NodeId> = Vec::new();
        let mut removed_deps: Vec<NodeId> = Vec::new();

        for attempt in self.ctx.get_attempts() {
            if graph.nodes[attempt.candidates.node_id]
                .props
                .t
                .is_alias_target()
            {
                added_deps.clear();
                removed_deps.clear();
                prev_node_id = None;

                for op in &attempt.ops {
                    match op {
                        Operation::Add(add) => {
                            if let Some(prev_id) = prev_node_id {
                                assert_eq!(prev_id, add.dependent_node_id);
                            }
                            added_deps.push(add.node_id);
                            prev_node_id = Some(add.dependent_node_id);
                        }
                        Operation::Remove(rm) => {
                            if let Some(prev_id) = prev_node_id {
                                assert_eq!(prev_id, rm.dependent_node_id);
                            }
                            removed_deps.push(rm.node_id);
                            if !added_deps.is_empty() {
                                candidates.push(Candidate {
                                    node_id: rm.dependent_node_id,
                                    added_deps: added_deps.clone(),
                                    removed_deps: removed_deps.clone(),
                                });
                            }
                            added_deps.clear();
                            removed_deps.clear();
                            prev_node_id = None;
                        }
                        _ => {}
                    }
                }
            }
        }

        candidates
    }

    pub fn process(&mut self) {
        let &ReduceSettings { graph, editor, .. } = self.ctx.settings;

        let candidates = self.get_candidates();

        'candidate: for candidate in &candidates {
            self.ctx.start_attempt(
                candidate.node_id,
                candidate.node_id,
                Some("Recover Alias".to_string()),
            );

            let node_label = graph.nodes[candidate.node_id].label.clone();
            for dep_node_id in &candidate.added_deps {
                let dep_label = graph.nodes[*dep_node_id].label.clone();
                if let Ok(edit) = editor.remove(&node_label, &dep_label, true) {
                    self.ctx.backup(&edit);
                    self.ctx.apply(&edit);
                } else {
                    self.ctx.restore_backup();
                    continue 'candidate;
                }
            }

            for dep_node_id in &candidate.removed_deps {
                let dep_label = graph.nodes[*dep_node_id].label.clone();
                if let Ok(edit) = editor.add(&node_label, &dep_label) {
                    self.ctx.backup(&edit);
                    self.ctx.apply(&edit);
                } else {
                    self.ctx.restore_backup();
                    continue 'candidate;
                }
            }

            match self.ctx.try_build() {
                Ok(status) => {
                    self.ctx.commit_changes();
                    self.ctx
                        .log(&format!("  Committed changes: {}\n\n", status));
                }
                Err(e) => {
                    self.ctx.log(&format!("  {}\n\n", e));
                    self.ctx.restore_backup();
                }
            }
        }

        println!("{:#?}", candidates);
    }
}

#[cfg(test)]
mod tests {
    use utils::*;

    use crate::{
        editors::BazelDepEditor,
        graph::bazel_xml_parser::{Query, convert_query_to_dep_graph, parse_bazel_xml},
        reducers::top_sort_reducer::TopSortReducer,
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
        let mut ctx = reducer.reduce(&settings).unwrap();

        let mut postprocessor = AliasTargetPostprocessor::new(&mut ctx);
        postprocessor.process();

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
    fn test_alias() {
        run_reducer_test(
            "test-alias-recovery-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/test-alias-recovery",
            "../../../examples/test-alias-recovery",
            "build.sh",
            "postprocessors/test-alias-recovery",
        );
    }
}
