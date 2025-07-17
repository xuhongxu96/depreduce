use clap::Parser;

use strace_parser::{
    analyzer::analyze, combiner::combine_syscall_lines, dep_extractor::extract_dependencies,
    lower::parse_syscall_desps, parser::parse_strace_from_path,
};
use utils::to_json_lines;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value = "strace.log")]
    input: String,

    #[arg(short, long)]
    cwd: String,

    #[arg(short, long, default_value = "output.deps.log")]
    output: String,

    #[arg(short, long, default_value = "debug_output")]
    debug_output_dir: String,
}

fn main() {
    let args = Args::parse();

    if !args.debug_output_dir.is_empty() {
        std::fs::create_dir_all(&args.debug_output_dir)
            .expect("Failed to create debug output directory");
    }

    let cwd = std::fs::canonicalize(&args.cwd)
        .unwrap_or_else(|_| panic!("Failed to canonicalize cwd: {}", args.cwd));

    let irs: Vec<_> = parse_syscall_desps(combine_syscall_lines(parse_strace_from_path(
        args.input.as_str(),
    )))
    .collect();

    if !args.debug_output_dir.is_empty() {
        let ir_content = to_json_lines(&irs);
        std::fs::write(format!("{}/irs.jsonl", args.debug_output_dir), ir_content)
            .expect("Failed to write IRs JSON");
    }

    let state = analyze(irs, cwd.to_str().unwrap());
    if !args.debug_output_dir.is_empty() {
        let state_content = state.to_json_lines();
        std::fs::write(
            format!("{}/state.jsonl", args.debug_output_dir),
            state_content,
        )
        .expect("Failed to write state JSON");
    }

    let dependency_extractor = extract_dependencies(&state);
    let dep_graph = dependency_extractor.get_dependencies();
    let content = to_json_lines(&dep_graph.to_sorted_vec());

    std::fs::write(&args.output, content).expect("Failed to write output file");
    println!("Dependencies written to {}", args.output);
}
