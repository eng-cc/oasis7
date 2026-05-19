use super::hosted_access::{hosted_player_access_contract, DeploymentMode};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const HOSTED_PLAYER_SESSION_ISSUE_ROUTE: &str = "/api/public/player-session/issue";
pub(super) const HOSTED_PLAYER_SESSION_RELEASE_ROUTE: &str = "/api/public/player-session/release";
pub(super) const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE: &str =
    "/api/public/player-session/admission";
pub(super) const HOSTED_PLAYER_SESSION_REFRESH_ROUTE: &str = "/api/public/player-session/refresh";
const ISSUE_WINDOW_MS: u64 = 60_000;
const PENDING_REGISTRATION_TTL_MS: u64 = 30_000;
const SLOT_LEASE_TTL_MS: u64 = 120_000;

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerSessionAdmissionSnapshot {
    pub(super) issue_rate_limit_per_minute: u64,
    pub(super) max_player_sessions: u64,
    pub(super) active_player_sessions: u64,
    pub(super) effective_player_sessions: u64,
    pub(super) runtime_bound_player_sessions: u64,
    pub(super) runtime_only_player_sessions: u64,
    pub(super) runtime_probe_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) runtime_probe_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_runtime_probe_unix_ms: Option<u64>,
    pub(super) slot_lease_ttl_ms: u64,
    pub(super) pending_registration_ttl_ms: u64,
    pub(super) issued_players_total: u64,
    pub(super) released_players_total: u64,
    pub(super) issued_in_current_window: u64,
    pub(super) remaining_issue_budget: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerSessionIssueGrant {
    pub(super) player_id: String,
    pub(super) device_session_id: String,
    pub(super) issued_at_unix_ms: u64,
    pub(super) auth_mode: String,
    pub(super) release_token: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerSessionIssueResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    pub(super) deployment_mode: String,
    pub(super) admission: HostedPlayerSessionAdmissionSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) grant: Option<HostedPlayerSessionIssueGrant>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerSessionReleaseResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    pub(super) deployment_mode: String,
    pub(super) admission: HostedPlayerSessionAdmissionSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerSessionAdmissionResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    pub(super) deployment_mode: String,
    pub(super) admission: HostedPlayerSessionAdmissionSnapshot,
}

#[derive(Debug, Default)]
pub(super) struct HostedPlayerSessionIssuer {
    next_sequence: u64,
    issued_players_total: u64,
    released_players_total: u64,
    issue_timestamps_unix_ms: VecDeque<u64>,
    active_release_tokens_by_player: BTreeMap<String, String>,
    active_players_by_release_token: BTreeMap<String, String>,
    last_seen_unix_ms_by_release_token: BTreeMap<String, u64>,
    last_observed_runtime_bound_player_sessions: u64,
    last_runtime_probe_unix_ms: Option<u64>,
    last_runtime_probe_error: Option<String>,
    last_runtime_active_players: BTreeSet<String>,
    runtime_seen_players: BTreeSet<String>,
    runtime_revoked_players: BTreeSet<String>,
}

impl HostedPlayerSessionIssuer {
    pub(super) fn observe_runtime_active_players<'a, I>(&mut self, active_players: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.prune_old_timestamps();
        let runtime_active_players: BTreeSet<String> = active_players
            .into_iter()
            .map(str::trim)
            .filter(|player_id| !player_id.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let observed_at_unix_ms = now_unix_ms();
        self.last_runtime_active_players = runtime_active_players.clone();
        self.last_observed_runtime_bound_player_sessions = runtime_active_players.len() as u64;
        self.last_runtime_probe_unix_ms = Some(observed_at_unix_ms);
        self.last_runtime_probe_error = None;

        for player_id in &runtime_active_players {
            if let Some(release_token) =
                self.active_release_tokens_by_player.get(player_id).cloned()
            {
                self.runtime_seen_players.insert(player_id.clone());
                self.runtime_revoked_players.remove(player_id);
                self.last_seen_unix_ms_by_release_token
                    .insert(release_token, observed_at_unix_ms);
            }
        }
        self.prune_expired_slots();

        let stale_players: Vec<String> = self
            .runtime_seen_players
            .iter()
            .filter(|player_id| {
                self.active_release_tokens_by_player
                    .contains_key(player_id.as_str())
                    && !runtime_active_players.contains(player_id.as_str())
            })
            .cloned()
            .collect();
        for player_id in stale_players {
            let _ = self.release_slot_for_player(player_id.as_str(), true);
        }
    }

