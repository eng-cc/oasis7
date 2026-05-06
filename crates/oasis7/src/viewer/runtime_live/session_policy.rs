use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone)]
pub(super) struct RuntimeRecoveryCursor {
    pub(super) snapshot_hash: String,
    pub(super) snapshot_height: u64,
    pub(super) log_cursor: u64,
    pub(super) stable_batch_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeSessionRevokeMetadata {
    pub(super) revoke_reason: Option<String>,
    pub(super) revoked_by: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RuntimeSessionPolicy {
    active_pubkey_by_player: BTreeMap<String, String>,
    revoked_pubkeys_by_player: BTreeMap<String, BTreeSet<String>>,
    session_epoch_by_player: BTreeMap<String, u64>,
}

impl RuntimeSessionPolicy {
    pub(super) fn register_session(
        &mut self,
        player_id: &str,
        public_key: &str,
    ) -> Result<u64, String> {
        let player_id = player_id.trim();
        let public_key = public_key.trim();
        if player_id.is_empty() {
            return Err("session_player_id_invalid: player_id cannot be empty".to_string());
        }
        if public_key.is_empty() {
            return Err("session_pubkey_invalid: session_pubkey cannot be empty".to_string());
        }
        if self
            .revoked_pubkeys_by_player
            .get(player_id)
            .is_some_and(|keys| keys.contains(public_key))
        {
            return Err(format!(
                "session_revoked: player {} session_pubkey {} is revoked",
                player_id, public_key
            ));
        }

        match self.active_pubkey_by_player.get(player_id) {
            Some(active) if active == public_key => {}
            Some(active) => {
                return Err(format!(
                    "session_key_mismatch: player {} active session_pubkey {} does not match {}",
                    player_id, active, public_key
                ));
            }
            None => {
                self.active_pubkey_by_player
                    .insert(player_id.to_string(), public_key.to_string());
                self.session_epoch_by_player
                    .entry(player_id.to_string())
                    .or_insert(1);
            }
        }

        Ok(self.session_epoch(player_id))
    }

    pub(super) fn validate_known_session_key(
        &self,
        player_id: &str,
        public_key: &str,
    ) -> Result<u64, String> {
        let player_id = player_id.trim();
        let public_key = public_key.trim();
        if player_id.is_empty() {
            return Err("session_player_id_invalid: player_id cannot be empty".to_string());
        }
        if public_key.is_empty() {
            return Err("session_pubkey_invalid: session_pubkey cannot be empty".to_string());
        }

        if self
            .revoked_pubkeys_by_player
            .get(player_id)
            .is_some_and(|keys| keys.contains(public_key))
        {
            return Err(format!(
                "session_revoked: player {} session_pubkey {} is revoked",
                player_id, public_key
            ));
        }
        match self.active_pubkey_by_player.get(player_id) {
            Some(active) if active == public_key => {}
            Some(active) => {
                return Err(format!(
                    "session_key_mismatch: player {} active session_pubkey {} does not match {}",
                    player_id, active, public_key
                ));
            }
            None => {
                return Err(format!(
                    "session_not_found: player {} has no active session_pubkey",
                    player_id
                ));
            }
        }
        Ok(self.session_epoch(player_id))
    }

    pub(super) fn revoke_session(
        &mut self,
        player_id: &str,
        session_pubkey: Option<&str>,
    ) -> Result<(String, u64), String> {
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err("session_player_id_invalid: player_id cannot be empty".to_string());
        }

        let target = session_pubkey
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| self.active_pubkey_by_player.get(player_id).cloned())
            .ok_or_else(|| {
                format!(
                    "session_not_found: player {} has no active session_pubkey",
                    player_id
                )
            })?;

        let revoked = self
            .revoked_pubkeys_by_player
            .entry(player_id.to_string())
            .or_default()
            .insert(target.clone());
        if self
            .active_pubkey_by_player
            .get(player_id)
            .is_some_and(|active| active == &target)
        {
            self.active_pubkey_by_player.remove(player_id);
        }

        if revoked {
            let next_epoch = self.session_epoch(player_id).saturating_add(1).max(1);
            self.session_epoch_by_player
                .insert(player_id.to_string(), next_epoch);
        }

        Ok((target, self.session_epoch(player_id)))
    }

    pub(super) fn rotate_session(
        &mut self,
        player_id: &str,
        old_session_pubkey: &str,
        new_session_pubkey: &str,
    ) -> Result<u64, String> {
        let player_id = player_id.trim();
        let old_session_pubkey = old_session_pubkey.trim();
        let new_session_pubkey = new_session_pubkey.trim();
        if player_id.is_empty() {
            return Err("session_player_id_invalid: player_id cannot be empty".to_string());
        }
        if old_session_pubkey.is_empty() || new_session_pubkey.is_empty() {
            return Err("session_pubkey_invalid: session_pubkey cannot be empty".to_string());
        }
        if old_session_pubkey == new_session_pubkey {
            return Err("session_rotation_invalid: old/new session_pubkey must differ".to_string());
        }

        if self
            .active_pubkey_by_player
            .get(player_id)
            .is_some_and(|active| active != old_session_pubkey)
        {
            return Err(format!(
                "session_key_mismatch: player {} active session_pubkey does not match {}",
                player_id, old_session_pubkey
            ));
        }
        if self
            .revoked_pubkeys_by_player
            .get(player_id)
            .is_some_and(|keys| keys.contains(new_session_pubkey))
        {
            return Err(format!(
                "session_rotation_invalid: new session_pubkey {} is already revoked",
                new_session_pubkey
            ));
        }

        self.revoked_pubkeys_by_player
            .entry(player_id.to_string())
            .or_default()
            .insert(old_session_pubkey.to_string());
        self.active_pubkey_by_player
            .insert(player_id.to_string(), new_session_pubkey.to_string());
        let next_epoch = self.session_epoch(player_id).saturating_add(1).max(1);
        self.session_epoch_by_player
            .insert(player_id.to_string(), next_epoch);
        Ok(next_epoch)
    }

    fn session_epoch(&self, player_id: &str) -> u64 {
        self.session_epoch_by_player
            .get(player_id)
            .copied()
            .unwrap_or(0)
    }
}

pub(super) fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn session_revoke_metadata_key(
    player_id: &str,
    session_pubkey: &str,
) -> (String, String) {
    (
        player_id.trim().to_string(),
        session_pubkey.trim().to_string(),
    )
}

pub(super) fn map_session_policy_error_code(message: &str) -> &'static str {
    if message.contains("session_revoked") {
        return "session_revoked";
    }
    if message.contains("session_key_mismatch") {
        return "session_key_mismatch";
    }
    if message.contains("session_not_found") {
        return "session_not_found";
    }
    if message.contains("session_pubkey_invalid")
        || message.contains("session_player_id_invalid")
        || message.contains("session_rotation_invalid")
    {
        return "session_invalid";
    }
    "session_policy_error"
}

pub(super) fn location_id_for_pos(pos: GeoPos) -> String {
    format!("runtime:{}:{}:{}", pos.x_cm, pos.y_cm, pos.z_cm)
}
