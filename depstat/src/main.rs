use clap::{Parser, Subcommand};
use depreduce::{
    configs::ReduceConfig,
    stats::rebuild_cost::RebuildCostCalculator,
    supports::{BazelSupport, BuildSystemSupport},
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
                let res = parse_logs(&log_dir);
                println!("{:#?}", res);
            }
        }
        return;
    }

    let support: Box<dyn BuildSystemSupport> = Box::new(BazelSupport::new(
        &args.workspace,
        &args.target,
        &ReduceConfig::default(),
    ));

    let graph = support.get_graph();
    let original_cost = RebuildCostCalculator::new(&graph).calculate_rebuild_cost_sum();
    println!("Rebuild cost: {}", original_cost);
}
