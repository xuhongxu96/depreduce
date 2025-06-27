use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use utils::indent_all_lines;

use crate::editors::{DepEditor, FileEdit};
use crate::graph::{DependencyGraph, EdgeId, NodeId};
use crate::reducers::candidate_generators::{
    ReductionCandidateGenerator, ReductionCandidateGeneratorFactory,
};

const INDENT_SIZE_FOR_STDOUT: usize = 8;

pub struct TopSortReducer {
    editor: Box<dyn DepEditor>,
}

pub struct ReduceSettings<'a> {
    pub reduction_candidate_generator_factory: &'a dyn ReductionCandidateGeneratorFactory,
    pub graph: &'a DependencyGraph,
    pub build_command: String,
    pub cwd: String,
}

pub struct ReduceContext<'a> {
    history: HashMap<String, String>,
    in_degrees: Vec<usize>,
    logs: String,

    settings: &'a ReduceSettings<'a>,
}

impl<'a> ReduceContext<'a> {
    pub fn new(settings: &'a ReduceSettings<'a>) -> Self {
        Self {
            history: HashMap::new(),
            in_degrees: Vec::new(),
            logs: String::new(),
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
        let mut log = String::new();
        log.push_str("  Restoring backups:\n");
        for (path, content) in &self.history {
            std::fs::write(path, content)
                .unwrap_or_else(|err| panic!("Failed to restore file {}: {}", path, err));
            log.push_str(&format!("    {}\n", path));
        }
        self.log(&log);
        self.history.clear();
    }

    fn try_build(&mut self) -> Result<String, std::io::Error> {
        let mut process = Command::new("/bin/bash")
            .arg(&self.settings.build_command)
            .current_dir(&self.settings.cwd)
            .stderr(Stdio::piped())
            .spawn()?;

        let stderr = process.stderr.as_mut().unwrap();
        let stderr_reader = BufReader::new(stderr);
        let stderr_lines = stderr_reader.lines();

        for line in stderr_lines {
            let line = line.expect("Failed to read line from bazel query output");
            self.log(&indent_all_lines(&line, INDENT_SIZE_FOR_STDOUT));
            self.log("\n");
        }

        let exit = process.wait()?;

        process.stdout.take().map(|mut stdout| {
            let mut output = String::new();
            stdout.read_to_string(&mut output).unwrap();
            if !output.is_empty() {
                self.log(&indent_all_lines("--- stdout ---", INDENT_SIZE_FOR_STDOUT));
                self.log(&indent_all_lines(&output, INDENT_SIZE_FOR_STDOUT));
            }
        });

        if exit.success() {
            return Ok(format!("Build succeeded"));
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Build failed with exit code {}", exit.code().unwrap_or(-1),),
            ));
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
            dependents_vec.sort();
        }

