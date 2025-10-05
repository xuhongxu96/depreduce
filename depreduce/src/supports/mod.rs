use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::DepEditor,
    graph::DependencyGraph,
};

pub trait BuildSystemSupport {
    fn get_graph(&self) -> &DependencyGraph;
    fn swap_graph(&mut self, out_graph: &mut DependencyGraph);
    fn skip_from_node_labels(&self, config: &ReduceConfig) -> SkipNodes;
    fn skip_to_node_labels(&self, config: &ReduceConfig) -> SkipNodes;
    fn create_editor(&self, workspace_root: &str) -> Box<dyn DepEditor>;
}

mod bazel_support;
mod buck_support;

pub use bazel_support::BazelSupport;
pub use buck_support::BuckSupport;
