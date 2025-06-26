use std::{
    io::Read,
    path::Path,
    process::{Command, exit},
};

use clap::Parser;

use depreduce::{
    editors::BazelDepEditor,
    graph::bazel_xml_parser::{Query, convert_query_to_dep_graph, parse_bazel_xml},
    reducers::{
        candidate_generators::NaiveReductionCandidateGeneratorFactory,
        top_sort_reducer::{ReduceSettings, TopSortReducer},
    },
};
use utils::get_test_data_path;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    workspace: String,

    #[arg(short, long, default_value = "${workspace}/build.sh")]
    command: String,

    #[arg(short, long, default_value = "depreduce.log")]
    log: String,
}

fn run_reducer_test(xml: &str, workspace_root: String, build_script: String) -> String {
    let query: Query = parse_bazel_xml(xml).unwrap();
    let graph = convert_query_to_dep_graph(&query).unwrap();
    let editor = BazelDepEditor::new(&query, workspace_root.to_string());

    let reducer = TopSortReducer::new(Box::new(editor));
    let settings = ReduceSettings {
        reduction_candidate_generator_factory: &NaiveReductionCandidateGeneratorFactory,
        graph: &graph,
        build_command: build_script,
        cwd: workspace_root,
    };
    reducer.reduce(&settings).unwrap()
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

    p.wait().expect("Bazel query did not finish successfully");
    let mut xml_str = String::new();
    p.stdout
        .expect("Failed to get bazel query stdout")
        .read_to_string(&mut xml_str)
        .expect("Failed to read bazel query output");

    let log = run_reducer_test(
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
    );
    std::fs::write(&args.log, log).expect("Failed to write log file");
}
