use crate::editors::DepEditor;
use crate::graph::DependencyGraph;
use crate::graph::bazel_xml_parser::Query;

struct TopSortReducer {
    graph: DependencyGraph,
    editor: Box<dyn DepEditor>,
}

impl TopSortReducer {
    pub fn new(graph: DependencyGraph, editor: Box<dyn DepEditor>) -> Self {
        Self { graph, editor }
    }
}
