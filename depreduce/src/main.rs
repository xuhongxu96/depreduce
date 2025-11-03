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
    supports::{BazelSupport, BuckSupport, BuildSystemSupport, CargoSupport},
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

    #[arg(
        short,
        long,
        default_value = "bazel",
        help = "Build system to use (currently supports: bazel, buck)"
    )]
    build_system: String,

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

fn create_support(
    build_system: &str,
    workspace: &str,
    target: &str,
    config: &ReduceConfig,
) -> Box<dyn BuildSystemSupport> {
    match build_system {
        "buck" => Box::new(BuckSupport::new(workspace, target, config)),
        "bazel" => Box::new(BazelSupport::new(workspace, target, config)),
        "rust" => Box::new(CargoSupport::new(workspace, target, config)),
        _ => {
            eprintln!("Unsupported build system: {}", build_system);
            exit(1);
        }
    }
}

fn run_reducer_test(args: &Args) -> (DependencyGraph, Vec<ReductionAttempt>, usize) {
    let command = args.command.replace("${workspace}", &args.workspace);

    if !check_if_multiline_bash_has_flag_e(&command) {
        eprintln!(
            "The script {} is multiline and does not have 'set -e' or 'set -o errexit'.",
            command
        );
        eprintln!("This may lead to false positives in the depreduce log.");
        exit(1);
    }

    let workspace_root = Path::new(&args.workspace).canonicalize().unwrap();
    let build_script = Path::new(&command).canonicalize().unwrap();

    println!(
        "Starting reduction test at {:?}",
        chrono::offset::Local::now()
    );

    println!("Workspace root: {:?}", workspace_root);
    println!("Build script: {:?}", build_script);
    println!("Args: {:#?}", args);

    let config = ReduceConfig::from_toml(
        &std::fs::read_to_string(&args.config).expect("Failed to read config file"),
    )
    .expect("Failed to parse config file");

    let mut support: Box<dyn BuildSystemSupport> = create_support(
        &args.build_system,
        workspace_root.to_str().unwrap(),
        &args.target,
        &config,
    );
    let editor = support.create_editor(workspace_root.to_str().unwrap());

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
        editor: editor.as_ref(),
        build_command: build_script.to_str().unwrap().to_string(),
        cwd: workspace_root.to_str().unwrap().to_string(),
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
    let mut graph = DependencyGraph::new();
    support.swap_graph(&mut graph);
    let mut ctx = reducer.reduce(graph, &settings).unwrap();

    if !settings.disable_dependency_flattening_for_alias_targets {
        let mut postprocessor = AliasTargetPostprocessor::new(&mut ctx);
        postprocessor.process();
    }

    println!("End reduction test at {:?}", chrono::offset::Local::now());

    let attempts = ctx.get_attempts().to_vec();

    // Recalculate the rebuild cost after reduction
    let new_support = create_support(
        &args.build_system,
        workspace_root.to_str().unwrap(),
        &args.target,
        &config,
    );
    let new_cost = RebuildCostCalculator::new(new_support.get_graph()).calculate_rebuild_cost_sum();
    println!("Rebuild cost: {} -> {}", original_cost, new_cost);

    // But we still want to return the original graph,
    // because the reduction attempts are based on the original graph.
    (ctx.graph, attempts, new_cost)
}

fn check_if_multiline_bash_has_flag_e(path: &str) -> bool {
    let bash = std::fs::read_to_string(path).expect("Failed to read bash script");
    let is_multiline = bash.lines().filter(|line| !line.trim().is_empty()).count() > 1;
    let has_e = bash.contains("set -e") || bash.contains("set -o errexit");
    !is_multiline || has_e
}

fn main() {
    let args = Args::parse();

    let (graph, attempts, _) = run_reducer_test(&args);

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let graph_path = Path::new(&args.output).join("00-graph.json");
    let attempt_json_path = Path::new(&args.output).join("01-attempts.jsonl");

    let graph_json = serde_json::to_string(&graph).expect("Failed to serialize graph to JSON");
    std::fs::write(graph_path, graph_json).expect("Failed to write graph to file");

    let attempt_json_lines = to_json_lines(&attempts);
    std::fs::write(attempt_json_path, attempt_json_lines)
        .expect("Failed to write attempts to file");
}

#[cfg(test)]
mod tests {
    use utils::get_test_data_path;

    use super::*;

    #[test]
    fn test_buck_e2e() {
        let workspace_root = get_test_data_path!("../../../examples/buck-rust");
        let build_sh = get_test_data_path!("build-buck.sh");

        let (_, _, new_cost) = run_reducer_test(&Args {
            workspace: workspace_root.to_str().unwrap().to_string(),
            command: build_sh.to_str().unwrap().to_string(),
            target: "//...".to_string(),
            output: "logs/".to_string(),
            build_system: "buck".to_string(),
            config: get_test_data_path!("empty-config.toml")
                .to_str()
                .unwrap()
                .to_string(),
            disable_dependency_flattening: false,
            enable_dependency_flattening_for_alias_targets: false,
            disable_dependency_lifting: false,
            disable_topological_sorting: false,
            enable_optimization_if_transitive_deps_exists: false,
        });

        assert!(new_cost == 2);
    }

    #[test]
    fn test_rust_e2e() {
        let workspace_root = get_test_data_path!("../../../examples/simple-rust-project");
        let build_sh = get_test_data_path!("build-rust.sh");

        let (_, _, new_cost) = run_reducer_test(&Args {
            workspace: workspace_root.to_str().unwrap().to_string(),
            command: build_sh.to_str().unwrap().to_string(),
            target: String::new(),
            output: "logs/".to_string(),
            build_system: "rust".to_string(),
            config: get_test_data_path!("empty-config.toml")
                .to_str()
                .unwrap()
                .to_string(),
            disable_dependency_flattening: false,
            enable_dependency_flattening_for_alias_targets: false,
            disable_dependency_lifting: false,
            disable_topological_sorting: false,
            enable_optimization_if_transitive_deps_exists: false,
        });

        assert!(new_cost == 1);
    }
}
