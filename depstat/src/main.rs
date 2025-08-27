use std::collections::HashSet;

use clap::{Parser, Subcommand};
use depreduce::{
    graph::bazel_xml_parser::parse_bazel_xml, stats::rebuild_cost::RebuildCostCalculator,
};
use depstat::parse_logs;
use utils::get_bazel_query;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    workspace: String,

    #[arg(short, long, default_value = "//...")]
    target: String,

    #[arg(short, long, default_value = "true")]
    deps_only: bool,

    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse depreduce logs to collect statistics.
    Parse {
        #[arg(short, long)]
        log_dir: String,
    },
}

fn main() {
    let args = Args::parse();

    if args.commands.is_some() {
        match args.commands.unwrap() {
            Commands::Parse { log_dir } => {
                parse_logs(&log_dir);
            }
        }
        return;
    }

    let xml_str = get_bazel_query(&args.workspace, &args.target);
    if let Ok(query) = parse_bazel_xml(&xml_str) {
        let graph = query.to_dep_graph(args.deps_only, &HashSet::new()).unwrap();
        let original_cost = RebuildCostCalculator::new(&graph).calculate_rebuild_cost_sum();
        println!("Rebuild cost: {}", original_cost);
    } else {
        eprintln!("Failed to parse bazel query xml.");
        eprintln!("{}", xml_str);
    }
}
