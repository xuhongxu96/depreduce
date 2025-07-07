use std::{collections::HashSet, path::Path, process::exit};

use clap::Parser;

use depreduce::{
    editors::BazelDepEditor,
    graph::{DependencyGraph, bazel_xml_parser::parse_bazel_xml},
    postprocessors::AliasTargetPostprocessor,
    reducers::{
        reduce_context::{ReduceSettings, ReductionAttempt},
        top_sort_reducer::TopSortReducer,
    },
};
use utils::{get_bazel_query, to_json_lines};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    workspace: String,

    #[arg(short, long, default_value = "${workspace}/build.sh")]
    command: String,

    #[arg(
        short,
        long,
        default_value = "logs/",
        help = "Output directory for reduction attempts and dep graph"
    )]
    output: String,

    #[arg(short, long)]
    deps_only: bool,

    #[arg(long, default_value = "allowrules.txt")]
    allow_rules: String,

    #[arg(long, default_value = "blockrules.txt")]
    block_rules: String,

    #[arg(
        long,
        default_value = "false",
        help = "Disable dependency flattening: prevents the reducer from adding dependencies of the node being optimized to the dependent node being reduced as dependencies"
    )]
    disable_dependency_flattening: bool,

    #[arg(
        long,
        default_value = "false",
        help = "Enable dependency flattening for alias targets. Disabled by default to avoid flattening the alias targets because they are usually used to simplify the dependency names or combine multiple dependencies as a whole."
    )]
    enable_dependency_flattening_for_alias_targets: bool,

    #[arg(
        long,
        default_value = "false",
        help = "Disable dependency lifting: prevents the reducer from adding the node being optimized to the dependents of the dependent node being reduced as a dependency"
    )]
    disable_dependency_lifting: bool,

    #[arg(
        long,
        default_value = "false",
        help = "Only can be set when disable_dependency_flattening and disable_dependency_lifting are both set"
    )]
    disable_topological_sorting: bool,

    #[arg(
        long,
        default_value = "false",
        help = "Also consider to remove a dependency even if it can still be accessed transitively. Disabled by default to avoid removing direct dependencies."
    )]
    enable_optimization_if_transitive_deps_exists: bool,
}

