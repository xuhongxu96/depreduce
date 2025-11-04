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

pub fn create_support(
    build_system: &str,
    workspace: &str,
    target: &str,
    config: &ReduceConfig,
) -> Result<Box<dyn BuildSystemSupport>, String> {
    match build_system {
        "buck" => Ok(Box::new(BuckSupport::new(workspace, target, config))),
        "bazel" => Ok(Box::new(BazelSupport::new(workspace, target, config))),
        "rust" | "cargo" => Ok(Box::new(CargoSupport::new(workspace, target, config))),
        _ => Err(format!("Unsupported build system: {}", build_system)),
    }
}

mod bazel_support;
mod buck_support;
mod cargo_support;

pub use bazel_support::BazelSupport;
pub use buck_support::BuckSupport;
pub use cargo_support::CargoSupport;
