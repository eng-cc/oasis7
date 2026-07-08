use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use wasm_module_observe::{ObserveRunRequest, run_observe};

fn observe_fixture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_observe_fixture() -> std::sync::MutexGuard<'static, ()> {
    observe_fixture_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

fn modified_fixture_spec() -> serde_json::Value {
    let mut spec: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_spec_path()).expect("read fixture"))
            .expect("parse fixture");
    spec["module"]["manifest_path"] = serde_json::json!(
        fixture_spec_path()
            .parent()
            .expect("fixture spec parent")
            .join("../Cargo.toml")
            .canonicalize()
            .expect("canonicalize fixture manifest")
    );
    spec
}

#[test]
fn observe_runner_executes_m1_rule_move_fixture() {
    let _guard = lock_observe_fixture();

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

#[test]
fn observe_runner_writes_failure_artifact_when_case_validation_fails() {
    let _guard = lock_observe_fixture();

    if !has_target_installed("wasm32-unknown-unknown") {
        eprintln!(
            "skip observe_runner_writes_failure_artifact_when_case_validation_fails: wasm32 target is not installed"
        );
        return;
    }

    let out_dir = unique_temp_dir("wasm-module-observe-failure");
    let spec_path = out_dir.join("module_observe.json");
    std::fs::create_dir_all(&out_dir).expect("create failure observe temp dir");

    let mut spec = modified_fixture_spec();
    spec["cases"][0]["expect"]["emit_count"] = serde_json::json!(99);
    std::fs::write(
        &spec_path,
        serde_json::to_string_pretty(&spec).expect("serialize modified observe spec"),
    )
    .expect("write modified observe spec");

    let err = run_observe(&ObserveRunRequest {
        spec_path,
        out_dir: Some(out_dir.clone()),
    })
    .expect_err("case validation should fail");
    assert!(err.contains("track-agent-registration-event"));

    let failed_summary_path = out_dir.join("failed_summary.json");
    assert!(
        failed_summary_path.exists(),
        "validation failures should preserve machine-readable observe evidence at {}",
        failed_summary_path.display()
    );
    let failed_summary: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&failed_summary_path).expect("read failed summary"),
    )
    .expect("parse failed summary");
    assert_eq!(failed_summary["module_id"], "m1.rule.move");
    assert_eq!(failed_summary["failure"]["stage"], "case");
    assert_eq!(
        failed_summary["failure"]["name"],
        "track-agent-registration-event"
    );
    assert!(
        failed_summary["failure"]["error"]
            .as_str()
            .unwrap()
            .contains("validation failed")
    );
    assert_eq!(
        failed_summary["case_results"][0]["name"],
        "track-agent-registration-event"
    );
    assert!(
        failed_summary["case_results"][0]["perf"]["runs"]
            .as_u64()
            .unwrap()
            >= 1
    );

    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn observe_runner_writes_failure_artifact_when_router_probe_validation_fails() {
    let _guard = lock_observe_fixture();

    if !has_target_installed("wasm32-unknown-unknown") {
        eprintln!(
            "skip observe_runner_writes_failure_artifact_when_router_probe_validation_fails: wasm32 target is not installed"
        );
        return;
    }

    let out_dir = unique_temp_dir("wasm-module-observe-router-failure");
    let spec_path = out_dir.join("module_observe.json");
    std::fs::create_dir_all(&out_dir).expect("create router failure observe temp dir");

    let mut spec = modified_fixture_spec();
    spec["router_probes"][0]["expect_match"] = serde_json::json!(false);
    std::fs::write(
        &spec_path,
        serde_json::to_string_pretty(&spec).expect("serialize modified observe spec"),
    )
    .expect("write modified observe spec");

    let err = run_observe(&ObserveRunRequest {
        spec_path,
        out_dir: Some(out_dir.clone()),
    })
    .expect_err("router probe validation should fail");
    assert!(err.contains("prepared-post-event-match"));

    let failed_summary_path = out_dir.join("failed_summary.json");
    assert!(
        failed_summary_path.exists(),
        "router probe validation failures should preserve machine-readable observe evidence at {}",
        failed_summary_path.display()
    );
    let failed_summary: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&failed_summary_path).expect("read failed summary"),
    )
    .expect("parse failed summary");
    assert_eq!(failed_summary["module_id"], "m1.rule.move");
    assert_eq!(failed_summary["failure"]["stage"], "router_probe");
    assert_eq!(
        failed_summary["failure"]["name"],
        "prepared-post-event-match"
    );
    assert_eq!(failed_summary["failure"]["run_index"], 1);
    assert!(failed_summary["case_results"].as_array().unwrap().len() >= 3);
    assert_eq!(
        failed_summary["router_probe_results"][0]["name"],
        "prepared-post-event-match"
    );
    assert!(
        failed_summary["router_probe_results"][0]["perf"]["runs"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        failed_summary["router_probe_results"][0]["router_delta"]["match_calls_total"]
            .as_u64()
            .unwrap()
            >= 1
    );

    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn observe_runner_writes_failure_artifact_when_router_probe_prepare_fails() {
    let _guard = lock_observe_fixture();

    if !has_target_installed("wasm32-unknown-unknown") {
        eprintln!(
            "skip observe_runner_writes_failure_artifact_when_router_probe_prepare_fails: wasm32 target is not installed"
        );
        return;
    }

    let out_dir = unique_temp_dir("wasm-module-observe-router-prepare-failure");
    let spec_path = out_dir.join("module_observe.json");
    std::fs::create_dir_all(&out_dir).expect("create router prepare failure observe temp dir");

    let mut spec = modified_fixture_spec();
    spec["subscriptions"][0]["filters"] = serde_json::json!({
        "event": [
            { "path": "payload.kind", "eq": "Domain" }
        ]
    });
    std::fs::write(
        &spec_path,
        serde_json::to_string_pretty(&spec).expect("serialize modified observe spec"),
    )
    .expect("write modified observe spec");

    let err = run_observe(&ObserveRunRequest {
        spec_path,
        out_dir: Some(out_dir.clone()),
    })
    .expect_err("router probe prepare should fail");
    assert!(err.contains("prepare router subscriptions failed"));

    let failed_summary_path = out_dir.join("failed_summary.json");
    assert!(
        failed_summary_path.exists(),
        "router probe prepare failures should preserve machine-readable observe evidence at {}",
        failed_summary_path.display()
    );
    let failed_summary: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&failed_summary_path).expect("read failed summary"),
    )
    .expect("parse failed summary");
    assert_eq!(failed_summary["failure"]["stage"], "router_probe_prepare");
    assert_eq!(
        failed_summary["failure"]["name"],
        "prepared-post-event-match"
    );
    assert_eq!(failed_summary["failure"]["run_index"], 0);
    assert!(
        failed_summary["failure"]["error"]
            .as_str()
            .unwrap()
            .contains("path must start with '/'")
    );
    assert_eq!(failed_summary["case_results"].as_array().unwrap().len(), 3);
    assert_eq!(
        failed_summary["router_probe_results"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let _ = std::fs::remove_dir_all(out_dir);
}
