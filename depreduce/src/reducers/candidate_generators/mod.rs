use crate::graph::{EdgeId, NodeId};

pub struct ReductionCandidateGenerator {
    dependents: Vec<(NodeId, EdgeId)>,
    iteration: usize,
}

impl ReductionCandidateGenerator {
    pub fn new(dependents: Vec<(NodeId, EdgeId)>) -> Self {
        Self {
            dependents,
            iteration: 0,
        }
    }

    pub fn report_result(&mut self, result: bool) {
        // no-op for now
    }

    pub fn next(&mut self) -> Option<Vec<(NodeId, EdgeId)>> {
        let mut res = Vec::new();

        if self.iteration >= self.dependents.len() {
            return None;
        }

        res.push(self.dependents[self.iteration]);
        self.iteration += 1;

        Some(res)
    }
}
