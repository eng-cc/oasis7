use serde::{Deserialize, Serialize};
mod rollback_v2;
pub use rollback_v2::*;
mod authoritative;
pub use authoritative::*;
mod director;
pub use director::*;
mod responses;
pub use responses::*;
mod world_feed;
pub use world_feed::*;
mod negotiation;
pub use negotiation::*;
mod collect_data;
pub use collect_data::*;
mod refine_quote;
pub use refine_quote::*;
mod schedule_recipe_quote;
pub use schedule_recipe_quote::*;
mod live_control_conversion;
mod product_validation_quote;
#[cfg(test)]
mod schedule_recipe_quote_tests;
pub use product_validation_quote::*;
mod power_survival_quote;
pub use power_survival_quote::*;
mod power_sale_quote;
pub use power_sale_quote::*;
mod fragment_refill_preview;
pub use fragment_refill_preview::*;
mod market_quote_decision;
pub use market_quote_decision::*;
mod social_quote;
pub use social_quote::*;
mod transfer_material_quote;
pub use transfer_material_quote::*;
/// Signed, advisory preflight for an existing governance proposal vote.
mod governance_vote_quote;
pub use governance_vote_quote::*;
/// Signed, advisory preflight for the existing DeclareWar action.
mod war_declaration_quote;
pub use war_declaration_quote::*;
pub const VIEWER_PROTOCOL_VERSION: u32 = 2;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAuthProof {
    #[serde(default)]
    pub scheme: PlayerAuthScheme,
    pub player_id: String,
    pub public_key: String,
    pub nonce: u64,
    pub signature: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStrongAuthGrant {
    pub version: u8,
    pub action_id: String,
    pub player_id: String,
    pub player_public_key: String,
    pub agent_id: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub signer_public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewerRequest {
    Hello {
        client: String,
        version: u32,
    },
    HelloV2 {
        client: String,
        version: u32,
        #[serde(default)]
        capabilities: Vec<String>,
    },
    Subscribe {
        streams: Vec<ViewerStream>,
        #[serde(default)]
        event_kinds: Vec<ViewerEventKind>,
    },
    RequestSnapshot,
    RequestWorldFeed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
        limit: usize,
    },
    PlaybackControl {
        mode: PlaybackControl,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<u64>,
    },
    LiveControl {
        mode: LiveControl,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<u64>,
    },
    // Legacy mixed control channel. Prefer PlaybackControl/LiveControl.
    Control {
        mode: ViewerControl,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<u64>,
    },
    PromptControl {
        command: Box<PromptControlCommand>,
    },
    AgentChat {
        request: AgentChatRequest,
    },
    GameplayAction {
        request: GameplayActionRequest,
    },
    CollectData {
        command: CollectDataCommand,
    },
    QuoteRefineCompound {
        request: RefineQuoteRequest,
    },
    QuoteScheduleRecipe {
        request: ScheduleRecipeQuoteRequest,
    },
    QuoteProductValidation {
        request: ProductValidationQuoteRequest,
    },
    QuotePowerSurvival {
        request: PowerSurvivalQuoteRequest,
    },
    QuotePowerSale {
        request: PowerSaleQuoteRequest,
    },
    QuoteDeclareSocialEdge {
        request: DeclareSocialEdgeQuoteRequest,
    },
    QuotePublishSocialFact {
        request: PublishSocialFactQuoteRequest,
    },
    QuoteAdjudicateSocialFact {
        request: AdjudicateSocialFactQuoteRequest,
    },
    QuoteRevokeSocialFact {
        request: RevokeSocialFactQuoteRequest,
    },
    QuoteSocialContact {
        request: SocialContactQuoteRequest,
    },
    QuoteGovernanceVote {
        request: GovernanceVoteQuoteRequest,
    },
    QuoteDeclareWar {
        request: WarDeclarationQuoteRequest,
    },
    PreviewFragmentReplenishment {
        request: FragmentRefillRequest,
    },
    QuoteMarketDecision {
        request: MarketQuoteDecisionRequest,
    },
    QuoteTransferMaterial {
        request: TransferMaterialQuoteRequest,
    },
    AuthoritativeChallenge {
        command: AuthoritativeChallengeCommand,
    },
    AuthoritativeRecovery {
        command: AuthoritativeRecoveryCommand,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PromptControlCommand {
    Preview {
        request: PromptControlApplyRequest,
    },
    Apply {
        request: PromptControlApplyRequest,
    },
    Rollback {
        request: PromptControlRollbackRequest,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlApplyRequest {
    pub agent_id: String,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strong_auth_grant: Option<HostedStrongAuthGrant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_override_field"
    )]
    pub system_prompt_override: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_override_field"
    )]
    pub short_term_goal_override: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_override_field"
    )]
    pub long_term_goal_override: Option<Option<String>>,
}
fn deserialize_override_field<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(Some(value))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlRollbackRequest {
    pub agent_id: String,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strong_auth_grant: Option<HostedStrongAuthGrant>,
    pub to_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatRequest {
    pub agent_id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_tick: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_seq: Option<u64>,
    /// Canonical world identity for the signed Agent Intent authority envelope.
    ///
    /// These fields are optional on the wire so legacy Viewer clients remain
    /// deserializable.  The authoritative V2 endpoint must require the complete
    /// tuple before accepting an intent; a partial tuple is invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reorg_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_scope: Option<String>,
    /// Explicit causal replacement target.  Ordinary retries leave this empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces_intent_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionRequest {
    pub action_id: String,
    pub target_agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_agent_id: Option<String>,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AuthoritativeChallengeCommand {
    Submit {
        request: AuthoritativeChallengeSubmitRequest,
    },
    Resolve {
        request: AuthoritativeChallengeResolveRequest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeChallengeSubmitRequest {
    pub batch_id: String,
    pub watcher_id: String,
    pub recomputed_state_root: String,
    pub recomputed_data_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeChallengeResolveRequest {
    pub challenge_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AuthoritativeRecoveryCommand {
    RegisterSession {
        request: AuthoritativeSessionRegisterRequest,
    },
    Rollback {
        request: AuthoritativeRollbackRequest,
    },
    RollbackV2 {
        request: AuthoritativeRollbackV2Request,
    },
    GetRollbackReceipt {
        request: RollbackReceiptAccessRequest,
    },
    ReevaluateRollbackReadiness {
        authorization_nonce: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audit_evidence: Option<RollbackStrictAuditEvidence>,
    },
    TransitionRollbackCompensation {
        request: RollbackCompensationTransitionRequest,
    },
    ResolveRollbackAttribution {
        request: RollbackAttributionResolutionRequest,
    },
    ReconnectSync {
        request: AuthoritativeReconnectSyncRequest,
    },
    RevokeSession {
        request: AuthoritativeSessionRevokeRequest,
    },
    RotateSession {
        request: AuthoritativeSessionRotateRequest,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeRollbackRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_batch_id: Option<String>,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<RollbackAuthorizationEnvelope>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackAuthorizationEnvelope {
    pub intent: RollbackIntent,
    pub signatures: Vec<RollbackApprovalSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackIntent {
    pub schema_version: u32,
    pub rollback_ticket: String,
    #[serde(default)]
    pub snapshot_hash: String,
    #[serde(default)]
    pub snapshot_journal_len: usize,
    #[serde(default)]
    pub target_journal_len: usize,
    #[serde(default)]
    pub expected_target_state_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_batch_id: Option<String>,
    pub reason: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub nonce: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_checkpoint: Option<RollbackCheckpointRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_target: Option<RollbackReplayTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_reorg_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replay_events: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replay_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackApprovalSignature {
    pub authority_id: String,
    pub role: RollbackAuthorityRole,
    pub signature_scheme: String,
    pub signature_hex: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackAuthorityRole {
    OnCall,
    Governance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeReconnectSyncRequest {
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_pubkey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_log_cursor: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_reorg_epoch: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeSessionRegisterRequest {
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_grant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_agent_id: Option<String>,
    #[serde(default)]
    pub force_rebind: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeSessionRevokeRequest {
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_pubkey: Option<String>,
    pub revoke_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeSessionRotateRequest {
    pub player_id: String,
    pub old_session_pubkey: String,
    pub new_session_pubkey: String,
    pub rotate_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotated_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewerStream {
    Snapshot,
    Events,
    Metrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewerEventKind {
    LocationRegistered,
    AgentRegistered,
    AgentMoved,
    AgentSpoke,
    TargetInspected,
    SimpleInteractionPerformed,
    ResourceTransferred,
    RadiationHarvested,
    ActionRejected,
    Power,
    PromptUpdated,
    RuntimeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewerControlProfile {
    #[default]
    Playback,
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PlaybackControl {
    Pause,
    Play,
    Step { count: usize },
    Seek { tick: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LiveControl {
    Pause,
    Play,
    Step { count: usize },
}

// Legacy mixed control channel. Prefer PlaybackControl/LiveControl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ViewerControl {
    Pause,
    Play,
    Step { count: usize },
    Seek { tick: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewerResponse<Snapshot, Event, DecisionTrace, Metrics, Time> {
    HelloAck {
        server: String,
        version: u32,
        #[serde(default)]
        min_version: u32,
        #[serde(default)]
        max_version: u32,
        #[serde(default)]
        capabilities: Vec<String>,
        world_id: String,
        #[serde(default)]
        control_profile: ViewerControlProfile,
    },
    Snapshot {
        snapshot: Snapshot,
    },
    Event {
        event: Event,
    },
    WorldFeed {
        feed: WorldFeedEnvelope,
    },
    AuthoritativeBatch {
        batch: AuthoritativeBatchFinality,
    },
    AuthoritativeChallengeAck {
        ack: AuthoritativeChallengeAck<Time>,
    },
    AuthoritativeChallengeError {
        error: AuthoritativeChallengeError,
    },
    AuthoritativeRecoveryAck {
        ack: AuthoritativeRecoveryAck<Time>,
    },
    AuthoritativeRecoveryError {
        error: AuthoritativeRecoveryError,
    },
    DecisionTrace {
        trace: DecisionTrace,
    },
    Metrics {
        time: Option<Time>,
        metrics: Metrics,
    },
    ControlCompletionAck {
        ack: ControlCompletionAck<Time>,
    },
    PromptControlAck {
        ack: PromptControlAck<Time>,
    },
    PromptControlError {
        error: PromptControlError,
    },
    AgentChatAck {
        ack: AgentChatAck<Time>,
    },
    AgentChatError {
        error: AgentChatError,
    },
    GameplayActionAck {
        ack: GameplayActionAck<Time>,
    },
    GameplayActionError {
        error: GameplayActionError,
    },
    CollectDataPreflight {
        quote: CollectDataPreflight,
    },
    RefineQuotePreflight {
        quote: RefineQuotePreflight,
    },
    ScheduleRecipeQuotePreflight {
        quote: ScheduleRecipeQuotePreflight,
    },
    ProductValidationQuotePreflight {
        quote: ProductValidationQuotePreflight,
    },
    PowerSurvivalQuotePreflight {
        quote: PowerSurvivalQuotePreflight,
    },
    PowerSaleQuotePreflight {
        quote: PowerSaleQuotePreflight,
    },
    DeclareSocialEdgeQuotePreflight {
        quote: DeclareSocialEdgeQuotePreflight,
    },
    PublishSocialFactQuotePreflight {
        quote: PublishSocialFactQuotePreflight,
    },
    AdjudicateSocialFactQuotePreflight {
        quote: AdjudicateSocialFactQuotePreflight,
    },
    RevokeSocialFactQuotePreflight {
        quote: RevokeSocialFactQuotePreflight,
    },
    SocialContactQuotePreflight {
        quote: SocialContactQuotePreflight,
    },
    GovernanceVoteQuotePreflight {
        quote: GovernanceVoteQuotePreflight,
    },
    WarDeclarationQuotePreflight {
        quote: WarDeclarationQuotePreflight,
    },
    FragmentRefillPreviewPreflight {
        quote: FragmentRefillResponse,
    },
    MarketQuoteDecisionPreflight {
        quote: MarketQuoteDecisionPreflight,
    },
    TransferMaterialQuotePreflight {
        quote: TransferMaterialQuotePreflight,
    },
    Error {
        message: String,
    },
}

impl From<PlaybackControl> for ViewerControl {
    fn from(value: PlaybackControl) -> Self {
        match value {
            PlaybackControl::Pause => Self::Pause,
            PlaybackControl::Play => Self::Play,
            PlaybackControl::Step { count } => Self::Step { count },
            PlaybackControl::Seek { tick } => Self::Seek { tick },
        }
    }
}

impl From<ViewerControl> for PlaybackControl {
    fn from(value: ViewerControl) -> Self {
        match value {
            ViewerControl::Pause => Self::Pause,
            ViewerControl::Play => Self::Play,
            ViewerControl::Step { count } => Self::Step { count },
            ViewerControl::Seek { tick } => Self::Seek { tick },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_playback_control_request_round_trip() {
        let request = ViewerRequest::PlaybackControl {
            mode: PlaybackControl::Seek { tick: 24 },
            request_id: Some(11),
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_live_control_request_round_trip() {
        let request = ViewerRequest::LiveControl {
            mode: LiveControl::Step { count: 3 },
            request_id: Some(13),
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_control_request_defaults_request_id_to_none_for_compat_payload() {
        let request = ViewerRequest::Control {
            mode: ViewerControl::Step { count: 2 },
            request_id: None,
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        assert!(!json.contains("request_id"));
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        let ViewerRequest::Control { request_id, .. } = parsed else {
            panic!("expected control request");
        };
        assert_eq!(request_id, None);
    }

    #[test]
    fn viewer_subscribe_round_trip_with_filters() {
        let request = ViewerRequest::Subscribe {
            streams: vec![ViewerStream::Events],
            event_kinds: vec![ViewerEventKind::AgentMoved, ViewerEventKind::Power],
        };
        let json = serde_json::to_string(&request).expect("serialize subscribe");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize subscribe");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_prompt_control_request_round_trip() {
        let request = ViewerRequest::PromptControl {
            command: Box::new(PromptControlCommand::Apply {
                request: PromptControlApplyRequest {
                    agent_id: "agent-0".to_string(),
                    player_id: "player-1".to_string(),
                    public_key: Some("pk-1".to_string()),
                    auth: Some(PlayerAuthProof {
                        scheme: PlayerAuthScheme::Ed25519,
                        player_id: "player-1".to_string(),
                        public_key: "pk-1".to_string(),
                        nonce: 7,
                        signature: "awviewauth:v1:deadbeef".to_string(),
                    }),
                    strong_auth_grant: None,
                    expected_version: Some(3),
                    updated_by: Some("tester".to_string()),
                    system_prompt_override: Some(Some("system".to_string())),
                    short_term_goal_override: Some(None),
                    long_term_goal_override: None,
                },
            }),
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("serialized request should be json");
        assert_eq!(value["type"], "prompt_control");
        assert_eq!(value["command"]["mode"], "apply");
        assert_eq!(value["command"]["request"]["agent_id"], "agent-0");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_agent_chat_request_round_trip() {
        let request = ViewerRequest::AgentChat {
            request: AgentChatRequest {
                agent_id: "agent-0".to_string(),
                message: "go to loc-2".to_string(),
                player_id: Some("player-1".to_string()),
                public_key: Some("pk-1".to_string()),
                auth: Some(PlayerAuthProof {
                    scheme: PlayerAuthScheme::Ed25519,
                    player_id: "player-1".to_string(),
                    public_key: "pk-1".to_string(),
                    nonce: 9,
                    signature: "awviewauth:v1:deadbeef".to_string(),
                }),
                intent_tick: Some(42),
                intent_seq: Some(9),
                world_id: Some("world-1".to_string()),
                reorg_epoch: Some(2),
                authority_scope: Some("player_agent_chat".to_string()),
                replaces_intent_id: Some("agent-intent-v2:prior".to_string()),
            },
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_gameplay_action_request_round_trip() {
        let request = ViewerRequest::GameplayAction {
            request: GameplayActionRequest {
                action_id: "build_factory_smelter_mk1".to_string(),
                target_agent_id: "agent-0".to_string(),
                actor_agent_id: None,
                player_id: "player-1".to_string(),
                public_key: Some("pk-1".to_string()),
                auth: Some(PlayerAuthProof {
                    scheme: PlayerAuthScheme::Ed25519,
                    player_id: "player-1".to_string(),
                    public_key: "pk-1".to_string(),
                    nonce: 11,
                    signature: "awviewauth:v1:deadbeef".to_string(),
                }),
            },
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_authoritative_challenge_submit_request_round_trip() {
        let request = ViewerRequest::AuthoritativeChallenge {
            command: AuthoritativeChallengeCommand::Submit {
                request: AuthoritativeChallengeSubmitRequest {
                    batch_id: "batch-1".to_string(),
                    watcher_id: "watcher-1".to_string(),
                    recomputed_state_root: "a".repeat(64),
                    recomputed_data_root: "b".repeat(64),
                    challenge_id: Some("challenge-1".to_string()),
                },
            },
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_authoritative_recovery_rotate_session_request_round_trip() {
        let request = ViewerRequest::AuthoritativeRecovery {
            command: AuthoritativeRecoveryCommand::RotateSession {
                request: AuthoritativeSessionRotateRequest {
                    player_id: "player-1".to_string(),
                    old_session_pubkey: "old-key".to_string(),
                    new_session_pubkey: "new-key".to_string(),
                    rotate_reason: "security_rotation".to_string(),
                    rotated_by: Some("ops".to_string()),
                },
            },
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn viewer_prompt_control_request_legacy_without_public_key_is_accepted() {
        let json = r#"{
            "type":"prompt_control",
            "command":{
                "mode":"apply",
                "request":{
                    "agent_id":"agent-0",
                    "player_id":"player-1"
                }
            }
        }"#;
        let parsed: ViewerRequest = serde_json::from_str(json).expect("deserialize legacy request");
        let ViewerRequest::PromptControl { command } = parsed else {
            panic!("expected prompt_control request");
        };
        let PromptControlCommand::Apply { request } = *command else {
            panic!("expected apply command");
        };
        assert_eq!(request.public_key, None);
        assert_eq!(request.auth, None);
    }

    #[test]
    fn viewer_agent_chat_request_legacy_without_auth_is_accepted() {
        let json = r#"{
            "type":"agent_chat",
            "request":{
                "agent_id":"agent-0",
                "message":"hello",
                "player_id":"player-1",
                "public_key":"pk-1"
            }
        }"#;
        let parsed: ViewerRequest = serde_json::from_str(json).expect("deserialize legacy request");
        let ViewerRequest::AgentChat { request } = parsed else {
            panic!("expected agent_chat request");
        };
        assert_eq!(request.auth, None);
        assert_eq!(request.intent_tick, None);
        assert_eq!(request.intent_seq, None);
    }

    #[test]
    fn viewer_response_round_trip_prompt_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::PromptControlAck {
            ack: PromptControlAck {
                agent_id: "agent-0".to_string(),
                operation: PromptControlOperation::Rollback,
                preview: false,
                version: 7,
                updated_at_tick: 42,
                applied_fields: vec![
                    "system_prompt_override".to_string(),
                    "short_term_goal_override".to_string(),
                ],
                digest: "abc".to_string(),
                rolled_back_to_version: Some(5),
            },
        };
        let json = serde_json::to_string(&response).expect("serialize response");
        let parsed: ViewerResponse<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        > = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(parsed, response);
    }

    #[test]
    fn viewer_response_round_trip_control_completion_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::ControlCompletionAck {
            ack: ControlCompletionAck {
                request_id: 42,
                status: ControlCompletionStatus::TimeoutNoProgress,
                delta_logical_time: 0,
                delta_event_seq: 0,
                error_code: None,
                error_message: None,
            },
        };
        let json = serde_json::to_string(&response).expect("serialize response");
        let parsed: ViewerResponse<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        > = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(parsed, response);
    }

    #[test]
    fn viewer_response_round_trip_agent_chat_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::AgentChatAck {
            ack: AgentChatAck {
                agent_id: "agent-0".to_string(),
                accepted_at_tick: 42,
                message_len: 11,
                player_id: Some("player-1".to_string()),
                intent_tick: Some(42),
                intent_seq: Some(17),
                idempotent_replay: true,
                intent_id: None,
                accepted_event_seq: None,
                status: None,
                receipt_ref: None,
                replaced_by: None,
            },
        };
        let json = serde_json::to_string(&response).expect("serialize response");
        let parsed: ViewerResponse<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        > = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(parsed, response);
    }

    #[test]
    fn viewer_response_round_trip_gameplay_action_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::GameplayActionAck {
            ack: GameplayActionAck {
                action_id: "build_factory_smelter_mk1".to_string(),
                target_agent_id: "agent-0".to_string(),
                player_id: "player-1".to_string(),
                runtime_action_id: 41,
                accepted_at_tick: 42,
                message: Some(
                    "advance 1-2 steps to apply the queued industrial action".to_string(),
                ),
            },
        };
        let json = serde_json::to_string(&response).expect("serialize response");
        let parsed: ViewerResponse<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        > = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(parsed, response);
    }

    #[test]
    fn world_feed_v1_round_trip_preserves_cursor_and_gap_contract() {
        let request_json = serde_json::json!({
            "type": "request_world_feed",
            "cursor": "opaque-cursor",
            "limit": 25
        });
        let request: ViewerRequest =
            serde_json::from_value(request_json.clone()).expect("decode world feed request");
        assert_eq!(
            serde_json::to_value(request).expect("encode request"),
            request_json
        );

        let legacy_response_json = serde_json::json!({
            "type": "world_feed",
            "feed": {
                "schema_version": "world_feed/v1",
                "world_id": "world-1",
                "reorg_epoch": 3,
                "cursor": "next-cursor",
                "events": [],
                "status": "gap",
                "gap_reason": "reorg_epoch_changed",
                "snapshot_reload_required": true
            }
        });
        let response: ViewerResponse<(), (), (), (), u64> =
            serde_json::from_value(legacy_response_json).expect("decode world feed response");
        let encoded = serde_json::to_value(response).expect("encode response");
        assert_eq!(encoded["feed"]["reorg_epoch"], serde_json::json!("3"));
    }

    #[test]
    fn world_feed_v1_event_keeps_explicit_receipt_reference_nullable() {
        let legacy_response_json = serde_json::json!({
            "type": "world_feed",
            "feed": {
                "schema_version": "world_feed/v1",
                "world_id": "world-1",
                "reorg_epoch": 0,
                "cursor": "cursor-7",
                "events": [{
                    "event_seq": 7,
                    "kind": "domain",
                    "summary": "Domain event",
                    "detail": "{}",
                    "receipt_ref": null
                }],
                "status": "ready",
                "snapshot_reload_required": false
            }
        });
        let response: ViewerResponse<(), (), (), (), u64> =
            serde_json::from_value(legacy_response_json).expect("decode world feed response");
        let encoded = serde_json::to_value(response).expect("encode response");
        assert_eq!(encoded["feed"]["reorg_epoch"], serde_json::json!("0"));
        assert_eq!(
            encoded["feed"]["events"][0]["event_seq"],
            serde_json::json!("7")
        );
        assert!(encoded["feed"]["events"][0]["receipt_ref"].is_null());
    }

    #[test]
    fn world_feed_v1_u64_identifiers_serialize_as_exact_decimal_strings() {
        let response = ViewerResponse::<(), (), (), (), u64>::WorldFeed {
            feed: WorldFeedEnvelope {
                schema_version: WORLD_FEED_SCHEMA_VERSION.to_string(),
                world_id: "world-max".to_string(),
                reorg_epoch: u64::MAX,
                cursor: "cursor-max".to_string(),
                events: vec![WorldFeedEvent {
                    event_seq: u64::MAX,
                    kind: "domain".to_string(),
                    summary: "Max event sequence".to_string(),
                    detail: "{}".to_string(),
                    receipt_ref: None,
                }],
                status: WorldFeedStatus::Ready,
                gap_reason: None,
                unavailable_reason: None,
                snapshot_reload_required: false,
            },
        };

        let encoded = serde_json::to_value(&response).expect("encode max u64 feed");
        assert_eq!(
            encoded["feed"]["reorg_epoch"],
            serde_json::json!(u64::MAX.to_string())
        );
        assert_eq!(
            encoded["feed"]["events"][0]["event_seq"],
            serde_json::json!(u64::MAX.to_string())
        );

        let parsed: ViewerResponse<(), (), (), (), u64> =
            serde_json::from_value(encoded).expect("decode max u64 feed");
        assert_eq!(parsed, response);
    }

    #[test]
    fn director_capability_grant_round_trip_preserves_signed_scope() {
        let grant_json = serde_json::json!({
            "version": 1,
            "action": "director_open",
            "audience": "viewer_director",
            "scope": "diagnostics_read",
            "player_id": "player-1",
            "player_public_key": "11".repeat(32),
            "server": "viewer-live-1",
            "session_epoch": 4,
            "nonce": "director-nonce-1",
            "issued_at_unix_ms": 1000,
            "expires_at_unix_ms": 2000,
            "signer_public_key": "22".repeat(32),
            "signature": "awdirectorgrant:v1:33".to_string() + &"33".repeat(63),
        });
        let grant: DirectorCapabilityGrant =
            serde_json::from_value(grant_json.clone()).expect("decode director grant");
        assert_eq!(
            serde_json::to_value(grant).expect("encode director grant"),
            grant_json
        );
    }
}

#[cfg(test)]
mod authoritative_batch_roundtrip_tests;
#[cfg(test)]
mod authoritative_challenge_roundtrip_tests;
#[cfg(test)]
#[path = "viewer/collect_data_tests.rs"]
mod collect_data_tests;
#[cfg(test)]
#[path = "viewer/control_roundtrip_tests.rs"]
mod control_roundtrip_tests;
#[cfg(test)]
mod market_quote_decision_tests;
#[cfg(test)]
#[path = "viewer/protocol_v2_tests.rs"]
mod protocol_v2_tests;
