use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::control::{
    MAINTENANCE_FILE, MAINTENANCE_SCHEMA, MaintenanceRecord, parse_grant_candidate,
    parse_policy_candidate,
};
use crate::error::SignerError;
use crate::identity::{canonical_json, operation_key, parse_sha256_hex, sha256_hex};
use crate::installation::{load_fixed_installation_config, require_root_admin};
use crate::local_fs::{
    CustodyLock, PublishOptions, atomic_publish, create_private_dir, reject_symlink_components,
    set_path_owner_mode, sync_dir, sync_file_and_parent, write_private_new,
};
use crate::protocol::validate_id;
use crate::store::{
    GRANTS_DIR, LOCK_FILE, POLICY_FILE, REVOKED_GRANTS_DIR, read_control, read_control_bytes,
    read_owned_key_file, validate_owned_directory, validate_store_layout,
};
use crate::types::{BatchGrant, InstallationConfig, Policy};

const MAX_CONTROL_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// The `After` prefix records fault-injection timing at each durability boundary.
#[allow(clippy::enum_variant_names)]
pub(crate) enum AdminFaultPoint {
    AfterIntentSync,
    AfterTargetPublishBeforeParentSync,
    AfterCreateOnlyLinkBeforeUnlink,
}

/// Root-only post-install control-plane access to a fixed local signer store.
///
/// This type does not create OS accounts, installation bindings, directories,
/// sudo rules, or backups. Those operations require a platform-specific
/// installer and are deliberately outside this API.
#[derive(Debug)]
pub struct AdminStore {
    pub(crate) installation: InstallationConfig,
    pub(crate) root: PathBuf,
    pub(crate) signer_uid: u32,
    pub(crate) signer_gid: u32,
    pub(crate) control_owner_uid: u32,
    pub(crate) control_group_gid: u32,
    #[cfg(test)]
    allow_nonroot_test_fixture: bool,
    #[cfg(test)]
    pub(crate) test_fault: std::cell::Cell<Option<AdminFaultPoint>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RevocationRecord {
    schema_version: String,
    installation_id: String,
    grant_id: String,
    revoked_at_ms: u64,
}

impl AdminStore {
    /// Opens the fixed installation for a directly invoked root admin command.
    /// Admin must never be added to the caller's no-password sudo allowlist.
    pub fn open_fixed_root() -> Result<Self, SignerError> {
        require_root_admin()?;
        let installation = load_fixed_installation_config()?;
        let root = PathBuf::from(&installation.store_dir);
        validate_store_layout(&root, installation.signer_uid, installation.signer_gid)?;
        let signer_uid = installation.signer_uid;
        let signer_gid = installation.signer_gid;
        Ok(Self {
            signer_uid,
            signer_gid,
            installation,
            root,
            control_owner_uid: 0,
            control_group_gid: signer_gid,
            #[cfg(test)]
            allow_nonroot_test_fixture: false,
            #[cfg(test)]
            test_fault: std::cell::Cell::new(None),
        })
    }

