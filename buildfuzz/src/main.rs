use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use buildfuzz::fuzz::BuildArtifacts;
use clap::Parser;
use utils::to_json_lines;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// Path to the input repository directory. Will use .gitignore to filter out irrelevant files.
    #[arg(short, long)]
    input: String,

    /// Path to the build artifact (output) directory
    #[arg(short, long)]
    artifact: String,

    /// Path to the build command script or executable. Must be a bash script.
    #[arg(short, long, default_value = "build.sh")]
    command: String,

    /// Working directory to run the build command. Default to the input directory.
    #[arg(long, default_value = "")]
    cwd: String,

    #[arg(short, long, default_value = "output.deps.log")]
    output: String,

    #[arg(short, long, default_value = "all")]
    touchers: String,

    #[arg(long, default_value = "false")]
    use_timestamp: bool,
}

fn enumerate_files(path: &str, ignore: bool) -> HashSet<String> {
    let path = Path::new(path).canonicalize().unwrap();

    let mut files = HashSet::new();

    ignore::WalkBuilder::new(path)
        .git_ignore(ignore)
        .follow_links(true)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().unwrap().is_file())
        .for_each(|entry| {
            let file_path = entry.path().to_str().unwrap().to_string();
            files.insert(file_path);
        });

    files
}

fn main() {
    let mut args = Args::parse();

    if args.cwd.is_empty() {
        // If cwd is not provided, use the input directory
        args.cwd = args.input.clone();
    }

    args.command = PathBuf::from(&args.command)
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let touchers: Vec<Box<dyn buildfuzz::touchers::Toucher>> = match args.touchers.as_str() {
        "all" => vec![
            Box::new(buildfuzz::touchers::CToucher {}),
            Box::new(buildfuzz::touchers::JavaToucher {}),
            Box::new(buildfuzz::touchers::KotlinToucher {}),
        ],
        _ => {
            eprintln!("Unknown toucher type: {}", args.touchers);
            std::process::exit(1);
        }
    };

    let artifacts = BuildArtifacts {
        inputs: enumerate_files(&args.input, true),
        outputs: enumerate_files(&args.artifact, false),
        command: args.command,
        cwd: args.cwd,
        touchers,
    };

    match artifacts.fuzz(!args.use_timestamp) {
        Ok(res) => {
            let content = to_json_lines(&res.to_sorted_vec());
            std::fs::write(&args.output, content).expect("Failed to write output file");
        }
        Err(e) => {
            eprintln!("Failed to build artifacts: {:?}", e);
            std::process::exit(1);
        }
    }
}
