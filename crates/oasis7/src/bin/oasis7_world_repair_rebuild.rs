use std::env;
use std::path::{Path, PathBuf};

use oasis7::runtime::{Journal, World, WorldError};

struct CliOptions {
    source_world_dir: PathBuf,
    output_world_dir: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("oasis7_world_repair_rebuild failed: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), WorldError> {
    let options = parse_args()?;
    let journal_path = options.source_world_dir.join("journal.json");
    let journal = Journal::load_json(journal_path.as_path())?;

    let seed_world = World::new();
    let seed_snapshot = seed_world.snapshot();
    let rebuilt = World::from_snapshot(seed_snapshot, journal)?;

    std::fs::create_dir_all(options.output_world_dir.as_path())?;
    rebuilt.save_to_dir(options.output_world_dir.as_path())?;
    let verified = World::load_from_dir(options.output_world_dir.as_path())?;

    println!("source_world_dir={}", options.source_world_dir.display());
    println!("output_world_dir={}", options.output_world_dir.display());
    println!("journal_events={}", rebuilt.journal().len());
    println!(
        "tick_consensus_records={}",
        verified.tick_consensus_records().len()
    );
    println!("world_time={}", verified.state().time);
    Ok(())
}

fn parse_args() -> Result<CliOptions, WorldError> {
    let mut source_world_dir = None;
    let mut output_world_dir = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-world-dir" => {
                source_world_dir = Some(parse_path_arg(args.next(), "--source-world-dir")?);
            }
            "--output-world-dir" => {
                output_world_dir = Some(parse_path_arg(args.next(), "--output-world-dir")?);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                return Err(WorldError::Io(format!("unknown argument: {arg}")));
            }
        }
    }

    Ok(CliOptions {
        source_world_dir: source_world_dir
            .ok_or_else(|| WorldError::Io("missing --source-world-dir".to_string()))?,
        output_world_dir: output_world_dir
            .ok_or_else(|| WorldError::Io("missing --output-world-dir".to_string()))?,
    })
}

fn parse_path_arg(value: Option<String>, flag: &str) -> Result<PathBuf, WorldError> {
    let raw = value.ok_or_else(|| WorldError::Io(format!("missing value for {flag}")))?;
    let path = Path::new(raw.as_str());
    if path.as_os_str().is_empty() {
        return Err(WorldError::Io(format!(
            "empty path is not allowed for {flag}"
        )));
    }
    Ok(path.to_path_buf())
}

fn print_usage() {
    println!(
        "Usage: oasis7_world_repair_rebuild --source-world-dir <path> --output-world-dir <path>"
    );
}
