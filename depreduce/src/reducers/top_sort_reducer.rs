use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::editors::{DepEditor, FileEdit};
use crate::graph::{DependencyGraph, EdgeId, NodeId};
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

    fn backup(&mut self, edit: &FileEdit) {
        let backup_content = std::fs::read_to_string(&edit.path)
            .unwrap_or_else(|err| panic!("Failed to read file {}: {}", edit.path, err));

        if !self.history.contains_key(&edit.path) {
            self.history.insert(edit.path.clone(), backup_content);
        }
    }

    fn apply(&self, edit: &FileEdit) {
        std::fs::write(&edit.path, &edit.content)
            .unwrap_or_else(|err| panic!("Failed to write file {}: {}", edit.path, err));
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

    fn try_add_transitive_deps(
        &self,
        ctx: &mut ReduceContext,
        candidates: &Vec<(NodeId, EdgeId)>,
        node_id: NodeId,
    ) -> bool {
        let label = ctx.settings.graph.nodes[node_id].label.clone();
        ctx.logs.push_str(&format!(
            "  Trying to add transitive dependencies for node {}\n",
            label
        ));

        let mut transitive_deps: HashSet<(NodeId, String)> = HashSet::new();
        if let Some(tgt2edge) = ctx.settings.graph.node2out_edges.get(&node_id) {
            tgt2edge.keys().for_each(|dep_node| {
                transitive_deps.insert((
                    *dep_node,
                    ctx.settings
                        .graph
                        .nodes
                        .get(*dep_node)
                        .unwrap()
                        .label
                        .clone(),
                ));
            });
        }

        for (dep_node, _edge_id) in candidates {
            let dep_node_label = ctx.settings.graph.nodes[*dep_node].label.clone();
            for (_, transitive_dep_label) in &transitive_deps {
                if let Ok(edit) = self.editor.add(&dep_node_label, transitive_dep_label) {
                    ctx.backup(&edit);
                    ctx.apply(&edit);
                    ctx.logs.push_str(&format!(
                        "  Added {} -> {}\n",
                        dep_node_label, transitive_dep_label
                    ));
                } else {
                    ctx.logs.push_str(&format!(
                        "Failed to add {} -> {}\n",
                        dep_node_label, transitive_dep_label
                    ));
                    ctx.restore_backup();
                    return false;
                }
            }
        }

        match ctx.try_build() {
            Ok(_) => {
                ctx.commit_changes();
                ctx.logs.push_str(&format!("  Build succeeded\n"));
                return true;
            }
            Err(e) => {
                ctx.restore_backup();
                ctx.logs.push_str(&format!("  Build failed: {}\n", e));
                return false;
            }
        }
    }

    fn try_remove_dep(
        &self,
        ctx: &mut ReduceContext,
        candidates: &Vec<(NodeId, EdgeId)>,
        node_id: NodeId,
    ) -> bool {
        let label = ctx.settings.graph.nodes[node_id].label.clone();
        for (dep_node, _edge_id) in candidates {
            let dep_node_label = ctx.settings.graph.nodes[*dep_node].label.clone();
            if let Ok(edit) = self.editor.remove(&dep_node_label, &label) {
                ctx.backup(&edit);
                ctx.apply(&edit);
                ctx.logs
                    .push_str(&format!("  Removed {} -> {}\n", dep_node_label, label));
            } else {
                ctx.logs.push_str(&format!(
                    "  Failed to remove {} -> {}\n",
                    dep_node_label, label
                ));
                ctx.restore_backup();
                return false;
            }
        }

        match ctx.try_build() {
            Ok(_) => {
                ctx.commit_changes();
                ctx.logs.push_str("  Build succeeded");
                true
            }
            Err(e) => {
                ctx.logs.push_str(&format!("  Build failed: {}\n", e));
                self.try_add_transitive_deps(ctx, candidates, node_id)
            }
        }
    }

    pub fn reduce(&self, settings: &ReduceSettings) -> Result<String, String> {
        let &ReduceSettings { graph, .. } = settings;

        let mut ctx = ReduceContext::new(settings);
        let sorted_nodes = graph.topsort();

        ctx.logs.push_str("Sorted nodes in topological order:\n");
        for node_id in sorted_nodes.iter() {
            ctx.logs
                .push_str(&format!("  {}\n", graph.nodes.get(*node_id).unwrap().label));
        }

        for node_id in sorted_nodes {
            ctx.logs.push_str(&format!(
                "Processing node: {}\n",
                graph.nodes.get(node_id).unwrap().label
            ));

            let mut generator = ctx.generate_reduction_candidates(node_id);

            while let Some(candidates) = generator.next() {
                let res = self.try_remove_dep(&mut ctx, &candidates, node_id);
                generator.report_result(res);
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
