use super::*;

#[test]
fn network_policy_blocks_observer_from_consensus_publish_lane() {
    let policy = NodeNetworkPolicy {
        deployment_mode: PeerDeploymentMode::Private,
        node_role_claim: PeerNodeRole::ObserverLight,
    };
    assert!(
        !policy.allows_lane_operation(NetworkLane::ConsensusGossip, NetworkLaneOperation::Publish)
    );
    assert!(policy.allows_lane_operation(
        NetworkLane::ConsensusGossip,
        NetworkLaneOperation::Subscribe
    ));
}

#[test]
fn network_policy_limits_relay_to_control_lane() {
    let policy = NodeNetworkPolicy {
        deployment_mode: PeerDeploymentMode::Public,
        node_role_claim: PeerNodeRole::Relay,
    };
    assert!(policy.allows_lane_operation(NetworkLane::Control, NetworkLaneOperation::Serve));
    assert!(!policy.allows_lane_operation(NetworkLane::Sync, NetworkLaneOperation::Request));
    assert!(!policy.allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Subscribe));
}

#[test]
fn network_policy_allows_observer_requests_but_blocks_data_serving() {
    let policy = NodeNetworkPolicy {
        deployment_mode: PeerDeploymentMode::Private,
        node_role_claim: PeerNodeRole::ObserverLight,
    };
    assert!(policy.allows_lane_operation(NetworkLane::Sync, NetworkLaneOperation::Request));
    assert!(policy.allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Request));
    assert!(!policy.allows_lane_operation(NetworkLane::Sync, NetworkLaneOperation::Serve));
    assert!(!policy.allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Serve));
}

#[test]
fn auto_join_requires_confirmation_before_public_entry_upgrade() {
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Storage,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection {
            observed_reachability: Some(PeerReachabilityClass::Public),
            hole_punch_viability: NodeHolePunchViability::Viable,
            relay_available: true,
            probe_stable: true,
            ..NodeReachabilityAutoDetection::default()
        },
        false,
    )
    .expect("recommendation");

    assert_eq!(
        recommendation.recommended_user_mode,
        NodeUserMode::PublicEntry
    );
    assert_eq!(
        recommendation.effective_user_mode,
        NodeUserMode::PrivateSafe
    );
    assert!(recommendation.requires_explicit_public_entry_confirmation);
    assert_eq!(
        recommendation.effective_policy.deployment_mode,
        PeerDeploymentMode::Private
    );
    assert_eq!(
        recommendation.effective_policy.node_role_claim,
        PeerNodeRole::FullStorage
    );
}

#[test]
fn auto_join_can_promote_to_public_entry_after_consent() {
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Observer,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection {
            observed_reachability: Some(PeerReachabilityClass::Hybrid),
            hole_punch_viability: NodeHolePunchViability::Viable,
            relay_available: true,
            probe_stable: true,
            ..NodeReachabilityAutoDetection::default()
        },
        true,
    )
    .expect("recommendation");

    assert_eq!(
        recommendation.recommended_user_mode,
        NodeUserMode::PublicEntry
    );
    assert_eq!(
        recommendation.effective_user_mode,
        NodeUserMode::PublicEntry
    );
    assert!(!recommendation.requires_explicit_public_entry_confirmation);
    assert_eq!(
        recommendation.effective_policy.deployment_mode,
        PeerDeploymentMode::Public
    );
    assert_eq!(
        recommendation.effective_policy.node_role_claim,
        PeerNodeRole::ObserverLight
    );
}

#[test]
fn unstable_probe_falls_back_to_private_safe() {
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Storage,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection {
            observed_reachability: Some(PeerReachabilityClass::Public),
            hole_punch_viability: NodeHolePunchViability::Viable,
            relay_available: true,
            probe_stable: false,
            ..NodeReachabilityAutoDetection::default()
        },
        true,
    )
    .expect("recommendation");

    assert_eq!(
        recommendation.recommended_user_mode,
        NodeUserMode::PrivateSafe
    );
    assert_eq!(
        recommendation.effective_user_mode,
        NodeUserMode::PrivateSafe
    );
    assert_eq!(
        recommendation.effective_policy.deployment_mode,
        PeerDeploymentMode::Private
    );
}

#[test]
fn sequencer_auto_join_never_auto_promotes_to_public_entry() {
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Sequencer,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection {
            observed_reachability: Some(PeerReachabilityClass::Public),
            hole_punch_viability: NodeHolePunchViability::Viable,
            relay_available: true,
            probe_stable: true,
            ..NodeReachabilityAutoDetection::default()
        },
        true,
    )
    .expect("recommendation");

    assert_eq!(
        recommendation.recommended_user_mode,
        NodeUserMode::PrivateSafe
    );
    assert_eq!(
        recommendation.effective_user_mode,
        NodeUserMode::PrivateSafe
    );
    assert_eq!(
        recommendation.effective_policy.node_role_claim,
        PeerNodeRole::ValidatorCore
    );
}

#[test]
fn autonat_public_without_reachability_hint_still_recommends_public_entry() {
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Observer,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection {
            autonat_status: NodeAutoNatStatus::Public,
            public_port_reachability: NodePublicPortReachability::Reachable,
            probe_stable: true,
            ..NodeReachabilityAutoDetection::default()
        },
        true,
    )
    .expect("recommendation");

    assert_eq!(
        recommendation.recommended_user_mode,
        NodeUserMode::PublicEntry
    );
    assert_eq!(
        recommendation.effective_user_mode,
        NodeUserMode::PublicEntry
    );
}