    /// Creates a purpose-scoped key and returns its public key only.
    ///
    /// M0 currently provisions rollback strict-audit keys. The secret is held
    /// in zeroizing memory, written create-only, and never returned or logged.
    pub fn create_key(&self, signer_id: &str, purpose: &str) -> Result<[u8; 32], SignerError> {
        self.ensure_root_admin()?;
        self.validate_signer_id(signer_id)?;
        if purpose != "rollback_strict_audit_v1" {
            return Err(SignerError::InvalidInput(
                "unsupported key purpose for M0".to_owned(),
            ));
        }
        let _lock = self.acquire_lock()?;
        crate::key_management::reject_catalog_id(self, signer_id)?;
        let keys_dir = self.root.join("keys");
        let key_dir = keys_dir.join(signer_id);
        let target_path = self.target_relative(&key_dir)?;
        let action = ["key-create", signer_id, purpose];
        let action_key = self.action_key(&action)?;
        if let Some(record) = self.pending_intent(&action_key, &target_path)? {
            let public_key = if path_exists(&key_dir)? {
                self.validate_key_directory(&key_dir)?
            } else {
                let staging =
                    keys_dir.join(format!(".{signer_id}.staging-{}", record.transaction_id));
                if !path_exists(&staging)? {
                    return Err(SignerError::RecoveryRequired);
                }
                let public_key = self
                    .validate_key_directory(&staging)
                    .map_err(|_| SignerError::RecoveryRequired)?;
                if sha256_hex(&public_key) != record.target_sha256 {
                    return Err(SignerError::RecoveryRequired);
                }
                fs::rename(&staging, &key_dir).map_err(SignerError::PersistenceFailed)?;
                self.maybe_inject_fault(AdminFaultPoint::AfterTargetPublishBeforeParentSync)?;
                sync_dir(&keys_dir)?;
                public_key
            };
            if sha256_hex(&public_key) != record.target_sha256 {
                return Err(SignerError::RecoveryRequired);
            }
            sync_file_and_parent(&key_dir.join("seed.bin"))?;
            sync_file_and_parent(&key_dir.join("public.bin"))?;
            sync_dir(&key_dir)?;
            sync_dir(&keys_dir)?;
            self.commit_intent(&record)?;
            return Ok(public_key);
        }
        if path_exists(&key_dir)? {
            return Err(SignerError::IdConflict);
        }

        let mut seed = Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut *seed).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = signing_key.verifying_key().to_bytes();
        let record =
            self.begin_intent(action_key, target_path, sha256_hex(&public_key), None, None)?;
        let staging = keys_dir.join(format!(".{signer_id}.staging-{}", record.transaction_id));
        create_private_dir(&staging)?;
        write_private_new(&staging.join("seed.bin"), &*seed)?;
        write_private_new(&staging.join("public.bin"), &public_key)?;
        set_path_owner_mode(
            &staging.join("seed.bin"),
            self.signer_uid,
            self.signer_gid,
            0o600,
        )?;
        set_path_owner_mode(
            &staging.join("public.bin"),
            self.signer_uid,
            self.signer_gid,
            0o600,
        )?;
        sync_dir(&staging)?;
        set_path_owner_mode(&staging, self.signer_uid, self.signer_gid, 0o700)?;
        if self.validate_key_directory(&staging)? != public_key {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        fs::rename(&staging, &key_dir).map_err(SignerError::PersistenceFailed)?;
        self.maybe_inject_fault(AdminFaultPoint::AfterTargetPublishBeforeParentSync)?;
        sync_dir(&keys_dir)?;
        self.validate_key_files(signer_id)?;
        self.commit_intent(&record)?;
        Ok(public_key)
    }

    /// Reads a key's raw 32-byte public key after validating its file custody.
    pub fn key_public(&self, signer_id: &str) -> Result<[u8; 32], SignerError> {
        self.ensure_root_admin()?;
        self.validate_signer_id(signer_id)?;
        let bytes = self.read_public_key(signer_id)?;
        bytes
            .as_slice()
            .try_into()
            .map_err(|_| SignerError::KeyOrAuthorityUnavailable)
    }

    /// Installs an exact-digest policy candidate. New candidates must start
    /// disabled; enabling is a separate explicit root-only operation.
    pub fn install_policy_candidate(
        &self,
        bytes: &[u8],
        expected_sha256: &str,
    ) -> Result<(), SignerError> {
        self.ensure_root_admin()?;
        if bytes.len() > MAX_CONTROL_BYTES {
            return Err(SignerError::InvalidInput(
                "policy candidate exceeds the size limit".to_owned(),
            ));
        }
        let candidate = parse_policy_candidate(bytes, expected_sha256, &self.installation)?;
        if !candidate.enabled_purposes.is_empty() {
            return Err(SignerError::InvalidInput(
                "policy candidate must install with all purposes disabled".to_owned(),
            ));
        }

        let _lock = self.acquire_lock()?;
        let path = self.root.join(POLICY_FILE);
        if path_exists(&path)? {
            let current: Policy =
                read_control(&path, self.control_owner_uid, self.control_group_gid)?;
            let current_bytes = canonical_json(&current)?;
            if current_bytes != bytes && current.policy_revision == candidate.policy_revision {
                return Err(SignerError::IdConflict);
            }
        }
        self.mutate_control_file(
            &["policy-install", expected_sha256],
            &path,
            bytes,
            false,
            None,
        )
    }

