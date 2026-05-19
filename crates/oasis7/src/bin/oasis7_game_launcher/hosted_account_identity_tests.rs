use super::*;
use std::time::Duration;

fn temp_store_path(name: &str) -> PathBuf {
    let unique = now_unix_ms();
    std::env::temp_dir().join(format!("oasis7-hosted-account-{name}-{unique}.json"))
}

#[test]
fn hosted_account_login_start_rejects_phone_channel() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("invalid")).expect("broker");
    let response = broker.start_login(DeploymentMode::HostedPublicJoin, "phone", "+1 415 555 0101");
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("unsupported_login_channel")
    );
    assert_eq!(
        response.error.as_deref(),
        Some("login_channel must be email")
    );
}

#[test]
fn hosted_account_login_complete_reuses_stable_player_id() {
    let path = temp_store_path("stable-player");
    let mut broker = HostedAccountIdentityBroker::with_store_path(path.clone()).expect("broker");
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
    broker.recent_start_timestamps_by_factor.clear();
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
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("wrong-otp")).expect("broker");
    let mut issuer = HostedPlayerSessionIssuer::default();
    let start = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    let challenge = start.challenge.expect("challenge");
    let wrong_code = if challenge.preview_code.as_deref() == Some("000000") {
        "000001"
    } else {
        "000000"
    };
    let response = broker.complete_login(
        DeploymentMode::HostedPublicJoin,
        challenge.challenge_id.as_str(),
        wrong_code,
        &mut issuer,
    );
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("otp_code_invalid"));
}

#[test]
fn hosted_account_login_complete_locks_after_repeated_invalid_otp() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("otp-lock")).expect("broker");
    let mut issuer = HostedPlayerSessionIssuer::default();
    let start = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    let challenge = start.challenge.expect("challenge");
    let wrong_code = if challenge.preview_code.as_deref() == Some("000000") {
        "000001"
    } else {
        "000000"
    };
    for _ in 0..(LOGIN_CHALLENGE_MAX_ATTEMPTS - 1) {
        let response = broker.complete_login(
            DeploymentMode::HostedPublicJoin,
            challenge.challenge_id.as_str(),
            wrong_code,
            &mut issuer,
        );
        assert_eq!(response.error_code.as_deref(), Some("otp_code_invalid"));
    }
    let locked = broker.complete_login(
        DeploymentMode::HostedPublicJoin,
        challenge.challenge_id.as_str(),
        wrong_code,
        &mut issuer,
    );
    assert_eq!(locked.error_code.as_deref(), Some("otp_code_locked"));

    let missing = broker.complete_login(
        DeploymentMode::HostedPublicJoin,
        challenge.challenge_id.as_str(),
        challenge.preview_code.as_deref().unwrap_or_default(),
        &mut issuer,
    );
    assert_eq!(missing.error_code.as_deref(), Some("challenge_not_found"));
}

#[test]
fn hosted_account_login_start_rolls_back_on_delivery_failure() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("smtp-fail")).expect("broker");
    broker.delivery_mode = HOSTED_LOGIN_DELIVERY_MODE_SMTP.to_string();
    let response = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("login_delivery_failed")
    );
    assert!(broker.pending_challenges.is_empty());
    assert!(broker.recent_start_timestamps_by_factor.is_empty());
}

#[test]
fn hosted_account_login_start_enforces_resend_cooldown() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("cooldown")).expect("broker");
    let first = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    assert!(first.ok);

    let second = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    assert!(!second.ok);
    assert_eq!(second.error_code.as_deref(), Some("login_retry_cooldown"));
    assert!(second.retry_after_seconds.unwrap_or_default() >= 1);
    assert_eq!(broker.pending_challenges.len(), 1);
}

#[test]
fn hosted_account_login_start_enforces_burst_rate_limit() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("burst")).expect("broker");
    let factor = factor_key("email", "player@example.com");
    let now = now_unix_ms();
    broker.recent_start_timestamps_by_factor.insert(
        factor,
        VecDeque::from(vec![now - 59_000, now - 40_000, now - 31_000]),
    );

    let blocked = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("login_rate_limited"));
    assert!(blocked.retry_after_seconds.unwrap_or_default() >= 1);
}

