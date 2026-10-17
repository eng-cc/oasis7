use super::*;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(super) fn resolve_oasis7_viewer_live_binary() -> Result<PathBuf, String> {
    resolve_binary_from_env_or_path(
        OASIS7_VIEWER_LIVE_BIN_ENV,
        "oasis7_viewer_live",
        "failed to locate `oasis7_viewer_live` binary; build it first or set",
    )
}

pub(super) fn resolve_oasis7_chain_runtime_binary() -> Result<PathBuf, String> {
    resolve_binary_from_env_or_path(
        OASIS7_CHAIN_RUNTIME_BIN_ENV,
        "oasis7_chain_runtime",
        "failed to locate `oasis7_chain_runtime` binary; build it first or set",
    )
}

pub(super) fn resolve_viewer_static_dir(raw: &str) -> Result<PathBuf, String> {
    let env_override = resolve_non_empty_env_override(GAME_STATIC_DIR_ENV);
    resolve_viewer_static_dir_with_override(
        raw,
        env_override
            .as_ref()
            .map(|(value, env_name)| (value.as_str(), *env_name)),
    )
}

pub(super) fn resolve_viewer_static_dir_with_override(
    raw: &str,
    env_override: Option<(&str, &str)>,
) -> Result<PathBuf, String> {
    if raw == DEFAULT_VIEWER_STATIC_DIR
        && let Some((override_path, env_name)) = env_override
    {
        if let Some(dir) = resolve_viewer_static_dir_candidate(override_path) {
            return Ok(dir);
        }
        return Err(format!(
            "{env_name} is set but viewer static dir not found: `{override_path}`"
        ));
    }

    if let Some(dir) = resolve_viewer_static_dir_candidate(raw) {
        return Ok(dir);
    }

    if raw == DEFAULT_VIEWER_STATIC_DIR
        && let Some(dev_fallback) = viewer_dev_dist_candidates()
            .into_iter()
            .find(|candidate| candidate.is_dir())
    {
        return Ok(dev_fallback);
    }

    Err(format!(
        "viewer static dir not found: `{raw}`; provide --viewer-static-dir <path> (expected trunk build output)"
    ))
}

pub(super) fn viewer_dev_dist_candidates() -> Vec<PathBuf> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    vec![repo_root.join("oasis7_viewer").join("dist")]
}

fn resolve_binary_from_env_or_path(
    env_name: &'static str,
    base_name: &str,
    missing_message: &str,
) -> Result<PathBuf, String> {
    if let Some((path, resolved_env_name)) = resolve_non_empty_env_override(env_name) {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(format!(
            "{resolved_env_name} is set but file does not exist: {}",
            candidate.display()
        ));
    }

    let binary_name = binary_name(base_name);
    let mut candidates = Vec::new();
    if let Ok(current_exe) = env::current_exe()
        && let Some(dir) = current_exe.parent()
    {
        candidates.push(dir.join(&binary_name));
        candidates.push(dir.join("..").join(&binary_name).to_path_buf());
    }

    if let Some(path_entry) = find_on_path(OsStr::new(binary_name.as_str())) {
        candidates.push(path_entry);
    }

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!("{missing_message} {env_name}"))
}

fn resolve_viewer_static_dir_candidate(raw: &str) -> Option<PathBuf> {
    let user_path = PathBuf::from(raw);
    if user_path.is_dir() {
        return Some(user_path);
    }

    if user_path.is_relative()
        && let Ok(current_exe) = env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        let sibling_candidate = bin_dir.join("..").join(&user_path);
        if sibling_candidate.is_dir() {
            return Some(sibling_candidate);
        }
    }
    None
}

fn resolve_non_empty_env_override(env_name: &'static str) -> Option<(String, &'static str)> {
    if let Ok(value) = env::var(env_name) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some((trimmed.to_string(), env_name));
        }
    }
    None
}

fn binary_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn find_on_path(file_name: &OsStr) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
