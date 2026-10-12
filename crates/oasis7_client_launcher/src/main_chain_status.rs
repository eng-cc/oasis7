use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainReplicationPeerHealth {
    pub(super) peer_id: String,
    pub(super) status: String,
    pub(super) issues: Vec<String>,
    pub(super) discovery_sources: Vec<String>,
    pub(super) active_path_kind: Option<String>,
    pub(super) source_operator: Option<String>,
    pub(super) source_asn: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainReplicationStatus {
    pub(super) local_peer_id: String,
    pub(super) connected_peers: Vec<String>,
    pub(super) peer_healths: Vec<WebChainReplicationPeerHealth>,
    #[serde(default)]
    pub(super) request_peer_scores: std::collections::BTreeMap<String, u8>,
}

impl crate::ClientLauncherApp {
    pub(super) fn invalidate_chain_session_evidence(&mut self, reason: &str) {
        self.chain_runtime_status = crate::ChainRuntimeStatus::Unreachable(reason.to_string());
        self.chain_identity = None;
        self.chain_p2p_status = None;
        self.chain_observability_status = None;
        self.chain_replication_status = None;
        self.chain_recovery = None;
        self.control_plane_snapshot_received = false;
    }
}

#[cfg(test)]
mod tests {
    use crate::{ChainRuntimeStatus, ClientLauncherApp};

    #[test]
    fn session_state_refresh_failure_invalidates_chain_evidence() {
        assert_session_request_failure_invalidates_chain_evidence(crate::WebApiEvent::State(Err(
            "offline".into(),
        )));
    }

    #[test]
    fn session_action_failure_invalidates_chain_evidence() {
        assert_session_request_failure_invalidates_chain_evidence(crate::WebApiEvent::Action(Err(
            "offline".into(),
        )));
    }

    fn assert_session_request_failure_invalidates_chain_evidence(event: crate::WebApiEvent) {
        let mut app = ClientLauncherApp::default();
        app.control_manage_service = false;
        app.chain_runtime_status = ChainRuntimeStatus::Ready;
        app.chain_identity = Some(crate::web_api_support::WebChainIdentityStatus {
            chain_id: "old".into(),
            network_id: "old".into(),
        });
        app.web_api_tx.send(event).expect("queue failure");
        app.poll_process();
        assert!(
            app.chain_identity.is_none(),
            "offline surface retained authority"
        );
        assert!(matches!(
            app.chain_runtime_status,
            ChainRuntimeStatus::Unreachable(_)
        ));
        assert!(!app.is_feedback_available());
    }
}