    /// Installs an approved exact-batch grant. Existing IDs are immutable and
    /// revoked IDs cannot be reinstalled.
    pub fn install_grant_candidate(
        &self,
        bytes: &[u8],
        expected_sha256: &str,
    ) -> Result<(), SignerError> {
        self.ensure_root_admin()?;
        if bytes.len() > MAX_CONTROL_BYTES {
            return Err(SignerError::InvalidInput(
                "grant candidate exceeds the size limit".to_owned(),
            ));
        }
        let _lock = self.acquire_lock()?;
        let policy: Policy = read_control(
            &self.root.join(POLICY_FILE),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        let grant = parse_grant_candidate(bytes, expected_sha256, &self.installation, &policy)?;
        self.validate_grant_bindings(&grant, &policy)?;

        let grant_path = self
            .root
            .join(GRANTS_DIR)
            .join(format!("{}.json", grant.grant_id));
        let revoked_path = self
            .root
            .join(REVOKED_GRANTS_DIR)
            .join(format!("{}.json", grant.grant_id));
        if path_exists(&revoked_path)? {
            return Err(SignerError::AuthorizationDenied);
        }
        let pending_action = ["grant-install", &grant.grant_id, expected_sha256];
        let pending = self.pending_intent(
            &self.action_key(&pending_action)?,
            &self.target_relative(&grant_path)?,
        )?;
        if path_exists(&grant_path)? {
            let current_bytes = if pending.is_some() {
                self.read_control_target(&grant_path, true)?
            } else {
                let current: BatchGrant =
                    read_control(&grant_path, self.control_owner_uid, self.control_group_gid)?;
                canonical_json(&current)?
            };
            if current_bytes != bytes {
                return Err(if pending.is_some() {
                    SignerError::RecoveryRequired
                } else {
                    SignerError::IdConflict
                });
            }
        }
        self.mutate_control_file(&pending_action, &grant_path, bytes, true, None)
    }

    /// Writes an immutable revocation marker; the grant record is retained.
    pub fn revoke_grant(&self, grant_id: &str) -> Result<(), SignerError> {
        self.ensure_root_admin()?;
        validate_id(grant_id).map_err(|error| SignerError::InvalidInput(error.to_string()))?;
        let _lock = self.acquire_lock()?;
        let grant_path = self.root.join(GRANTS_DIR).join(format!("{grant_id}.json"));
        let _: BatchGrant =
            read_control(&grant_path, self.control_owner_uid, self.control_group_gid)?;
        let path = self
            .root
            .join(REVOKED_GRANTS_DIR)
            .join(format!("{grant_id}.json"));
        let action = ["grant-revoke", grant_id];
        let action_key = self.action_key(&action)?;
        let target_relative = self.target_relative(&path)?;
        let pending = self.pending_intent(&action_key, &target_relative)?;
        let existing = if path_exists(&path)? {
            let marker: RevocationRecord = if pending.is_some() {
                let bytes = self.read_control_target(&path, true)?;
                serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?
            } else {
                read_control(&path, self.control_owner_uid, self.control_group_gid)?
            };
            if marker.installation_id != self.installation.installation_id
                || marker.grant_id != grant_id
                || marker.schema_version != "oasis7.local_signer_revocation.v1"
                || marker.revoked_at_ms == 0
            {
                return Err(SignerError::RecoveryRequired);
            }
            Some(marker.revoked_at_ms)
        } else {
            None
        };
        let revoked_at_ms = match (&pending, existing) {
            (Some(record), _) => record.timestamp_ms.ok_or(SignerError::RecoveryRequired)?,
            (None, Some(timestamp)) => timestamp,
            (None, None) => unix_time_ms()?,
        };
        let marker = RevocationRecord {
            schema_version: "oasis7.local_signer_revocation.v1".to_owned(),
            installation_id: self.installation.installation_id.clone(),
            grant_id: grant_id.to_owned(),
            revoked_at_ms,
        };
        let bytes = canonical_json(&marker)?;
        self.mutate_control_file(&action, &path, &bytes, true, Some(revoked_at_ms))
    }

    /// Explicitly enables or disables one already-bound purpose in policy.
    pub fn set_purpose_enabled(&self, purpose: &str, enabled: bool) -> Result<(), SignerError> {
        self.ensure_root_admin()?;
        validate_id(purpose).map_err(|error| SignerError::InvalidInput(error.to_string()))?;
        let _lock = self.acquire_lock()?;
        let path = self.root.join(POLICY_FILE);
        let current_bytes =
            read_control_bytes(&path, self.control_owner_uid, self.control_group_gid)?;
        let mut policy: Policy =
            serde_json::from_slice(&current_bytes).map_err(|_| SignerError::RecoveryRequired)?;
        policy
            .validate(&self.installation)
            .map_err(|_| SignerError::InstallationDrift)?;
        if enabled {
            let bindings: Vec<_> = policy
                .key_bindings
                .iter()
                .filter(|binding| binding.purpose == purpose)
                .collect();
            if bindings.is_empty() {
                return Err(SignerError::KeyOrAuthorityUnavailable);
            }
            for binding in bindings {
                let public_key = self.read_public_key(&binding.signer_id)?;
                if sha256_hex(&public_key) != binding.public_key_sha256 {
                    return Err(SignerError::CryptoOrBindingInvalid);
                }
                self.validate_key_files(&binding.signer_id)?;
            }
            if !policy.enabled_purposes.iter().any(|value| value == purpose) {
                policy.enabled_purposes.push(purpose.to_owned());
                policy.enabled_purposes.sort();
            }
        } else {
            policy.enabled_purposes.retain(|value| value != purpose);
        }
        policy
            .validate(&self.installation)
            .map_err(SignerError::InvalidInput)?;
        let bytes = canonical_json(&policy)?;
        let action = if enabled {
            ["purpose-enable", purpose]
        } else {
            ["purpose-disable", purpose]
        };
        self.mutate_control_file(&action, &path, &bytes, false, None)
    }

    pub(crate) fn acquire_lock(&self) -> Result<CustodyLock, SignerError> {
        CustodyLock::acquire(&self.root.join(LOCK_FILE), self.signer_uid, self.signer_gid)
    }

    pub(crate) fn ensure_root_admin(&self) -> Result<(), SignerError> {
        #[cfg(test)]
        if self.allow_nonroot_test_fixture {
            return Ok(());
        }
        require_root_admin()
    }

    pub(crate) fn validate_signer_id(&self, signer_id: &str) -> Result<(), SignerError> {
        validate_id(signer_id).map_err(|error| SignerError::InvalidInput(error.to_string()))
    }

    fn read_public_key(&self, signer_id: &str) -> Result<Vec<u8>, SignerError> {
        self.validate_signer_id(signer_id)?;
        let key_dir = self.root.join("keys").join(signer_id);
        reject_symlink_components(&key_dir)?;
        validate_owned_directory(&key_dir, self.signer_uid, self.signer_gid, 0o700)
            .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
        read_owned_key_file(
            &key_dir.join("public.bin"),
            self.signer_uid,
            self.signer_gid,
        )
    }

    pub(crate) fn validate_key_files(&self, signer_id: &str) -> Result<(), SignerError> {
        self.validate_key_directory(&self.root.join("keys").join(signer_id))?;
        Ok(())
    }

    pub(crate) fn validate_key_directory(&self, key_dir: &Path) -> Result<[u8; 32], SignerError> {
        reject_symlink_components(key_dir)?;
        validate_owned_directory(key_dir, self.signer_uid, self.signer_gid, 0o700)
            .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
        let public_key = read_owned_key_file(
            &key_dir.join("public.bin"),
            self.signer_uid,
            self.signer_gid,
        )?;
        let public_bytes: [u8; 32] = public_key
            .as_slice()
            .try_into()
            .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
        let seed_bytes = Zeroizing::new(read_owned_key_file(
            &key_dir.join("seed.bin"),
            self.signer_uid,
            self.signer_gid,
        )?);
        let seed: Zeroizing<[u8; 32]> = Zeroizing::new(
            seed_bytes
                .as_slice()
                .try_into()
                .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?,
        );
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = VerifyingKey::from_bytes(&public_bytes)
            .map_err(|_| SignerError::CryptoOrBindingInvalid)?;
        if signing_key.verifying_key() != verifying_key {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        Ok(public_bytes)
    }

    fn validate_grant_bindings(
        &self,
        grant: &BatchGrant,
        policy: &Policy,
    ) -> Result<(), SignerError> {
        for item in &grant.items {
            let mut bindings = policy.key_bindings.iter().filter(|binding| {
                binding.purpose == item.purpose && binding.signer_id == item.signer_id
            });
            let binding = bindings.next().ok_or(SignerError::AuthorizationDenied)?;
            if bindings.next().is_some() {
                return Err(SignerError::AuthorizationDenied);
            }
            if binding.public_key_sha256 != item.public_key_sha256
                || (!binding.provider_ids.is_empty()
                    && item
                        .provider_id
                        .0
                        .as_ref()
                        .is_none_or(|provider| !binding.provider_ids.contains(provider)))
                || !binding
                    .protocol_authorities
                    .contains(&item.protocol_bindings.authority_id)
                || !binding.deployment_ids.contains(&grant.deployment_id)
                || !binding.network_ids.contains(&grant.network_id)
            {
                return Err(SignerError::AuthorizationDenied);
            }
            let payload_digest = parse_sha256_hex(&item.payload_sha256)?;
            let expected_operation_key = operation_key(
                &grant.installation_id,
                &grant.deployment_id,
                &grant.network_id,
                &item.purpose,
                &item.signer_id,
                &payload_digest,
            )?;
            if expected_operation_key != item.operation_key {
                return Err(SignerError::CryptoOrBindingInvalid);
            }
        }
        Ok(())
    }

    pub(crate) fn action_key(&self, arguments: &[&str]) -> Result<String, SignerError> {
        let arguments: Vec<String> = arguments.iter().map(|value| (*value).to_owned()).collect();
        Ok(sha256_hex(&canonical_json(&arguments)?))
    }

    pub(crate) fn target_relative(&self, path: &Path) -> Result<String, SignerError> {
        path.strip_prefix(&self.root)
            .ok()
            .and_then(Path::to_str)
            .map(|value| value.to_owned())
            .ok_or(SignerError::InstallationDrift)
    }

    pub(crate) fn pending_intent(
        &self,
        operation_key: &str,
        target_path: &str,
    ) -> Result<Option<MaintenanceRecord>, SignerError> {
        let Some(record) = self.read_maintenance()? else {
            return Ok(None);
        };
        if record.phase == "committed" {
            return Ok(None);
        }
        if record.phase != "prepared"
            || record.operation_key != operation_key
            || record.target_path != target_path
        {
            return Err(SignerError::RecoveryRequired);
        }
        Ok(Some(record))
    }

    fn validate_maintenance_record(&self, record: &MaintenanceRecord) -> Result<(), SignerError> {
        if record.schema_version != MAINTENANCE_SCHEMA
            || record.installation_id != self.installation.installation_id
            || !matches!(record.phase.as_str(), "prepared" | "committed")
            || parse_sha256_hex(&record.operation_key).is_err()
            || parse_sha256_hex(&record.target_sha256).is_err()
            || record
                .base_sha256
                .as_deref()
                .is_some_and(|digest| parse_sha256_hex(digest).is_err())
            || validate_id(&record.transaction_id).is_err()
        {
            return Err(SignerError::RecoveryRequired);
        }
        self.maintenance_target_path(&record.target_path)?;
        Ok(())
    }

    pub(crate) fn begin_intent(
        &self,
        operation_key: String,
        target_path: String,
        target_sha256: String,
        base_sha256: Option<String>,
        timestamp_ms: Option<u64>,
    ) -> Result<MaintenanceRecord, SignerError> {
        let path = self.root.join(MAINTENANCE_FILE);
        if let Some(current) = self.read_maintenance()? {
            if current.phase == "prepared" {
                return Err(SignerError::RecoveryRequired);
            }
        }
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
        let record = MaintenanceRecord {
            schema_version: MAINTENANCE_SCHEMA.to_owned(),
            installation_id: self.installation.installation_id.clone(),
            transaction_id: hex::encode(random),
            phase: "prepared".to_owned(),
            operation_key,
            target_path,
            target_sha256,
            base_sha256,
            timestamp_ms,
        };
        let bytes = canonical_json(&record)?;
        atomic_publish(
            &path,
            &bytes,
            PublishOptions {
                owner_uid: self.control_owner_uid,
                owner_gid: self.control_group_gid,
                mode: 0o640,
                replace: true,
            },
        )?;
        self.maybe_inject_fault(AdminFaultPoint::AfterIntentSync)?;
        Ok(record)
    }

    fn prepare_control_intent(
        &self,
        action_arguments: &[&str],
        target: &Path,
        bytes: &[u8],
        create_only: bool,
        timestamp_ms: Option<u64>,
    ) -> Result<MaintenanceRecord, SignerError> {
        let operation_key = self.action_key(action_arguments)?;
        let target_path = self.target_relative(target)?;
        if let Some(record) = self.pending_intent(&operation_key, &target_path)? {
            if record.target_sha256 != sha256_hex(bytes) {
                return Err(SignerError::RecoveryRequired);
            }
            return Ok(record);
        }
        let current = if path_exists(target)? {
            Some(self.read_control_target(target, create_only)?)
        } else {
            None
        };
        if create_only && current.as_deref().is_some_and(|current| current != bytes) {
            return Err(SignerError::IdConflict);
        }
        self.begin_intent(
            operation_key,
            target_path,
            sha256_hex(bytes),
            current.as_deref().map(sha256_hex),
            timestamp_ms,
        )
    }

    pub(crate) fn mutate_control_file(
        &self,
        action_arguments: &[&str],
        target: &Path,
        bytes: &[u8],
        create_only: bool,
        timestamp_ms: Option<u64>,
    ) -> Result<(), SignerError> {
        let record = self.prepare_control_intent(
            action_arguments,
            target,
            bytes,
            create_only,
            timestamp_ms,
        )?;
        self.publish_recorded_control(&record, target, bytes, !create_only)?;
        self.commit_intent(&record)
    }

    pub(crate) fn read_maintenance(&self) -> Result<Option<MaintenanceRecord>, SignerError> {
        let path = self.root.join(MAINTENANCE_FILE);
        if !path_exists(&path)? {
            return Ok(None);
        }
        let record: MaintenanceRecord =
            read_control(&path, self.control_owner_uid, self.control_group_gid)?;
        self.validate_maintenance_record(&record)?;
        if record.phase == "committed" {
            self.validate_committed_target(&record)?;
        }
        Ok(Some(record))
    }

    fn maintenance_target_path(&self, relative: &str) -> Result<PathBuf, SignerError> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(SignerError::RecoveryRequired);
        }
        let components: Vec<_> = path
            .components()
            .filter_map(|component| match component {
                std::path::Component::Normal(value) => value.to_str(),
                _ => None,
            })
            .collect();
        let permitted = match components.as_slice() {
            [
                "control",
                "policy.json" | "key-catalog.json" | "restore.json",
            ] => true,
            ["control", "grants" | "revoked-grants", file] => file
                .strip_suffix(".json")
                .is_some_and(|id| validate_id(id).is_ok()),
            ["keys", signer_id] => validate_id(signer_id).is_ok(),
            _ => false,
        };
        if !permitted {
            return Err(SignerError::RecoveryRequired);
        }
        Ok(self.root.join(path))
    }

