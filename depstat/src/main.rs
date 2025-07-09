use clap::Parser;
use depreduce::{
    graph::bazel_xml_parser::parse_bazel_xml, stats::rebuild_cost::RebuildCostCalculator,
};
use utils::get_bazel_query;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    workspace: String,

    #[arg(short, long)]
    deps_only: bool,
}

fn main() {
    let args = Args::parse();
    let xml_str = get_bazel_query(&args.workspace);
    let query = parse_bazel_xml(&xml_str).unwrap();
    let graph = query.to_dep_graph(args.deps_only).unwrap();
    let original_cost = RebuildCostCalculator::new(&graph).calculate_rebuild_cost_sum();
    println!("Rebuild cost: {}", original_cost);
}
