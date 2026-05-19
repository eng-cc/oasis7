use serde::Serialize;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

pub(super) const DEFAULT_DEPLOYMENT_MODE: &str = "trusted_local_only";
#[allow(dead_code)]
pub(super) const HOSTED_PLAYER_ACCESS_VERDICT: &str = "specified_not_implemented";
#[allow(dead_code)]
const DEFAULT_MAX_GUEST_SESSIONS: u64 = 32;
#[allow(dead_code)]
const DEFAULT_MAX_PLAYER_SESSIONS: u64 = 8;
#[allow(dead_code)]
const DEFAULT_ISSUE_RATE_LIMIT_PER_MINUTE: u64 = 60;
#[allow(dead_code)]
const DEFAULT_WORLD_FULL_POLICY: &str = "reject";
#[allow(dead_code)]
const DEFAULT_KICK_POLICY: &str = "operator_audit_required";
const HOSTED_STRONG_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY";
const HOSTED_STRONG_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY";
const HOSTED_STRONG_AUTH_APPROVAL_CODE_ENV: &str = "OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE";

#[cfg(test)]
pub(super) fn hosted_strong_auth_test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeploymentMode {
    TrustedLocalOnly,
    HostedPublicJoin,
}

impl DeploymentMode {
    pub(super) fn parse(raw: &str, label: &str) -> Result<Self, String> {
        match raw.trim() {
            "trusted_local_only" => Ok(Self::TrustedLocalOnly),
            "hosted_public_join" => Ok(Self::HostedPublicJoin),
            _ => Err(format!(
                "{label} must be one of: trusted_local_only|hosted_public_join"
            )),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::TrustedLocalOnly => "trusted_local_only",
            Self::HostedPublicJoin => "hosted_public_join",
        }
    }

