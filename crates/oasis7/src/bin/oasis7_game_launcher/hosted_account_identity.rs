use super::hosted_access::DeploymentMode;
use super::hosted_player_session::{
    HostedPlayerSessionAdmissionSnapshot, HostedPlayerSessionIssueGrant,
    HostedPlayerSessionIssueResponse, HostedPlayerSessionIssuer,
};
use super::{emit_stderr_or_event, Level};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const HOSTED_ACCOUNT_LOGIN_START_ROUTE: &str = "/api/public/hosted-account/login/start";
pub(super) const HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE: &str =
    "/api/public/hosted-account/login/complete";
const HOSTED_ACCOUNT_STORE_PATH_ENV: &str = "OASIS7_HOSTED_ACCOUNT_STORE_PATH";
const HOSTED_LOGIN_DELIVERY_MODE_ENV: &str = "OASIS7_HOSTED_LOGIN_DELIVERY_MODE";
const HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE: &str = "preview_inline";
const HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY: &str = "server_log_only";
const HOSTED_LOGIN_DELIVERY_MODE_SMTP: &str = "smtp";
const HOSTED_LOGIN_SMTP_HOST_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_HOST";
const HOSTED_LOGIN_SMTP_PORT_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_PORT";
const HOSTED_LOGIN_SMTP_USERNAME_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_USERNAME";
const HOSTED_LOGIN_SMTP_PASSWORD_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_PASSWORD";
const HOSTED_LOGIN_SMTP_FROM_EMAIL_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_FROM_EMAIL";
const HOSTED_LOGIN_SMTP_FROM_NAME_ENV: &str = "OASIS7_HOSTED_LOGIN_SMTP_FROM_NAME";
const HOSTED_LOGIN_SMTP_DEFAULT_HOST: &str = "smtpdm.aliyun.com";
const HOSTED_LOGIN_SMTP_DEFAULT_PORT: u16 = 465;
const LOGIN_CHALLENGE_TTL_MS: u64 = 10 * 60 * 1000;
const LOGIN_START_RESEND_COOLDOWN_MS: u64 = 30_000;
const LOGIN_START_BURST_WINDOW_MS: u64 = 60_000;
const LOGIN_START_BURST_LIMIT_PER_HANDLE: usize = 3;
const LOGIN_START_EXTENDED_WINDOW_MS: u64 = 10 * 60_000;
const LOGIN_START_EXTENDED_LIMIT_PER_HANDLE: usize = 10;
const LOGIN_CHALLENGE_MAX_ATTEMPTS: u8 = 5;
const HOSTED_LOGIN_EMAIL_SUBJECT: &str = "Oasis7 login code";

#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAccountLoginStartResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) retry_after_seconds: Option<u64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) preview_code: Option<String>,
}