#[test]
fn hosted_account_login_start_enforces_extended_rate_limit() {
    let mut broker =
        HostedAccountIdentityBroker::with_store_path(temp_store_path("extended")).expect("broker");
    let factor = factor_key("email", "player@example.com");
    let now = now_unix_ms();
    broker.recent_start_timestamps_by_factor.insert(
        factor,
        VecDeque::from(vec![
            now - 9 * 60_000,
            now - 8 * 60_000,
            now - 7 * 60_000,
            now - 6 * 60_000,
            now - 5 * 60_000,
            now - 4 * 60_000,
            now - 3 * 60_000,
            now - 2 * 60_000,
            now - 90_000,
            now - 31_000,
        ]),
    );

    let blocked = broker.start_login(
        DeploymentMode::HostedPublicJoin,
        "email",
        "player@example.com",
    );
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("login_rate_limited"));
    assert!(blocked.retry_after_seconds.unwrap_or_default() >= 1);
    assert!(blocked
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("last 10 minutes"));
}

#[test]
fn normalize_delivery_mode_accepts_smtp() {
    assert_eq!(
        normalize_delivery_mode(Some(" smtp ")),
        HOSTED_LOGIN_DELIVERY_MODE_SMTP
    );
    assert_eq!(
        normalize_delivery_mode(Some("server_log_only")),
        HOSTED_LOGIN_DELIVERY_MODE_SERVER_LOG_ONLY
    );
    assert_eq!(
        normalize_delivery_mode(Some("unexpected")),
        HOSTED_LOGIN_DELIVERY_MODE_PREVIEW_INLINE
    );
}

#[test]
fn hosted_login_smtp_config_defaults_to_aliyun_relay() {
    let config = HostedLoginSmtpConfig::from_lookup(|key| match key {
        HOSTED_LOGIN_SMTP_FROM_EMAIL_ENV => Some("account@mail.oasis7.tech".to_string()),
        HOSTED_LOGIN_SMTP_PASSWORD_ENV => Some("smtp-secret".to_string()),
        _ => None,
    })
    .expect("config");
    assert_eq!(config.host, HOSTED_LOGIN_SMTP_DEFAULT_HOST);
    assert_eq!(config.port, HOSTED_LOGIN_SMTP_DEFAULT_PORT);
    assert_eq!(config.username, "account@mail.oasis7.tech");
    assert_eq!(config.from_email, "account@mail.oasis7.tech");
    assert_eq!(config.from_name.as_deref(), None);
}

#[test]
fn hosted_login_smtp_config_supports_custom_sender_fields() {
    let config = HostedLoginSmtpConfig::from_lookup(|key| match key {
        HOSTED_LOGIN_SMTP_FROM_EMAIL_ENV => Some("account@mail.oasis7.tech".to_string()),
        HOSTED_LOGIN_SMTP_PASSWORD_ENV => Some("smtp-secret".to_string()),
        HOSTED_LOGIN_SMTP_HOST_ENV => Some("smtp.example.com".to_string()),
        HOSTED_LOGIN_SMTP_PORT_ENV => Some("587".to_string()),
        HOSTED_LOGIN_SMTP_USERNAME_ENV => Some("mailer".to_string()),
        HOSTED_LOGIN_SMTP_FROM_NAME_ENV => Some("Oasis7 Accounts".to_string()),
        _ => None,
    })
    .expect("config");
    assert_eq!(config.host, "smtp.example.com");
    assert_eq!(config.port, 587);
    assert_eq!(config.username, "mailer");
    assert_eq!(config.from_name.as_deref(), Some("Oasis7 Accounts"));
    assert_eq!(
        config.from_mailbox().expect("mailbox").to_string(),
        "Oasis7 Accounts <account@mail.oasis7.tech>"
    );
}

#[test]
#[ignore = "requires live SMTP credentials and a recipient mailbox"]
fn hosted_login_smtp_live_smoke() {
    let config = HostedLoginSmtpConfig::from_env().expect("smtp config from env");
    let target = std::env::var("OASIS7_HOSTED_LOGIN_SMTP_SMOKE_TO_EMAIL")
        .expect("OASIS7_HOSTED_LOGIN_SMTP_SMOKE_TO_EMAIL");
    assert!(
        !target.trim().is_empty(),
        "OASIS7_HOSTED_LOGIN_SMTP_SMOKE_TO_EMAIL must not be empty"
    );
    config
        .send_login_code(target.trim(), "123456")
        .expect("live SMTP delivery");
}
