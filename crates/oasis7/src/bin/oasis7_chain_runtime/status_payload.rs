use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeNetworkPolicy, NodeReachabilityAutoDetection,
    NodeUserModeRecommendation,
};
use serde::Serialize;

use super::p2p_status::peer_reachability_as_str;

#[derive(Debug, Serialize)]
pub(super) struct ChainP2pStatus {
    pub(super) requested_user_mode: String,
    pub(super) recommended_user_mode: String,
    pub(super) effective_user_mode: String,
    pub(super) applied_effective_user_mode: Option<String>,
    pub(super) requires_explicit_public_entry_confirmation: bool,
    pub(super) detected_reachability: Option<String>,
    pub(super) hole_punch_viability: String,
    pub(super) autonat_status: String,
    pub(super) public_port_reachability: String,
    pub(super) observed_public_addr: Option<String>,
    pub(super) confirmed_external_direct_addrs: Vec<String>,
    pub(super) relay_available: bool,
    pub(super) probe_stable: bool,
    pub(super) deployment_mode: String,
    pub(super) node_role_claim: String,
    pub(super) rationale: Vec<String>,
}

pub(super) fn build_chain_p2p_status(
    live_p2p_recommendation: &NodeUserModeRecommendation,
    applied_effective_user_mode: Option<String>,
    effective_p2p_policy: NodeNetworkPolicy,
    live_snapshot: &Libp2pReachabilitySnapshot,
    p2p_detection: NodeReachabilityAutoDetection,
) -> ChainP2pStatus {
    ChainP2pStatus {
        requested_user_mode: live_p2p_recommendation
            .requested_user_mode
            .as_str()
            .to_string(),
        recommended_user_mode: live_p2p_recommendation
            .recommended_user_mode
            .as_str()
            .to_string(),
        effective_user_mode: live_p2p_recommendation
            .effective_user_mode
            .as_str()
            .to_string(),
        applied_effective_user_mode,
        requires_explicit_public_entry_confirmation: live_p2p_recommendation
            .requires_explicit_public_entry_confirmation,
        detected_reachability: p2p_detection
            .observed_reachability
            .map(peer_reachability_as_str)
            .map(str::to_string),
        hole_punch_viability: p2p_detection.hole_punch_viability.to_string(),
        autonat_status: p2p_detection.autonat_status.to_string(),
        public_port_reachability: p2p_detection.public_port_reachability.to_string(),
        observed_public_addr: live_snapshot.observed_public_addr.clone(),
        confirmed_external_direct_addrs: live_snapshot.confirmed_external_direct_addrs.clone(),
        relay_available: p2p_detection.relay_available,
        probe_stable: p2p_detection.probe_stable,
        deployment_mode: effective_p2p_policy.deployment_mode.as_str().to_string(),
        node_role_claim: effective_p2p_policy.node_role_claim.as_str().to_string(),
        rationale: live_p2p_recommendation.rationale.clone(),
    }
}
