use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;

pub(super) const NODE_TABLE_KEY: &str = "node";
pub(super) const NODE_PRIVATE_KEY_FIELD: &str = "private_key";
pub(super) const NODE_PUBLIC_KEY_FIELD: &str = "public_key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NodeKeypairConfig {
    pub private_key_hex: String,
    pub public_key_hex: String,
}

pub(super) fn ensure_node_keypair_in_config(path: &Path) -> Result<NodeKeypairConfig, String> {
    let _lock = ConfigFileLock::acquire(path)?;
    ensure_node_keypair_in_config_unlocked(path)
}

fn ensure_node_keypair_in_config_unlocked(path: &Path) -> Result<NodeKeypairConfig, String> {
    let mut table = load_config_table(path)?;
    let mut wrote = false;

    let node_table = table
        .entry(NODE_TABLE_KEY.to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let node_table = node_table
        .as_table_mut()
        .ok_or_else(|| "config field 'node' must be a table".to_string())?;

    let existing_private = node_table
        .get(NODE_PRIVATE_KEY_FIELD)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let existing_public = node_table
        .get(NODE_PUBLIC_KEY_FIELD)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let keypair = match (existing_private, existing_public) {
        (Some(private_hex), Some(public_hex)) => {
            validate_node_keypair_hex(private_hex.as_str(), public_hex.as_str())?;
            NodeKeypairConfig {
                private_key_hex: private_hex,
                public_key_hex: public_hex,
            }
        }
        (Some(private_hex), None) => {
            let signing_key = signing_key_from_hex(private_hex.as_str())?;
            let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
            node_table.insert(
                NODE_PUBLIC_KEY_FIELD.to_string(),
                toml::Value::String(public_key_hex.clone()),
            );
            wrote = true;
            NodeKeypairConfig {
                private_key_hex: private_hex,
                public_key_hex,
            }
        }
        _ => {
            let signing_key = SigningKey::generate(&mut rand_core::OsRng);
            let private_key_hex = hex::encode(signing_key.to_bytes());
            let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
            node_table.insert(
                NODE_PRIVATE_KEY_FIELD.to_string(),
                toml::Value::String(private_key_hex.clone()),
            );
            node_table.insert(
                NODE_PUBLIC_KEY_FIELD.to_string(),
                toml::Value::String(public_key_hex.clone()),
            );
            wrote = true;
            NodeKeypairConfig {
                private_key_hex,
                public_key_hex,
            }
        }
    };

    if wrote {
        write_config_table(path, &table)?;
    }
    Ok(keypair)
}

struct ConfigFileLock {
    path: PathBuf,
}

impl ConfigFileLock {
    fn acquire(config_path: &Path) -> Result<Self, String> {
        let path = config_lock_path(config_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "create config lock parent dir {} failed: {}",
                        parent.display(),
                        err
                    )
                })?;
            }
        }

        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Self { path }),
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        return Err(format!("timed out waiting for config lock {}", path.display()));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => {
                    return Err(format!("create config lock {} failed: {}", path.display(), err));
                }
            }
        }
    }
}

impl Drop for ConfigFileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn config_lock_path(config_path: &Path) -> PathBuf {
    let mut lock_path = config_path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

fn load_config_table(path: &Path) -> Result<toml::map::Map<String, toml::Value>, String> {
    if !path.exists() {
        return Ok(toml::map::Map::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|err| format!("read {} failed: {}", path.display(), err))?;
    if content.trim().is_empty() {
        return Ok(toml::map::Map::new());
    }

    let value: toml::Value = toml::from_str(content.as_str())
        .map_err(|err| format!("parse {} failed: {}", path.display(), err))?;
    value
        .as_table()
        .cloned()
        .ok_or_else(|| format!("{} root must be a table", path.display()))
}

fn write_config_table(
    path: &Path,
    table: &toml::map::Map<String, toml::Value>,
) -> Result<(), String> {
    let content = toml::to_string_pretty(table)
        .map_err(|err| format!("serialize {} failed: {}", path.display(), err))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "create config parent dir {} failed: {}",
                    parent.display(),
                    err
                )
            })?;
        }
    }
    fs::write(path, content).map_err(|err| format!("write {} failed: {}", path.display(), err))
}

fn validate_node_keypair_hex(private_key_hex: &str, public_key_hex: &str) -> Result<(), String> {
    let signing_key = signing_key_from_hex(private_key_hex)?;
    let expected_public_hex = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_hex != public_key_hex {
        return Err("node.public_key does not match node.private_key".to_string());
    }
    Ok(())
}

fn signing_key_from_hex(private_key_hex: &str) -> Result<SigningKey, String> {
    let private_bytes = hex::decode(private_key_hex)
        .map_err(|_| "node.private_key must be valid hex".to_string())?;
    let private_array: [u8; 32] = private_bytes
        .try_into()
        .map_err(|_| "node.private_key must be 32-byte hex".to_string())?;
    Ok(SigningKey::from_bytes(&private_array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("oasis7-node-keypair-config-{label}-{unique}"))
            .join("config.toml")
    }

    #[test]
    fn concurrent_first_writers_share_one_generated_keypair() {
        let config_path = temp_config_path("concurrent");
        let lock_path = config_lock_path(&config_path);
        let barrier = Arc::new(Barrier::new(8));
        let handles = (0..8)
            .map(|_| {
                let config_path = config_path.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    ensure_node_keypair_in_config(&config_path).expect("ensure keypair")
                })
            })
            .collect::<Vec<_>>();

        let keypairs = handles
            .into_iter()
            .map(|handle| handle.join().expect("join keypair worker"))
            .collect::<Vec<_>>();
        let first = keypairs.first().expect("first keypair");
        assert!(keypairs.iter().all(|keypair| keypair == first));
        assert!(!lock_path.exists(), "config lock should be removed");

        let persisted = ensure_node_keypair_in_config(&config_path).expect("read persisted keypair");
        assert_eq!(persisted, *first);
        let _ = fs::remove_dir_all(config_path.parent().expect("config parent"));
    }
}
