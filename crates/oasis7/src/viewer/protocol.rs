use oasis7_proto::viewer as proto;

use crate::simulator::{
    AgentDecisionTrace, ChunkCoord, FragmentRefillPreview, RunnerMetrics, WorldEvent,
    WorldEventKind, WorldSnapshot, WorldTime,
};

pub use proto::{
    AgentChatError, AgentChatRequest, AuthoritativeBatchFinality, AuthoritativeChallengeAck,
    AuthoritativeChallengeCommand, AuthoritativeChallengeError,
    AuthoritativeChallengeResolveRequest, AuthoritativeChallengeStatus,
    AuthoritativeChallengeSubmitRequest, AuthoritativeFinalityState,
    AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryAck, AuthoritativeRecoveryCommand,
    AuthoritativeRecoveryError, AuthoritativeRecoveryStatus, AuthoritativeRollbackReceipt,
    AuthoritativeRollbackRequest, AuthoritativeRollbackV2Request,
    AuthoritativeSessionRegisterRequest, AuthoritativeSessionRevokeRequest,
    AuthoritativeSessionRotateRequest, CollectDataCommand, CollectDataPreflight,
    CollectDataRequest, ControlCompletionStatus, FragmentRefillElementRemaining,
    FragmentRefillPreviewChunk, FragmentRefillPreviewProtocolRequest,
    FragmentRefillPreviewResponse, GOVERNED_ROLLBACK_REPLAY_CAPABILITY, GameplayActionError,
    GameplayActionRequest, HostedStrongAuthGrant, LiveControl, NegotiatedViewerProtocol,
    MarketQuoteDecisionPreflight, MarketQuoteDecisionRequest, MarketQuoteMaterialContribution,
    MarketQuoteMaterialRequest, PlaybackControl, PlayerActionDisposition,
    PlayerAuthProof, PlayerAuthScheme, PlayerCompensationState, PlayerCompensationStatus,
    PlayerRollbackDisposition,
    PowerSurvivalQuotePreflight, PowerSurvivalQuoteRequest, ProductValidationQuotePreflight,
    ProductValidationQuoteRequest, PromptControlApplyRequest, PromptControlCommand,
    PromptControlError, PromptControlOperation, PromptControlRollbackRequest, RefineQuotePreflight,
    RefineQuoteRequest, RollbackApprovalSignature, RollbackAttributionResolution,
    RollbackAttributionResolutionRequest, RollbackAuthorityRole, RollbackAuthorizationEnvelope,
    RollbackCheckpointRef, RollbackCompensationTransitionRequest, RollbackIntent,
    RollbackOperatorAuthorization, RollbackReceiptAccessRequest, RollbackReplayTarget,
    RollbackSourceEventRef, RollbackStrictAuditEvidence, VIEWER_PROTOCOL_VERSION, ViewerControl,
    ViewerControlProfile, ViewerEventKind, ViewerRequest, ViewerStream,
};

pub type ViewerResponse =
    proto::ViewerResponse<WorldSnapshot, WorldEvent, AgentDecisionTrace, RunnerMetrics, WorldTime>;
pub type PromptControlAck = proto::PromptControlAck<WorldTime>;
pub type AgentChatAck = proto::AgentChatAck<WorldTime>;
pub type GameplayActionAck = proto::GameplayActionAck<WorldTime>;
pub type ControlCompletionAck = proto::ControlCompletionAck<WorldTime>;

/// A signed, read-only request for the current chunk's fragment-replenishment forecast.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FragmentRefillPreviewRequest {
    pub chunk: ChunkCoord,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The authoritative, non-mutating kernel forecast returned for a fragment-refill preflight.
pub type FragmentRefillPreviewPreflight = FragmentRefillPreview;

pub fn viewer_event_kind_matches(filter: &ViewerEventKind, kind: &WorldEventKind) -> bool {
    match (filter, kind) {
        (ViewerEventKind::LocationRegistered, WorldEventKind::LocationRegistered { .. }) => true,
        (ViewerEventKind::AgentRegistered, WorldEventKind::AgentRegistered { .. }) => true,
        (ViewerEventKind::AgentMoved, WorldEventKind::AgentMoved { .. }) => true,
        (ViewerEventKind::AgentSpoke, WorldEventKind::AgentSpoke { .. }) => true,
        (ViewerEventKind::TargetInspected, WorldEventKind::TargetInspected { .. }) => true,
        (
            ViewerEventKind::SimpleInteractionPerformed,
            WorldEventKind::SimpleInteractionPerformed { .. },
        ) => true,
        (ViewerEventKind::ResourceTransferred, WorldEventKind::ResourceTransferred { .. }) => true,
        (ViewerEventKind::RadiationHarvested, WorldEventKind::RadiationHarvested { .. }) => true,
        (ViewerEventKind::ActionRejected, WorldEventKind::ActionRejected { .. }) => true,
        (ViewerEventKind::Power, WorldEventKind::Power(_)) => true,
        (ViewerEventKind::PromptUpdated, WorldEventKind::AgentPromptUpdated { .. }) => true,
        (ViewerEventKind::RuntimeEvent, WorldEventKind::RuntimeEvent { .. }) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_request_round_trip() {
        let request = ViewerRequest::Control {
            mode: ViewerControl::Step { count: 2 },
            request_id: Some(1),
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_event_kind_matches_world_event_kind() {
        assert!(viewer_event_kind_matches(
            &ViewerEventKind::AgentMoved,
            &WorldEventKind::AgentMoved {
                agent_id: "a1".to_string(),
                from: "loc-a".to_string(),
                to: "loc-b".to_string(),
                distance_cm: 100,
                electricity_cost: 1,
            },
        ));
        assert!(!viewer_event_kind_matches(
            &ViewerEventKind::PromptUpdated,
            &WorldEventKind::AgentMoved {
                agent_id: "a1".to_string(),
                from: "loc-a".to_string(),
                to: "loc-b".to_string(),
                distance_cm: 100,
                electricity_cost: 1,
            },
        ));
        assert!(viewer_event_kind_matches(
            &ViewerEventKind::AgentSpoke,
            &WorldEventKind::AgentSpoke {
                agent_id: "a1".to_string(),
                location_id: "loc-a".to_string(),
                message: "hi".to_string(),
                target_agent_id: None,
            },
        ));
        assert!(viewer_event_kind_matches(
            &ViewerEventKind::TargetInspected,
            &WorldEventKind::TargetInspected {
                agent_id: "a1".to_string(),
                target_kind: "location".to_string(),
                target_id: "loc-a".to_string(),
            },
        ));
        assert!(viewer_event_kind_matches(
            &ViewerEventKind::SimpleInteractionPerformed,
            &WorldEventKind::SimpleInteractionPerformed {
                agent_id: "a1".to_string(),
                target_kind: "location".to_string(),
                target_id: "loc-a".to_string(),
                interaction: "press_console".to_string(),
            },
        ));
        assert!(viewer_event_kind_matches(
            &ViewerEventKind::RuntimeEvent,
            &WorldEventKind::RuntimeEvent {
                kind: "snapshot_created".to_string(),
                domain_kind: None,
            },
        ));
    }
}