    fn validate_committed_target(&self, record: &MaintenanceRecord) -> Result<(), SignerError> {
        let target = self.maintenance_target_path(&record.target_path)?;
        if record.target_path.starts_with("keys/") {
            let public_key = self.validate_key_directory(&target)?;
            if sha256_hex(&public_key) != record.target_sha256 {
                return Err(SignerError::RecoveryRequired);
            }
        } else {
            let bytes =
                read_control_bytes(&target, self.control_owner_uid, self.control_group_gid)?;
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
            if canonical_json(&value)? != bytes || sha256_hex(&bytes) != record.target_sha256 {
                return Err(SignerError::RecoveryRequired);
            }
        }
        Ok(())
    }

    fn publish_recorded_control(
        &self,
        record: &MaintenanceRecord,
        target: &Path,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), SignerError> {
        if self.target_relative(target)? != record.target_path
            || sha256_hex(bytes) != record.target_sha256
        {
            return Err(SignerError::RecoveryRequired);
        }
        let current = if path_exists(target)? {
            Some(self.read_control_target(target, !replace)?)
        } else {
            None
        };
        if current.as_deref() == Some(bytes) {
            return sync_file_and_parent(target);
        }
        if current.as_ref().map(|value| sha256_hex(value)) != record.base_sha256 {
            return Err(SignerError::RecoveryRequired);
        }
        crate::local_fs::atomic_publish_with_hooks(
            target,
            bytes,
            PublishOptions {
                owner_uid: self.control_owner_uid,
                owner_gid: self.control_group_gid,
                mode: 0o640,
                replace,
            },
            || {
                if replace {
                    Ok(())
                } else {
                    self.maybe_inject_fault(AdminFaultPoint::AfterCreateOnlyLinkBeforeUnlink)
                }
            },
            || self.maybe_inject_fault(AdminFaultPoint::AfterTargetPublishBeforeParentSync),
        )
    }