#[derive(Debug)]
pub(super) struct HostedAccountIdentityBroker {
    store_path: PathBuf,
    store: HostedAccountStore,
    delivery_mode: String,
    smtp_config: Option<HostedLoginSmtpConfig>,
    next_challenge_sequence: u64,
    otp_secret: u64,
    recent_start_timestamps_by_factor: BTreeMap<String, VecDeque<u64>>,
    pending_challenges: BTreeMap<String, PendingHostedLoginChallenge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedLoginSmtpConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    from_email: String,
    from_name: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingHostedLoginChallenge {
    challenge_id: String,
    login_channel: String,
    normalized_login_hint: String,
    masked_login_hint: String,
    otp_code: String,
    expires_at_unix_ms: u64,
    failed_attempts: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedLoginStartBlock {
    error_code: String,
    error: String,
    retry_after_seconds: u64,
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
        let delivery_mode = normalize_delivery_mode(
            std::env::var(HOSTED_LOGIN_DELIVERY_MODE_ENV)
                .ok()
                .as_deref(),
        );
        let smtp_config = if delivery_mode == HOSTED_LOGIN_DELIVERY_MODE_SMTP {
            Some(HostedLoginSmtpConfig::from_env()?)
        } else {
            None
        };
        Ok(Self {
            store_path,
            store,
            delivery_mode,
            smtp_config,
            next_challenge_sequence: 0,
            otp_secret: now_unix_ms() ^ 0xa5a5_1357_5a5a_c3c3,
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
            smtp_config: None,
            next_challenge_sequence: 0,
            otp_secret: now_unix_ms() ^ 0x5a5a_c3c3_a5a5_1357,
            recent_start_timestamps_by_factor: BTreeMap::new(),
            pending_challenges: BTreeMap::new(),
        })
    }

    pub(super) fn disabled() -> Self {
        Self {
            store_path: PathBuf::new(),
            store: HostedAccountStore::default(),
            delivery_mode: HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE.to_string(),
            smtp_config: None,
            next_challenge_sequence: 0,
            otp_secret: 0,
            recent_start_timestamps_by_factor: BTreeMap::new(),
            pending_challenges: BTreeMap::new(),
        }
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
                retry_after_seconds: None,
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        }
        self.prune_expired_challenges();
        let Some(channel) = normalize_login_channel(login_channel) else {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("unsupported_login_channel".to_string()),
                error: Some("login_channel must be email".to_string()),
                retry_after_seconds: None,
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        };
        let Some(normalized_login_hint) = normalize_login_hint(channel, login_hint) else {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("login_hint_invalid".to_string()),
                error: Some(format!("{channel} login_hint is invalid")),
                retry_after_seconds: None,
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        };
        if let Some(block) = self.login_start_block(channel, normalized_login_hint.as_str()) {
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some(block.error_code),
                error: Some(block.error),
                retry_after_seconds: Some(block.retry_after_seconds),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        }
        let issued_at_unix_ms = now_unix_ms();
        self.next_challenge_sequence = self.next_challenge_sequence.saturating_add(1);
        let factor_key = factor_key(channel, normalized_login_hint.as_str());
        let challenge_id =
            build_login_challenge_id(issued_at_unix_ms, self.next_challenge_sequence);
        let otp_code = build_preview_otp_code(
            self.otp_secret,
            challenge_id.as_str(),
            normalized_login_hint.as_str(),
        );
        let masked_login_hint = mask_login_hint(channel, normalized_login_hint.as_str());
        let expires_at_unix_ms = issued_at_unix_ms.saturating_add(LOGIN_CHALLENGE_TTL_MS);
        let challenge = PendingHostedLoginChallenge {
            challenge_id: challenge_id.clone(),
            login_channel: channel.to_string(),
            normalized_login_hint: normalized_login_hint.clone(),
            masked_login_hint: masked_login_hint.clone(),
            otp_code: otp_code.clone(),
            expires_at_unix_ms,
            failed_attempts: 0,
        };
        if let Err(err) = self.deliver_login_challenge(&challenge) {
            let log_message = format!(
                "hosted account login delivery failed: channel={} target={} reason={err}",
                challenge.login_channel, challenge.masked_login_hint
            );
            emit_stderr_or_event(
                Level::ERROR,
                log_message.as_str(),
                "hosted account login delivery failed",
            );
            return HostedAccountLoginStartResponse {
                ok: false,
                error_code: Some("login_delivery_failed".to_string()),
                error: Some("failed to deliver login verification code; retry shortly".to_string()),
                retry_after_seconds: Some(5),
                deployment_mode: deployment_mode.as_str().to_string(),
                challenge: None,
            };
        }
        self.recent_start_timestamps_by_factor
            .entry(factor_key)
            .or_default()
            .push_back(issued_at_unix_ms);
        self.pending_challenges
            .insert(challenge_id.clone(), challenge);
        HostedAccountLoginStartResponse {
            ok: true,
            error_code: None,
            error: None,
            retry_after_seconds: None,
            deployment_mode: deployment_mode.as_str().to_string(),
            challenge: Some(HostedAccountLoginChallengeSnapshot {
                challenge_id,
                login_channel: channel.to_string(),
                masked_login_hint,
                delivery_mode: self.delivery_mode.clone(),
                expires_at_unix_ms,
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
        let Some(mut challenge) = self.pending_challenges.remove(normalized_challenge_id) else {
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
            challenge.failed_attempts = challenge.failed_attempts.saturating_add(1);
            let locked = challenge.failed_attempts >= LOGIN_CHALLENGE_MAX_ATTEMPTS;
            if !locked {
                self.pending_challenges
                    .insert(challenge.challenge_id.clone(), challenge);
            }
            return HostedAccountLoginCompleteResponse {
                ok: false,
                error_code: Some(
                    if locked {
                        "otp_code_locked"
                    } else {
                        "otp_code_invalid"
                    }
                    .to_string(),
                ),
                error: Some(
                    if locked {
                        "login challenge locked after too many invalid attempts"
                    } else {
                        "login verification code is invalid"
                    }
                    .to_string(),
                ),
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
                .saturating_add(LOGIN_START_EXTENDED_WINDOW_MS)
                <= now
            {
                timestamps.pop_front();
            }
        }
    }

    fn login_start_block(
        &mut self,
        channel: &str,
        normalized_login_hint: &str,
    ) -> Option<HostedLoginStartBlock> {
        self.prune_expired_challenges();
        let factor_key = factor_key(channel, normalized_login_hint);
        let timestamps = self
            .recent_start_timestamps_by_factor
            .get(factor_key.as_str())?;
        let now = now_unix_ms();
        if let Some(last_issued_at_unix_ms) = timestamps.back().copied() {
            let next_allowed_at_unix_ms =
                last_issued_at_unix_ms.saturating_add(LOGIN_START_RESEND_COOLDOWN_MS);
            if next_allowed_at_unix_ms > now {
                let retry_after_seconds =
                    retry_after_seconds(next_allowed_at_unix_ms.saturating_sub(now));
                return Some(HostedLoginStartBlock {
                    error_code: "login_retry_cooldown".to_string(),
                    error: format!(
                        "a login code was just sent for this email; retry in {retry_after_seconds} seconds"
                    ),
                    retry_after_seconds,
                });
            }
        }
        let burst_count = timestamps
            .iter()
            .filter(|issued_at_unix_ms| {
                issued_at_unix_ms.saturating_add(LOGIN_START_BURST_WINDOW_MS) > now
            })
            .count();
        if burst_count >= LOGIN_START_BURST_LIMIT_PER_HANDLE {
            let retry_after_seconds = timestamps
                .iter()
                .find(|issued_at_unix_ms| {
                    issued_at_unix_ms.saturating_add(LOGIN_START_BURST_WINDOW_MS) > now
                })
                .copied()
                .map(|issued_at_unix_ms| {
                    retry_after_seconds(
                        issued_at_unix_ms
                            .saturating_add(LOGIN_START_BURST_WINDOW_MS)
                            .saturating_sub(now),
                    )
                })
                .unwrap_or(60);
            return Some(HostedLoginStartBlock {
                error_code: "login_rate_limited".to_string(),
                error: format!(
                    "too many login codes were requested for this email; retry in {retry_after_seconds} seconds"
                ),
                retry_after_seconds,
            });
        }
        if timestamps.len() >= LOGIN_START_EXTENDED_LIMIT_PER_HANDLE {
            let oldest_tracked_unix_ms = timestamps.front().copied().unwrap_or(now);
            let retry_after_seconds = retry_after_seconds(
                oldest_tracked_unix_ms
                    .saturating_add(LOGIN_START_EXTENDED_WINDOW_MS)
                    .saturating_sub(now),
            );
            return Some(HostedLoginStartBlock {
                error_code: "login_rate_limited".to_string(),
                error: format!(
                    "too many login codes were requested for this email in the last 10 minutes; retry in {retry_after_seconds} seconds"
                ),
                retry_after_seconds,
            });
        }
        None
    }

    fn deliver_login_challenge(
        &self,
        challenge: &PendingHostedLoginChallenge,
    ) -> Result<(), String> {
        match self.delivery_mode.as_str() {
            HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE => Ok(()),
            HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY => {
                emit_delivery_notice(
                    challenge.login_channel.as_str(),
                    challenge.masked_login_hint.as_str(),
                    challenge.otp_code.as_str(),
                );
                Ok(())
            }
            HOSTED_LOGIN_DELIVERY_MODE_SMTP => self
                .smtp_config
                .as_ref()
                .ok_or_else(|| {
                    "smtp delivery mode selected without a resolved SMTP configuration".to_string()
                })?
                .send_login_code(
                    challenge.normalized_login_hint.as_str(),
                    challenge.otp_code.as_str(),
                ),
            other => Err(format!("unsupported hosted login delivery mode `{other}`")),
        }
    }
}

impl HostedLoginSmtpConfig {
    fn from_env() -> Result<Self, String> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup<F>(mut lookup: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let from_email = required_trimmed_lookup(
            &mut lookup,
            HOSTED_LOGIN_SMTP_FROM_EMAIL_ENV,
            "hosted login SMTP sender email",
        )?;
        if normalize_email(from_email.as_str()).is_none() {
            return Err(format!(
                "hosted login SMTP sender email `{from_email}` must be a valid email address"
            ));
        }
        let password = required_trimmed_lookup(
            &mut lookup,
            HOSTED_LOGIN_SMTP_PASSWORD_ENV,
            "hosted login SMTP password",
        )?;
        let host = optional_trimmed_lookup(&mut lookup, HOSTED_LOGIN_SMTP_HOST_ENV)
            .unwrap_or_else(|| HOSTED_LOGIN_SMTP_DEFAULT_HOST.to_string());
        let port = match optional_trimmed_lookup(&mut lookup, HOSTED_LOGIN_SMTP_PORT_ENV) {
            Some(raw) => raw
                .parse::<u16>()
                .map_err(|err| format!("hosted login SMTP port `{raw}` is invalid: {err}"))?,
            None => HOSTED_LOGIN_SMTP_DEFAULT_PORT,
        };
        let username = optional_trimmed_lookup(&mut lookup, HOSTED_LOGIN_SMTP_USERNAME_ENV)
            .unwrap_or_else(|| from_email.clone());
        let from_name = optional_trimmed_lookup(&mut lookup, HOSTED_LOGIN_SMTP_FROM_NAME_ENV);
        Ok(Self {
            host,
            port,
            username,
            password,
            from_email,
            from_name,
        })
    }

    fn send_login_code(&self, target_email: &str, otp_code: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(self.from_mailbox()?)
            .to(parse_mailbox(target_email, "target email")?)
            .subject(HOSTED_LOGIN_EMAIL_SUBJECT)
            .body(build_login_email_body(otp_code))
            .map_err(|err| format!("failed to build hosted login SMTP email: {err}"))?;
        let transport = SmtpTransport::relay(self.host.as_str())
            .map_err(|err| {
                format!(
                    "failed to configure hosted login SMTP relay `{}`: {err}",
                    self.host
                )
            })?
            .port(self.port)
            .credentials(Credentials::new(
                self.username.clone(),
                self.password.clone(),
            ))
            .build();
        transport.send(&email).map_err(|err| {
            format!(
                "failed to send hosted login SMTP email via {}:{}: {err}",
                self.host, self.port
            )
        })?;
        Ok(())
    }

    fn from_mailbox(&self) -> Result<Mailbox, String> {
        let raw = if let Some(name) = self.from_name.as_deref() {
            format!("{name} <{}>", self.from_email)
        } else {
            self.from_email.clone()
        };
        parse_mailbox(raw.as_str(), "sender email")
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
    #[cfg(unix)]
    {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|err| {
                format!(
                    "failed to open hosted account store `{}`: {err}",
                    path.display()
                )
            })?;
        file.write_all(raw.as_bytes()).map_err(|err| {
            format!(
                "failed to write hosted account store `{}`: {err}",
                path.display()
            )
        })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|err| {
                format!(
                    "failed to secure hosted account store permissions `{}`: {err}",
                    path.display()
                )
            })?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        fs::write(path, raw).map_err(|err| {
            format!(
                "failed to write hosted account store `{}`: {err}",
                path.display()
            )
        })
    }
}

fn normalize_delivery_mode(raw: Option<&str>) -> String {
    match raw.unwrap_or("").trim() {
        HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY => {
            HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY.to_string()
        }
        HOSTED_LOGIN_DELIVERY_MODE_SMTP => HOSTED_LOGIN_DELIVERY_MODE_SMTP.to_string(),
        _ => HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE.to_string(),
    }
}

fn emit_delivery_notice(channel: &str, masked_login_hint: &str, otp_code: &str) {
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
        _ => "***".to_string(),
    }
}

fn factor_key(channel: &str, normalized_login_hint: &str) -> String {
    format!("{channel}:{normalized_login_hint}")
}

fn optional_trimmed_lookup<F>(lookup: &mut F, key: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(key).and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn required_trimmed_lookup<F>(lookup: &mut F, key: &str, label: &str) -> Result<String, String>
where
    F: FnMut(&str) -> Option<String>,
{
    optional_trimmed_lookup(lookup, key)
        .ok_or_else(|| format!("{label} env `{key}` is required when SMTP delivery is enabled"))
}

fn parse_mailbox(raw: &str, label: &str) -> Result<Mailbox, String> {
    raw.parse()
        .map_err(|err| format!("invalid hosted login SMTP {label} `{raw}`: {err}"))
}

fn build_login_email_body(otp_code: &str) -> String {
    let expires_in_minutes = LOGIN_CHALLENGE_TTL_MS / 60_000;
    format!(
        "Your Oasis7 login code is {otp_code}.\n\nThis code expires in {expires_in_minutes} minutes.\nIf you did not request this code, you can ignore this email.\n"
    )
}

fn retry_after_seconds(remaining_ms: u64) -> u64 {
    remaining_ms.saturating_add(999).saturating_div(1000).max(1)
}

fn build_login_challenge_id(issued_at_unix_ms: u64, sequence: u64) -> String {
    format!("hosted-login-challenge-{issued_at_unix_ms:016x}-{sequence:08x}")
}

fn build_preview_otp_code(
    otp_secret: u64,
    challenge_id: &str,
    normalized_login_hint: &str,
) -> String {
    let mut hasher = DefaultHasher::new();
    otp_secret.hash(&mut hasher);
    challenge_id.hash(&mut hasher);
    normalized_login_hint.hash(&mut hasher);
    format!("{:06}", hasher.finish() % 1_000_000)
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
#[path = "hosted_account_identity_tests.rs"]
mod tests;
