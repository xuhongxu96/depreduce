use crate::graph::NodeId;

pub trait ReductionCandidateGenerator {
    fn report_result(&mut self, result: bool);
    fn next(&mut self) -> Option<Vec<NodeId>>;
}

pub trait ReductionCandidateGeneratorFactory {
    fn create(&self, dependents: Vec<NodeId>) -> Box<dyn ReductionCandidateGenerator>;
}

mod naive_reduction_candidate_generator;

pub use naive_reduction_candidate_generator::*;
