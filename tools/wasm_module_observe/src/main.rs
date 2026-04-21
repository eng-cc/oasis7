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
                spec_path = Some(required_arg_value(&args, index, "--spec")?);
            }
            "--out-dir" => {
                index += 1;
                out_dir = Some(required_arg_value(&args, index, "--out-dir")?);
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

fn required_arg_value(args: &[String], index: usize, flag: &str) -> Result<PathBuf, String> {
    let Some(value) = args.get(index) else {
        return Err(format!("{flag} requires a value"));
    };
    if value.starts_with('-') {
        return Err(format!("{flag} requires a value"));
    }
    Ok(PathBuf::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_observe_args_rejects_missing_spec_value() {
        let err = parse_observe_args(vec!["--spec".to_string()]).expect_err("spec should fail");
        assert_eq!(err, "--spec requires a value");
    }

    #[test]
    fn parse_observe_args_rejects_missing_out_dir_value() {
        let err = parse_observe_args(vec![
            "--spec".to_string(),
            "spec.json".to_string(),
            "--out-dir".to_string(),
        ])
        .expect_err("out-dir should fail");
        assert_eq!(err, "--out-dir requires a value");
    }
}