fn run_reducer_test(
    xml: &str,
    workspace_root: String,
    build_script: String,
    args: &Args,
) -> (DependencyGraph, Vec<ReductionAttempt>) {
    println!(
        "Starting reduction test at {:?}",
        chrono::offset::Local::now()
    );

    println!("Workspace root: {}", workspace_root);
    println!("Build script: {}", build_script);
    println!("Args: {:#?}", args);

    let query = parse_bazel_xml(xml).unwrap();
    let graph = query.to_dep_graph(args.deps_only).unwrap();
    let node_and_rule_class = query.to_node_and_rule_class();

    let mut skip_node_ids = HashSet::new();
    if !args.allow_rules.is_empty() {
        let allow_rules: Vec<String> = std::fs::read_to_string(&args.allow_rules)
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim().to_string())
            .collect();

        let allow_rule_strs: Vec<&str> = allow_rules
            .iter()
            .filter(|s| !s.starts_with("regex:"))
            .map(|s| s.as_str())
            .collect();

        let allow_rule_regexes: Vec<regex::Regex> = allow_rules
            .iter()
            .filter(|s| s.starts_with("regex:"))
            .map(|s| regex::Regex::new(&s.trim_start_matches("regex:")).unwrap())
            .collect();

        if !allow_rules.is_empty() {
            node_and_rule_class.iter().for_each(|(node, class)| {
                if !allow_rule_strs.contains(&class.as_str())
                    && !allow_rule_regexes.iter().any(|r| r.is_match(class))
                {
                    graph.get_node_id(node).map(|id| {
                        skip_node_ids.insert((id, class));
                    });
                }
            });
        }
    }

    if !args.block_rules.is_empty() {
        let block_rules: Vec<String> = std::fs::read_to_string(&args.block_rules)
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim().to_string())
            .collect();

        let block_rule_strs: Vec<&str> = block_rules
            .iter()
            .filter(|s| !s.starts_with("regex:"))
            .map(|s| s.as_str())
            .collect();

        let block_rule_regexes: Vec<regex::Regex> = block_rules
            .iter()
            .filter(|s| s.starts_with("regex:"))
            .map(|s| regex::Regex::new(&s.trim_start_matches("regex:")).unwrap())
            .collect();

        if !block_rules.is_empty() {
            node_and_rule_class.iter().for_each(|(node, class)| {
                if block_rule_strs.contains(&class.as_str())
                    || block_rule_regexes.iter().any(|r| r.is_match(class))
                {
                    graph.get_node_id(node).map(|id| {
                        skip_node_ids.insert((id, class));
                    });
                }
            });
        }
    }

    println!(
        "Skipping nodes: {:#?}",
        skip_node_ids
            .iter()
            .map(|(id, class)| { (&graph.nodes[*id].label, class) })
            .collect::<Vec<_>>()
    );

    let editor = if args.deps_only {
        BazelDepEditor::new_with_custom_keywords(
            &query,
            workspace_root.to_string(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        )
    } else {
        BazelDepEditor::new(&query, workspace_root.to_string())
    };

    let reducer = TopSortReducer {};
    let settings = ReduceSettings {
        editor: &editor,
        build_command: build_script,
        cwd: workspace_root,
        save_build_log: true,

        disable_dependency_flattening: args.disable_dependency_flattening,
        disable_dependency_flattening_for_alias_targets: !args
            .enable_dependency_flattening_for_alias_targets,
        disable_dependency_lifting: args.disable_dependency_lifting,
        disable_topological_sorting: args.disable_topological_sorting,
        disable_optimization_if_transitive_deps_exists: !args
            .enable_optimization_if_transitive_deps_exists,

        skip_node_ids: skip_node_ids.iter().map(|(id, _class)| *id).collect(),
    };
    let mut ctx = reducer.reduce(graph, &settings).unwrap();

    let mut postprocessor = AliasTargetPostprocessor::new(&mut ctx);
    postprocessor.process();

    println!("End reduction test at {:?}", chrono::offset::Local::now());

    let attempts = ctx.get_attempts().to_vec();
    (ctx.graph, attempts)
}

fn check_if_multiline_bash_has_flag_e(path: &str) -> bool {
    let bash = std::fs::read_to_string(path).expect("Failed to read bash script");
    let is_multiline = bash.lines().filter(|line| !line.trim().is_empty()).count() > 1;
    let has_e = bash.contains("set -e") || bash.contains("set -o errexit");
    !is_multiline || has_e
}

fn main() {
    let args = Args::parse();

    let command = args.command.replace("${workspace}", &args.workspace);

    if !check_if_multiline_bash_has_flag_e(&command) {
        eprintln!(
            "The script {} is multiline and does not have 'set -e' or 'set -o errexit'.",
            command
        );
        eprintln!("This may lead to false positives in the depreduce log.");
        exit(1);
    }

    let xml_str = get_bazel_query(&args.workspace);

    let (graph, attempts) = run_reducer_test(
        &xml_str,
        Path::new(&args.workspace)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        Path::new(&command)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        &args,
    );

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let graph_path = Path::new(&args.output).join("00-graph.json");
    let attempt_json_path = Path::new(&args.output).join("01-attempts.jsonl");

    let graph_json = serde_json::to_string(&graph).expect("Failed to serialize graph to JSON");
    std::fs::write(graph_path, graph_json).expect("Failed to write graph to file");

    let attempt_json_lines = to_json_lines(&attempts);
    std::fs::write(attempt_json_path, attempt_json_lines)
        .expect("Failed to write attempts to file");
}
