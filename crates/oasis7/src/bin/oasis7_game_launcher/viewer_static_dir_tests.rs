use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    resolve_viewer_static_dir_with_override, viewer_dev_dist_candidates, DEFAULT_VIEWER_STATIC_DIR,
    GAME_STATIC_DIR_ENV,
};

#[test]
fn resolve_viewer_static_dir_with_override_prefers_env_for_default_static_dir() {
    let override_dir = make_temp_dir("viewer_static_override");
    let override_raw = override_dir.to_string_lossy().to_string();

    let resolved = resolve_viewer_static_dir_with_override(
        DEFAULT_VIEWER_STATIC_DIR,
        Some((override_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect("resolve should succeed");

    assert_eq!(resolved, override_dir);
    let _ = fs::remove_dir_all(override_dir);
}

#[test]
fn resolve_viewer_static_dir_with_override_keeps_explicit_path_priority() {
    let explicit_dir = make_temp_dir("viewer_static_explicit");
    let override_dir = make_temp_dir("viewer_static_override_ignored");
    let explicit_raw = explicit_dir.to_string_lossy().to_string();
    let override_raw = override_dir.to_string_lossy().to_string();

    let resolved = resolve_viewer_static_dir_with_override(
        explicit_raw.as_str(),
        Some((override_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect("resolve should succeed");

    assert_eq!(resolved, explicit_dir);
    let _ = fs::remove_dir_all(explicit_dir);
    let _ = fs::remove_dir_all(override_dir);
}

#[test]
fn resolve_viewer_static_dir_with_override_rejects_missing_env_dir() {
    let missing_path = make_missing_temp_path("viewer_static_missing_env");
    let missing_raw = missing_path.to_string_lossy().to_string();

    let err = resolve_viewer_static_dir_with_override(
        DEFAULT_VIEWER_STATIC_DIR,
        Some((missing_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect_err("missing override path should fail");

    assert!(err.contains(GAME_STATIC_DIR_ENV));
    assert!(err.contains("not found"));
}

#[test]
fn viewer_dev_dist_candidates_only_return_oasis7_path() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let candidates = viewer_dev_dist_candidates();

    assert_eq!(
        candidates,
        vec![repo_root.join("oasis7_viewer").join("dist")]
    );
}

pub(super) fn make_temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_launcher_test_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn make_missing_temp_path(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_launcher_missing_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    path
}
