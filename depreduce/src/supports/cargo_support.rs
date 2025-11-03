use cargo_metadata::{Metadata, MetadataCommand};

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::DepEditor,
    filters::BuildSystemSpecificInfo,
    graph::{DependencyGraph, EdgeProps, NodeProps, NodeType, TargetType},
    supports::BuildSystemSupport,
};

pub struct CargoSupport {
    metadata: Metadata,
    graph: DependencyGraph,
}

fn get_metadata(workspace: &str) -> (Metadata, DependencyGraph) {
    let metadata = MetadataCommand::new()
        .manifest_path(format!("{}/Cargo.toml", workspace))
        .exec()
        .expect("Failed to get cargo metadata");

    let mut graph = DependencyGraph::new();

    let nodes = &metadata.resolve.as_ref().unwrap().nodes;
    nodes.iter().for_each(|node| {
        graph
            .add_node(
                node.id.repr.clone(),
                NodeProps {
                    t: NodeType::Target(TargetType { is_alias: false }),
                },
            )
            .unwrap();
    });

    nodes.iter().for_each(|node| {
        node.deps.iter().for_each(|dep| {
            graph
                .add_edge(
                    graph.get_node_id(&node.id.repr).unwrap(),
                    graph.get_node_id(&dep.pkg.repr).unwrap(),
                    EdgeProps { unremovable: false },
                )
                .unwrap();
        });
    });

    (metadata, graph)
}

impl CargoSupport {
    pub fn new(workspace: &str, target: &str, config: &ReduceConfig) -> Self {
        if !config.readonly_deps_attrs.is_empty() {
            eprintln!(
                "Warning: readonly_deps_attrs is currently not supported for Cargo. Ignoring it."
            );
        }

        let (metadata, graph) = get_metadata(workspace);

        Self { metadata, graph }
    }

    fn get_info(&self) -> BuildSystemSpecificInfo {
        BuildSystemSpecificInfo::Cargo()
    }
}

impl BuildSystemSupport for CargoSupport {
    fn get_graph(&self) -> &DependencyGraph {
        &self.graph
    }

    fn swap_graph(&mut self, out_graph: &mut DependencyGraph) {
        std::mem::swap(&mut self.graph, out_graph);
    }

    fn skip_from_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.from.get_skip_nodes(&self.graph, &self.get_info())
    }

    fn skip_to_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.to.get_skip_nodes(&self.graph, &self.get_info())
    }

    fn create_editor(&self, workspace_root: &str) -> Box<dyn DepEditor> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use utils::get_test_data_path;

    use super::*;

    #[test]
    fn test_get_metadata() {
        let workspace = get_test_data_path!("../../../examples/simple-rust-project");
        let (metadata, graph) = get_metadata(workspace.to_str().unwrap());

        let mut name2id = HashMap::new();

        metadata.packages.iter().for_each(|pkg| {
            name2id.insert(pkg.name.clone(), pkg.id.repr.clone());
        });

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 3);

        let liba_id = graph.get_node_id(name2id.get("liba").unwrap()).unwrap();
        let libb_id = graph.get_node_id(name2id.get("libb").unwrap()).unwrap();
        let main_id = graph.get_node_id(name2id.get("main").unwrap()).unwrap();

        assert_eq!(
            graph.get_out_edges(liba_id).map(|e| e.len()).unwrap_or(0),
            0
        );
        assert_eq!(graph.get_out_edges(libb_id).unwrap().len(), 1);
        assert_eq!(graph.get_out_edges(main_id).unwrap().len(), 2);
    }
}
