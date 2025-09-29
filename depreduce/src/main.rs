use std::{path::Path, process::exit};

use clap::Parser;

use depreduce::{
    configs::ReduceConfig,
    graph::DependencyGraph,
    postprocessors::AliasTargetPostprocessor,
    reducers::{
        reduce_context::{ReduceSettings, ReductionAttempt},
        top_sort_reducer::TopSortReducer,
    },
    stats::rebuild_cost::RebuildCostCalculator,
    supports::BazelSupport,
};
use utils::to_json_lines;

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
        default_value = "//...",
        help = "Target to query dependencies for"
    )]
    target: String,

    #[arg(
        short,
        long,
        default_value = "logs/",
        help = "Output directory for reduction attempts and dep graph"
    )]
    output: String,

    #[arg(long, default_value = "depreduce.toml")]
    config: String,

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
    workspace_root: &str,
    build_script: &str,
    args: &Args,
) -> (DependencyGraph, Vec<ReductionAttempt>) {
    println!(
        "Starting reduction test at {:?}",
        chrono::offset::Local::now()
    );

    println!("Workspace root: {}", workspace_root);
    println!("Build script: {}", build_script);
    println!("Args: {:#?}", args);

    let config = ReduceConfig::from_toml(
        &std::fs::read_to_string(&args.config).expect("Failed to read config file"),
    )
    .expect("Failed to parse config file");

    let support = BazelSupport::new(&workspace_root, &args.target, &config);
    let editor = support.create_editor(&workspace_root);

    println!("Parsed dep graph");
    let original_cost =
        RebuildCostCalculator::new(support.get_graph()).calculate_rebuild_cost_sum();
    println!("Original rebuild cost: {}", original_cost);

    let skip_from_node_labels = support.skip_from_node_labels(&config);
    let skip_to_node_labels = support.skip_to_node_labels(&config);

    println!(
        "Skipping `from` nodes for removal ({}): {:#?}",
        skip_from_node_labels.for_removal.len(),
        skip_from_node_labels.for_removal
    );
    println!(
        "Skipping `to` nodes for removal ({}): {:#?}",
        skip_to_node_labels.for_removal.len(),
        skip_to_node_labels.for_removal
    );

    println!(
        "Skipping `from` nodes for addition ({}): {:#?}",
        skip_from_node_labels.for_addition.len(),
        skip_from_node_labels.for_addition
    );
    println!(
        "Skipping `to` nodes for addition ({}): {:#?}",
        skip_to_node_labels.for_addition.len(),
        skip_to_node_labels.for_addition
    );

    let reducer = TopSortReducer {};
    let settings = ReduceSettings {
        editor: &editor,
        build_command: build_script.to_string(),
        cwd: workspace_root.to_string(),
        save_build_log: true,

        disable_dependency_flattening: args.disable_dependency_flattening,
        disable_dependency_flattening_for_alias_targets: !args
            .enable_dependency_flattening_for_alias_targets,
        disable_dependency_lifting: args.disable_dependency_lifting,
        disable_topological_sorting: args.disable_topological_sorting,
        disable_optimization_if_transitive_deps_exists: !args
            .enable_optimization_if_transitive_deps_exists,
        timeout_seconds: config.timeout_seconds,

        skip_from_node_ids_for_removal: skip_from_node_labels
            .for_removal
            .iter()
            .map(|label| {
                support
                    .get_graph()
                    .get_node_id(label)
                    .expect(&format!("Node {} not found in graph", label))
            })
            .collect(),
        skip_to_node_ids_for_removal: skip_to_node_labels
            .for_removal
            .iter()
            .map(|label| {
                support
                    .get_graph()
                    .get_node_id(label)
                    .expect(&format!("Node {} not found in graph", label))
            })
            .collect(),
        skip_from_node_ids_for_addition: skip_from_node_labels
            .for_addition
            .iter()
            .map(|label| {
                support
                    .get_graph()
                    .get_node_id(label)
                    .expect(&format!("Node {} not found in graph", label))
            })
            .collect(),
        skip_to_node_ids_for_addition: skip_to_node_labels
            .for_addition
            .iter()
            .map(|label| {
                support
                    .get_graph()
                    .get_node_id(label)
                    .expect(&format!("Node {} not found in graph", label))
            })
            .collect(),
    };
    let mut ctx = reducer.reduce(support.move_out_graph(), &settings).unwrap();

    if !settings.disable_dependency_flattening_for_alias_targets {
        let mut postprocessor = AliasTargetPostprocessor::new(&mut ctx);
        postprocessor.process();
    }

    println!("End reduction test at {:?}", chrono::offset::Local::now());

    let attempts = ctx.get_attempts().to_vec();

    // Recalculate the rebuild cost after reduction
    let new_support = BazelSupport::new(&workspace_root, &args.target, &config);
    let new_cost = RebuildCostCalculator::new(new_support.get_graph()).calculate_rebuild_cost_sum();
    println!("Rebuild cost: {} -> {}", original_cost, new_cost);

    // But we still want to return the original graph,
    // because the reduction attempts are based on the original graph.
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

    let (graph, attempts) = run_reducer_test(
        Path::new(&args.workspace)
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap(),
        Path::new(&command)
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap(),
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