    fn read_control_target(&self, path: &Path, create_only: bool) -> Result<Vec<u8>, SignerError> {
        match read_control_bytes(path, self.control_owner_uid, self.control_group_gid) {
            Ok(bytes) => Ok(bytes),
            Err(SignerError::InstallationDrift) if create_only => {
                self.remove_published_hardlink_stage(path)?;
                read_control_bytes(path, self.control_owner_uid, self.control_group_gid)
            }
            Err(error) => Err(error),
        }
    }

    fn remove_published_hardlink_stage(&self, target: &Path) -> Result<(), SignerError> {
        use std::os::unix::fs::MetadataExt;

        let target_metadata = fs::symlink_metadata(target)?;
        if !target_metadata.is_file()
            || target_metadata.file_type().is_symlink()
            || target_metadata.uid() != self.control_owner_uid
            || target_metadata.gid() != self.control_group_gid
            || target_metadata.permissions().mode() & 0o7777 != 0o640
            || target_metadata.nlink() != 2
        {
            return Err(SignerError::RecoveryRequired);
        }
        let parent = target.parent().ok_or(SignerError::InstallationDrift)?;
        let name = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(SignerError::InstallationDrift)?;
        let prefix = format!(".{name}.stage-");
        let mut matches = Vec::new();
        for entry in fs::read_dir(parent)? {
            let entry = entry?;
            if !entry.file_name().to_string_lossy().starts_with(&prefix) {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(SignerError::RecoveryRequired);
            }
            if metadata.dev() == target_metadata.dev() && metadata.ino() == target_metadata.ino() {
                if metadata.nlink() != 2
                    || metadata.uid() != self.control_owner_uid
                    || metadata.gid() != self.control_group_gid
                    || metadata.permissions().mode() & 0o7777 != 0o640
                {
                    return Err(SignerError::RecoveryRequired);
                }
                matches.push(entry.path());
            }
        }
        if matches.len() != 1 {
            return Err(SignerError::RecoveryRequired);
        }
        fs::remove_file(&matches[0])?;
        sync_dir(parent)
    }

