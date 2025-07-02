use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Command, exit},
    time::SystemTime,
};

use clap::Parser;

use depreduce::{
    editors::BazelDepEditor,
    graph::{
        DependencyGraph,
        bazel_xml_parser::{Query, convert_query_to_dep_graph, parse_bazel_xml},
    },
    postprocessors::AliasTargetPostprocessor,
    reducers::{
        reduce_context::{ReduceSettings, ReductionAttempt},
        top_sort_reducer::TopSortReducer,
    },
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
        default_value = "logs/",
        help = "Output directory for reduction attempts and dep graph"
    )]
    output: String,

    #[arg(short, long)]
    deps_only: bool,

    #[arg(
        long,
        help = "Disable dependency flattening: prevents the reducer from adding dependencies of the node being optimized to the dependent node being reduced as dependencies"
    )]
    disable_dependency_flattening: bool,

    #[arg(
        long,
        help = "Disable dependency lifting: prevents the reducer from adding the node being optimized to the dependents of the dependent node being reduced as a dependency"
    )]
    disable_dependency_lifting: bool,

    #[arg(
        long,
        help = "Only can be set when disable_dependency_flattening and disable_dependency_lifting are both set"
    )]
    disable_topological_sorting: bool,
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

    let query: Query = parse_bazel_xml(xml).unwrap();
    let graph = convert_query_to_dep_graph(&query).unwrap();
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
        graph: &graph,
        build_command: build_script,
        cwd: workspace_root,
        save_build_log: true,

        disable_dependency_flattening: args.disable_dependency_flattening,
        disable_dependency_lifting: args.disable_dependency_lifting,
        disable_topological_sorting: args.disable_topological_sorting,
    };
    let mut ctx = reducer.reduce(&settings).unwrap();

    let mut postprocessor = AliasTargetPostprocessor::new(&mut ctx);
    postprocessor.process();

    println!("End reduction test at {:?}", chrono::offset::Local::now());

    let attempts = ctx.get_attempts().to_vec();
    (graph, attempts)
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

    let mut p = Command::new("bazel")
        .arg("query")
        .arg("deps(//...)")
        .arg("--notool_deps")
        .arg("--noimplicit_deps")
        .arg("--output")
        .arg("xml")
        .current_dir(&args.workspace)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run bazel query");

    let mut xml_str = String::new();
    let stdout = p.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for (i, line) in stdout_lines.enumerate() {
        let line = line.expect("Failed to read line from bazel query output");

        xml_str.push_str(&line);
        if i % 1000 == 0 {
            eprintln!("Read {} lines from bazel query output...", i);
        }
    }

    p.wait().expect("Bazel query did not finish successfully");

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
