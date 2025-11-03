use crate::reducers::reduce_context::{AddOperation, Operation};
use crate::{
    graph::NodeId,
    reducers::reduce_context::{ReduceContext, ReduceSettings},
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
        let graph = &self.ctx.graph;
        let mut candidates: Vec<Candidate> = Vec::new();

        let mut added_deps: Vec<&AddOperation> = Vec::new();
        let mut removed_deps: Vec<NodeId> = Vec::new();

        for attempt in self.ctx.get_attempts() {
            if graph.nodes[attempt.candidates.node_id]
                .props
                .t
                .is_alias_target()
            {
                added_deps.clear();
                removed_deps.clear();

                for op in &attempt.ops {
                    match op {
                        Operation::Add(add) => {
                            added_deps.push(add);
                        }
                        Operation::Remove(rm) => {
                            let filtered_added_deps: Vec<_> = added_deps
                                .iter()
                                .filter(|a| a.dependent_node_id == rm.dependent_node_id)
                                .map(|a| a.node_id)
                                .collect();
                            if !filtered_added_deps.is_empty() {
                                removed_deps.push(rm.node_id);
                                candidates.push(Candidate {
                                    node_id: rm.dependent_node_id,
                                    added_deps: filtered_added_deps,
                                    removed_deps: removed_deps.clone(),
                                });
                            }
                            added_deps.clear();
                            removed_deps.clear();
                        }
                        _ => {}
                    }
                }
            }
        }

        candidates.reverse();
        candidates
    }

    pub fn process(&mut self) {
        let &ReduceSettings { editor, .. } = self.ctx.settings;

        let candidates = self.get_candidates();

        'candidate: for candidate in &candidates {
            self.ctx.start_attempt(
                candidate.node_id,
                candidate.node_id,
                Some("Recover Alias".to_string()),
            );

            let node_label = self.ctx.graph.nodes[candidate.node_id].label.clone();
            for dep_node_id in &candidate.added_deps {
                let dep_label = self.ctx.graph.nodes[*dep_node_id].label.clone();
                if let Ok(edit) = editor.remove(&node_label, &dep_label) {
                    self.ctx.backup(&edit);
                    self.ctx.apply(edit);
                } else {
                    self.ctx.restore_backup();
                    continue 'candidate;
                }
            }

            for dep_node_id in &candidate.removed_deps {
                let dep_label = self.ctx.graph.nodes[*dep_node_id].label.clone();
                if let Ok(edit) = editor.add(&node_label, &dep_label, &node_label) {
                    self.ctx.backup(&edit);
                    self.ctx.apply(edit);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reducers::top_sort_reducer::tests::run_reducer_test;

    #[test]
    fn test_alias() {
        run_reducer_test(
            "test-alias-recovery-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/test-alias-recovery",
            "../../../examples/test-alias-recovery",
            "build.sh",
            "postprocessors/test-alias-recovery",
            |ctx| {
                let mut postprocessor = AliasTargetPostprocessor::new(ctx);
                postprocessor.process();
            },
            |_| {},
        );
    }
}
