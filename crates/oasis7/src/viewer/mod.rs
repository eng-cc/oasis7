mod auth;
#[cfg(not(target_arch = "wasm32"))]
mod demo;
mod gameplay_actions;
#[cfg(not(target_arch = "wasm32"))]
mod live;
mod protocol;
#[cfg(not(target_arch = "wasm32"))]
mod runtime_live;
#[cfg(not(target_arch = "wasm32"))]
mod server;
#[cfg(not(target_arch = "wasm32"))]
mod web_bridge;

pub use auth::{
    sign_agent_chat_auth_proof, sign_gameplay_action_auth_proof,
    sign_hosted_prompt_control_strong_auth_grant, sign_prompt_control_apply_auth_proof,
    sign_prompt_control_rollback_auth_proof, sign_session_register_auth_proof,
    verify_agent_chat_auth_proof, verify_gameplay_action_auth_proof,
    verify_hosted_prompt_control_apply_strong_auth_grant,
    verify_hosted_prompt_control_rollback_strong_auth_grant,
    verify_prompt_control_apply_auth_proof, verify_prompt_control_rollback_auth_proof,
    verify_session_register_auth_proof, PromptControlAuthIntent, VerifiedPlayerAuth,
    VIEWER_HOSTED_STRONG_AUTH_GRANT_SIGNATURE_V1_PREFIX, VIEWER_PLAYER_AUTH_SIGNATURE_V1_PREFIX,
};
#[cfg(not(target_arch = "wasm32"))]
pub use demo::{generate_viewer_demo, ViewerDemoError, ViewerDemoSummary};
#[cfg(not(target_arch = "wasm32"))]
pub use gameplay_actions::build_runtime_action_from_gameplay_request;
pub use gameplay_actions::{
    gameplay_action_requires_actor_agent, ACTION_BUILD_ASSEMBLER_MK1, ACTION_BUILD_SMELTER_MK1,
    ACTION_CLAIM_AGENT, ACTION_RELEASE_AGENT_CLAIM, ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP,
    ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE, ACTION_SCHEDULE_ASSEMBLER_GEAR,
    ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE, ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK,
    ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1, ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK,
    ACTION_SCHEDULE_SMELTER_ALLOY_PLATE, ACTION_SCHEDULE_SMELTER_COPPER_WIRE,
    ACTION_SCHEDULE_SMELTER_IRON_INGOT, ACTION_SCHEDULE_SMELTER_POLYMER_RESIN,
    FACTORY_ASSEMBLER_MK1, FACTORY_SMELTER_MK1,
};
#[cfg(not(target_arch = "wasm32"))]
pub use live::{
    ViewerLiveDecisionMode, ViewerLiveServer, ViewerLiveServerConfig, ViewerLiveServerError,
};
pub use protocol::{
    AgentChatAck, AgentChatError, AgentChatRequest, AuthoritativeBatchFinality,
    AuthoritativeChallengeAck, AuthoritativeChallengeCommand, AuthoritativeChallengeError,
    AuthoritativeChallengeResolveRequest, AuthoritativeChallengeStatus,
    AuthoritativeChallengeSubmitRequest, AuthoritativeFinalityState,
    AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryAck, AuthoritativeRecoveryCommand,
    AuthoritativeRecoveryError, AuthoritativeRecoveryStatus, AuthoritativeRollbackRequest,
    AuthoritativeSessionRegisterRequest, AuthoritativeSessionRevokeRequest,
    AuthoritativeSessionRotateRequest, ControlCompletionAck, ControlCompletionStatus,
    GameplayActionAck, GameplayActionError, GameplayActionRequest, HostedStrongAuthGrant,
    LiveControl, PlaybackControl, PlayerAuthProof, PlayerAuthScheme, PromptControlAck,
    PromptControlApplyRequest, PromptControlCommand, PromptControlError, PromptControlOperation,
    PromptControlRollbackRequest, ViewerControl, ViewerControlProfile, ViewerRequest,
    ViewerResponse, ViewerStream, VIEWER_PROTOCOL_VERSION,
};
#[cfg(not(target_arch = "wasm32"))]
pub use runtime_live::{
    ViewerRuntimeLiveServer, ViewerRuntimeLiveServerConfig, ViewerRuntimeLiveServerError,
};
#[cfg(not(target_arch = "wasm32"))]
pub use server::{ViewerServer, ViewerServerConfig, ViewerServerError};
#[cfg(not(target_arch = "wasm32"))]
pub use web_bridge::{ViewerWebBridge, ViewerWebBridgeConfig, ViewerWebBridgeError};
