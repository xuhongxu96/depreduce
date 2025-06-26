use std::collections::HashMap;
use std::process::Command;

use crate::editors::{DepEditor, FileEdit};
use crate::graph::{DependencyGraph, NodeId};
use crate::reducers::candidate_generators::{
    ReductionCandidateGenerator, ReductionCandidateGeneratorFactory,
};

struct TopSortReducer {
    editor: Box<dyn DepEditor>,
}

struct ReduceSettings<'a> {
    reduction_candidate_generator_factory: &'a dyn ReductionCandidateGeneratorFactory,
    graph: &'a DependencyGraph,
    build_command: String,
    cwd: String,
}

struct ReduceContext<'a> {
    history: HashMap<String, String>,
    logs: String,

    settings: &'a ReduceSettings<'a>,
}

impl<'a> ReduceContext<'a> {
    pub fn new(settings: &'a ReduceSettings<'a>) -> Self {
        Self {
            logs: String::new(),
            history: HashMap::new(),
            settings,
        }
    }

    fn backup_and_apply(&mut self, edit: FileEdit) {
        let backup_content = std::fs::read_to_string(&edit.path)
            .unwrap_or_else(|err| panic!("Failed to read file {}: {}", edit.path, err));

        std::fs::write(&edit.path, &edit.content)
            .unwrap_or_else(|err| panic!("Failed to write file {}: {}", edit.path, err));

        self.history.insert(edit.path, backup_content);
    }

    fn restore_backup(&mut self) {
        for (path, content) in &self.history {
            std::fs::write(path, content)
                .unwrap_or_else(|err| panic!("Failed to restore file {}: {}", path, err));
        }
        self.history.clear();
    }

    fn try_build(&mut self) -> Result<(), std::io::Error> {
        let exit = Command::new("/bin/bash")
            .arg(&self.settings.build_command)
            .current_dir(&self.settings.cwd)
            .spawn()?
            .wait()?;
        if exit.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Build failed",
            ))
        }
    }

    fn commit_changes(&mut self) {
        self.history.clear();
    }

    fn generate_reduction_candidates(
        &mut self,
        node_id: NodeId,
    ) -> Box<dyn ReductionCandidateGenerator> {
        let mut dependents_vec = Vec::new();

        if let Some(dependents) = self.settings.graph.node2in_edges.get(&node_id) {
            dependents_vec = dependents.iter().map(|(a, b)| (*a, *b)).collect();
        }

        self.settings
            .reduction_candidate_generator_factory
            .create(dependents_vec)
    }
}

impl TopSortReducer {
    pub fn new(editor: Box<dyn DepEditor>) -> Self {
        Self { editor }
    }

    pub fn reduce(&self, settings: &ReduceSettings) -> Result<String, String> {
        let &ReduceSettings { graph, .. } = settings;

        let mut ctx = ReduceContext::new(settings);

        let mut sorted_nodes = graph.topsort();

        ctx.logs.push_str("Sorted nodes in topological order:\n");
        for node_id in sorted_nodes.iter() {
            ctx.logs
                .push_str(&format!("  {}\n", graph.nodes.get(*node_id).unwrap().label));
        }

        for node_id in sorted_nodes {
            let label = graph.nodes[node_id].label.clone();
            let mut generator = ctx.generate_reduction_candidates(node_id);

            while let Some(candidates) = generator.next() {
                for (dep_node, _edge_id) in candidates {
                    let dep_node_label = graph.nodes[dep_node].label.clone();
                    if let Ok(edit) = self.editor.remove(&dep_node_label, &label) {
                        ctx.backup_and_apply(edit);
                        ctx.logs.push_str(&format!(
                            "Applied reduction candidate for {} -> {}\n",
                            label, dep_node_label
                        ));

                        match ctx.try_build() {
                            Ok(_) => {
                                ctx.commit_changes();
                                ctx.logs.push_str(&format!(
                                    "  Build succeeded after applying reduction candidate for {} -> {}\n",
                                    label, dep_node_label
                                ));
                                generator.report_result(true);
                            }
                            Err(e) => {
                                ctx.restore_backup();
                                ctx.logs.push_str(&format!(
                                    "  Build failed after applying reduction candidate for {} -> {}, error: {}\n",
                                    label, dep_node_label, e
                                ));
                                generator.report_result(false);
                            }
                        }
                    } else {
                        ctx.logs.push_str(&format!(
                            "Failed to apply reduction candidate for {} -> {}\n",
                            label, dep_node_label
                        ));
                        generator.report_result(false);
                    }
                }
            }
        }

        Ok(ctx.logs)
    }
}

#[cfg(test)]
mod tests {
    use utils::*;

    use crate::{
        editors::BazelDepEditor,
        graph::bazel_xml_parser::{Query, convert_query_to_dep_graph, parse_bazel_xml},
        reducers::candidate_generators::NaiveReductionCandidateGeneratorFactory,
    };

    use super::*;

    #[test]
    fn test_cxx() {
        let xml = read_test_data!("cxx-deps.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(
            &query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project".to_string(),
        );

        let reducer = TopSortReducer::new(Box::new(editor));
        let settings = ReduceSettings {
            reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
            graph: &graph,
            build_command: get_test_data_path!("build.sh")
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!("../../../examples/simple-cxx-project")
                .to_string_lossy()
                .to_string(),
        };
        let res = reducer.reduce(&settings);
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }

    #[test]
    fn test_java() {
        let xml = read_test_data!("java-deps.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(
            &query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project".to_string(),
        );

        let reducer = TopSortReducer::new(Box::new(editor));
        let settings = ReduceSettings {
            reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
            graph: &graph,
            build_command: get_test_data_path!("build.sh")
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!("../../../examples/simple-java-project")
                .to_string_lossy()
                .to_string(),
        };
        let res = reducer.reduce(&settings);
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }

    #[test]
    fn test_kotlin() {
        let xml = read_test_data!("kt-deps.xml");
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(
            &query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-kotlin-project".to_string(),
        );

        let reducer = TopSortReducer::new(Box::new(editor));
        let settings = ReduceSettings {
            reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
            graph: &graph,
            build_command: get_test_data_path!("build.sh")
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!("../../../examples/simple-kotlin-project")
                .to_string_lossy()
                .to_string(),
        };
        let res = reducer.reduce(&settings);
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }
}
