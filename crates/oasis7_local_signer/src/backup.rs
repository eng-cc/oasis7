//! Encrypted custody snapshots and create-only recovery into an empty installation.
use crate::{
    admin::AdminStore,
    error::SignerError,
    identity::{canonical_json, sha256_hex},
    local_fs::{
        PublishOptions, atomic_publish, create_private_dir, read_regular, set_path_owner_mode,
        sync_dir,
    },
    store::{POLICY_FILE, read_control},
    types::Policy,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Component, Path},
};
use zeroize::{Zeroize, Zeroizing};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILES: usize = 8192;
const RECEIPT: &str = "control/restore.json";
#[cfg(test)]
thread_local! {pub(crate) static FAIL_RESTORE_DIR_SYNC: std::cell::Cell<bool> = const {std::cell::Cell::new(false)};}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: String,
    installation_id: String,
    deployment_id: String,
    created_at_ms: u64,
    files: BTreeMap<String, String>,
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        for v in self.files.values_mut() {
            v.zeroize();
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RestorePlan {
    pub schema_version: String,
    pub installation_sha256: String,
    pub encrypted_sha256: String,
    pub restored_tree_sha256: String,
    pub file_count: usize,
    pub signing_enabled: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreReceipt {
    schema_version: String,
    plan: RestorePlan,
    phase: String,
}

fn path_kind(path: &str) -> Result<bool, SignerError> {
    let p = Path::new(path);
    if p.is_absolute()
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
        || path.len() > 512
    {
        return Err(SignerError::CryptoOrBindingInvalid);
    }
    let parts: Vec<_> = path.split('/').collect();
    let valid = match parts.as_slice() {
        ["keys", id, "seed.bin" | "public.bin"] => crate::protocol::validate_id(id).is_ok(),
        ["control", "policy.json" | "key-catalog.json"] => true,
        ["control", "grants" | "revoked-grants", file] => file
            .strip_suffix(".json")
            .is_some_and(|id| crate::protocol::validate_id(id).is_ok()),
        ["state", "records", id, "reservation.json"] => crate::types::validate_sha256(id).is_ok(),
        ["state", "records", id, "result", file] => {
            crate::types::validate_sha256(id).is_ok()
                && matches!(*file, "response.json" | "audit.json" | "manifest.json")
        }
        _ => false,
    };
    if !valid {
        return Err(SignerError::RecoveryRequired);
    }
    Ok(parts[0] == "control")
}
fn collect(
    a: &AdminStore,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
    total: &mut usize,
) -> Result<(), SignerError> {
    for e in fs::read_dir(dir)? {
        let path = e?.path();
        let rel = path
            .strip_prefix(&a.root)
            .map_err(|_| SignerError::InstallationDrift)?
            .to_str()
            .ok_or(SignerError::RecoveryRequired)?
            .to_owned();
        if matches!(
            rel.as_str(),
            "state/custody.lock" | "control/maintenance.json" | RECEIPT
        ) {
            continue;
        }
        let m = fs::symlink_metadata(&path)?;
        if m.file_type().is_symlink() || m.dev() != fs::metadata(&a.root)?.dev() {
            return Err(SignerError::InstallationDrift);
        }
        let control = rel.starts_with("control/") || rel == "control";
        if m.uid()
            != if control {
                a.control_owner_uid
            } else {
                a.signer_uid
            }
            || m.gid()
                != if control {
                    a.control_group_gid
                } else {
                    a.signer_gid
                }
            || m.permissions().mode() & 0o7777
                != if m.is_dir() {
                    if control { 0o750 } else { 0o700 }
                } else if control {
                    0o640
                } else {
                    0o600
                }
        {
            return Err(SignerError::InstallationDrift);
        }
        if m.is_dir() {
            collect(a, &path, out, total)?;
        } else {
            path_kind(&rel)?;
            if !m.is_file() || m.nlink() != 1 || out.len() >= MAX_FILES {
                return Err(SignerError::RecoveryRequired);
            }
            let bytes = Zeroizing::new(read_regular(&path, MAX_BYTES)?);
            *total = total
                .checked_add(bytes.len())
                .ok_or(SignerError::RecoveryRequired)?;
            if *total > MAX_BYTES {
                return Err(SignerError::RecoveryRequired);
            }
            out.insert(
                rel,
                base64::engine::general_purpose::STANDARD.encode(&*bytes),
            );
        }
    }
    Ok(())
}
impl AdminStore {
    pub fn backup_encrypted(
        &self,
        password: &[u8],
        include_nonexportable: bool,
    ) -> Result<Vec<u8>, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        if self
            .read_maintenance()?
            .is_some_and(|r| r.phase != "committed")
        {
            return Err(SignerError::RecoveryRequired);
        }
        // A stopped authority is required, so backup does not capture live advancing signing state.
        let policy: Policy = read_control(
            &self.root.join(POLICY_FILE),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        if !policy.enabled_purposes.is_empty() {
            return Err(SignerError::AuthorizationDenied);
        }
        let mut snapshot = Snapshot {
            schema_version: "oasis7.custody_snapshot.v1".into(),
            installation_id: self.installation.installation_id.clone(),
            deployment_id: self.installation.deployment_id.clone(),
            created_at_ms: crate::key_management::now_ms()?,
            files: BTreeMap::new(),
        };
        let mut total = 0;
        for d in ["keys", "control", "state"] {
            collect(self, &self.root.join(d), &mut snapshot.files, &mut total)?;
        }
        validate_snapshot(self, &snapshot)?;
        if !include_nonexportable {
            let catalog = snapshot
                .files
                .get(crate::key_management::CATALOG)
                .map(|b| decode_file::<crate::key_management::Catalog>(b))
                .transpose()?;
            for path in snapshot.files.keys().filter(|p| p.ends_with("/seed.bin")) {
                let id = path
                    .split('/')
                    .nth(1)
                    .ok_or(SignerError::RecoveryRequired)?;
                if catalog.as_ref().is_none_or(|c| {
                    !c.keys
                        .iter()
                        .any(|k| k.signer_id == id && k.exportable && k.state != "deleted")
                }) {
                    return Err(SignerError::AuthorizationDenied);
                }
            }
        }
        let bytes = Zeroizing::new(canonical_json(&snapshot)?);
        crate::key_envelope::seal("backup", &bytes, password)
    }
    pub fn restore_plan(
        &self,
        encrypted: &[u8],
        password: &[u8],
    ) -> Result<RestorePlan, SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        let (_, files) = decode_snapshot(self, encrypted, password)?;
        restore_baseline(self, &files, None)?;
        plan(self, encrypted, &files)
    }
    pub fn restore_commit(
        &self,
        encrypted: &[u8],
        password: &[u8],
        expected_plan: &RestorePlan,
    ) -> Result<(), SignerError> {
        self.ensure_root_admin()?;
        let _lock = self.acquire_lock()?;
        let (_snapshot, files) = decode_snapshot(self, encrypted, password)?;
        if plan(self, encrypted, &files)? != *expected_plan {
            return Err(SignerError::InstallationDrift);
        }
        let old = match fs::symlink_metadata(self.root.join(RECEIPT)) {
            Ok(_) => Some(read_control::<RestoreReceipt>(
                &self.root.join(RECEIPT),
                self.control_owner_uid,
                self.control_group_gid,
            )?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(r) = &old {
            if r.schema_version != "oasis7.restore_receipt.v1"
                || r.plan != *expected_plan
                || !matches!(r.phase.as_str(), "prepared" | "committed")
            {
                return Err(SignerError::RecoveryRequired);
            }
        }
        restore_baseline(self, &files, old.as_ref())?;
        let complete = canonical_json(&RestoreReceipt {
            schema_version: "oasis7.restore_receipt.v1".into(),
            plan: expected_plan.clone(),
            phase: "committed".into(),
        })?;
        if old.as_ref().is_some_and(|r| r.phase == "committed") {
            if files.iter().any(|(p, b)| {
                read_regular(&self.root.join(p), MAX_BYTES).ok().as_deref() != Some(b.as_slice())
            }) {
                return Err(SignerError::RecoveryRequired);
            }
            sync_restored_tree(self, &files)?;
            crate::local_fs::sync_file_and_parent(&self.root.join(RECEIPT))?;
            let action = self.action_key(&["restore", &sha256_hex(encrypted)])?;
            if let Some(record) = self.pending_intent(&action, RECEIPT)? {
                if record.target_sha256 != sha256_hex(&complete) {
                    return Err(SignerError::RecoveryRequired);
                }
                crate::local_fs::sync_file_and_parent(&self.root.join(RECEIPT))?;
                self.commit_intent(&record)?;
            } else {
                self.read_maintenance()?;
            }
            return Ok(());
        }
        let action = self.action_key(&["restore", &sha256_hex(encrypted)])?;
        let record = if let Some(r) = self.pending_intent(&action, RECEIPT)? {
            if r.target_sha256 != sha256_hex(&complete) {
                return Err(SignerError::RecoveryRequired);
            }
            r
        } else {
            self.begin_intent(action, RECEIPT.into(), sha256_hex(&complete), None, None)?
        };
        publish_receipt(
            self,
            &canonical_json(&RestoreReceipt {
                schema_version: "oasis7.restore_receipt.v1".into(),
                plan: expected_plan.clone(),
                phase: "prepared".into(),
            })?,
        )?;
        for (rel, bytes) in &files {
            let target = self.root.join(rel);
            let parent = target.parent().ok_or(SignerError::RecoveryRequired)?;
            ensure_parents(self, parent)?;
            if target.exists() {
                if read_regular(&target, MAX_BYTES)?.as_slice() != bytes.as_slice() {
                    return Err(SignerError::RecoveryRequired);
                }
            } else {
                let control = path_kind(rel)?;
                atomic_publish(
                    &target,
                    bytes,
                    PublishOptions {
                        owner_uid: if control {
                            self.control_owner_uid
                        } else {
                            self.signer_uid
                        },
                        owner_gid: if control {
                            self.control_group_gid
                        } else {
                            self.signer_gid
                        },
                        mode: if control { 0o640 } else { 0o600 },
                        replace: false,
                    },
                )?;
            }
        }
        sync_restored_tree(self, &files)?;
        publish_receipt(self, &complete)?;
        self.commit_intent(&record)
    }
}
fn plan(
    a: &AdminStore,
    e: &[u8],
    files: &BTreeMap<String, Zeroizing<Vec<u8>>>,
) -> Result<RestorePlan, SignerError> {
    let hashes: BTreeMap<_, _> = files.iter().map(|(p, b)| (p, sha256_hex(b))).collect();
    Ok(RestorePlan {
        schema_version: "oasis7.restore_plan.v1".into(),
        installation_sha256: sha256_hex(&canonical_json(&a.installation)?),
        encrypted_sha256: sha256_hex(e),
        restored_tree_sha256: sha256_hex(&canonical_json(&hashes)?),
        file_count: files.len(),
        signing_enabled: false,
    })
}
fn decode_snapshot(
    a: &AdminStore,
    e: &[u8],
    pw: &[u8],
) -> Result<(Snapshot, BTreeMap<String, Zeroizing<Vec<u8>>>), SignerError> {
    let plaintext = crate::key_envelope::open("backup", e, pw)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&plaintext).map_err(|_| SignerError::CryptoOrBindingInvalid)?;
    if canonical_json(&snapshot)? != *plaintext {
        return Err(SignerError::RecoveryRequired);
    }
    validate_snapshot(a, &snapshot)?;
    let mut files: BTreeMap<_, _> = snapshot
        .files
        .iter()
        .map(|(p, b)| {
            Ok((
                p.clone(),
                Zeroizing::new(
                    base64::engine::general_purpose::STANDARD
                        .decode(b)
                        .map_err(|_| SignerError::CryptoOrBindingInvalid)?,
                ),
            ))
        })
        .collect::<Result<_, SignerError>>()?;
    let mut policy: Policy = serde_json::from_slice(
        files
            .get(POLICY_FILE)
            .ok_or(SignerError::RecoveryRequired)?,
    )
    .map_err(|_| SignerError::RecoveryRequired)?;
    policy.enabled_purposes.clear();
    files.insert(POLICY_FILE.into(), Zeroizing::new(canonical_json(&policy)?));
    // Every restored grant is permanently revoked; historical budgets/results remain intact.
    let grants: Vec<_> = files
        .keys()
        .filter_map(|p| {
            p.strip_prefix("control/grants/")
                .and_then(|s| s.strip_suffix(".json"))
                .map(str::to_owned)
        })
        .collect();
    for id in grants {
        let path = format!("control/revoked-grants/{id}.json");
        if !files.contains_key(&path) {
            files.insert(path,Zeroizing::new(canonical_json(&serde_json::json!({
        "schema_version":"oasis7.local_signer_revocation.v1","installation_id":a.installation.installation_id,"grant_id":id,"revoked_at_ms":snapshot.created_at_ms}))?));
        }
    }
    Ok((snapshot, files))
}
fn decode_file<T: serde::de::DeserializeOwned + Serialize>(
    encoded: &str,
) -> Result<T, SignerError> {
    let bytes = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| SignerError::RecoveryRequired)?,
    );
    let value: T = serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
    if canonical_json(&value)? != *bytes {
        return Err(SignerError::RecoveryRequired);
    }
    Ok(value)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Revocation {
    schema_version: String,
    installation_id: String,
    grant_id: String,
    revoked_at_ms: u64,
}
fn validate_snapshot(a: &AdminStore, s: &Snapshot) -> Result<(), SignerError> {
    if s.schema_version != "oasis7.custody_snapshot.v1"
        || s.installation_id != a.installation.installation_id
        || s.deployment_id != a.installation.deployment_id
        || s.created_at_ms == 0
        || s.files.len() > MAX_FILES
    {
        return Err(SignerError::CryptoOrBindingInvalid);
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for (p, b) in &s.files {
        path_kind(p)?;
        let bytes = Zeroizing::new(
            base64::engine::general_purpose::STANDARD
                .decode(b)
                .map_err(|_| SignerError::RecoveryRequired)?,
        );
        total = total
            .checked_add(bytes.len())
            .ok_or(SignerError::RecoveryRequired)?;
        if total > MAX_BYTES {
            return Err(SignerError::RecoveryRequired);
        }
        files.insert(p.clone(), bytes);
    }
    let policy: Policy = decode_file(
        s.files
            .get(POLICY_FILE)
            .ok_or(SignerError::RecoveryRequired)?,
    )?;
    policy
        .validate(&a.installation)
        .map_err(|_| SignerError::RecoveryRequired)?;
    if !policy.enabled_purposes.is_empty() {
        return Err(SignerError::AuthorizationDenied);
    }
    let catalog = s
        .files
        .get(crate::key_management::CATALOG)
        .map(|b| decode_file::<crate::key_management::Catalog>(b))
        .transpose()?;
    if let Some(c) = &catalog {
        c.validate()?;
        if c.installation_id != a.installation.installation_id {
            return Err(SignerError::RecoveryRequired);
        }
    }
    let mut grants = BTreeMap::new();
    for (p, b) in &files {
        if p.ends_with("/seed.bin") {
            let raw: Zeroizing<[u8; 32]> = Zeroizing::new(
                b.as_slice()
                    .try_into()
                    .map_err(|_| SignerError::CryptoOrBindingInvalid)?,
            );
            if files
                .get(&p.replace("/seed.bin", "/public.bin"))
                .map(|v| v.as_slice())
                != Some(
                    ed25519_dalek::SigningKey::from_bytes(&raw)
                        .verifying_key()
                        .as_bytes(),
                )
            {
                return Err(SignerError::CryptoOrBindingInvalid);
            }
        } else if p.ends_with("/public.bin") {
            if b.len() != 32 {
                return Err(SignerError::RecoveryRequired);
            }
            let id = p.split('/').nth(1).ok_or(SignerError::RecoveryRequired)?;
            let deleted = catalog.as_ref().is_some_and(|c| {
                c.keys
                    .iter()
                    .any(|k| k.signer_id == id && k.state == "deleted")
            });
            if !deleted && !files.contains_key(&p.replace("/public.bin", "/seed.bin")) {
                return Err(SignerError::RecoveryRequired);
            }
        } else if let Some(name) = p.strip_prefix("control/grants/") {
            let grant: crate::types::BatchGrant =
                decode_file(s.files.get(p).ok_or(SignerError::RecoveryRequired)?)?;
            let mut historical_policy = policy.clone();
            historical_policy.policy_revision = grant.policy_revision.clone();
            grant
                .validate(&a.installation, &historical_policy)
                .map_err(|_| SignerError::RecoveryRequired)?;
            if name != format!("{}.json", grant.grant_id) {
                return Err(SignerError::RecoveryRequired);
            }
            grants.insert(grant.grant_id.clone(), grant);
        } else if let Some(name) = p.strip_prefix("control/revoked-grants/") {
            let r: Revocation = decode_file(s.files.get(p).ok_or(SignerError::RecoveryRequired)?)?;
            if r.schema_version != "oasis7.local_signer_revocation.v1"
                || r.installation_id != a.installation.installation_id
                || r.revoked_at_ms == 0
                || name != format!("{}.json", r.grant_id)
            {
                return Err(SignerError::RecoveryRequired);
            }
        }
    }
    if let Some(c) = &catalog {
        for k in &c.keys {
            let public = files.get(&format!("keys/{}/public.bin", k.signer_id));
            if k.state != "deleted"
                && (public.is_none()
                    || !files.contains_key(&format!("keys/{}/seed.bin", k.signer_id)))
            {
                return Err(SignerError::RecoveryRequired);
            }
            if public.is_some_and(|b| hex::encode(&**b) != k.public_key_hex) {
                return Err(SignerError::RecoveryRequired);
            }
        }
    }
    for k in &policy.key_bindings {
        let public = files
            .get(&format!("keys/{}/public.bin", k.signer_id))
            .ok_or(SignerError::RecoveryRequired)?;
        if sha256_hex(public) != k.public_key_sha256 {
            return Err(SignerError::RecoveryRequired);
        }
    }
    let records: std::collections::BTreeSet<_> = files
        .keys()
        .filter_map(|p| {
            p.strip_prefix("state/records/")
                .and_then(|r| r.split('/').next())
        })
        .collect();
    for id in records {
        let prefix = format!("state/records/{id}");
        let reservation = files
            .get(&format!("{prefix}/reservation.json"))
            .ok_or(SignerError::RecoveryRequired)?;
        let result = ["response.json", "audit.json", "manifest.json"].map(|name| {
            files
                .get(&format!("{prefix}/result/{name}"))
                .map(|b| b.as_slice())
        });
        let complete = match result {
            [None, None, None] => None,
            [Some(r), Some(a), Some(m)] => Some((r, a, m)),
            _ => return Err(SignerError::RecoveryRequired),
        };
        crate::store::validate_snapshot_record(
            id,
            reservation,
            complete,
            &a.installation,
            &grants,
        )?;
    }
    Ok(())
}
fn sync_restored_tree(
    a: &AdminStore,
    files: &BTreeMap<String, Zeroizing<Vec<u8>>>,
) -> Result<(), SignerError> {
    let mut dirs = std::collections::BTreeSet::new();
    for (rel, expected) in files {
        let path = a.root.join(rel);
        if read_regular(&path, MAX_BYTES)?.as_slice() != expected.as_slice() {
            return Err(SignerError::RecoveryRequired);
        }
        crate::local_fs::sync_file_and_parent(&path)?;
        let mut parent = path.parent();
        while let Some(p) = parent {
            dirs.insert(p.to_path_buf());
            if p == a.root {
                break;
            }
            parent = p.parent();
        }
    }
    for dir in dirs.iter().rev() {
        #[cfg(test)]
        if FAIL_RESTORE_DIR_SYNC.with(|fault| fault.replace(false)) {
            return Err(SignerError::PersistenceFailed(std::io::Error::other(
                "injected restore directory sync failure",
            )));
        }
        sync_dir(dir)?;
    }
    Ok(())
}
fn restore_baseline(
    a: &AdminStore,
    files: &BTreeMap<String, Zeroizing<Vec<u8>>>,
    receipt: Option<&RestoreReceipt>,
) -> Result<(), SignerError> {
    let mut existing = Snapshot {
        schema_version: String::new(),
        installation_id: String::new(),
        deployment_id: String::new(),
        created_at_ms: 0,
        files: BTreeMap::new(),
    };
    let mut total = 0;
    for d in ["keys", "control", "state"] {
        collect(a, &a.root.join(d), &mut existing.files, &mut total)?;
    }
    if receipt.is_none() && !existing.files.is_empty() {
        return Err(SignerError::AuthorizationDenied);
    }
    for (p, b) in &existing.files {
        let bytes = Zeroizing::new(
            base64::engine::general_purpose::STANDARD
                .decode(b)
                .map_err(|_| SignerError::RecoveryRequired)?,
        );
        if files.get(p).is_none_or(|expected| **expected != *bytes) {
            return Err(SignerError::RecoveryRequired);
        }
    }
    Ok(())
}
fn ensure_parents(a: &AdminStore, parent: &Path) -> Result<(), SignerError> {
    if parent == a.root {
        return Ok(());
    }
    let outer = parent.parent().ok_or(SignerError::RecoveryRequired)?;
    ensure_parents(a, outer)?;
    if !parent.exists() {
        create_private_dir(parent)?;
        let control = parent.starts_with(a.root.join("control"));
        set_path_owner_mode(
            parent,
            if control {
                a.control_owner_uid
            } else {
                a.signer_uid
            },
            if control {
                a.control_group_gid
            } else {
                a.signer_gid
            },
            if control { 0o750 } else { 0o700 },
        )?;
        sync_dir(outer)?;
    }
    Ok(())
}
fn publish_receipt(a: &AdminStore, bytes: &[u8]) -> Result<(), SignerError> {
    atomic_publish(
        &a.root.join(RECEIPT),
        bytes,
        PublishOptions {
            owner_uid: a.control_owner_uid,
            owner_gid: a.control_group_gid,
            mode: 0o640,
            replace: true,
        },
    )
}
