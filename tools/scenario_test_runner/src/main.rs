use scenario_test_runner::{ScenarioOutcome, discover_scenario_files, run_scenario_file};
use std::env;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let mut scenario_path: Option<PathBuf> = None;
    let mut dir_path: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                if let Some(value) = args.next() {
                    scenario_path = Some(PathBuf::from(value));
                } else {
                    eprintln!("--scenario requires a path");
                    usage();
                    exit(2);
                }
            }
            "--dir" => {
                if let Some(value) = args.next() {
                    dir_path = Some(PathBuf::from(value));
                } else {
                    eprintln!("--dir requires a path");
                    usage();
                    exit(2);
                }
            }
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => {
                eprintln!("unknown argument: {arg}");
                usage();
                exit(2);
            }
        }
    }

    if scenario_path.is_some() == dir_path.is_some() {
        eprintln!("choose exactly one of --scenario or --dir");
        usage();
        exit(2);
    }

    let outcomes = if let Some(path) = scenario_path {
        vec![run_scenario_file(&path).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            exit(1);
        })]
    } else {
        let dir = dir_path.expect("dir path");
        let files = discover_scenario_files(&dir).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            exit(1);
        });
        if files.is_empty() {
            eprintln!("no scenario files found in {}", dir.display());
            exit(1);
        }
        let mut outcomes = Vec::with_capacity(files.len());
        for path in files {
            match run_scenario_file(&path) {
                Ok(outcome) => outcomes.push(outcome),
                Err(err) => {
                    eprintln!("error: {err}");
                    exit(1);
                }
            }
        }
        outcomes
    };

    print_outcomes(&outcomes);
    if outcomes.iter().any(|outcome| !outcome.passed) {
        exit(1);
    }
}

fn print_outcomes(outcomes: &[ScenarioOutcome]) {
    let mut passed = 0usize;
    for outcome in outcomes {
        if outcome.passed {
            passed += 1;
            println!("ok: {} ({})", outcome.name, outcome.source);
        } else {
            println!("fail: {} ({})", outcome.name, outcome.source);
            for failure in &outcome.failures {
                println!("  - {failure}");
            }
        }
    }
    println!("summary: {passed}/{} passed", outcomes.len());
}

fn usage() {
    println!("scenario_test_runner --scenario <path> | --dir <path>");
}
