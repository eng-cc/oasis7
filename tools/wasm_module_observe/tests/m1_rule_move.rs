use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use wasm_module_observe::{run_observe, ObserveRunRequest};

fn fixture_spec_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json",
    )
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn has_target_installed(target: &str) -> bool {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim() == target)
}

#[test]
fn observe_runner_executes_m1_rule_move_fixture() {
    if !has_target_installed("wasm32-unknown-unknown") {
        eprintln!(
            "skip observe_runner_executes_m1_rule_move_fixture: wasm32 target is not installed"
        );
        return;
    }

    let out_dir = unique_temp_dir("wasm-module-observe-m1");
    let output = run_observe(&ObserveRunRequest {
        spec_path: fixture_spec_path(),
        out_dir: Some(out_dir.clone()),
    })
    .expect("run module observe");

    assert_eq!(output.summary.module_id, "m1.rule.move");
    assert_eq!(output.summary.case_results.len(), 3);
    assert_eq!(output.summary.router_probe_results.len(), 2);
    assert!(output.summary.case_results[1].actual.success);
    assert!(output.summary_json_path.exists());
    assert!(output.summary_md_path.exists());

    let _ = std::fs::remove_dir_all(out_dir);
}