    pub(crate) fn commit_intent(&self, record: &MaintenanceRecord) -> Result<(), SignerError> {
        let mut committed = record.clone();
        "committed".clone_into(&mut committed.phase);
        let bytes = canonical_json(&committed)?;
        atomic_publish(
            &self.root.join(MAINTENANCE_FILE),
            &bytes,
            PublishOptions {
                owner_uid: self.control_owner_uid,
                owner_gid: self.control_group_gid,
                mode: 0o640,
                replace: true,
            },
        )
    }

    fn maybe_inject_fault(&self, point: AdminFaultPoint) -> Result<(), SignerError> {
        #[cfg(test)]
        if self.test_fault.get() == Some(point) {
            self.test_fault.set(None);
            return Err(SignerError::PersistenceFailed(std::io::Error::other(
                "injected admin persistence fault",
            )));
        }
        #[cfg(not(test))]
        let _ = point;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn from_config_fixture(installation: InstallationConfig) -> Self {
        Self {
            root: PathBuf::from(&installation.store_dir),
            signer_uid: installation.signer_uid,
            signer_gid: installation.signer_gid,
            control_owner_uid: installation.signer_uid,
            control_group_gid: installation.signer_gid,
            installation,
            allow_nonroot_test_fixture: true,
            test_fault: std::cell::Cell::new(None),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_signer_fixture(store: &crate::store::SignerStore) -> Self {
        Self {
            installation: store.installation.clone(),
            root: store.root.clone(),
            signer_uid: store.signer_uid,
            signer_gid: store.installation.signer_gid,
            control_owner_uid: store.control_owner_uid,
            control_group_gid: store.control_group_gid,
            allow_nonroot_test_fixture: true,
            test_fault: std::cell::Cell::new(None),
        }
    }
}

fn path_exists(path: &Path) -> Result<bool, SignerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SignerError::RecoveryRequired),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(SignerError::PersistenceFailed(error)),
    }
}

fn unix_time_ms() -> Result<u64, SignerError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SignerError::CryptoOrBindingInvalid)?;
    u64::try_from(duration.as_millis()).map_err(|_| SignerError::CryptoOrBindingInvalid)
}
