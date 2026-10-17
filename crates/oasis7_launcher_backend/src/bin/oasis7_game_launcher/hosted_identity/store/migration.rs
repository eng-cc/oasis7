//! Explicit maintenance-only legacy JSON import; the source is never rewritten.
use super::super::model::*;
use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(serde::Deserialize)]
struct LegacyStore {
    next_account_sequence: u64,
    next_player_sequence: u64,
    accounts_by_id: BTreeMap<String, LegacyAccount>,
    account_id_by_factor: BTreeMap<String, String>,
}
#[derive(serde::Deserialize)]
struct LegacyAccount {
    hosted_account_id: String,
    player_id: String,
    login_channel: String,
    normalized_login_hint: String,
    status: String,
    created_at_unix_ms: u64,
    last_verified_at_unix_ms: u64,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct MigrationReceipt {
    source_digest: String,
    world_scope: String,
    accounts: u64,
}
impl IdentityStore {
    pub fn migrate_legacy_json(
        &mut self,
        path: &Path,
        world_scope: &str,
        now: u64,
        write_fenced: bool,
        protect_email: impl Fn(&str) -> Result<CredentialMaterial, StoreError>,
    ) -> Result<u64, StoreError> {
        if !write_fenced || world_scope.is_empty() {
            return Err(StoreError::Rejected(
                "legacy_writer_fence_and_world_scope_required".into(),
            ));
        }
        let bytes = std::fs::read(path)
            .map_err(|_| StoreError::Unavailable("legacy snapshot read failed".into()))?;
        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(&bytes));
        let legacy: LegacyStore =
            serde_json::from_slice(&bytes).map_err(|_| StoreError::Corrupt)?;
        let mut factors = BTreeSet::new();
        let mut players = BTreeSet::new();
        let mut prepared = Vec::new();
        for (key, record) in &legacy.accounts_by_id {
            if key != &record.hosted_account_id
                || record.login_channel != "email"
                || record.normalized_login_hint.is_empty()
                || !matches!(record.status.as_str(), "active" | "frozen" | "disabled")
                || !players.insert(record.player_id.clone())
            {
                return Err(StoreError::Rejected("legacy_identity_conflict".into()));
            }
            let factor = format!("email:{}", record.normalized_login_hint);
            if legacy.account_id_by_factor.get(&factor) != Some(key) || !factors.insert(factor) {
                return Err(StoreError::Rejected("legacy_factor_conflict".into()));
            }
            let material = protect_email(&record.normalized_login_hint)?;
            if !matches!(material, CredentialMaterial::Email { .. }) {
                return Err(StoreError::Rejected(
                    "migration_email_protection_required".into(),
                ));
            }
            let mut handle = vec![0u8; 16];
            getrandom::fill(&mut handle)
                .map_err(|_| StoreError::Unavailable("entropy unavailable".into()))?;
            prepared.push((record, handle, material));
        }
        if legacy.account_id_by_factor.len() != prepared.len() {
            return Err(StoreError::Rejected("legacy_orphan_factor".into()));
        }
        self.transaction(|tx| {
            if let Some(old) = tx.get::<MigrationReceipt>("migration:legacy-json")? {
                if old.value.source_digest != digest || old.value.world_scope != world_scope {
                    return Err(StoreError::Rejected("migration_source_changed".into()));
                }
                return Ok(old.value.accounts);
            }
            if !tx
                .list::<AccountRecord>("account", None, None, 1)?
                .is_empty()
            {
                return Err(StoreError::Rejected(
                    "migration_requires_empty_identity_store".into(),
                ));
            }
            for (old, handle, mut material) in prepared {
                // Unknown historical bound_at must mature from import, never created_at.
                if let CredentialMaterial::Email {
                    ref mut bound_at,
                    ref mut recovery_eligible_since,
                    ..
                } = material
                {
                    *bound_at = now;
                    *recovery_eligible_since = now;
                }
                let account = AccountRecord {
                    account_id: old.hosted_account_id.clone(),
                    user_handle: handle.clone(),
                    display_name: "Player".into(),
                    status: old.status.clone(),
                    policy_version: 1,
                    security_epoch: 1,
                    has_ever_public_key: false,
                    created_at: old.created_at_unix_ms,
                    last_authenticated_at: Some(old.last_verified_at_unix_ms),
                    last_recovery_id: None,
                };
                let cid = format!("migrated-email:{}", old.hosted_account_id);
                let lookup = match &material {
                    CredentialMaterial::Email { lookup_hmac, .. } => lookup_hmac.clone(),
                    _ => unreachable!(),
                };
                let credential = CredentialRecord {
                    credential_record_id: cid.clone(),
                    account_id: account.account_id.clone(),
                    state: "active".into(),
                    version: 1,
                    label: "Email".into(),
                    created_at: now,
                    last_used_at: Some(old.last_verified_at_unix_ms),
                    public_material: material,
                    registered_via: "legacy_email_json_v1".into(),
                };
                let binding = PlayerBinding {
                    account_id: account.account_id.clone(),
                    world_identity_scope: world_scope.into(),
                    player_id: old.player_id.clone(),
                    source: "legacy_email_json_v1".into(),
                    binding_version: 1,
                };
                tx.create(
                    &format!("account:{}", account.account_id),
                    "account",
                    Some(&account.account_id),
                    None,
                    &account,
                )?;
                tx.create(
                    &format!("credential:{cid}"),
                    "credential",
                    Some(&account.account_id),
                    None,
                    &credential,
                )?;
                tx.create(
                    &format!("email_lookup:{lookup}"),
                    "credential_lookup",
                    Some(&account.account_id),
                    None,
                    &CredentialLookup {
                        scope: "email:v1".into(),
                        key_digest: lookup,
                        canonical_key: Vec::new(),
                        credential_record_id: cid,
                        account_id: account.account_id.clone(),
                        state: "active".into(),
                    },
                )?;
                tx.create(
                    &format!("binding:{}:{world_scope}", account.account_id),
                    "player_binding",
                    Some(&account.account_id),
                    None,
                    &binding,
                )?;
                tx.create(
                    &format!(
                        "user_handle:{}",
                        handle
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    "user_handle_lookup",
                    Some(&account.account_id),
                    None,
                    &account.account_id,
                )?;
            }
            tx.create(
                "sequence:legacy",
                "sequence",
                None,
                None,
                &(legacy.next_account_sequence, legacy.next_player_sequence),
            )?;
            let count = legacy.accounts_by_id.len() as u64;
            tx.create(
                "migration:legacy-json",
                "migration",
                None,
                None,
                &MigrationReceipt {
                    source_digest: digest,
                    world_scope: world_scope.into(),
                    accounts: count,
                },
            )?;
            Ok(count)
        })
    }
}
