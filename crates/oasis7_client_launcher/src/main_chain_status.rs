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
        self.web_state_generation = self.web_state_generation.wrapping_add(1);
        self.chain_runtime_status = crate::ChainRuntimeStatus::Unreachable(reason.to_string());
        self.clear_chain_session_evidence();
    }

    pub(super) fn clear_chain_session_evidence(&mut self) {
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
        assert_session_request_failure_invalidates_chain_evidence(crate::WebApiEvent::State(
            0,
            Err("offline".into()),
        ));
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

#[cfg(test)]
mod response_order_tests {
    use crate::{
        ChainRuntimeStatus, ClientLauncherApp, WebApiEvent, WebRequestDomain, WebStateSnapshot,
    };

    #[test]
    fn late_state_poll_cannot_restore_evidence_after_control_action() {
        let mut app = ClientLauncherApp::default();
        app.control_manage_service = false;
        let snapshot = WebStateSnapshot {
            status: "idle".into(),
            detail: None,
            chain_status: "ready".into(),
            chain_detail: None,
            chain_identity: Some(crate::web_api_support::WebChainIdentityStatus {
                chain_id: "old".into(),
                network_id: "old".into(),
            }),
            chain_p2p_status: None,
            chain_observability_status: None,
            chain_replication_status: None,
            chain_recovery: None,
            game_url: "http://localhost/".into(),
            config: app.config.clone(),
            logs: vec![],
        };
        app.config.chain_enabled = true;
        app.chain_runtime_status = ChainRuntimeStatus::Ready;
        let mut fresh = snapshot.clone();
        fresh
            .chain_identity
            .as_mut()
            .expect("fresh identity")
            .chain_id = "current".into();
        app.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        assert!(
            !app.is_feedback_available(),
            "pending control retained Ready"
        );
        app.web_api_tx
            .send(WebApiEvent::State(
                app.web_state_generation,
                Ok(snapshot.clone()),
            ))
            .expect("queue poll during action");
        app.poll_process();
        assert!(
            app.chain_identity.is_none(),
            "poll applied while control action was pending"
        );
        app.set_web_request_inflight(WebRequestDomain::ControlAction, false);
        app.web_api_tx
            .send(WebApiEvent::State(0, Ok(snapshot)))
            .expect("queue late poll");
        app.poll_process();
        assert!(
            app.chain_identity.is_none(),
            "late poll restored previous session identity"
        );
        app.web_api_tx
            .send(WebApiEvent::State(app.web_state_generation, Ok(fresh)))
            .expect("queue current poll");
        app.poll_process();
        assert_eq!(
            app.chain_identity
                .as_ref()
                .expect("current identity")
                .chain_id,
            "current"
        );
        app.web_api_tx
            .send(WebApiEvent::State(0, Err("old failure".into())))
            .expect("queue old failure");
        app.poll_process();
        assert!(
            app.chain_identity.is_some(),
            "old failure invalidated current evidence"
        );
    }
}