    pub(super) fn browser_signer_bootstrap_mode(self) -> &'static str {
        match self {
            Self::TrustedLocalOnly => "trusted_local_bootstrap_allowed",
            Self::HostedPublicJoin => "disabled_for_public_player_plane",
        }
    }

    pub(super) fn allows_local_chain_runtime(self) -> bool {
        matches!(self, Self::TrustedLocalOnly)
    }

    pub(super) fn local_chain_runtime_mode(self) -> &'static str {
        match self {
            Self::TrustedLocalOnly => "launcher_managed_local_runtime_allowed",
            Self::HostedPublicJoin => "blocked_for_public_player_plane",
        }
    }

    pub(super) fn node_admission_mode(self) -> &'static str {
        match self {
            Self::TrustedLocalOnly => "trusted_local_preview_only",
            Self::HostedPublicJoin => "operator_managed_node_onboarding_only",
        }
    }

    pub(super) fn gui_agent_action_surface(self) -> &'static str {
        match self {
            Self::TrustedLocalOnly => "legacy_shared_local_preview",
            Self::HostedPublicJoin => "legacy_private_control_plane_only",
        }
    }

    #[allow(dead_code)]
    pub(super) fn requires_loopback_private_control(self) -> bool {
        matches!(self, Self::HostedPublicJoin)
    }

    #[allow(dead_code)]
    pub(super) fn disables_browser_signer_bootstrap(self) -> bool {
        matches!(self, Self::HostedPublicJoin)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedAdmissionControlContract {
    pub(super) max_guest_sessions: u64,
    pub(super) max_player_sessions: u64,
    pub(super) issue_rate_limit_per_minute: u64,
    pub(super) world_full_policy: String,
    pub(super) kick_policy: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedPlayerAccessContract {
    pub(super) deployment_mode: String,
    pub(super) verdict: String,
    pub(super) browser_signer_bootstrap: String,
    pub(super) local_chain_runtime: String,
    pub(super) node_admission: String,
    pub(super) gui_agent_action_surface: String,
    pub(super) public_state_route: String,
    pub(super) public_endpoints: Vec<String>,
    pub(super) private_endpoints: Vec<String>,
    pub(super) session_ladder: Vec<String>,
    pub(super) action_matrix: Vec<HostedActionAccessPolicy>,
    pub(super) admission: HostedAdmissionControlContract,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedActionAccessPolicy {
    pub(super) action_id: String,
    pub(super) required_auth: String,
    pub(super) availability: String,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub(super) struct HostedViewerAccessHint {
    pub(super) deployment_mode: String,
    pub(super) verdict: String,
    pub(super) browser_signer_bootstrap: String,
    pub(super) local_chain_runtime: String,
    pub(super) node_admission: String,
    pub(super) session_ladder: Vec<String>,
    pub(super) action_matrix: Vec<HostedActionAccessPolicy>,
}

#[allow(dead_code)]
pub(super) fn hosted_player_access_contract(mode: DeploymentMode) -> HostedPlayerAccessContract {
    HostedPlayerAccessContract {
        deployment_mode: mode.as_str().to_string(),
        verdict: HOSTED_PLAYER_ACCESS_VERDICT.to_string(),
        browser_signer_bootstrap: mode.browser_signer_bootstrap_mode().to_string(),
        local_chain_runtime: mode.local_chain_runtime_mode().to_string(),
        node_admission: mode.node_admission_mode().to_string(),
        gui_agent_action_surface: mode.gui_agent_action_surface().to_string(),
        public_state_route: "/api/public/state".to_string(),
        public_endpoints: web_launcher_public_endpoints()
            .into_iter()
            .map(|value| (*value).to_string())
            .collect(),
        private_endpoints: web_launcher_private_endpoints()
            .into_iter()
            .map(|value| (*value).to_string())
            .collect(),
        session_ladder: vec![
            "guest_session".to_string(),
            "player_session".to_string(),
            "strong_auth".to_string(),
        ],
        action_matrix: hosted_action_matrix(mode),
        admission: HostedAdmissionControlContract {
            max_guest_sessions: DEFAULT_MAX_GUEST_SESSIONS,
            max_player_sessions: DEFAULT_MAX_PLAYER_SESSIONS,
            issue_rate_limit_per_minute: DEFAULT_ISSUE_RATE_LIMIT_PER_MINUTE,
            world_full_policy: DEFAULT_WORLD_FULL_POLICY.to_string(),
            kick_policy: DEFAULT_KICK_POLICY.to_string(),
        },
    }
}

#[allow(dead_code)]
pub(super) fn hosted_viewer_access_hint(mode: DeploymentMode) -> HostedViewerAccessHint {
    HostedViewerAccessHint {
        deployment_mode: mode.as_str().to_string(),
        verdict: HOSTED_PLAYER_ACCESS_VERDICT.to_string(),
        browser_signer_bootstrap: mode.browser_signer_bootstrap_mode().to_string(),
        local_chain_runtime: mode.local_chain_runtime_mode().to_string(),
        node_admission: mode.node_admission_mode().to_string(),
        session_ladder: vec![
            "guest_session".to_string(),
            "player_session".to_string(),
            "strong_auth".to_string(),
        ],
        action_matrix: hosted_action_matrix(mode),
    }
}

#[allow(dead_code)]
fn hosted_action_matrix(mode: DeploymentMode) -> Vec<HostedActionAccessPolicy> {
    let hosted_strong_auth_backend_grant_enabled = hosted_strong_auth_backend_grant_enabled();
    let prompt_strong_auth_availability = match mode {
        DeploymentMode::TrustedLocalOnly => "trusted_local_preview_only",
        DeploymentMode::HostedPublicJoin if hosted_strong_auth_backend_grant_enabled => {
            "public_player_plane_with_backend_reauth_preview"
        }
        DeploymentMode::HostedPublicJoin => "blocked_until_strong_auth",
    };
    let prompt_strong_auth_reason = match mode {
        DeploymentMode::TrustedLocalOnly => {
            "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
        }
        DeploymentMode::HostedPublicJoin if hosted_strong_auth_backend_grant_enabled => {
            "hosted public join allows prompt_control through browser-local player auth plus short-lived backend strong-auth grant; this remains preview-grade until stronger custody lands"
        }
        DeploymentMode::HostedPublicJoin => {
            "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
        }
    };
    let asset_strong_auth_availability = match mode {
        DeploymentMode::TrustedLocalOnly => "trusted_local_preview_only",
        DeploymentMode::HostedPublicJoin => "blocked_until_strong_auth",
    };
    let asset_strong_auth_reason = match mode {
        DeploymentMode::TrustedLocalOnly => {
            "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
        }
        DeploymentMode::HostedPublicJoin => {
            "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
        }
    };
    vec![
        HostedActionAccessPolicy {
            action_id: "gameplay_action".to_string(),
            required_auth: "player_session".to_string(),
            availability: "public_player_plane".to_string(),
            reason: "core gameplay input stays on the player_session lane".to_string(),
        },
        HostedActionAccessPolicy {
            action_id: "agent_chat".to_string(),
            required_auth: "player_session".to_string(),
            availability: "public_player_plane".to_string(),
            reason: "agent chat currently stays on the low-risk player_session lane".to_string(),
        },
        HostedActionAccessPolicy {
            action_id: "prompt_control_preview".to_string(),
            required_auth: "strong_auth".to_string(),
            availability: prompt_strong_auth_availability.to_string(),
            reason: prompt_strong_auth_reason.to_string(),
        },
        HostedActionAccessPolicy {
            action_id: "prompt_control_apply".to_string(),
            required_auth: "strong_auth".to_string(),
            availability: prompt_strong_auth_availability.to_string(),
            reason: prompt_strong_auth_reason.to_string(),
        },
        HostedActionAccessPolicy {
            action_id: "prompt_control_rollback".to_string(),
            required_auth: "strong_auth".to_string(),
            availability: prompt_strong_auth_availability.to_string(),
            reason: prompt_strong_auth_reason.to_string(),
        },
        HostedActionAccessPolicy {
            action_id: "main_token_transfer".to_string(),
            required_auth: "strong_auth".to_string(),
            availability: asset_strong_auth_availability.to_string(),
            reason: asset_strong_auth_reason.to_string(),
        },
    ]
}

#[allow(dead_code)]
pub(super) fn web_launcher_public_endpoints() -> &'static [&'static str] {
    &[
        "/healthz",
        "/api/public/player-session/admission",
        "/api/public/player-session/refresh",
        "/api/public/hosted-account/login/start",
        "/api/public/hosted-account/login/complete",
        "/api/public/state",
        "/api/public/player-session/issue",
        "/api/public/player-session/release",
        "/api/public/strong-auth/grant",
        "/api/chain/transfer",
        "/api/chain/transfer/accounts",
        "/api/chain/transfer/status",
        "/api/chain/transfer/history",
        "/api/chain/explorer/overview",
        "/api/chain/explorer/transactions",
        "/api/chain/explorer/transaction",
        "/api/chain/explorer/blocks",
        "/api/chain/explorer/block",
        "/api/chain/explorer/txs",
        "/api/chain/explorer/tx",
        "/api/chain/explorer/search",
        "/api/chain/explorer/address",
        "/api/chain/explorer/contracts",
        "/api/chain/explorer/contract",
        "/api/chain/explorer/assets",
        "/api/chain/explorer/mempool",
        "/api/chain/feedback",
    ]
}

fn hosted_strong_auth_backend_grant_enabled() -> bool {
    env_non_empty(HOSTED_STRONG_AUTH_PUBLIC_KEY_ENV)
        && env_non_empty(HOSTED_STRONG_AUTH_PRIVATE_KEY_ENV)
        && env_non_empty(HOSTED_STRONG_AUTH_APPROVAL_CODE_ENV)
}

fn env_non_empty(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false)
}

#[allow(dead_code)]
pub(super) fn web_launcher_private_endpoints() -> &'static [&'static str] {
    &[
        "/",
        "/api/state",
        "/api/gui-agent/capabilities",
        "/api/gui-agent/state",
        "/api/gui-agent/action",
        "/api/ui/schema",
        "/api/start",
        "/api/stop",
        "/api/chain/start",
        "/api/chain/stop",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_env() {
        for name in [
            HOSTED_STRONG_AUTH_PUBLIC_KEY_ENV,
            HOSTED_STRONG_AUTH_PRIVATE_KEY_ENV,
            HOSTED_STRONG_AUTH_APPROVAL_CODE_ENV,
        ] {
            std::env::remove_var(name);
        }
    }

    fn prompt_control_apply_policy(mode: DeploymentMode) -> HostedActionAccessPolicy {
        hosted_viewer_access_hint(mode)
            .action_matrix
            .into_iter()
            .find(|policy| policy.action_id == "prompt_control_apply")
            .expect("prompt_control_apply policy")
    }

    fn main_token_transfer_policy(mode: DeploymentMode) -> HostedActionAccessPolicy {
        hosted_viewer_access_hint(mode)
            .action_matrix
            .into_iter()
            .find(|policy| policy.action_id == "main_token_transfer")
            .expect("main_token_transfer policy")
    }

    #[test]
    fn hosted_public_join_prompt_control_stays_blocked_without_backend_grant_env() {
        let _guard = hosted_strong_auth_test_env_lock().lock().expect("env lock");
        clear_env();
        let policy = prompt_control_apply_policy(DeploymentMode::HostedPublicJoin);
        assert_eq!(policy.required_auth, "strong_auth");
        assert_eq!(policy.availability, "blocked_until_strong_auth");
    }

    #[test]
    fn hosted_public_join_prompt_control_exposes_backend_reauth_preview_when_env_ready() {
        let _guard = hosted_strong_auth_test_env_lock().lock().expect("env lock");
        clear_env();
        std::env::set_var(
            HOSTED_STRONG_AUTH_PUBLIC_KEY_ENV,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        std::env::set_var(
            HOSTED_STRONG_AUTH_PRIVATE_KEY_ENV,
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        std::env::set_var(HOSTED_STRONG_AUTH_APPROVAL_CODE_ENV, "preview-code");
        let policy = prompt_control_apply_policy(DeploymentMode::HostedPublicJoin);
        assert_eq!(
            policy.availability,
            "public_player_plane_with_backend_reauth_preview"
        );
        assert!(policy.reason.contains("backend strong-auth grant"));
        clear_env();
    }

    #[test]
    fn hosted_public_join_main_token_transfer_stays_blocked_even_when_prompt_reauth_env_ready() {
        let _guard = hosted_strong_auth_test_env_lock().lock().expect("env lock");
        clear_env();
        std::env::set_var(
            HOSTED_STRONG_AUTH_PUBLIC_KEY_ENV,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        std::env::set_var(
            HOSTED_STRONG_AUTH_PRIVATE_KEY_ENV,
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        std::env::set_var(HOSTED_STRONG_AUTH_APPROVAL_CODE_ENV, "preview-code");
        let policy = main_token_transfer_policy(DeploymentMode::HostedPublicJoin);
        assert_eq!(policy.required_auth, "strong_auth");
        assert_eq!(policy.availability, "blocked_until_strong_auth");
        assert!(policy.reason.contains("dedicated proof lane"));
        clear_env();
    }

    #[test]
    fn web_launcher_public_endpoints_expose_generic_strong_auth_grant_route() {
        assert!(web_launcher_public_endpoints().contains(&"/api/public/strong-auth/grant"));
        assert!(!web_launcher_public_endpoints()
            .contains(&"/api/public/strong-auth/grant/prompt-control"));
    }

    #[test]
    fn web_launcher_public_endpoints_expose_hosted_account_login_routes() {
        assert!(web_launcher_public_endpoints().contains(&"/api/public/hosted-account/login/start"));
        assert!(
            web_launcher_public_endpoints().contains(&"/api/public/hosted-account/login/complete")
        );
    }
}
