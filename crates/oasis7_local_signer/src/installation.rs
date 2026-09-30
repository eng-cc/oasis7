use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use nix::unistd::{getegid, geteuid, getuid};

use crate::error::SignerError;
use crate::identity::sha256_hex;
use crate::local_fs::{read_regular, reject_symlink_components};
use crate::types::{INSTALLATION_SCHEMA, InstallationConfig};

#[cfg(target_os = "macos")]
pub const INSTALLATION_CONFIG_PATH: &str = "/private/etc/oasis7/local-signer-installation.json";
#[cfg(target_os = "linux")]
pub const INSTALLATION_CONFIG_PATH: &str = "/etc/oasis7/local-signer-installation.json";
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const INSTALLATION_CONFIG_PATH: &str = "";
pub const FIXED_SUDO_PATH: &str = "/usr/bin/sudo";
const MAX_INSTALLATION_BYTES: usize = 64 * 1024;

pub fn current_uid() -> u32 {
    getuid().as_raw()
}

pub fn current_egid() -> u32 {
    getegid().as_raw()
}

pub fn current_euid() -> u32 {
    geteuid().as_raw()
}

pub fn require_root_admin() -> Result<(), SignerError> {
    require_root_admin_uid(current_euid())
}

pub(crate) fn require_root_admin_uid(euid: u32) -> Result<(), SignerError> {
    if euid == 0 {
        Ok(())
    } else {
        Err(SignerError::AuthorizationDenied)
    }
}

pub fn fixed_sudo_path() -> &'static Path {
    Path::new(FIXED_SUDO_PATH)
}

