use super::{ReductionCandidateGenerator, ReductionCandidateGeneratorFactory};
use crate::graph::{EdgeId, NodeId};

pub struct NaiveReductionCandidateGeneratorFactory;

pub struct NaiveReductionCandidateGenerator {
    dependents: Vec<(NodeId, EdgeId)>,
    iteration: usize,
}

impl ReductionCandidateGeneratorFactory for NaiveReductionCandidateGeneratorFactory {
    fn create(&self, dependents: Vec<(NodeId, EdgeId)>) -> Box<dyn ReductionCandidateGenerator> {
        Box::new(NaiveReductionCandidateGenerator {
            dependents,
            iteration: 0,
        })
    }
}

impl ReductionCandidateGenerator for NaiveReductionCandidateGenerator {
    fn report_result(&mut self, _result: bool) {
        // no-op for now
    }

    fn next(&mut self) -> Option<Vec<(NodeId, EdgeId)>> {
        let mut res = Vec::new();

        if self.iteration >= self.dependents.len() {
            return None;
        }

        res.push(self.dependents[self.iteration]);
        self.iteration += 1;

        Some(res)
    }
}
