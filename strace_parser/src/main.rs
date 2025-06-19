use std::fs;
use std::io::Write;

use clap::Parser;

use crate::{
    analyzer::analyze, combiner::combine_syscall_lines, dep_extractor::extract_dependencies,
    lower::parse_syscall_desps, parser::parse_strace_from_path,
};

mod analyzer;
mod combiner;
mod dep_extractor;
mod lower;
mod parser;
mod syntax;
mod syscall_line;
mod utils;
mod vfs;

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

    let dep_graph = extract_dependencies(&state);

    let mut f = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(args.output.as_str())
        .unwrap();
    for (i, path) in &dep_graph.final_dep_caches {
        writeln!(f, "\n{}: {}", i, dep_graph.get_path(*i).unwrap()).unwrap();

        for dep in path {
            writeln!(f, "  -> {}: {}", dep, dep_graph.get_path(*dep).unwrap()).unwrap();
        }
    }
}