pub fn load_fixed_installation_config() -> Result<InstallationConfig, SignerError> {
    if INSTALLATION_CONFIG_PATH.is_empty() {
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    load_installation_config(Path::new(INSTALLATION_CONFIG_PATH))
}

pub fn load_installation_config(path: &Path) -> Result<InstallationConfig, SignerError> {
    reject_symlink_components(path)?;
    let bytes = read_regular(path, MAX_INSTALLATION_BYTES)?;
    let metadata = fs::symlink_metadata(path).map_err(SignerError::PersistenceFailed)?;
    validate_protected_file(&metadata)?;
    let config: InstallationConfig =
        serde_json::from_slice(&bytes).map_err(|_| SignerError::InstallationDrift)?;
    config
        .validate()
        .map_err(|_| SignerError::InstallationDrift)?;
    if config.schema_version != INSTALLATION_SCHEMA {
        return Err(SignerError::InstallationDrift);
    }
    validate_store_identity(&config)?;
    validate_worker_binary(&config)?;
    Ok(config)
}

pub fn trusted_sudo_caller(config: &InstallationConfig) -> Result<u32, SignerError> {
    let sudo_uid = std::env::var("SUDO_UID").ok();
    trusted_sudo_caller_for(config, current_euid(), current_egid(), sudo_uid.as_deref())
}

/// Resolve caller provenance for a worker launched through the fixed sudo
/// boundary. sudo executes the worker as the signer account, while SUDO_UID
/// identifies the account that invoked sudo; these identities are expected to
/// differ and must be checked independently.
fn trusted_sudo_caller_for(
    config: &InstallationConfig,
    effective_uid: u32,
    effective_gid: u32,
    sudo_uid: Option<&str>,
) -> Result<u32, SignerError> {
    if effective_uid != config.signer_uid || effective_gid != config.signer_gid {
        return Err(SignerError::AuthorizationDenied);
    }
    let caller_uid = sudo_uid
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(SignerError::AuthorizationDenied)?;
    if config.caller(caller_uid).is_none() {
        return Err(SignerError::AuthorizationDenied);
    }
    Ok(caller_uid)
}

pub fn validate_fixed_sudo() -> Result<PathBuf, SignerError> {
    let path = fixed_sudo_path();
    reject_symlink_components(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
    if !metadata.is_file() || metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0 {
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    let canonical = fs::canonicalize(path).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
    if canonical != path {
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    Ok(canonical)
}

fn validate_store_identity(config: &InstallationConfig) -> Result<(), SignerError> {
    let store = Path::new(&config.store_dir);
    reject_symlink_components(store)?;
    let metadata = fs::symlink_metadata(store).map_err(|_| SignerError::InstallationDrift)?;
    if !metadata.is_dir()
        || metadata.dev() != config.store_device_id
        || metadata.ino() != config.store_inode
    {
        return Err(SignerError::InstallationDrift);
    }
    Ok(())
}

fn validate_worker_binary(config: &InstallationConfig) -> Result<(), SignerError> {
    let executable = Path::new(&config.worker_executable);
    reject_symlink_components(executable)?;
    let metadata = fs::symlink_metadata(executable).map_err(|_| SignerError::InstallationDrift)?;
    if !metadata.is_file() || metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0 {
        return Err(SignerError::InstallationDrift);
    }
    let bytes = read_regular(executable, 128 * 1024 * 1024)?;
    if sha256_hex(&bytes) != config.worker_sha256 {
        return Err(SignerError::InstallationDrift);
    }
    Ok(())
}

fn validate_protected_file(metadata: &fs::Metadata) -> Result<(), SignerError> {
    if !metadata.is_file() || metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0 {
        return Err(SignerError::InstallationDrift);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CallerBinding;

    fn test_installation() -> InstallationConfig {
        InstallationConfig {
            schema_version: INSTALLATION_SCHEMA.to_owned(),
            installation_id: "install-01".to_owned(),
            deployment_id: "deployment-01".to_owned(),
            store_dir: "/private/var/db/oasis7/store".to_owned(),
            store_device_id: 1,
            store_inode: 2,
            signer_uid: 700,
            signer_gid: 701,
            callers: vec![CallerBinding {
                uid: 501,
                work_subdir: "caller-501".to_owned(),
            }],
            release_id: "release-01".to_owned(),
            worker_executable: "/Library/Application Support/oasis7/worker".to_owned(),
            worker_sha256: "ab".repeat(32),
            control_schema_version: "control-v1".to_owned(),
        }
    }

    #[test]
    fn sudo_caller_is_distinct_from_target_signer_identity() {
        let config = test_installation();
        assert_eq!(
            trusted_sudo_caller_for(&config, 700, 701, Some("501"))
                .expect("configured sudo caller"),
            501
        );
    }

    #[test]
    fn sudo_caller_rejects_missing_malformed_or_unbound_sudo_uid() {
        let config = test_installation();
        for sudo_uid in [
            None,
            Some(""),
            Some("abc"),
            Some("-1"),
            Some("4294967296"),
            Some("700"),
        ] {
            assert!(matches!(
                trusted_sudo_caller_for(&config, 700, 701, sudo_uid),
                Err(SignerError::AuthorizationDenied)
            ));
        }
    }

    #[test]
    fn sudo_caller_still_requires_exact_signer_effective_uid_and_gid() {
        let config = test_installation();
        assert!(matches!(
            trusted_sudo_caller_for(&config, 501, 701, Some("501")),
            Err(SignerError::AuthorizationDenied)
        ));
        assert!(matches!(
            trusted_sudo_caller_for(&config, 700, 702, Some("501")),
            Err(SignerError::AuthorizationDenied)
        ));
    }

    #[test]
    fn root_admin_gate_is_exact_and_caller_cannot_become_admin() {
        assert!(require_root_admin_uid(0).is_ok());
        assert!(matches!(
            require_root_admin_uid(501),
            Err(SignerError::AuthorizationDenied)
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fixed_binding_uses_canonical_macos_etc_path_without_symlink_alias() {
        assert_eq!(
            INSTALLATION_CONFIG_PATH,
            "/private/etc/oasis7/local-signer-installation.json"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn fixed_binding_uses_linux_etc_path() {
        assert_eq!(
            INSTALLATION_CONFIG_PATH,
            "/etc/oasis7/local-signer-installation.json"
        );
    }
}
