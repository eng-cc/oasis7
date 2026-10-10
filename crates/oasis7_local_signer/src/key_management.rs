//! Root-only key inventory and lifecycle. No secrets appear in catalog or audit output.
use crate::{
    admin::AdminStore,
    error::SignerError,
    identity::{canonical_json, sha256_hex},
    local_fs::{create_private_dir, set_path_owner_mode, sync_dir, write_private_new},
    protocol::validate_id,
    store::{read_control, read_owned_key_file},
};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
use zeroize::{Zeroize, Zeroizing};

pub(crate) const CATALOG: &str = "control/key-catalog.json";
const SCHEMA: &str = "oasis7.key_catalog.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KeyInfo {
    pub signer_id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub purpose: String,
    pub algorithm: String,
    pub public_key_hex: String,
    pub state: String,
    pub exportable: bool,
    pub created_at_ms: u64,
    pub rotated_from: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Catalog {
    pub schema_version: String,
    pub installation_id: String,
    pub keys: Vec<KeyInfo>,
    pub events: Vec<KeyEvent>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyEvent {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub action: String,
    pub signer_id: String,
    pub public_key_sha256: String,
}

pub(crate) fn supported_purpose(p: &str) -> bool {
    matches!(p, "rollback_strict_audit_v1" | "file_ed25519_v1")
}
fn invalid(reason: &str) -> SignerError {
    SignerError::InvalidInput(reason.into())
}
pub(crate) fn now_ms() -> Result<u64, SignerError> {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SignerError::CryptoOrBindingInvalid)?;
    u64::try_from(d.as_millis()).map_err(|_| SignerError::CryptoOrBindingInvalid)
}
impl Catalog {
    pub(crate) fn validate(&self) -> Result<(), SignerError> {
        if self.schema_version != SCHEMA || self.keys.len() > 256 || self.events.len() > 4096 {
            return Err(SignerError::RecoveryRequired);
        }
        let mut ids = BTreeSet::new();
        let mut pubs = BTreeSet::new();
        for k in &self.keys {
            validate_info(k)?;
            if !ids.insert(&k.signer_id) || !pubs.insert(&k.public_key_hex) {
                return Err(SignerError::IdConflict);
            }
        }
        for (i, e) in self.events.iter().enumerate() {
            if e.sequence != i as u64 + 1 || e.timestamp_ms == 0 || !ids.contains(&e.signer_id) {
                return Err(SignerError::RecoveryRequired);
            }
            crate::types::validate_sha256(&e.public_key_sha256)
                .map_err(|_| SignerError::RecoveryRequired)?;
        }
        Ok(())
    }
}
fn validate_info(k: &KeyInfo) -> Result<(), SignerError> {
    validate_id(&k.signer_id)?;
    if !supported_purpose(&k.purpose)
        || k.algorithm != "ed25519"
        || k.name.is_empty()
        || k.name.len() > 128
        || k.description.len() > 2048
        || k.tags.len() > 16
        || k.created_at_ms == 0
        || !matches!(
            k.state.as_str(),
            "active" | "inactive" | "archived" | "deleted"
        )
    {
        return Err(invalid("invalid key metadata"));
    }
    crate::types::validate_sha256(&k.public_key_hex).map_err(|_| invalid("invalid public key"))?;
    for t in &k.tags {
        validate_id(t)?;
    }
    if k.tags.iter().collect::<BTreeSet<_>>().len() != k.tags.len() {
        return Err(invalid("duplicate tags"));
    }
    if let Some(id) = &k.rotated_from {
        validate_id(id)?;
        if id == &k.signer_id {
            return Err(invalid("self rotation"));
        }
    }
    Ok(())
}
pub(crate) fn load_catalog(
    root: &Path,
    uid: u32,
    gid: u32,
) -> Result<Option<Catalog>, SignerError> {
    match fs::symlink_metadata(root.join(CATALOG)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(_) => {
            let c: Catalog = read_control(&root.join(CATALOG), uid, gid)?;
            c.validate()?;
            Ok(Some(c))
        }
    }
}
pub(crate) fn reject_catalog_id(a: &AdminStore, id: &str) -> Result<(), SignerError> {
    if load_catalog(&a.root, a.control_owner_uid, a.control_group_gid)?
        .is_some_and(|c| c.keys.iter().any(|k| k.signer_id == id))
    {
        return Err(SignerError::IdConflict);
    }
    Ok(())
}
pub(crate) fn require_active_key(
    root: &Path,
    uid: u32,
    gid: u32,
    installation_id: &str,
    id: &str,
    purpose: &str,
) -> Result<(), SignerError> {
    if let Some(c) = load_catalog(root, uid, gid)? {
        if c.installation_id != installation_id {
            return Err(SignerError::InstallationDrift);
        }
        if let Some(k) = c.keys.iter().find(|k| k.signer_id == id) {
            return if k.state == "active" && k.purpose == purpose {
                Ok(())
            } else {
                Err(SignerError::AuthorizationDenied)
            };
        }
    }
    // Existing installations retain their rollback-only legacy keys. A generic key must be catalogued.
    if purpose == "rollback_strict_audit_v1" {
        Ok(())
    } else {
        Err(SignerError::AuthorizationDenied)
    }
}
impl AdminStore {
    pub(crate) fn catalog(&self) -> Result<Catalog, SignerError> {
        let c = load_catalog(&self.root, self.control_owner_uid, self.control_group_gid)?
            .unwrap_or(Catalog {
                schema_version: SCHEMA.into(),
                installation_id: self.installation.installation_id.clone(),
                keys: vec![],
                events: vec![],
            });
        if c.installation_id != self.installation.installation_id {
            return Err(SignerError::InstallationDrift);
        }
        Ok(c)
    }
    fn save_catalog(&self, mut c: Catalog, action: &str, id: &str) -> Result<(), SignerError> {
        let k = c
            .keys
            .iter()
            .find(|k| k.signer_id == id)
            .ok_or(SignerError::KeyOrAuthorityUnavailable)?;
        c.events.push(KeyEvent {
            sequence: c.events.len() as u64 + 1,
            timestamp_ms: now_ms()?,
            action: action.into(),
            signer_id: id.into(),
            public_key_sha256: sha256_hex(
                &hex::decode(&k.public_key_hex).map_err(|_| SignerError::CryptoOrBindingInvalid)?,
            ),
        });
        c.validate()?;
        let bytes = canonical_json(&c)?;
        self.mutate_control_file(
            &[action, id, &sha256_hex(&bytes)],
            &self.root.join(CATALOG),
            &bytes,
            false,
            None,
        )
    }
    pub fn key_list(&self) -> Result<Vec<KeyInfo>, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        };
        let mut c = self.catalog()?;
        for e in fs::read_dir(self.root.join("keys"))? {
            let e = e?;
            let id = e
                .file_name()
                .into_string()
                .map_err(|_| SignerError::RecoveryRequired)?;
            if id.starts_with('.') {
                return Err(SignerError::RecoveryRequired);
            }
            if !c.keys.iter().any(|k| k.signer_id == id) {
                let public = self.validate_key_directory(&e.path())?;
                c.keys.push(KeyInfo {
                    signer_id: id.clone(),
                    name: id,
                    description: "Legacy rollback key; not exportable".into(),
                    tags: vec![],
                    purpose: "rollback_strict_audit_v1".into(),
                    algorithm: "ed25519".into(),
                    public_key_hex: hex::encode(public),
                    state: "active".into(),
                    exportable: false,
                    created_at_ms: 1,
                    rotated_from: None,
                });
            }
        }
        c.keys.sort_by(|a, b| a.signer_id.cmp(&b.signer_id));
        Ok(c.keys)
    }
    pub fn key_get(&self, id: &str) -> Result<KeyInfo, SignerError> {
        self.validate_signer_id(id)?;
        self.key_list()?
            .into_iter()
            .find(|k| k.signer_id == id)
            .ok_or(SignerError::KeyOrAuthorityUnavailable)
    }
    pub fn key_create_managed(
        &self,
        id: &str,
        purpose: &str,
        name: &str,
        exportable: bool,
    ) -> Result<KeyInfo, SignerError> {
        self.ensure_root_admin()?;
        self.validate_signer_id(id)?;
        if name.is_empty() || name.len() > 128 || !supported_purpose(purpose) {
            return Err(invalid("invalid managed key metadata"));
        }
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        }
        reject_catalog_id(self, id)?;
        let dir = self.root.join("keys").join(id);
        if fs::symlink_metadata(&dir).is_ok() {
            return Err(SignerError::IdConflict);
        }
        let mut seed = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut *seed).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
        let public = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
        let mut c = self.catalog()?;
        let mut k = KeyInfo {
            signer_id: id.into(),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            purpose: purpose.into(),
            algorithm: "ed25519".into(),
            public_key_hex: hex::encode(public),
            state: "inactive".into(),
            exportable,
            created_at_ms: now_ms()?,
            rotated_from: None,
        };
        validate_info(&k)?;
        c.keys.push(k.clone());
        self.save_catalog(c, "key-create-intent", id)?;
        create_private_dir(&dir)?;
        set_path_owner_mode(&dir, self.signer_uid, self.signer_gid, 0o700)?;
        for (name, content) in [
            ("seed.bin", seed.as_slice()),
            ("public.bin", public.as_slice()),
        ] {
            let p = dir.join(name);
            write_private_new(&p, content)?;
            set_path_owner_mode(&p, self.signer_uid, self.signer_gid, 0o600)?;
        }
        sync_dir(&dir)?;
        sync_dir(dir.parent().unwrap())?;
        self.validate_key_files(id)?;
        k.state = "active".into();
        let mut c = self.catalog()?;
        *c.keys.iter_mut().find(|key| key.signer_id == id).unwrap() = k.clone();
        self.save_catalog(c, "key-create", id)?;
        Ok(k)
    }
    pub fn key_update(
        &self,
        id: &str,
        name: &str,
        description: &str,
        tags: Vec<String>,
    ) -> Result<KeyInfo, SignerError> {
        let legacy = self.key_get(id)?;
        let _lock = self.acquire_lock()?;
        let mut c = self.catalog()?;
        if !c.keys.iter().any(|k| k.signer_id == id) {
            c.keys.push(legacy);
        }
        let k = c.keys.iter_mut().find(|k| k.signer_id == id).unwrap();
        if k.state == "deleted" {
            return Err(SignerError::AuthorizationDenied);
        }
        k.name = name.into();
        k.description = description.into();
        k.tags = tags;
        validate_info(k)?;
        let result = k.clone();
        self.save_catalog(c, "key-update", id)?;
        Ok(result)
    }
    pub fn key_set_state(&self, id: &str, state: &str) -> Result<KeyInfo, SignerError> {
        if !matches!(state, "active" | "inactive" | "archived" | "deleted") {
            return Err(invalid("invalid key state"));
        }
        let legacy = self.key_get(id)?;
        let _lock = self.acquire_lock()?;
        let mut c = self.catalog()?;
        if !c.keys.iter().any(|k| k.signer_id == id) {
            c.keys.push(legacy);
        }
        let k = c.keys.iter_mut().find(|k| k.signer_id == id).unwrap();
        if k.state == "deleted" && state != "deleted" {
            return Err(SignerError::AuthorizationDenied);
        }
        if state == "deleted" && !matches!(k.state.as_str(), "inactive" | "archived" | "deleted") {
            return Err(invalid("disable or archive key before deleting"));
        }
        if state == "active" {
            self.validate_key_files(id)?;
        }
        k.state = state.into();
        let result = k.clone();
        self.save_catalog(c, "key-state", id)?;
        if state == "deleted" {
            let path = self.root.join("keys").join(id).join("seed.bin");
            match fs::symlink_metadata(&path) {
                Ok(_) => {
                    let _seed = Zeroizing::new(read_owned_key_file(
                        &path,
                        self.signer_uid,
                        self.signer_gid,
                    )?);
                    fs::remove_file(&path)?;
                    sync_dir(path.parent().unwrap())?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(result)
    }
    pub fn key_rotate(&self, old: &str, new: &str) -> Result<KeyInfo, SignerError> {
        let old_info = self.key_get(old)?;
        if !matches!(old_info.state.as_str(), "active" | "inactive") {
            return Err(SignerError::AuthorizationDenied);
        }
        // Fail closed first. New keys are never silently substituted into policy or existing grants.
        self.key_set_state(old, "inactive")?;
        let result =
            self.key_create_managed(new, &old_info.purpose, &old_info.name, old_info.exportable)?;
        let _lock = self.acquire_lock()?;
        let mut c = self.catalog()?;
        c.keys
            .iter_mut()
            .find(|k| k.signer_id == new)
            .unwrap()
            .rotated_from = Some(old.into());
        self.save_catalog(c, "key-rotate", new)?;
        Ok(KeyInfo {
            rotated_from: Some(old.into()),
            ..result
        })
    }
    pub fn key_events(&self) -> Result<Vec<KeyEvent>, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        };
        Ok(self.catalog()?.events)
    }
    pub fn recover_management(
        &self,
        operation_key: &str,
        target_sha256: &str,
    ) -> Result<serde_json::Value, SignerError> {
        self.ensure_root_admin()?;
        crate::types::validate_sha256(operation_key)
            .map_err(|_| invalid("invalid operation key"))?;
        crate::types::validate_sha256(target_sha256).map_err(|_| invalid("invalid target hash"))?;
        let _lock = self.acquire_lock()?;
        let r = self
            .read_maintenance()?
            .ok_or(SignerError::RecoveryRequired)?;
        if r.operation_key != operation_key || r.target_sha256 != target_sha256 {
            return Err(SignerError::AuthorizationDenied);
        }
        let target = self.root.join(&r.target_path);
        let actual = if r.target_path.starts_with("keys/") {
            let public = self.validate_key_directory(&target)?;
            crate::local_fs::sync_file_and_parent(&target.join("seed.bin"))?;
            crate::local_fs::sync_file_and_parent(&target.join("public.bin"))?;
            sync_dir(&target)?;
            sha256_hex(&public)
        } else {
            let bytes = crate::store::read_control_bytes(
                &target,
                self.control_owner_uid,
                self.control_group_gid,
            )?;
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
            if canonical_json(&value)? != bytes {
                return Err(SignerError::RecoveryRequired);
            }
            if r.target_path == CATALOG {
                let c: Catalog =
                    serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
                c.validate()?;
                for k in &c.keys {
                    if k.state == "active" {
                        self.validate_key_files(&k.signer_id)?;
                    }
                }
            }
            if r.target_path == "control/restore.json" {
                return Err(invalid(
                    "use exact restore-apply to recover a restore transaction",
                ));
            }
            crate::local_fs::sync_file_and_parent(&target)?;
            sha256_hex(&bytes)
        };
        if actual != target_sha256 {
            return Err(SignerError::RecoveryRequired);
        }
        if r.phase == "prepared" {
            self.commit_intent(&r)?;
        }
        Ok(
            serde_json::json!({"status":"MANAGEMENT_RECOVERED","operation_key":operation_key,"target_sha256":target_sha256}),
        )
    }
    pub fn control_inventory(&self) -> Result<serde_json::Value, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        }
        let policy = match fs::symlink_metadata(self.root.join(crate::store::POLICY_FILE)) {
            Ok(_) => Some(read_control::<crate::types::Policy>(
                &self.root.join(crate::store::POLICY_FILE),
                self.control_owner_uid,
                self.control_group_gid,
            )?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(p) = &policy {
            p.validate(&self.installation)
                .map_err(|_| SignerError::InstallationDrift)?;
        }
        let mut grants = Vec::new();
        let now = now_ms()?;
        for e in fs::read_dir(self.root.join(crate::store::GRANTS_DIR))? {
            if grants.len() >= 4096 {
                return Err(SignerError::RecoveryRequired);
            }
            let e = e?;
            let grant: crate::types::BatchGrant =
                read_control(&e.path(), self.control_owner_uid, self.control_group_gid)?;
            if e.file_name().to_str() != Some(&format!("{}.json", grant.grant_id)) {
                return Err(SignerError::InstallationDrift);
            }
            let revoked = match fs::symlink_metadata(
                self.root
                    .join(crate::store::REVOKED_GRANTS_DIR)
                    .join(format!("{}.json", grant.grant_id)),
            ) {
                Ok(m) if !m.file_type().is_symlink() => true,
                Ok(_) => return Err(SignerError::InstallationDrift),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                Err(e) => return Err(e.into()),
            };
            grants.push(serde_json::json!({"grant":grant,"revoked":revoked,"expired":now>=grant.expires_at,"not_yet_valid":now<grant.not_before}));
        }
        grants.sort_by_key(|v| {
            v["grant"]["grant_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        });
        Ok(
            serde_json::json!({"installation_id":self.installation.installation_id,"policy":policy,"grants":grants}),
        )
    }
    pub fn signing_audit(&self) -> Result<Vec<serde_json::Value>, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        }
        let mut out = Vec::new();
        for e in fs::read_dir(self.root.join("state/records"))? {
            if out.len() >= 4096 {
                return Err(SignerError::RecoveryRequired);
            }
            let path = e?.path();
            crate::store::validate_owned_directory(&path, self.signer_uid, self.signer_gid, 0o700)?;
            let bytes = crate::local_fs::read_regular(&path.join("reservation.json"), 1024 * 1024)?;
            let r: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
            let mut summary = serde_json::Map::new();
            for k in [
                "request_id",
                "operation_key",
                "purpose",
                "signer_id",
                "grant_id",
                "policy_revision",
                "payload_sha256",
                "context",
            ] {
                summary.insert(
                    k.into(),
                    r.get(k).cloned().ok_or(SignerError::RecoveryRequired)?,
                );
            }
            summary.insert(
                "committed".into(),
                serde_json::Value::Bool(path.join("result/manifest.json").exists()),
            );
            out.push(summary.into());
        }
        Ok(out)
    }
    pub fn key_export(&self, id: &str, password: &[u8]) -> Result<Vec<u8>, SignerError> {
        self.ensure_root_admin()?;
        self.validate_signer_id(id)?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        };
        let k = self
            .catalog()?
            .keys
            .into_iter()
            .find(|k| k.signer_id == id)
            .ok_or(SignerError::AuthorizationDenied)?;
        if !k.exportable || k.state == "deleted" {
            return Err(SignerError::AuthorizationDenied);
        }
        let public = self.validate_key_directory(&self.root.join("keys").join(id))?;
        if hex::encode(public) != k.public_key_hex {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        let seed = Zeroizing::new(read_owned_key_file(
            &self.root.join("keys").join(id).join("seed.bin"),
            self.signer_uid,
            self.signer_gid,
        )?);
        let mut transfer = KeyTransfer {
            schema_version: "oasis7.key_transfer.v1".into(),
            key: k.clone(),
            seed_hex: hex::encode(&*seed),
        };
        let bytes = Zeroizing::new(canonical_json(&transfer)?);
        transfer.seed_hex.zeroize();
        let encrypted = crate::key_envelope::seal("key", &bytes, password)?;
        let c = self.catalog()?;
        self.save_catalog(c, "key-export", id)?;
        Ok(encrypted)
    }
    pub fn key_import(
        &self,
        id: &str,
        encrypted: &[u8],
        password: &[u8],
        exportable: bool,
    ) -> Result<KeyInfo, SignerError> {
        self.ensure_root_admin()?;
        self.validate_signer_id(id)?;
        let bytes = crate::key_envelope::open("key", encrypted, password)?;
        let transfer: KeyTransfer =
            serde_json::from_slice(&bytes).map_err(|_| invalid("invalid key transfer"))?;
        validate_info(&transfer.key)?;
        if transfer.schema_version != "oasis7.key_transfer.v1" || transfer.key.state == "deleted" {
            return Err(invalid("invalid key transfer"));
        }
        let seed = Zeroizing::new(
            hex::decode(&transfer.seed_hex).map_err(|_| invalid("invalid private seed"))?,
        );
        let raw: Zeroizing<[u8; 32]> = Zeroizing::new(
            seed.as_slice()
                .try_into()
                .map_err(|_| invalid("invalid private seed length"))?,
        );
        let public = SigningKey::from_bytes(&raw).verifying_key().to_bytes();
        if hex::encode(public) != transfer.key.public_key_hex {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        };
        reject_catalog_id(self, id)?;
        for key in fs::read_dir(self.root.join("keys"))? {
            let key = key?;
            let path = key.path();
            let existing =
                read_owned_key_file(&path.join("public.bin"), self.signer_uid, self.signer_gid)?;
            if existing == public {
                return Err(SignerError::IdConflict);
            }
        }
        let mut c = self.catalog()?;
        if c.keys
            .iter()
            .any(|k| k.public_key_hex == hex::encode(public))
        {
            return Err(SignerError::IdConflict);
        }
        let dir = self.root.join("keys").join(id);
        if dir.exists() {
            return Err(SignerError::IdConflict);
        }
        // Publish inactive catalog first: interruption cannot expose an uncatalogued imported key to signing.
        let mut k = transfer.key.clone();
        k.signer_id = id.into();
        k.exportable = exportable;
        k.state = "inactive".into();
        k.created_at_ms = now_ms()?;
        k.rotated_from = None;
        c.keys.push(k.clone());
        self.save_catalog(c, "key-import-intent", id)?;
        create_private_dir(&dir)?;
        set_path_owner_mode(&dir, self.signer_uid, self.signer_gid, 0o700)?;
        for (name, content) in [
            ("seed.bin", seed.as_slice()),
            ("public.bin", public.as_slice()),
        ] {
            let path = dir.join(name);
            write_private_new(&path, content)?;
            set_path_owner_mode(&path, self.signer_uid, self.signer_gid, 0o600)?;
        }
        sync_dir(&dir)?;
        sync_dir(dir.parent().unwrap())?;
        self.validate_key_files(id)?;
        self.save_catalog(self.catalog()?, "key-import", id)?;
        Ok(k)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyTransfer {
    schema_version: String,
    key: KeyInfo,
    seed_hex: String,
}
impl Drop for KeyTransfer {
    fn drop(&mut self) {
        self.seed_hex.zeroize();
    }
}
