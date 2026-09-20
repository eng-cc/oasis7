#[cfg(not(target_arch = "wasm32"))]
use std::env;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_launcher_binary_path() -> PathBuf {
    if let Some((_, path)) = crate::read_named_env_value(&["OASIS7_GAME_LAUNCHER_BIN"]) {
        return PathBuf::from(path);
    }

    if let Ok(current_exe) = env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        return bin_dir.join(binary_name("oasis7_game_launcher"));
    }

    PathBuf::from(binary_name("oasis7_game_launcher"))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_launcher_binary_path() -> PathBuf {
    PathBuf::from(binary_name("oasis7_game_launcher"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_chain_runtime_binary_path() -> PathBuf {
    if let Some((_, path)) = crate::read_named_env_value(&["OASIS7_CHAIN_RUNTIME_BIN"]) {
        return PathBuf::from(path);
    }

    if let Ok(current_exe) = env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        return bin_dir.join(binary_name("oasis7_chain_runtime"));
    }

    PathBuf::from(binary_name("oasis7_chain_runtime"))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_chain_runtime_binary_path() -> PathBuf {
    PathBuf::from(binary_name("oasis7_chain_runtime"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_web_launcher_binary_path() -> PathBuf {
    if let Some((_, path)) = crate::read_named_env_value(&["OASIS7_WEB_LAUNCHER_BIN"]) {
        return PathBuf::from(path);
    }

    if let Ok(current_exe) = env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        return bin_dir.join(binary_name("oasis7_web_launcher"));
    }

    PathBuf::from(binary_name("oasis7_web_launcher"))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_web_launcher_binary_path() -> PathBuf {
    PathBuf::from(binary_name("oasis7_web_launcher"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_static_dir_path() -> PathBuf {
    if let Some((_, path)) = crate::read_named_env_value(&["OASIS7_GAME_STATIC_DIR"]) {
        return PathBuf::from(path);
    }

    let mut candidates = Vec::new();
    if let Ok(current_exe) = env::current_exe()
        && let Some(bin_dir) = current_exe.parent()
    {
        candidates.push(bin_dir.join("..").join("web"));
        candidates.push(bin_dir.join("..").join("..").join("web"));
    }
    candidates.extend(viewer_dev_dist_candidates());
    candidates.push(PathBuf::from("web"));

    first_existing_dir(candidates).unwrap_or_else(|| PathBuf::from("web"))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_static_dir_path() -> PathBuf {
    PathBuf::from("web")
}

pub(crate) fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().ok_or_else(|| "window is not available".to_string())?;
        window
            .open_with_url(url)
            .map_err(|err| format!("window.open failed: {err:?}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(url)
            .status()
            .map_err(|err| format!("run open failed: {err}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("open exited with {status}"))
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(url)
            .status()
            .map_err(|err| format!("run cmd /C start failed: {err}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("cmd /C start exited with {status}"));
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        not(target_os = "windows"),
        not(target_os = "macos")
    ))]
    {
        let status = Command::new("xdg-open")
            .arg(url)
            .status()
            .map_err(|err| format!("run xdg-open failed: {err}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("xdg-open exited with {status}"))
    }
}

fn binary_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn first_existing_dir(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.is_dir())
}

pub(crate) fn viewer_dev_dist_candidates() -> Vec<PathBuf> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    vec![repo_root.join("oasis7_viewer").join("dist")]
}

#[cfg(test)]
mod tests {
    use super::{first_existing_dir, viewer_dev_dist_candidates};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn first_existing_dir_selects_first_existing_path() {
        let missing = make_temp_path("missing");
        let valid = make_temp_path("valid");
        fs::create_dir_all(&valid).expect("create valid dir");

        let resolved = first_existing_dir(vec![missing, valid.clone()]);
        assert_eq!(resolved, Some(valid.clone()));

        let _ = fs::remove_dir_all(valid);
    }

    #[test]
    fn first_existing_dir_returns_none_without_existing_dirs() {
        let resolved = first_existing_dir(vec![
            make_temp_path("missing_a"),
            make_temp_path("missing_b"),
        ]);
        assert!(resolved.is_none());
    }

    #[test]
    fn viewer_dev_dist_candidates_only_return_current_oasis7_name() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let candidates = viewer_dev_dist_candidates();

        assert_eq!(
            candidates,
            vec![repo_root.join("oasis7_viewer").join("dist")]
        );
    }

    fn make_temp_path(label: &str) -> PathBuf {
        let mut path = env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!(
            "oasis7_platform_ops_{label}_{}_{}",
            std::process::id(),
            stamp
        ));
        path
    }
}
