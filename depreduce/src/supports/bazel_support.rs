use utils::get_bazel_query;

use crate::{
    configs::{ReduceConfig, SkipNodes},
    editors::BazelDepEditor,
    graph::{
        DependencyGraph,
        bazel_xml_parser::{Query, parse_bazel_xml},
    },
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

    pub fn get_query(&self) -> &Query {
        &self.query
    }

    pub fn get_graph(&self) -> &DependencyGraph {
        &self.graph
    }

    pub fn move_out_graph(self) -> DependencyGraph {
        self.graph
    }

    pub fn skip_from_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.from.get_skip_nodes(&self.graph, &self.query)
    }

    pub fn skip_to_node_labels(&self, config: &ReduceConfig) -> SkipNodes {
        config.to.get_skip_nodes(&self.graph, &self.query)
    }

    pub fn create_editor(&self, workspace_root: &str) -> BazelDepEditor {
        BazelDepEditor::new(&self.query, workspace_root)
    }
}
