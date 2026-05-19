use super::hosted_access::DeploymentMode;
use super::hosted_player_session::{
    HostedPlayerSessionAdmissionSnapshot, HostedPlayerSessionIssueGrant,
    HostedPlayerSessionIssueResponse, HostedPlayerSessionIssuer,
};
use super::{emit_stderr_or_event, Level};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const HOSTED_ACCOUNT_LOGIN_START_ROUTE: &str = "/api/public/hosted-account/login/start";
pub(super) const HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE: &str =
    "/api/public/hosted-account/login/complete";
const HOSTED_ACCOUNT_STORE_PATH_ENV: &str = "OASIS7_HOSTED_ACCOUNT_STORE_PATH";
const HOSTED_LOGIN_DELIVERY_MODE_ENV: &str = "OASIS7_HOSTED_LOGIN_DELIVERY_MODE";
const HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE: &str = "preview_inline";
const HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY: &str = "server_log_only";
const LOGIN_CHALLENGE_TTL_MS: u64 = 10 * 60 * 1000;
const LOGIN_START_RATE_LIMIT_WINDOW_MS: u64 = 60_000;
const LOGIN_START_RATE_LIMIT_PER_HANDLE: u64 = 3;

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAccountLoginStartResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    pub(super) deployment_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) challenge: Option<HostedAccountLoginChallengeSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAccountLoginCompleteResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    pub(super) deployment_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) account: Option<HostedAccountSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) grant: Option<HostedPlayerSessionIssueGrant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) admission: Option<HostedPlayerSessionAdmissionSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAccountSummary {
    pub(super) hosted_account_id: String,
    pub(super) player_id: String,
    pub(super) login_channel: String,
    pub(super) masked_login_hint: String,
    pub(super) status: String,
    pub(super) last_verified_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAccountLoginChallengeSnapshot {
    pub(super) challenge_id: String,
    pub(super) login_channel: String,
    pub(super) masked_login_hint: String,
    pub(super) delivery_mode: String,
    pub(super) expires_at_unix_ms: u64,
    pub(super) account_exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) preview_code: Option<String>,
}

#[derive(Debug)]
pub(super) struct HostedAccountIdentityBroker {
    store_path: PathBuf,
    store: HostedAccountStore,
    delivery_mode: String,
    next_challenge_sequence: u64,
    recent_start_timestamps_by_factor: BTreeMap<String, VecDeque<u64>>,
    pending_challenges: BTreeMap<String, PendingHostedLoginChallenge>,
}

