use utils::get_bazel_query;

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::{BazelDepEditor, DepEditor},
    filters::BuildSystemSpecificInfo,
    graph::{
        DependencyGraph,
        bazel_xml_parser::{Query, parse_bazel_xml},
    },
    supports::BuildSystemSupport,
};

pub struct BazelSupport {
    query: Query,
    graph: DependencyGraph,
}

impl BazelSupport {
    pub fn new(workspace: &str, target: &str, config: &ReduceConfig) -> Self {
        let query_xml = get_bazel_query(workspace, target);
        let query = parse_bazel_xml(&query_xml).unwrap();
        let graph = query.to_dep_graph(&config.readonly_deps_attrs).unwrap();

        BazelSupport { query, graph }
    }

    fn get_info(&self) -> BuildSystemSpecificInfo {
        BuildSystemSpecificInfo::Bazel(&self.query)
    }
}

impl BuildSystemSupport for BazelSupport {
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
        Box::new(BazelDepEditor::new(&self.query, workspace_root))
    }
}
