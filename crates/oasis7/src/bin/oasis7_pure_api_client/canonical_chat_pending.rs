use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Original {
    pub version: u32,
    pub endpoint: String,
    pub world: oasis7::world_service::WorldIdentity,
    pub request: AgentChatRequest,
}
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
const MAX_BYTES: u64 = 1024 * 1024;

#[cfg(unix)]
pub(super) fn save(
    chat: &AgentChatRequest,
    world: &oasis7::world_service::WorldIdentity,
    endpoint: &str,
) -> Result<PathBuf, String> {
    let base = state_base(
        std::env::var_os("XDG_STATE_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )?;
    save_at(chat, world, endpoint, &base)
}

#[cfg(unix)]
pub(super) fn state_base(
    xdg: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, String> {
    let base = if let Some(xdg) = xdg {
        PathBuf::from(xdg)
    } else if let Some(home) = home {
        let home = PathBuf::from(home);
        if !home.is_absolute() {
            return Err("HOME must be absolute for durable pending state".into());
        }
        home.join(".local/state")
    } else {
        return Err("durable pending state requires absolute XDG_STATE_HOME or HOME".into());
    };
    if !base.is_absolute() {
        return Err("XDG_STATE_HOME must be absolute for durable pending state".into());
    }
    Ok(base)
}

#[cfg(unix)]
fn validate_ancestors(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path);
        absolute.as_path()
    };
    let owner = unsafe { libc::geteuid() };
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::Normal(_) => {
                current.push(component.as_os_str())
            }
            _ => return Err("pending state path must use absolute normal components".into()),
        }
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        // Root-owned sticky temporary parents protect our owned private entry.
        // This component check is not a directory-descriptor race guarantee.
        let trusted_sticky = metadata.uid() == 0 && metadata.mode() & 0o1000 != 0;
        if !metadata.is_dir()
            || (metadata.uid() != owner && metadata.uid() != 0)
            || (metadata.mode() & 0o022 != 0 && !trusted_sticky)
        {
            return Err(
                "pending state ancestor must be a trusted non-symlink safe directory".into(),
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_directory(path: &Path, private: bool) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    validate_ancestors(path)?;
    if !path.exists() {
        let parent = path.parent().ok_or("state directory parent missing")?;
        ensure_directory(parent, false)?;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|e| e.to_string())?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || (private && metadata.mode() & 0o777 != 0o700)
    {
        return Err("pending state requires owned safe directories; private application directories must be 0700".into());
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn save_at(
    chat: &AgentChatRequest,
    world: &oasis7::world_service::WorldIdentity,
    endpoint: &str,
    base: &Path,
) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    if !base.is_absolute() {
        return Err("pending state base must be absolute".into());
    }
    ensure_directory(base, false)?;
    let app = base.join("oasis7");
    ensure_directory(&app, true)?;
    let root = app.join("canonical-chat");
    ensure_directory(&root, true)?;
    let nonce = chat.auth.as_ref().ok_or("owner proof missing")?.nonce;
    let dir = root.join(format!(
        "oasis7-canonical-chat-{}-{nonce}",
        std::process::id()
    ));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|e| format!("create private pending directory: {e}"))?;
    let path = dir.join("original-request.json");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&Original {
        version: 1,
        endpoint: endpoint.into(),
        world: world.clone(),
        request: chat.clone(),
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("original request exceeds pending artifact limit".into());
    }
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::File::open(&dir)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::File::open(dir.parent().ok_or("pending base missing")?)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(path)
}
#[cfg(not(unix))]
pub(super) fn save(
    _: &AgentChatRequest,
    _: &oasis7::world_service::WorldIdentity,
    _: &str,
) -> Result<PathBuf, String> {
    Err("canonical pending artifacts require Unix private-file permissions on this client".into())
}
#[cfg(unix)]
pub(super) fn load(path: &Path) -> Result<Original, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let parent = path.parent().ok_or("pending artifact parent missing")?;
    validate_ancestors(parent)?;
    let directory = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    // The signed goal is private caller data, even though it contains no private key.
    let owner = unsafe { libc::geteuid() };
    if !directory.is_dir() || directory.uid() != owner || directory.mode() & 0o777 != 0o700 {
        return Err("pending artifact requires an owned private directory".into());
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.uid() != owner
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() > MAX_BYTES
    {
        return Err("pending artifact must be an owned private bounded regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("pending artifact exceeds size limit".into());
    }
    let raw: Value =
        serde_json::from_slice(&bytes).map_err(|_| "invalid pending artifact JSON".to_string())?;
    let chat: Original = serde_json::from_value(raw.clone()).map_err(|_| {
        "invalid pending artifact schema; expected saved canonical request".to_string()
    })?;
    if serde_json::to_value(&chat).map_err(|e| e.to_string())? != raw {
        return Err("pending artifact contains unknown or noncanonical fields".into());
    }
    Ok(chat)
}
#[cfg(not(unix))]
pub(super) fn load(_: &Path) -> Result<Original, String> {
    Err("canonical pending artifact permission validation requires Unix".into())
}
pub(super) fn complete(path: &Path) -> Result<(), String> {
    fs::remove_file(path)
        .map_err(|e| format!("committed; pending artifact removal failed: {e}"))?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        // An externally supplied retry file may share its private directory.
        if parent
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("oasis7-canonical-chat-"))
        {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}