#[derive(Debug, Clone)]
struct PendingHostedLoginChallenge {
    challenge_id: String,
    login_channel: String,
    normalized_login_hint: String,
    masked_login_hint: String,
    otp_code: String,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HostedAccountStore {
    next_account_sequence: u64,
    next_player_sequence: u64,
    accounts_by_id: BTreeMap<String, HostedAccountRecord>,
    account_id_by_factor: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedAccountRecord {
    hosted_account_id: String,
    player_id: String,
    login_channel: String,
    normalized_login_hint: String,
    masked_login_hint: String,
    status: String,
    created_at_unix_ms: u64,
    last_verified_at_unix_ms: u64,
}

impl HostedAccountIdentityBroker {
    pub(super) fn from_env() -> Result<Self, String> {
        let store_path = resolve_store_path();
        let store = load_store(store_path.as_path())?;
        Ok(Self {
            store_path,
            store,
            delivery_mode: normalize_delivery_mode(
                std::env::var(HOSTED_LOGIN_DELIVERY_MODE_ENV)
                    .ok()
                    .as_deref(),
            ),
            next_challenge_sequence: 0,
            recent_start_timestamps_by_factor: BTreeMap::new(),
            pending_challenges: BTreeMap::new(),
        })
    }

    #[cfg(test)]
    fn with_store_path(store_path: PathBuf) -> Result<Self, String> {
        let store = load_store(store_path.as_path())?;
        Ok(Self {
            store_path,
            store,
            delivery_mode: HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE.to_string(),
            next_challenge_sequence: 0,
            recent_start_timestamps_by_factor: BTreeMap::new(),
            pending_challenges: BTreeMap::new(),
        })
    }

    pub(super) fn start_login(
        &mut self,
        deployment_mode: DeploymentMode,
        login_channel: &str,
        login_hint: &str,
    ) -> HostedAccountLoginStartResponse {
        if deployment_mode != DeploymentMode::HostedPublicJoin {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("hosted_account_login_disabled".to_string()),
                error: Some(
                    "hosted account login is only available on hosted_public_join".to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        }
        self.prune_expired_challenges();
        let Some(channel) = normalize_login_channel(login_channel) else {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("unsupported_login_channel".to_string()),
                error: Some("login_channel must be one of: email|phone".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        };
        let Some(normalized_login_hint) = normalize_login_hint(channel, login_hint) else {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("login_hint_invalid".to_string()),
                error: Some(format!("{channel} login_hint is invalid")),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        };
        if self.rate_limited(channel, normalized_login_hint.as_str()) {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("login_rate_limited".to_string()),
                error: Some(
                    "too many login challenges were issued for this handle; retry in a minute"
                        .to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        }
        let issued_at_unix_ms = now_unix_ms();
        self.next_challenge_sequence = self.next_challenge_sequence.saturating_add(1);
        let factor_key = factor_key(channel, normalized_login_hint.as_str());
        let challenge_id =
            build_login_challenge_id(issued_at_unix_ms, self.next_challenge_sequence);
        let otp_code = build_preview_otp_code(issued_at_unix_ms, self.next_challenge_sequence);
        let masked_login_hint = mask_login_hint(channel, normalized_login_hint.as_str());
        let expires_at_unix_ms = issued_at_unix_ms.saturating_add(LOGIN_CHALLENGE_TTL_MS);
        let account_exists = self
            .store
            .account_id_by_factor
            .contains_key(factor_key.as_str());
        self.recent_start_timestamps_by_factor
            .entry(factor_key)
            .or_default()
            .push_back(issued_at_unix_ms);
        let challenge = PendingHostedLoginChallenge {
            challenge_id: challenge_id.clone(),
            login_channel: channel.to_string(),
            normalized_login_hint: normalized_login_hint.clone(),
            masked_login_hint: masked_login_hint.clone(),
            otp_code: otp_code.clone(),
            expires_at_unix_ms,
        };
        self.pending_challenges
            .insert(challenge_id.clone(), challenge);
        emit_delivery_notice(
            self.delivery_mode.as_str(),
            channel,
            masked_login_hint.as_str(),
            otp_code.as_str(),
        );
        HostedAccountLoginStartResponse {
            ok: true,
            error_code: None,
            error: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            challenge: Some(HostedAccountLoginChallengeSnapshot {
                challenge_id,
                login_channel: channel.to_string(),
                masked_login_hint,
                delivery_mode: self.delivery_mode.clone(),
                expires_at_unix_ms,
                account_exists,
                preview_code: if self.delivery_mode == HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE {
                    Some(otp_code)
                } else {
                    None
                },
            }),
        }
    }

    pub(super) fn complete_login(
        &mut self,
        deployment_mode: DeploymentMode,
        challenge_id: &str,
        otp_code: &str,
        issuer: &mut HostedPlayerSessionIssuer,
    ) -> HostedAccountLoginCompleteResponse {
        if deployment_mode != DeploymentMode::HostedPublicJoin {
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("hosted_account_login_disabled".to_string()),
                error: Some(
                    "hosted account login is only available on hosted_public_join".to_string(),
                ),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        }
        self.prune_expired_challenges();
        let normalized_challenge_id = challenge_id.trim();
        if normalized_challenge_id.is_empty() {
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("challenge_id_required".to_string()),
                error: Some("challenge_id is required".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        }
        let Some(challenge) = self.pending_challenges.remove(normalized_challenge_id) else {
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("challenge_not_found".to_string()),
                error: Some("login challenge is missing or expired".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        };
        if challenge.expires_at_unix_ms <= now_unix_ms() {
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("challenge_expired".to_string()),
                error: Some("login challenge expired; request a fresh code".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        }
        if challenge.otp_code != otp_code.trim() {
            self.pending_challenges
                .insert(challenge.challenge_id.clone(), challenge);
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("otp_code_invalid".to_string()),
                error: Some("login verification code is invalid".to_string()),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        }
        let factor_key = factor_key(
            challenge.login_channel.as_str(),
            challenge.normalized_login_hint.as_str(),
        );
        let verified_at_unix_ms = now_unix_ms();
        let account_id = self
            .store
            .account_id_by_factor
            .get(factor_key.as_str())
            .cloned()
            .unwrap_or_else(|| {
                self.store.next_account_sequence =
                    self.store.next_account_sequence.saturating_add(1);
                build_hosted_account_id(self.store.next_account_sequence)
            });
        let player_id = if let Some(record) = self.store.accounts_by_id.get_mut(account_id.as_str())
        {
            record.last_verified_at_unix_ms = verified_at_unix_ms;
            record.status = "active".to_string();
            record.player_id.clone()
        } else {
            self.store.next_player_sequence = self.store.next_player_sequence.saturating_add(1);
            let player_id = build_hosted_player_id(self.store.next_player_sequence);
            self.store.accounts_by_id.insert(
                account_id.clone(),
                HostedAccountRecord {
                    hosted_account_id: account_id.clone(),
                    player_id: player_id.clone(),
                    login_channel: challenge.login_channel.clone(),
                    normalized_login_hint: challenge.normalized_login_hint.clone(),
                    masked_login_hint: challenge.masked_login_hint.clone(),
                    status: "active".to_string(),
                    created_at_unix_ms: verified_at_unix_ms,
                    last_verified_at_unix_ms: verified_at_unix_ms,
                },
            );
            self.store
                .account_id_by_factor
                .insert(factor_key, account_id.clone());
            player_id
        };
        if let Err(err) = save_store(self.store_path.as_path(), &self.store) {
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some("account_store_persist_failed".to_string()),
                error: Some(err),
                deployment_mode: deployment_mode.as_str().to_string(),
                account: None,
                grant: None,
                admission: None,
            };
        }
        let issue = issuer.issue_for_player(deployment_mode, player_id.as_str());
        let account = self
            .store
            .accounts_by_id
            .get(account_id.as_str())
            .cloned()
            .map(account_summary_from_record);
        response_from_issue(deployment_mode, issue, account)
    }

    fn prune_expired_challenges(&mut self) {
        let now = now_unix_ms();
        self.pending_challenges
            .retain(|_, challenge| challenge.expires_at_unix_ms > now);
        for timestamps in self.recent_start_timestamps_by_factor.values_mut() {
            while timestamps
                .front()
                .copied()
                .unwrap_or(0)
                .saturating_add(LOGIN_START_RATE_LIMIT_WINDOW_MS)
                <= now
            {
                timestamps.pop_front();
            }
        }
    }

    fn rate_limited(&mut self, channel: &str, normalized_login_hint: &str) -> bool {
        self.prune_expired_challenges();
        let factor_key = factor_key(channel, normalized_login_hint);
        self.recent_start_timestamps_by_factor
            .get(factor_key.as_str())
            .map(|timestamps| timestamps.len() as u64 >= LOGIN_START_RATE_LIMIT_PER_HANDLE)
            .unwrap_or(false)
    }
}

fn response_from_issue(
    deployment_mode: DeploymentMode,
    issue: HostedPlayerSessionIssueResponse,
    account: Option<HostedAccountSummary>,
) -> HostedAccountLoginCompleteResponse {
    HostedAccountLoginCompleteResponse {
        ok: issue.ok && issue.grant.is_some() && account.is_some(),
        error_code: issue.error_code,
        error: issue.error,
        deployment_mode: deployment_mode.as_str().to_string(),
        account,
        grant: issue.grant,
        admission: Some(issue.admission),
    }
}

fn account_summary_from_record(record: HostedAccountRecord) -> HostedAccountSummary {
    HostedAccountSummary {
        hosted_account_id: record.hosted_account_id,
        player_id: record.player_id,
        login_channel: record.login_channel,
        masked_login_hint: record.masked_login_hint,
        status: record.status,
        last_verified_at_unix_ms: record.last_verified_at_unix_ms,
    }
}

fn resolve_store_path() -> PathBuf {
    if let Ok(raw) = std::env::var(HOSTED_ACCOUNT_STORE_PATH_ENV) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".oasis7-hosted-account-store.json")
}

fn load_store(path: &Path) -> Result<HostedAccountStore, String> {
    if !path.exists() {
        return Ok(HostedAccountStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read hosted account store `{}`: {err}",
            path.display()
        )
    })?;
    serde_json::from_str(raw.as_str()).map_err(|err| {
        format!(
            "failed to parse hosted account store `{}`: {err}",
            path.display()
        )
    })
}

fn save_store(path: &Path, store: &HostedAccountStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create hosted account store directory `{}`: {err}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("failed to serialize hosted account store: {err}"))?;
    fs::write(path, raw).map_err(|err| {
        format!(
            "failed to write hosted account store `{}`: {err}",
            path.display()
        )
    })
}

fn normalize_delivery_mode(raw: Option<&str>) -> String {
    match raw.unwrap_or("").trim() {
        HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY => {
            HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY.to_string()
        }
        _ => HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE.to_string(),
    }
}

fn emit_delivery_notice(
    delivery_mode: &str,
    channel: &str,
    masked_login_hint: &str,
    otp_code: &str,
) {
    if delivery_mode != HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY {
        return;
    }
    let message = format!(
        "hosted account login challenge issued: channel={channel} target={masked_login_hint} otp={otp_code}"
    );
    emit_stderr_or_event(
        Level::INFO,
        message.as_str(),
        "hosted account login challenge issued",
    );
}

fn normalize_login_channel(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "email" => Some("email"),
        "phone" => Some("phone"),
        _ => None,
    }
}

fn normalize_login_hint(channel: &str, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match channel {
        "email" => normalize_email(trimmed),
        "phone" => normalize_phone(trimmed),
        _ => None,
    }
}

fn normalize_email(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    let (local, domain) = normalized.split_once('@')?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return None;
    }
    Some(normalized)
}

fn normalize_phone(raw: &str) -> Option<String> {
    let mut normalized = String::new();
    for (index, ch) in raw.trim().chars().enumerate() {
        if ch.is_ascii_digit() {
            normalized.push(ch);
            continue;
        }
        if ch == '+' && index == 0 {
            normalized.push(ch);
        }
    }
    let digits = normalized.chars().filter(|ch| ch.is_ascii_digit()).count();
    if digits < 7 || digits > 18 {
        return None;
    }
    Some(normalized)
}

fn mask_login_hint(channel: &str, normalized_login_hint: &str) -> String {
    match channel {
        "email" => {
            let (local, domain) = normalized_login_hint
                .split_once('@')
                .unwrap_or((normalized_login_hint, ""));
            let local_chars: Vec<char> = local.chars().collect();
            let visible = local_chars.iter().take(2).collect::<String>();
            format!("{visible}***@{domain}")
        }
        "phone" => {
            let suffix: String = normalized_login_hint
                .chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let prefix = if normalized_login_hint.starts_with('+') {
                "+"
            } else {
                ""
            };
            format!("{prefix}***{suffix}")
        }
        _ => "***".to_string(),
    }
}

fn factor_key(channel: &str, normalized_login_hint: &str) -> String {
    format!("{channel}:{normalized_login_hint}")
}

fn build_login_challenge_id(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("hosted-login-challenge-{issued_at_unix_ms:016x}-{sequence:08x}")
}

fn build_preview_otp_code(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("{:06}", ((issued_at_unix_ms ^ sequence) % 1_000_000))
}

fn build_hosted_account_id(sequence: u64) -> String {
    format!("oasis-account-{sequence:08x}")
}

fn build_hosted_player_id(sequence: u64) -> String {
    format!("hosted-player-account-{sequence:08x}")
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
    use std::time::Duration;

    fn temp_store_path(name: &str) -> PathBuf {
        let unique = now_unix_ms();
        std::env::temp_dir().join(format!("oasis7-hosted-account-{name}-{unique}.json"))
    }

    #[test]
    fn hosted_account_login_start_rejects_invalid_channel() {
        let mut broker = HostedAccountIdentityBroker::with_store_path(temp_store_path("invalid"))
            .expect("broker");
        let response = broker.start_login(
            DeploymentMode::HostedPublicJoin,
            "telegram",
            "user@example.com",
        );
        assert!(!response.ok);
        assert_eq!(
            response.error_code.as_deref(),
            Some("unsupported_login_channel")
        );
    }

    #[test]
    fn hosted_account_login_complete_reuses_stable_player_id() {
        let path = temp_store_path("stable-player");
        let mut broker =
            HostedAccountIdentityBroker::with_store_path(path.clone()).expect("broker");
        let mut issuer = HostedPlayerSessionIssuer::default();

        let start = broker.start_login(
            DeploymentMode::HostedPublicJoin,
            "email",
            "player@example.com",
        );
        let challenge = start.challenge.expect("challenge");
        let first = broker.complete_login(
            DeploymentMode::HostedPublicJoin,
            challenge.challenge_id.as_str(),
            challenge.preview_code.as_deref().unwrap_or_default(),
            &mut issuer,
        );
        assert!(first.ok);
        let first_account = first.account.clone().expect("account");
        let first_grant = first.grant.clone().expect("grant");
        assert_eq!(first_account.player_id, first_grant.player_id);

        std::thread::sleep(Duration::from_millis(2));
        let start_second = broker.start_login(
            DeploymentMode::HostedPublicJoin,
            "email",
            "player@example.com",
        );
        let challenge_second = start_second.challenge.expect("challenge");
        let second = broker.complete_login(
            DeploymentMode::HostedPublicJoin,
            challenge_second.challenge_id.as_str(),
            challenge_second.preview_code.as_deref().unwrap_or_default(),
            &mut issuer,
        );
        assert!(second.ok);
        let second_account = second.account.expect("second account");
        let second_grant = second.grant.expect("second grant");
        assert_eq!(
            first_account.hosted_account_id,
            second_account.hosted_account_id
        );
        assert_eq!(first_account.player_id, second_account.player_id);
        assert_eq!(first_grant.player_id, second_grant.player_id);
        assert_ne!(first_grant.release_token, second_grant.release_token);

        let reloaded = HostedAccountIdentityBroker::with_store_path(path).expect("reloaded broker");
        assert_eq!(reloaded.store.accounts_by_id.len(), 1);
    }

    #[test]
    fn hosted_account_login_complete_rejects_wrong_otp() {
        let mut broker = HostedAccountIdentityBroker::with_store_path(temp_store_path("wrong-otp"))
            .expect("broker");
        let mut issuer = HostedPlayerSessionIssuer::default();
        let start =
            broker.start_login(DeploymentMode::HostedPublicJoin, "phone", "+1 415 555 0101");
        let challenge = start.challenge.expect("challenge");
        let response = broker.complete_login(
            DeploymentMode::HostedPublicJoin,
            challenge.challenge_id.as_str(),
            "000000",
            &mut issuer,
        );
        assert!(!response.ok);
        assert_eq!(response.error_code.as_deref(), Some("otp_code_invalid"));
    }
}