        self.settings
            .reduction_candidate_generator_factory
            .create(dependents_vec)
    }

    fn log(&mut self, message: &str) {
        self.logs.push_str(message);
        if !cfg!(test) {
            print!("{}", message);
        }
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
        let mut changed = false;

        let label = ctx.settings.graph.nodes[node_id].label.clone();
        ctx.log(&format!(
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
        let mut transitive_deps: Vec<_> = transitive_deps
            .into_iter()
            .map(|(id, label)| (id, label))
            .collect();
        transitive_deps.sort();

        for (dep_node, _edge_id) in candidates {
            let dep_node_label = ctx.settings.graph.nodes[*dep_node].label.clone();
            for (transitive_dep_id, transitive_dep_label) in &transitive_deps {
                if let Some(edges) = ctx.settings.graph.node2out_edges.get(dep_node) {
                    if edges.contains_key(transitive_dep_id) {
                        ctx.log(&format!(
                            "  Skipping {} -> {} (already exists)\n",
                            dep_node_label, transitive_dep_label
                        ));
                        continue;
                    }
                }

                match ctx.settings.graph.nodes[*transitive_dep_id].props.t {
                    crate::graph::NodeType::Target => {}
                    _ => {
                        ctx.log(&format!(
                            "  Skipping {} -> {} (non-target)\n",
                            dep_node_label, transitive_dep_label
                        ));
                        continue;
                    }
                }

                match self.editor.add(&dep_node_label, transitive_dep_label) {
                    Ok(edit) => {
                        changed = true;
                        ctx.backup(&edit);
                        ctx.apply(&edit);
                        ctx.log(&format!(
                            "  Added {} -> {}\n",
                            dep_node_label, transitive_dep_label
                        ));
                    }
                    Err(e) => {
                        ctx.log(&format!(
                            "Failed to add {} -> {}: {}\n",
                            dep_node_label, transitive_dep_label, e
                        ));
                        ctx.restore_backup();
                        return false;
                    }
                }
            }
        }

        if !changed {
            ctx.log("  No changes made, skipping build\n");
            ctx.restore_backup();
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                ctx.commit_changes();
                ctx.log(&format!("  Committed changes: {}\n", status));
                return true;
            }
            Err(e) => {
                ctx.restore_backup();
                ctx.log(&format!("  {}\n", e));
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
        let deps_keyword = HashSet::from(["deps".to_string()]);

        ctx.log("  Trying a new candidate set\n");

        let label = ctx.settings.graph.nodes[node_id].label.clone();
        for (dep_node, _edge_id) in candidates {
            if ctx.in_degrees[*dep_node] == 0 {
                ctx.log(
                    format!(
                        "    Only consider deps for {} -> {} (because of no in-degree)\n",
                        label, ctx.settings.graph.nodes[*dep_node].label
                    )
                    .as_str(),
                );
            }

            let dep_node_label = ctx.settings.graph.nodes[*dep_node].label.clone();
            match self
                .editor
                .remove(&dep_node_label, &label, ctx.in_degrees[*dep_node] == 0)
            {
                Ok(edit) => {
                    ctx.backup(&edit);
                    ctx.apply(&edit);
                    ctx.log(&format!("    Removed {} -> {}\n", dep_node_label, label));
                }
                Err(e) => {
                    ctx.log(&format!(
                        "    Failed to remove {} -> {}: {}\n",
                        dep_node_label, label, e
                    ));
                    continue;
                }
            }
        }

        if ctx.history.is_empty() {
            ctx.log("  No changes made, skipping build\n");
            return false;
        }

        match ctx.try_build() {
            Ok(status) => {
                ctx.commit_changes();
                ctx.log(&format!("  Committed changes: {}\n\n", status));
                true
            }
            Err(e) => {
                ctx.log(&format!("  {}\n\n", e));
                self.try_add_transitive_deps(ctx, candidates, node_id)
            }
        }
    }

    pub fn reduce(&self, settings: &ReduceSettings) -> Result<String, String> {
        let &ReduceSettings { graph, .. } = settings;

        let mut ctx = ReduceContext::new(settings);
        let sorted_nodes = graph.topsort();

        ctx.log("Sorted nodes in topological order:\n");
        for node_id in sorted_nodes.iter() {
            ctx.log(&format!("  {}\n", graph.nodes.get(*node_id).unwrap().label));
        }

        for i in 0..graph.nodes.len() {
            ctx.in_degrees
                .push(graph.node2in_edges.get(&i).map_or(0, |edges| edges.len()));
        }

        for node_id in sorted_nodes {
            ctx.log(&format!(
                "Processing node: {}\n",
                graph.nodes.get(node_id).unwrap().label
            ));

            let mut generator = ctx.generate_reduction_candidates(node_id);

            while let Some(candidates) = generator.next() {
                let res = self.try_remove_dep(&mut ctx, &candidates, node_id);
                generator.report_result(res);
                if res {
                    ctx.in_degrees[node_id] -= candidates.len();
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

    fn run_reducer_test(
        xml_file: &str,
        workspace_root: &str,
        project_dir: &str,
        build_script: &str,
        expected_out: &str,
    ) {
        let xml = read_test_data!(xml_file);
        let query: Query = parse_bazel_xml(&xml).unwrap();
        let graph = convert_query_to_dep_graph(&query).unwrap();
        let editor = BazelDepEditor::new(&query, workspace_root.to_string());

        let reducer = TopSortReducer::new(Box::new(editor));
        let settings = ReduceSettings {
            reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
            graph: &graph,
            build_command: get_test_data_path!(build_script)
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!(project_dir)
                .to_string_lossy()
                .to_string(),
        };
        let res =
            remove_lines_with_indent(&reducer.reduce(&settings).unwrap(), INDENT_SIZE_FOR_STDOUT);
        assert_eq!(res, read_or_create_test_data!(expected_out, res));
    }

    #[test]
    fn test_cxx() {
        run_reducer_test(
            "cxx-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
            "../../../examples/simple-cxx-project",
            "build.sh",
            "reducers/cxx.out",
        );
    }

    #[test]
    fn test_java() {
        run_reducer_test(
            "java-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project",
            "../../../examples/simple-java-project",
            "build.sh",
            "reducers/java.out",
        );
    }

    #[test]
    fn test_kotlin() {
        run_reducer_test(
            "kt-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-kotlin-project",
            "../../../examples/simple-kotlin-project",
            "build.sh",
            "reducers/kt.out",
        );
    }

    #[test]
    fn test_kotlin_transitive() {
        // What we want to test it whether the reducer can add deps of deps correctly, and
        // it also needs to deduplicate the added deps because main already depends on a.
        // See examples/kotlin-transitive/README.md for details.
        run_reducer_test(
            "kt-transitive-deps.xml",
            "/data/h445xu/repo/bazel-dep-reduce/examples/kotlin-transitive",
            "../../../examples/kotlin-transitive",
            "build.sh",
            "reducers/kt-transitive.out",
        );
    }
}
