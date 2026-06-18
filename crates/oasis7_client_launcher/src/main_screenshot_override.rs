#[cfg(not(target_arch = "wasm32"))]
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use std::env;

#[cfg(not(target_arch = "wasm32"))]
impl ClientLauncherApp {
    pub(super) fn apply_screenshot_modal_override(&mut self) {
        let Ok(raw) = env::var(OASIS7_CLIENT_LAUNCHER_SCREENSHOT_MODAL_ENV) else {
            return;
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "startup" | "startup_guide" | "guide" => {
                self.startup_guide_state.open = true;
                self.startup_guide_state.target = config_ui::StartupGuideTarget::Game;
                self.startup_guide_state.first_check_done = true;
                self.onboarding_state.open = false;
            }
            "onboarding" | "first_run" => {
                self.onboarding_state.open = true;
                self.onboarding_state.dismissed = false;
            }
            "feedback" => {
                self.onboarding_state.open = false;
            }
            "settings" | "llm_settings" => {
                self.onboarding_state.open = false;
            }
            "advanced_config" | "config" => {
                self.onboarding_state.open = false;
            }
            "transfer" | "chain_transfer" => {
                self.onboarding_state.open = false;
                self.apply_screenshot_heavy_modal_seed("transfer");
            }
            "explorer" | "blockchain_explorer" => {
                self.onboarding_state.open = false;
                self.apply_screenshot_heavy_modal_seed("explorer");
            }
            "peer" | "peer_details" | "p2p" => {
                self.onboarding_state.open = false;
                self.apply_screenshot_heavy_modal_seed("peer_details");
            }
            _ => {}
        }
    }

    fn apply_screenshot_heavy_modal_seed(&mut self, modal: &str) {
        self.config.chain_enabled = true;
        self.chain_runtime_status = ChainRuntimeStatus::Ready;

        if matches!(modal, "transfer") {
            self.transfer_draft.from_account_id = "oasis1qxy2...8k7d3p".to_string();
            self.transfer_draft.to_account_id = "oasis1w9f3...zt681m".to_string();
            self.transfer_draft.amount = "12".to_string();
            self.transfer_draft.nonce = "41".to_string();
        }

        if matches!(modal, "explorer") {
            self.explorer_panel_state.overview =
                Some(explorer_window::WebExplorerOverviewResponse {
                    ok: true,
                    observed_at_unix_ms: 1_765_942_800_000,
                    node_id: "oasis-node-local".to_string(),
                    world_id: "oasis7-mainnet-1".to_string(),
                    latest_height: 1_234_567,
                    committed_height: 1_234_564,
                    network_committed_height: 1_234_567,
                    last_block_hash: Some("0x9f3a7c2d5e6f8091b2c3d4e5f6a7b8c9".to_string()),
                    last_execution_block_hash: Some(
                        "0x2d7b91f0aa3c45d6e778899001122334".to_string(),
                    ),
                    tracked_records: 154_321,
                    transfer_total: 154_321,
                    transfer_accepted: 81,
                    transfer_pending: 12,
                    transfer_confirmed: 154_102,
                    transfer_failed: 9,
                    transfer_timeout: 3,
                    error_code: None,
                    error: None,
                });
        }

        if matches!(modal, "peer_details") {
            self.chain_observability_status = Some(WebChainNodeObservabilityStatus {
                status: "ok".to_string(),
                summary: "3 peers connected, replication stable, one relay path observed"
                    .to_string(),
                ready: true,
                connected_peer_count: 3,
                active_peer_count: 2,
                candidate_peer_count: 1,
                suspect_peer_count: 0,
                blocked_peer_count: 0,
                peer_with_issues_count: 1,
                known_peer_heads: 3,
                network_head_available: true,
                network_height_lag: 0,
                transport_stable: true,
                transport_stability_score: 92,
                reachability_policy_ok: true,
                recent_replication_error_count: 0,
                storage_degraded: false,
                reward_runtime_degraded: false,
                alerts: vec![web_api_support::WebChainNodeObservabilityAlert {
                    severity: "warn".to_string(),
                    code: "relay_path_detected".to_string(),
                    summary: "One peer is reachable through relay fallback".to_string(),
                }],
            });
            self.chain_replication_status = Some(WebChainReplicationStatus {
                local_peer_id: "12D3KooWLocalPeer11111111111111111111".to_string(),
                connected_peers: vec![
                    "12D3KooWLt7u...aBcD1234EfGh5678IjKl".to_string(),
                    "12D3KooWSm9v...xYzA9876BcDe5432FgHi".to_string(),
                    "12D3KooWRp3q...LmNo2468QrSt1357UvWx".to_string(),
                ],
                peer_healths: vec![
                    WebChainReplicationPeerHealth {
                        peer_id: "12D3KooWLt7u...aBcD1234EfGh5678IjKl".to_string(),
                        status: "active".to_string(),
                        issues: Vec::new(),
                        discovery_sources: vec!["bootstrap".to_string()],
                        active_path_kind: Some("direct".to_string()),
                        source_operator: Some("Oasis7".to_string()),
                        source_asn: Some("AS64500".to_string()),
                    },
                    WebChainReplicationPeerHealth {
                        peer_id: "12D3KooWSm9v...xYzA9876BcDe5432FgHi".to_string(),
                        status: "active".to_string(),
                        issues: Vec::new(),
                        discovery_sources: vec!["mdns".to_string()],
                        active_path_kind: Some("direct".to_string()),
                        source_operator: None,
                        source_asn: None,
                    },
                    WebChainReplicationPeerHealth {
                        peer_id: "12D3KooWRp3q...LmNo2468QrSt1357UvWx".to_string(),
                        status: "candidate".to_string(),
                        issues: vec!["relay fallback".to_string()],
                        discovery_sources: vec!["relay".to_string()],
                        active_path_kind: Some("relay".to_string()),
                        source_operator: Some("Relay".to_string()),
                        source_asn: Some("AS64501".to_string()),
                    },
                ],
                request_peer_scores: std::collections::BTreeMap::new(),
            });
        }
    }
}
