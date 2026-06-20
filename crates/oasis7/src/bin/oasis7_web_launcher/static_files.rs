use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) enum StaticAsset {
    Ok {
        content_type: &'static str,
        body: Vec<u8>,
    },
    NotFound,
    InvalidPath,
}

pub(super) fn load_console_static_asset(static_dir: &Path, request_path: &str) -> StaticAsset {
    let relative_path = match normalize_request_path(request_path) {
        Ok(path) => path,
        Err(err) => return err,
    };
    let candidate = static_dir.join(relative_path.as_path());
    let resolved = if candidate.is_dir() {
        candidate.join("index.html")
    } else {
        candidate
    };

    match fs::read(resolved.as_path()) {
        Ok(body) => StaticAsset::Ok {
            content_type: content_type_for_path(resolved.as_path()),
            body,
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if should_fallback_to_index(relative_path.as_path()) {
                let index = static_dir.join("index.html");
                match fs::read(index.as_path()) {
                    Ok(body) => StaticAsset::Ok {
                        content_type: "text/html; charset=utf-8",
                        body,
                    },
                    Err(_) => StaticAsset::NotFound,
                }
            } else {
                StaticAsset::NotFound
            }
        }
        Err(_) => StaticAsset::NotFound,
    }
}

fn normalize_request_path(request_path: &str) -> Result<PathBuf, StaticAsset> {
    if request_path == "/" {
        return Ok(PathBuf::from("index.html"));
    }

    let raw = request_path.strip_prefix('/').unwrap_or(request_path);
    if raw.is_empty() {
        return Ok(PathBuf::from("index.html"));
    }

    let path = Path::new(raw);
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(StaticAsset::InvalidPath);
        }
    }
    Ok(path.to_path_buf())
}

fn should_fallback_to_index(path: &Path) -> bool {
    path.extension().is_none()
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{StaticAsset, load_console_static_asset};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_console_static_asset_rejects_parent_traversal() {
        let static_dir = make_temp_dir("invalid_path");
        let outcome = load_console_static_asset(static_dir.as_path(), "/../secret.txt");
        assert!(matches!(outcome, StaticAsset::InvalidPath));
        let _ = fs::remove_dir_all(static_dir);
    }

    #[test]
    fn load_console_static_asset_serves_index_for_root() {
        let static_dir = make_temp_dir("root_index");
        fs::write(static_dir.join("index.html"), "<html>launcher</html>").expect("write index");

        let outcome = load_console_static_asset(static_dir.as_path(), "/");
        match outcome {
            StaticAsset::Ok { content_type, body } => {
                assert_eq!(content_type, "text/html; charset=utf-8");
                assert_eq!(
                    String::from_utf8_lossy(body.as_slice()),
                    "<html>launcher</html>"
                );
            }
            _ => panic!("expected static asset to be served"),
        }

        let _ = fs::remove_dir_all(static_dir);
    }

    #[test]
    fn load_console_static_asset_falls_back_to_index_for_spa_path() {
        let static_dir = make_temp_dir("spa_fallback");
        fs::write(static_dir.join("index.html"), "<html>spa</html>").expect("write index");

        let outcome = load_console_static_asset(static_dir.as_path(), "/dashboard");
        match outcome {
            StaticAsset::Ok { content_type, body } => {
                assert_eq!(content_type, "text/html; charset=utf-8");
                assert_eq!(String::from_utf8_lossy(body.as_slice()), "<html>spa</html>");
            }
            _ => panic!("expected spa fallback"),
        }

        let _ = fs::remove_dir_all(static_dir);
    }

    fn make_temp_dir(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!(
            "oasis7_oasis7_web_launcher_static_files_{label}_{}_{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
