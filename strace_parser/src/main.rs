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
}

fn main() {
    let args = Args::parse();

    let state = analyze(
        parse_syscall_desps(combine_syscall_lines(parse_strace_from_path(
            args.input.as_str(),
        ))),
        args.cwd.as_str(),
    );

    let dependency_extractor = extract_dependencies(&state);
    let dep_graph = dependency_extractor.get_dependencies();
    let content = to_json_lines(&dep_graph.to_sorted_vec());

    std::fs::write(&args.output, content).expect("Failed to write output file");
    println!("Dependencies written to {}", args.output);
}
