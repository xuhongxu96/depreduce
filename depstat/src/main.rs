use clap::{Parser, Subcommand};
use depreduce::{
    configs::ReduceConfig,
    stats::rebuild_cost::RebuildCostCalculator,
    supports::{BazelSupport, BuildSystemSupport, create_support},
};
use depstat::parse_logs;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    workspace: String,

    #[arg(short, long, default_value = "//...")]
    target: String,

    #[command(subcommand)]
    commands: Option<Commands>,

    #[arg(
        short,
        long,
        default_value = "bazel",
        help = "Build system to use (currently supports: bazel, buck, cargo)"
    )]
    build_system: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse depreduce logs to collect statistics.
    Parse {
        #[arg(short, long)]
        log_dir: String,
    },

    /// Compute rebuild set
    ComputeRebuild {
        #[arg(short, long)]
        output: String,
    },
}

fn main() {
    let args = Args::parse();

    if args.commands.is_some() {
        match args.commands.unwrap() {
            Commands::Parse { log_dir } => {
                let res = parse_logs(&log_dir);
                println!("{:#?}", res);
            }
            Commands::ComputeRebuild { output } => {
                let support = create_support(
                    &args.build_system,
                    &args.workspace,
                    &args.target,
                    &ReduceConfig::default(),
                )
                .expect("Failed to create build system support");

                let graph = support.get_graph();
                let rebuild_set = depstat::compute_rebuild_set(&graph);
                let serialized =
                    serde_json::to_string_pretty(&rebuild_set).expect("Failed to serialize");
                std::fs::write(&output, serialized).expect("Failed to write output file");
            }
        }
        return;
    }

    let support = create_support(
        &args.build_system,
        &args.workspace,
        &args.target,
        &ReduceConfig::default(),
    )
    .expect("Failed to create build system support");

    let graph = support.get_graph();
    let original_cost = RebuildCostCalculator::new(&graph).calculate_rebuild_cost_sum();
    println!("Rebuild cost: {}", original_cost);
}
