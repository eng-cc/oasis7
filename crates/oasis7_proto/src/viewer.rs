use serde::{Deserialize, Serialize};

pub const VIEWER_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlayerAuthScheme {
    #[default]
    Ed25519,
}

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
    Subscribe {
        streams: Vec<ViewerStream>,
        #[serde(default)]
        event_kinds: Vec<ViewerEventKind>,
    },
    RequestSnapshot,
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
        command: PromptControlCommand,
    },
    AgentChat {
        request: AgentChatRequest,
    },
    GameplayAction {
        request: GameplayActionRequest,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionRequest {
    pub action_id: String,
    pub target_agent_id: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoritativeFinalityState {
    Pending,
    Confirmed,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeBatchFinality {
    pub batch_id: String,
    pub tx_hash: String,
    pub commit_tick: u64,
    pub confirm_height: u64,
    pub final_height: u64,
    pub state_root: String,
    pub data_root: String,
    pub finality_state: AuthoritativeFinalityState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_seq_start: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_seq_end: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub settlement_ready: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ranking_ready: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub challenge_open: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub slashed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_challenge_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoritativeChallengeStatus {
    Challenged,
    ResolvedNoFraud,
    ResolvedFraudSlashed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeChallengeAck<Time> {
    pub challenge_id: String,
    pub batch_id: String,
    pub watcher_id: String,
    pub status: AuthoritativeChallengeStatus,
    pub submitted_at_tick: Time,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at_tick: Option<Time>,
    #[serde(skip_serializing_if = "is_false")]
    pub slash_applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slash_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeChallengeError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoritativeRecoveryStatus {
    SessionRegistered,
    RolledBack,
    CatchUpReady,
    SessionRevoked,
    SessionRotated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeRecoveryAck<Time> {
    pub status: AuthoritativeRecoveryStatus,
    pub reorg_epoch: u64,
    pub snapshot_height: u64,
    pub snapshot_hash: String,
    pub log_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_pubkey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by_pubkey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    pub acknowledged_at_tick: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeRecoveryError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_pubkey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
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
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCompletionStatus {
    Advanced,
    TimeoutNoProgress,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCompletionAck<Time> {
    pub request_id: u64,
    pub status: ControlCompletionStatus,
    pub delta_logical_time: Time,
    pub delta_event_seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptControlOperation {
    Apply,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlAck<Time> {
    pub agent_id: String,
    pub operation: PromptControlOperation,
    pub preview: bool,
    pub version: u64,
    pub updated_at_tick: Time,
    pub applied_fields: Vec<String>,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolled_back_to_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatAck<Time> {
    pub agent_id: String,
    pub accepted_at_tick: Time,
    pub message_len: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_tick: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub idempotent_replay: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionAck<Time> {
    pub action_id: String,
    pub target_agent_id: String,
    pub player_id: String,
    pub runtime_action_id: u64,
    pub accepted_at_tick: Time,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
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

impl From<LiveControl> for ViewerControl {
    fn from(value: LiveControl) -> Self {
        match value {
            LiveControl::Pause => Self::Pause,
            LiveControl::Play => Self::Play,
            LiveControl::Step { count } => Self::Step { count },
        }
    }
}

impl TryFrom<ViewerControl> for LiveControl {
    type Error = &'static str;

    fn try_from(value: ViewerControl) -> Result<Self, Self::Error> {
        match value {
            ViewerControl::Pause => Ok(Self::Pause),
            ViewerControl::Play => Ok(Self::Play),
            ViewerControl::Step { count } => Ok(Self::Step { count }),
            ViewerControl::Seek { .. } => Err("seek is not valid in live control mode"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_request_round_trip() {
        let request = ViewerRequest::Control {
            mode: ViewerControl::Step { count: 2 },
            request_id: Some(7),
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

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
            command: PromptControlCommand::Apply {
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
            },
        };
        let json = serde_json::to_string(&request).expect("serialize request");
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
        let PromptControlCommand::Apply { request } = command else {
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
    fn viewer_response_round_trip_authoritative_batch() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::AuthoritativeBatch {
            batch: AuthoritativeBatchFinality {
                batch_id: "batch-7".to_string(),
                tx_hash: "tx-hash-7".to_string(),
                commit_tick: 70,
                confirm_height: 72,
                final_height: 75,
                state_root: "state-root-7".to_string(),
                data_root: "data-root-7".to_string(),
                finality_state: AuthoritativeFinalityState::Confirmed,
                event_seq_start: Some(101),
                event_seq_end: Some(110),
                settlement_ready: false,
                ranking_ready: false,
                challenge_open: true,
                slashed: false,
                active_challenge_id: Some("challenge-9".to_string()),
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
    fn viewer_response_round_trip_authoritative_challenge_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::AuthoritativeChallengeAck {
            ack: AuthoritativeChallengeAck {
                challenge_id: "challenge-1".to_string(),
                batch_id: "batch-1".to_string(),
                watcher_id: "watcher-1".to_string(),
                status: AuthoritativeChallengeStatus::ResolvedFraudSlashed,
                submitted_at_tick: 40,
                resolved_at_tick: Some(42),
                slash_applied: true,
                slash_reason: Some("state_root_mismatch".to_string()),
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
    fn viewer_response_round_trip_authoritative_recovery_ack() {
        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::AuthoritativeRecoveryAck {
            ack: AuthoritativeRecoveryAck {
                status: AuthoritativeRecoveryStatus::SessionRotated,
                reorg_epoch: 2,
                snapshot_height: 88,
                snapshot_hash: "snapshot-hash-1".to_string(),
                log_cursor: 123,
                stable_batch_id: Some("batch-9".to_string()),
                player_id: Some("player-1".to_string()),
                agent_id: Some("agent-7".to_string()),
                session_pubkey: Some("old-key".to_string()),
                replaced_by_pubkey: Some("new-key".to_string()),
                session_epoch: Some(5),
                message: Some("session rotated".to_string()),
                revoke_reason: Some("compromised".to_string()),
                revoked_by: Some("ops".to_string()),
                acknowledged_at_tick: 89,
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
    fn viewer_hello_ack_defaults_to_playback_profile_for_default_payload() {
        let json = r#"{
            "type":"hello_ack",
            "server":"oasis7",
            "version":1,
            "world_id":"w1"
        }"#;
        let parsed: ViewerResponse<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        > = serde_json::from_str(json).expect("deserialize hello ack");
        let ViewerResponse::HelloAck {
            control_profile, ..
        } = parsed
        else {
            panic!("expected hello ack");
        };
        assert_eq!(control_profile, ViewerControlProfile::Playback);
    }
}
