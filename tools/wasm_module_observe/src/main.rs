use std::env;
use std::path::PathBuf;
use wasm_module_observe::{run_observe, ObserveRunRequest};

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("error: {err}");
        std::process::exit(2);
    }
}

fn run_cli() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Err("missing command".to_string());
    };

    match command.as_str() {
        "observe" => {
            let request = parse_observe_args(args.collect())?;
            let output = run_observe(&request)?;
            println!("out_dir={}", output.out_dir.to_string_lossy());
            println!(
                "summary_json={}",
                output.summary_json_path.to_string_lossy()
            );
            println!("summary_md={}", output.summary_md_path.to_string_lossy());
            println!("module_id={}", output.summary.module_id);
            println!("wasm_hash_sha256={}", output.summary.wasm_hash_sha256);
            Ok(())
        }
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_observe_args(args: Vec<String>) -> Result<ObserveRunRequest, String> {
    let mut spec_path: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--spec" => {
                index += 1;
                spec_path = args.get(index).map(PathBuf::from);
            }
            "--out-dir" => {
                index += 1;
                out_dir = args.get(index).map(PathBuf::from);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown option for observe: {other}"));
            }
        }
        index += 1;
    }
    Ok(ObserveRunRequest {
        spec_path: spec_path.ok_or_else(|| "--spec is required".to_string())?,
        out_dir,
    })
}

fn print_usage() {
    println!("wasm_module_observe observe --spec <path> [options]");
    println!("options:");
    println!("  --out-dir <path>      default: .tmp/wasm_module_observe/<module_id>");
}