    pub(super) fn record_runtime_probe_failure(&mut self, error: String) {
        self.last_runtime_probe_unix_ms = Some(now_unix_ms());
        self.last_runtime_probe_error = Some(error);
    }

    pub(super) fn admission(
        &mut self,
        deployment_mode: DeploymentMode,
    ) -> HostedPlayerSessionAdmissionResponse {
        let contract = hosted_player_access_contract(deployment_mode);
        self.prune_old_timestamps();
        self.prune_expired_slots();
        HostedPlayerSessionAdmissionResponse {
            ok: true,
            error_code: None,
            error: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            admission: self.admission_snapshot(
                contract.admission.issue_rate_limit_per_minute,
                contract.admission.max_player_sessions,
            ),
        }
    }

    pub(super) fn refresh(
        &mut self,
        deployment_mode: DeploymentMode,
        player_id: &str,
        release_token: &str,
    ) -> HostedPlayerSessionAdmissionResponse {
        let contract = hosted_player_access_contract(deployment_mode);
        self.prune_old_timestamps();
        self.prune_expired_slots();
        let admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );
        if deployment_mode != DeploymentMode::HostedPublicJoin {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("player_session_refresh_disabled".to_string()),
                error: Some(
                    "hosted player-session refresh is only available on hosted_public_join"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let token = release_token.trim();
        if token.is_empty() {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("release_token_required".to_string()),
                error: Some("release_token is required".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let expected_player_id = player_id.trim();
        if expected_player_id.is_empty() {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("player_id_required".to_string()),
                error: Some("player_id is required".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        if self.runtime_revoked_players.contains(expected_player_id) {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("session_revoked".to_string()),
                error: Some(
                    "player session was revoked by runtime presence reconciliation".to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let Some(bound_player_id) = self.active_players_by_release_token.get(token) else {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("release_token_invalid".to_string()),
                error: Some("release_token does not map to an active player slot".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        };
        if bound_player_id != expected_player_id {
            return HostedPlayerSessionAdmissionResponse {
                ok: false,
                error_code: Some("player_id_mismatch".to_string()),
                error: Some("player_id does not match the active slot owner".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        self.last_seen_unix_ms_by_release_token
            .insert(token.to_string(), now_unix_ms());
        HostedPlayerSessionAdmissionResponse {
            ok: true,
            error_code: None,
            error: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            admission: self.admission_snapshot(
                contract.admission.issue_rate_limit_per_minute,
                contract.admission.max_player_sessions,
            ),
        }
    }

    pub(super) fn issue(
        &mut self,
        deployment_mode: DeploymentMode,
    ) -> HostedPlayerSessionIssueResponse {
        self.issue_internal(deployment_mode, None)
    }

    pub(super) fn issue_for_player(
        &mut self,
        deployment_mode: DeploymentMode,
        player_id: &str,
    ) -> HostedPlayerSessionIssueResponse {
        self.issue_internal(deployment_mode, Some(player_id))
    }

    fn issue_internal(
        &mut self,
        deployment_mode: DeploymentMode,
        player_id_override: Option<&str>,
    ) -> HostedPlayerSessionIssueResponse {
        let contract = hosted_player_access_contract(deployment_mode);
        self.prune_old_timestamps();
        self.prune_expired_slots();
        let mut admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );

        if deployment_mode != DeploymentMode::HostedPublicJoin {
            return HostedPlayerSessionIssueResponse {
                ok: false,
                error_code: Some("player_session_issue_disabled".to_string()),
                error: Some(
                    "hosted player-session issue is only available on hosted_public_join"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
                grant: None,
            };
        }

        if admission.issued_in_current_window >= admission.issue_rate_limit_per_minute {
            return HostedPlayerSessionIssueResponse {
                ok: false,
                error_code: Some("rate_limited".to_string()),
                error: Some(
                    "hosted player-session issue rate limit exceeded; retry in a minute"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
                grant: None,
            };
        }
        let issued_at_unix_ms = now_unix_ms();
        self.next_sequence = self.next_sequence.saturating_add(1);
        let player_id = player_id_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| build_player_id(issued_at_unix_ms, self.next_sequence));
        let _ = self.release_slot_for_player(player_id.as_str(), true);
        admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );
        if admission.effective_player_sessions >= admission.max_player_sessions {
            return HostedPlayerSessionIssueResponse {
                ok: false,
                error_code: Some("world_full".to_string()),
                error: Some(
                    "hosted player-session effective occupancy is full; wait for a player to leave"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
                grant: None,
            };
        }

        self.issued_players_total = self.issued_players_total.saturating_add(1);
        self.issue_timestamps_unix_ms.push_back(issued_at_unix_ms);
        let device_session_id = build_device_session_id(issued_at_unix_ms, self.next_sequence);
        let release_token = build_release_token(issued_at_unix_ms, self.next_sequence);
        self.active_release_tokens_by_player
            .insert(player_id.clone(), release_token.clone());
        self.active_players_by_release_token
            .insert(release_token.clone(), player_id.clone());
        self.last_seen_unix_ms_by_release_token
            .insert(release_token.clone(), issued_at_unix_ms);
        self.runtime_seen_players.remove(player_id.as_str());
        self.runtime_revoked_players.remove(player_id.as_str());
        admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );

        HostedPlayerSessionIssueResponse {
            ok: true,
            error_code: None,
            error: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            admission,
            grant: Some(HostedPlayerSessionIssueGrant {
                player_id,
                device_session_id,
                issued_at_unix_ms,
                auth_mode: "browser_local_ephemeral_ed25519".to_string(),
                release_token,
            }),
        }
    }

    pub(super) fn release(
        &mut self,
        deployment_mode: DeploymentMode,
        player_id: &str,
        release_token: &str,
    ) -> HostedPlayerSessionReleaseResponse {
        let contract = hosted_player_access_contract(deployment_mode);
        self.prune_old_timestamps();
        self.prune_expired_slots();
        let admission = self.admission_snapshot(
            contract.admission.issue_rate_limit_per_minute,
            contract.admission.max_player_sessions,
        );
        if deployment_mode != DeploymentMode::HostedPublicJoin {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("player_session_release_disabled".to_string()),
                error: Some(
                    "hosted player-session release is only available on hosted_public_join"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let token = release_token.trim();
        if token.is_empty() {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("release_token_required".to_string()),
                error: Some("release_token is required".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let expected_player_id = player_id.trim();
        if expected_player_id.is_empty() {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("player_id_required".to_string()),
                error: Some("player_id is required".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        if self.runtime_revoked_players.contains(expected_player_id) {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("session_revoked".to_string()),
                error: Some(
                    "player session was revoked by runtime presence reconciliation".to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let Some(bound_player_id) = self.active_players_by_release_token.get(token).cloned() else {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("release_token_invalid".to_string()),
                error: Some("release_token does not map to an active player slot".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        };
        if bound_player_id != expected_player_id {
            return HostedPlayerSessionReleaseResponse {
                ok: false,
                error_code: Some("player_id_mismatch".to_string()),
                error: Some("player_id does not match the active slot owner".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                admission,
            };
        }
        let _ = self.release_slot_for_player(bound_player_id.as_str(), false);
        HostedPlayerSessionReleaseResponse {
            ok: true,
            error_code: None,
            error: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            admission: self.admission_snapshot(
                contract.admission.issue_rate_limit_per_minute,
                contract.admission.max_player_sessions,
            ),
        }
    }

    fn prune_old_timestamps(&mut self) {
        let cutoff = now_unix_ms().saturating_sub(ISSUE_WINDOW_MS);
        while self
            .issue_timestamps_unix_ms
            .front()
            .is_some_and(|issued_at| *issued_at < cutoff)
        {
            let _ = self.issue_timestamps_unix_ms.pop_front();
        }
    }

    fn prune_expired_slots(&mut self) {
        let now_unix_ms = now_unix_ms();
        let mut expired_tokens = Vec::new();
        for (token, last_seen) in &self.last_seen_unix_ms_by_release_token {
            let Some(player_id) = self.active_players_by_release_token.get(token.as_str()) else {
                expired_tokens.push(token.clone());
                continue;
            };
            let ttl_ms = if self.runtime_seen_players.contains(player_id.as_str()) {
                SLOT_LEASE_TTL_MS
            } else {
                PENDING_REGISTRATION_TTL_MS
            };
            if now_unix_ms.saturating_sub(*last_seen) > ttl_ms {
                expired_tokens.push(token.clone());
            }
        }
        for token in expired_tokens {
            if let Some(player_id) = self.active_players_by_release_token.remove(token.as_str()) {
                self.active_release_tokens_by_player
                    .remove(player_id.as_str());
                self.runtime_seen_players.remove(player_id.as_str());
                self.runtime_revoked_players.remove(player_id.as_str());
                self.released_players_total = self.released_players_total.saturating_add(1);
            }
            self.last_seen_unix_ms_by_release_token
                .remove(token.as_str());
        }
    }

    fn release_slot_for_player(&mut self, player_id: &str, runtime_revoked: bool) -> bool {
        let player_id = player_id.trim();
        let Some(token) = self.active_release_tokens_by_player.remove(player_id) else {
            return false;
        };
        self.active_players_by_release_token.remove(token.as_str());
        self.last_seen_unix_ms_by_release_token
            .remove(token.as_str());
        self.runtime_seen_players.remove(player_id);
        if runtime_revoked {
            self.runtime_revoked_players.insert(player_id.to_string());
        } else {
            self.runtime_revoked_players.remove(player_id);
        }
        self.released_players_total = self.released_players_total.saturating_add(1);
        true
    }

    fn admission_snapshot(
        &self,
        issue_rate_limit_per_minute: u64,
        max_player_sessions: u64,
    ) -> HostedPlayerSessionAdmissionSnapshot {
        let issued_in_current_window = self.issue_timestamps_unix_ms.len() as u64;
        let active_player_sessions = self.active_release_tokens_by_player.len() as u64;
        let runtime_only_player_sessions = self
            .last_runtime_active_players
            .iter()
            .filter(|player_id| {
                !self
                    .active_release_tokens_by_player
                    .contains_key(player_id.as_str())
            })
            .count() as u64;
        let effective_player_sessions =
            active_player_sessions.saturating_add(runtime_only_player_sessions);
        let runtime_probe_status = if self.last_runtime_probe_unix_ms.is_none() {
            "not_started"
        } else if self.last_runtime_probe_error.is_some() {
            "error"
        } else {
            "ok"
        };
        HostedPlayerSessionAdmissionSnapshot {
            issue_rate_limit_per_minute,
            max_player_sessions,
            active_player_sessions,
            effective_player_sessions,
            runtime_bound_player_sessions: self.last_observed_runtime_bound_player_sessions,
            runtime_only_player_sessions,
            runtime_probe_status: runtime_probe_status.to_string(),
            runtime_probe_error: self.last_runtime_probe_error.clone(),
            last_runtime_probe_unix_ms: self.last_runtime_probe_unix_ms,
            slot_lease_ttl_ms: SLOT_LEASE_TTL_MS,
            pending_registration_ttl_ms: PENDING_REGISTRATION_TTL_MS,
            issued_players_total: self.issued_players_total,
            released_players_total: self.released_players_total,
            issued_in_current_window,
            remaining_issue_budget: issue_rate_limit_per_minute
                .saturating_sub(issued_in_current_window),
        }
    }
}

fn build_player_id(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("hosted-player-{issued_at_unix_ms:016x}-{sequence:08x}")
}

fn build_release_token(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("hosted-release-{issued_at_unix_ms:016x}-{sequence:08x}")
}

fn build_device_session_id(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("hosted-device-session-{issued_at_unix_ms:016x}-{sequence:08x}")
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_player_session_issue_returns_structured_grant() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let response = issuer.issue(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
        assert_eq!(response.error_code, None);
        assert_eq!(response.deployment_mode, "hosted_public_join");
        let grant = response.grant.expect("grant");
        assert!(grant.player_id.starts_with("hosted-player-"));
        assert!(grant
            .device_session_id
            .starts_with("hosted-device-session-"));
        assert_eq!(grant.auth_mode, "browser_local_ephemeral_ed25519");
        assert!(grant.release_token.starts_with("hosted-release-"));
        assert_eq!(response.admission.active_player_sessions, 1);
        assert_eq!(response.admission.effective_player_sessions, 1);
        assert_eq!(response.admission.issued_players_total, 1);
        assert_eq!(response.admission.issued_in_current_window, 1);
    }

    #[test]
    fn hosted_player_session_issue_for_player_reuses_stable_player_id() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let first = issuer.issue_for_player(DeploymentMode::HostedPublicJoin, "stable-player-1");
        let second = issuer.issue_for_player(DeploymentMode::HostedPublicJoin, "stable-player-1");
        let first_grant = first.grant.expect("first grant");
        let second_grant = second.grant.expect("second grant");
        assert_eq!(first_grant.player_id, "stable-player-1");
        assert_eq!(second_grant.player_id, "stable-player-1");
        assert_ne!(first_grant.release_token, second_grant.release_token);
    }

    #[test]
    fn hosted_player_session_issue_is_disabled_for_trusted_local_only() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let response = issuer.issue(DeploymentMode::TrustedLocalOnly);
        assert!(!response.ok);
        assert_eq!(
            response.error_code.as_deref(),
            Some("player_session_issue_disabled")
        );
        assert!(response.grant.is_none());
    }

    #[test]
    fn hosted_player_session_issue_enforces_max_player_sessions() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        for _ in 0..8 {
            let response = issuer.issue(DeploymentMode::HostedPublicJoin);
            assert!(response.ok);
        }
        let response = issuer.issue(DeploymentMode::HostedPublicJoin);
        assert!(!response.ok);
        assert_eq!(response.error_code.as_deref(), Some("world_full"));
        assert_eq!(response.admission.active_player_sessions, 8);
        assert_eq!(response.admission.effective_player_sessions, 8);
    }

    #[test]
    fn hosted_player_session_issue_counts_runtime_only_occupancy_toward_world_full() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        issuer.observe_runtime_active_players([
            "runtime-player-1",
            "runtime-player-2",
            "runtime-player-3",
            "runtime-player-4",
            "runtime-player-5",
            "runtime-player-6",
            "runtime-player-7",
            "runtime-player-8",
        ]);

        let response = issuer.issue(DeploymentMode::HostedPublicJoin);
        assert!(!response.ok);
        assert_eq!(response.error_code.as_deref(), Some("world_full"));
        assert_eq!(response.admission.active_player_sessions, 0);
        assert_eq!(response.admission.runtime_bound_player_sessions, 8);
        assert_eq!(response.admission.runtime_only_player_sessions, 8);
        assert_eq!(response.admission.effective_player_sessions, 8);
    }

    #[test]
    fn hosted_player_session_release_frees_active_slot() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        let release = issuer.release(
            DeploymentMode::HostedPublicJoin,
            grant.player_id.as_str(),
            grant.release_token.as_str(),
        );
        assert!(release.ok);
        assert_eq!(release.admission.active_player_sessions, 0);
        assert_eq!(release.admission.released_players_total, 1);
    }

    #[test]
    fn hosted_player_session_admission_reports_current_snapshot() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let _ = issuer.issue(DeploymentMode::HostedPublicJoin);
        let response = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
        assert_eq!(response.admission.active_player_sessions, 1);
        assert_eq!(response.admission.effective_player_sessions, 1);
        assert_eq!(response.admission.max_player_sessions, 8);
        assert_eq!(response.admission.runtime_bound_player_sessions, 0);
        assert_eq!(response.admission.runtime_only_player_sessions, 0);
        assert_eq!(response.admission.runtime_probe_status, "not_started");
        assert_eq!(response.admission.runtime_probe_error, None);
        assert_eq!(response.admission.slot_lease_ttl_ms, SLOT_LEASE_TTL_MS);
        assert_eq!(
            response.admission.pending_registration_ttl_ms,
            PENDING_REGISTRATION_TTL_MS
        );
    }

    #[test]
    fn hosted_player_session_refresh_keeps_slot_alive() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let token = issue.grant.expect("grant").release_token;
        let response = issuer.refresh(
            DeploymentMode::HostedPublicJoin,
            "hosted-player-test",
            token.as_str(),
        );
        assert!(!response.ok);
        assert_eq!(response.error_code.as_deref(), Some("player_id_mismatch"));

        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        let response = issuer.refresh(
            DeploymentMode::HostedPublicJoin,
            grant.player_id.as_str(),
            grant.release_token.as_str(),
        );
        assert!(response.ok);
        assert_eq!(response.admission.active_player_sessions, 1);
        assert_eq!(response.admission.effective_player_sessions, 1);
    }

    #[test]
    fn hosted_player_session_release_requires_matching_player_id() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");

        let missing_player_id = issuer.release(
            DeploymentMode::HostedPublicJoin,
            "",
            grant.release_token.as_str(),
        );
        assert!(!missing_player_id.ok);
        assert_eq!(
            missing_player_id.error_code.as_deref(),
            Some("player_id_required")
        );

        let mismatch = issuer.release(
            DeploymentMode::HostedPublicJoin,
            "hosted-player-other",
            grant.release_token.as_str(),
        );
        assert!(!mismatch.ok);
        assert_eq!(mismatch.error_code.as_deref(), Some("player_id_mismatch"));

        let ok = issuer.release(
            DeploymentMode::HostedPublicJoin,
            grant.player_id.as_str(),
            grant.release_token.as_str(),
        );
        assert!(ok.ok);
    }

    #[test]
    fn hosted_player_session_runtime_reconcile_releases_seen_players_missing_from_runtime() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");

        issuer.observe_runtime_active_players([grant.player_id.as_str()]);
        let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert_eq!(admission.admission.active_player_sessions, 1);
        assert_eq!(admission.admission.effective_player_sessions, 1);
        assert_eq!(admission.admission.runtime_bound_player_sessions, 1);
        assert_eq!(admission.admission.runtime_only_player_sessions, 0);
        assert_eq!(admission.admission.runtime_probe_status, "ok");

        issuer.observe_runtime_active_players(std::iter::empty::<&str>());
        let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert_eq!(admission.admission.active_player_sessions, 0);
        assert_eq!(admission.admission.effective_player_sessions, 0);
        assert_eq!(admission.admission.runtime_bound_player_sessions, 0);
        assert_eq!(admission.admission.runtime_only_player_sessions, 0);
        assert_eq!(admission.admission.released_players_total, 1);

        let refresh = issuer.refresh(
            DeploymentMode::HostedPublicJoin,
            grant.player_id.as_str(),
            grant.release_token.as_str(),
        );
        assert!(!refresh.ok);
        assert_eq!(refresh.error_code.as_deref(), Some("session_revoked"));
    }

    #[test]
    fn hosted_player_session_runtime_probe_failure_surfaces_in_admission() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        issuer.record_runtime_probe_failure("connect runtime live failed".to_string());
        let response = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert_eq!(response.admission.runtime_probe_status, "error");
        assert_eq!(
            response.admission.runtime_probe_error.as_deref(),
            Some("connect runtime live failed")
        );
        assert!(response.admission.last_runtime_probe_unix_ms.is_some());
    }

    #[test]
    fn hosted_player_session_admission_reports_runtime_only_occupancy_separately() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        issuer.observe_runtime_active_players([grant.player_id.as_str(), "runtime-player-extra"]);

        let response = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
        assert_eq!(response.admission.active_player_sessions, 1);
        assert_eq!(response.admission.runtime_bound_player_sessions, 2);
        assert_eq!(response.admission.runtime_only_player_sessions, 1);
        assert_eq!(response.admission.effective_player_sessions, 2);
    }

    #[test]
    fn hosted_player_session_pending_registration_slots_expire_before_full_lease_ttl() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        let token = grant.release_token;
        let stale_seen_at = now_unix_ms()
            .saturating_sub(PENDING_REGISTRATION_TTL_MS)
            .saturating_sub(1);
        issuer
            .last_seen_unix_ms_by_release_token
            .insert(token.clone(), stale_seen_at);

        let response = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
        assert_eq!(response.admission.active_player_sessions, 0);
        assert_eq!(response.admission.effective_player_sessions, 0);
        assert_eq!(response.admission.released_players_total, 1);
    }

    #[test]
    fn hosted_player_session_runtime_seen_slots_keep_full_lease_ttl() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        issuer.observe_runtime_active_players([grant.player_id.as_str()]);
        let token = grant.release_token;
        let still_alive_seen_at = now_unix_ms()
            .saturating_sub(PENDING_REGISTRATION_TTL_MS)
            .saturating_sub(1);
        issuer
            .last_seen_unix_ms_by_release_token
            .insert(token.clone(), still_alive_seen_at);

        let response = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
        assert_eq!(response.admission.active_player_sessions, 1);
        assert_eq!(response.admission.effective_player_sessions, 1);
    }

    #[test]
    fn hosted_player_session_runtime_probe_refreshes_runtime_bound_slot_before_expiry_prune() {
        let mut issuer = HostedPlayerSessionIssuer::default();
        let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
        let grant = issue.grant.expect("grant");
        issuer.observe_runtime_active_players([grant.player_id.as_str()]);

        let stale_seen_at = now_unix_ms()
            .saturating_sub(SLOT_LEASE_TTL_MS)
            .saturating_sub(1);
        issuer
            .last_seen_unix_ms_by_release_token
            .insert(grant.release_token.clone(), stale_seen_at);

        issuer.observe_runtime_active_players([grant.player_id.as_str()]);

        let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
        assert!(admission.ok);
        assert_eq!(admission.admission.active_player_sessions, 1);
        assert_eq!(admission.admission.runtime_bound_player_sessions, 1);
        assert_eq!(admission.admission.runtime_only_player_sessions, 0);
        assert_eq!(admission.admission.effective_player_sessions, 1);
        assert_eq!(admission.admission.released_players_total, 0);

        let refresh = issuer.refresh(
            DeploymentMode::HostedPublicJoin,
            grant.player_id.as_str(),
            grant.release_token.as_str(),
        );
        assert!(refresh.ok);
    }
}
