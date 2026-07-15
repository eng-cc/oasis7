use super::build_chain_status_payload;
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7::simulator::RuntimePerfSnapshot;
use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeConsensusSnapshot, NodeNetworkPolicy,
    NodeReachabilityAutoDetection, NodeRole, NodeSnapshot, NodeUserMode,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole};
use std::path::Path;

pub(super) fn build_minimal_status_payload_with_world_dir_runtime_perf_wasm_traffic_and_observer_error(
    execution_world_dir: &Path,
    execution_records_dir: Option<&Path>,
    runtime_perf: Option<RuntimePerfSnapshot>,
    wasm: super::wasm_status::ChainWasmStatus,
    traffic: super::ChainTrafficStatus,
    consensus_progress_observer_error: Option<String>,
) -> super::status_payload::ChainStatusResponse {
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: false,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus: NodeConsensusSnapshot::default(),
        consensus_progress_observer_error,
        last_error: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Sequencer,
        NodeUserMode::PrivateSafe,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");

    build_chain_status_payload(
        snapshot,
        execution_world_dir,
        execution_records_dir,
        None,
        &recommendation,
        None,
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::ValidatorCore,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        ReleaseSecurityPolicy::default(),
        super::status_payload_tests::minimal_reward_runtime_metrics(),
        super::status_payload_tests::minimal_storage_metrics(),
        wasm,
        runtime_perf,
        traffic,
        super::status_payload_tests::minimal_transfer_status(),
        super::ChainReplicationDebugStatus::default(),
    )
}

#[test]
fn build_chain_status_payload_marks_consensus_progress_observer_error_critical_not_ready() {
    const OBSERVER_ERROR: &str = "consensus progress observer queue saturated";

    let payload =
        build_minimal_status_payload_with_world_dir_runtime_perf_wasm_traffic_and_observer_error(
            Path::new("/tmp/execution-world"),
            None,
            None,
            super::status_payload_tests::minimal_wasm_status(),
            super::ChainTrafficStatus {
                udp_gossip: None,
                libp2p_replication: oasis7_node::Libp2pTrafficMetricsSnapshot::default(),
            },
            Some(OBSERVER_ERROR.to_string()),
        );

    assert_eq!(
        payload.consensus_progress_observer_error.as_deref(),
        Some(OBSERVER_ERROR),
    );
    assert!(payload.observability.alerts.iter().any(|alert| {
        alert.severity == "critical" && alert.code == "consensus_progress_observer_error"
    }));
    assert!(!payload.readiness.ready);
    assert_eq!(payload.readiness.status, "not_ready");
    assert_eq!(
        payload.readiness.failed_gates,
        vec!["consensus_progress_observer_error"],
    );
}
