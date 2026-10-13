//! Identity v2 keeps the slot token server-side and never releases an old slot
//! while a candidate control key is waiting for runtime's atomic registration.
use super::*;

impl HostedPlayerSessionIssuer {
    pub(in super::super) fn reserve_v2_slot(
        &mut self,
        player: &str,
        token: &str,
        now: u64,
    ) -> Result<(), String> {
        self.prune_old_timestamps();
        self.prune_expired_slots();
        let digest = release_token_digest(token);
        if let Some(existing) = self.active_release_tokens_by_player.get(player) {
            if existing != &digest {
                return Err("slot_owner_conflict".into());
            }
            return self.touch_v2_slot(player, token, now);
        }
        let contract = hosted_player_access_contract(DeploymentMode::HostedPublicJoin);
        let admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );
        if admission.issued_in_current_window >= admission.issue_rate_limit_per_minute {
            return Err("rate_limited".into());
        }
        if admission.effective_player_sessions >= admission.max_player_sessions
            && !self.last_runtime_active_players.contains(player)
        {
            return Err("world_full".into());
        }
        let snapshot = self.clone();
        self.active_release_tokens_by_player
            .insert(player.into(), digest.clone());
        self.active_players_by_release_token
            .insert(digest.clone(), player.into());
        self.last_seen_unix_ms_by_release_token.insert(digest, now);
        self.issued_players_total = self.issued_players_total.saturating_add(1);
        self.issue_timestamps_unix_ms.push_back(now);
        self.runtime_seen_players.remove(player);
        self.runtime_revoked_players.remove(player);
        if self.persist_ledger().is_err() {
            *self = snapshot;
            return Err("session_ledger_unavailable".into());
        }
        Ok(())
    }

    pub(in super::super) fn touch_v2_slot(
        &mut self,
        player: &str,
        token: &str,
        now: u64,
    ) -> Result<(), String> {
        let digest = release_token_digest(token);
        if self.active_release_tokens_by_player.get(player) != Some(&digest)
            || self
                .active_players_by_release_token
                .get(&digest)
                .map(String::as_str)
                != Some(player)
        {
            return Err("world_slot_expired".into());
        }
        let previous = self
            .last_seen_unix_ms_by_release_token
            .insert(digest.clone(), now);
        if self.persist_ledger().is_err() {
            if let Some(previous) = previous {
                self.last_seen_unix_ms_by_release_token
                    .insert(digest, previous);
            }
            return Err("session_ledger_unavailable".into());
        }
        Ok(())
    }

    pub(in super::super) fn release_v2_slot(
        &mut self,
        player: &str,
        token: &str,
    ) -> Result<(), String> {
        let digest = release_token_digest(token);
        match self.active_release_tokens_by_player.get(player) {
            None => return Ok(()),
            Some(existing) if existing == &digest => {}
            _ => return Err("slot_owner_conflict".into()),
        }
        let snapshot = self.clone();
        self.release_slot_for_player_with_reason(player, SlotReleaseReason::ExplicitRelease, None);
        if self.persist_ledger().is_err() {
            *self = snapshot;
            return Err("session_ledger_unavailable".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_candidate_does_not_release_existing_slot() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let now = now_unix_ms();
        issuer.reserve_v2_slot("player", "original", now).unwrap();
        assert!(issuer.reserve_v2_slot("player", "candidate", now).is_err());
        assert!(issuer.release_v2_slot("player", "candidate").is_err());
        assert!(issuer.touch_v2_slot("player", "original", now).is_ok());
    }
}
