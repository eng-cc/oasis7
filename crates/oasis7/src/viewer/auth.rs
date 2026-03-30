use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;

use super::protocol::{
    AgentChatRequest, AuthoritativeSessionRegisterRequest, GameplayActionRequest,
    HostedStrongAuthGrant, PlayerAuthProof, PlayerAuthScheme, PromptControlApplyRequest,
    PromptControlRollbackRequest,
};

const VIEWER_PLAYER_AUTH_PAYLOAD_VERSION: u8 = 1;
pub const VIEWER_PLAYER_AUTH_SIGNATURE_V1_PREFIX: &str = "awviewauth:v1:";
const VIEWER_HOSTED_STRONG_AUTH_GRANT_PAYLOAD_VERSION: u8 = 1;
pub const VIEWER_HOSTED_STRONG_AUTH_GRANT_SIGNATURE_V1_PREFIX: &str = "awhostedgrant:v1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptControlAuthIntent {
    Preview,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPlayerAuth {
    pub player_id: String,
    pub public_key: String,
    pub nonce: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PromptFieldMode {
    Unchanged,
    Clear,
    Set,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PromptFieldPatch<'a> {
    mode: PromptFieldMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PromptControlApplySigningPayload<'a> {
    operation: &'static str,
    agent_id: &'a str,
    player_id: &'a str,
    public_key: &'a str,
    nonce: u64,
    expected_version: Option<u64>,
    updated_by: Option<&'a str>,
    system_prompt_override: PromptFieldPatch<'a>,
    short_term_goal_override: PromptFieldPatch<'a>,
    long_term_goal_override: PromptFieldPatch<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PromptControlRollbackSigningPayload<'a> {
    operation: &'static str,
    agent_id: &'a str,
    player_id: &'a str,
    public_key: &'a str,
    nonce: u64,
    to_version: u64,
    expected_version: Option<u64>,
    updated_by: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct AgentChatSigningPayload<'a> {
    operation: &'static str,
    agent_id: &'a str,
    player_id: &'a str,
    public_key: &'a str,
    nonce: u64,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent_tick: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct GameplayActionSigningPayload<'a> {
    operation: &'static str,
    action_id: &'a str,
    target_agent_id: &'a str,
    player_id: &'a str,
    public_key: &'a str,
    nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SessionRegisterSigningPayload<'a> {
    operation: &'static str,
    player_id: &'a str,
    public_key: &'a str,
    nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_agent_id: Option<&'a str>,
    force_rebind: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ViewerPlayerAuthSigningEnvelope<'a, T>
where
    T: Serialize,
{
    version: u8,
    payload: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct HostedPromptControlStrongAuthGrantSigningPayload<'a> {
    operation: &'static str,
    agent_id: &'a str,
    player_id: &'a str,
    player_public_key: &'a str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct HostedStrongAuthGrantSigningEnvelope<T>
where
    T: Serialize,
{
    version: u8,
    payload: T,
}

pub fn sign_prompt_control_apply_auth_proof(
    intent: PromptControlAuthIntent,
    request: &PromptControlApplyRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    if nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let player_id =
        normalize_required_field(request.player_id.as_str(), "prompt_control player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "prompt_control public_key",
    )?;
    let signer_public_key =
        normalize_public_key_field(signer_public_key_hex, "prompt_control signer public key")?;
    if signer_public_key != request_public_key {
        return Err("prompt_control public_key does not match signer public key".to_string());
    }

    let signing_key =
        signing_key_from_hex(signer_private_key_hex, "prompt_control signer private key")?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "prompt_control signer public key",
    )?;

    let signing_payload = build_prompt_control_apply_signing_payload(
        intent,
        request,
        player_id.as_str(),
        request_public_key.as_str(),
        nonce,
    )?;
    sign_player_auth_proof(
        signing_key,
        player_id,
        signer_public_key,
        nonce,
        signing_payload,
    )
}

pub fn verify_prompt_control_apply_auth_proof(
    intent: PromptControlAuthIntent,
    request: &PromptControlApplyRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_proof_scheme(proof)?;
    let request_player_id =
        normalize_required_field(request.player_id.as_str(), "prompt_control player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "prompt_control public_key",
    )?;
    let proof_player_id =
        normalize_required_field(proof.player_id.as_str(), "auth proof player_id")?;
    let proof_public_key =
        normalize_public_key_field(proof.public_key.as_str(), "auth proof public key")?;
    if request_player_id != proof_player_id {
        return Err("auth proof player_id does not match request player_id".to_string());
    }
    if request_public_key != proof_public_key {
        return Err("auth proof public_key does not match request public_key".to_string());
    }
    if proof.nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let signing_payload = build_prompt_control_apply_signing_payload(
        intent,
        request,
        proof_player_id.as_str(),
        proof_public_key.as_str(),
        proof.nonce,
    )?;
    verify_player_auth_signature(
        proof_public_key.as_str(),
        proof.signature.as_str(),
        signing_payload.as_slice(),
    )?;
    Ok(VerifiedPlayerAuth {
        player_id: proof_player_id,
        public_key: proof_public_key,
        nonce: proof.nonce,
    })
}

pub fn sign_prompt_control_rollback_auth_proof(
    request: &PromptControlRollbackRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    if nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let player_id =
        normalize_required_field(request.player_id.as_str(), "prompt_control player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "prompt_control public_key",
    )?;
    let signer_public_key =
        normalize_public_key_field(signer_public_key_hex, "prompt_control signer public key")?;
    if signer_public_key != request_public_key {
        return Err("prompt_control public_key does not match signer public key".to_string());
    }

    let signing_key =
        signing_key_from_hex(signer_private_key_hex, "prompt_control signer private key")?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "prompt_control signer public key",
    )?;

    let signing_payload = build_prompt_control_rollback_signing_payload(
        request,
        player_id.as_str(),
        request_public_key.as_str(),
        nonce,
    )?;
    sign_player_auth_proof(
        signing_key,
        player_id,
        signer_public_key,
        nonce,
        signing_payload,
    )
}

pub fn verify_prompt_control_rollback_auth_proof(
    request: &PromptControlRollbackRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_proof_scheme(proof)?;
    let request_player_id =
        normalize_required_field(request.player_id.as_str(), "prompt_control player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "prompt_control public_key",
    )?;
    let proof_player_id =
        normalize_required_field(proof.player_id.as_str(), "auth proof player_id")?;
    let proof_public_key =
        normalize_public_key_field(proof.public_key.as_str(), "auth proof public key")?;
    if request_player_id != proof_player_id {
        return Err("auth proof player_id does not match request player_id".to_string());
    }
    if request_public_key != proof_public_key {
        return Err("auth proof public_key does not match request public_key".to_string());
    }
    if proof.nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let signing_payload = build_prompt_control_rollback_signing_payload(
        request,
        proof_player_id.as_str(),
        proof_public_key.as_str(),
        proof.nonce,
    )?;
    verify_player_auth_signature(
        proof_public_key.as_str(),
        proof.signature.as_str(),
        signing_payload.as_slice(),
    )?;
    Ok(VerifiedPlayerAuth {
        player_id: proof_player_id,
        public_key: proof_public_key,
        nonce: proof.nonce,
    })
}

pub fn sign_hosted_prompt_control_strong_auth_grant(
    action_id: &str,
    player_id: &str,
    player_public_key: &str,
    agent_id: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<HostedStrongAuthGrant, String> {
    if issued_at_unix_ms == 0 {
        return Err(
            "hosted strong-auth grant issued_at_unix_ms must be greater than zero".to_string(),
        );
    }
    if expires_at_unix_ms <= issued_at_unix_ms {
        return Err(
            "hosted strong-auth grant expires_at_unix_ms must be greater than issued_at_unix_ms"
                .to_string(),
        );
    }
    let operation = normalize_prompt_control_grant_operation(action_id)?;
    let player_id = normalize_required_field(player_id, "hosted strong-auth player_id")?;
    let player_public_key =
        normalize_public_key_field(player_public_key, "hosted strong-auth player_public_key")?;
    let agent_id = normalize_required_field(agent_id, "hosted strong-auth agent_id")?;
    let signer_public_key = normalize_public_key_field(
        signer_public_key_hex,
        "hosted strong-auth signer public key",
    )?;
    let signing_key = signing_key_from_hex(
        signer_private_key_hex,
        "hosted strong-auth signer private key",
    )?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "hosted strong-auth signer public key",
    )?;
    let signing_payload = build_hosted_prompt_control_strong_auth_grant_payload(
        operation,
        player_id.as_str(),
        player_public_key.as_str(),
        agent_id.as_str(),
        issued_at_unix_ms,
        expires_at_unix_ms,
    )?;
    let signature = signing_key.sign(signing_payload.as_slice());
    Ok(HostedStrongAuthGrant {
        version: VIEWER_HOSTED_STRONG_AUTH_GRANT_PAYLOAD_VERSION,
        action_id: operation.to_string(),
        player_id,
        player_public_key,
        agent_id,
        issued_at_unix_ms,
        expires_at_unix_ms,
        signer_public_key,
        signature: format!(
            "{VIEWER_HOSTED_STRONG_AUTH_GRANT_SIGNATURE_V1_PREFIX}{}",
            hex::encode(signature.to_bytes())
        ),
    })
}

pub fn verify_hosted_prompt_control_apply_strong_auth_grant(
    intent: PromptControlAuthIntent,
    request: &PromptControlApplyRequest,
    grant: &HostedStrongAuthGrant,
    required_signer_public_key: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    verify_hosted_prompt_control_strong_auth_grant(
        prompt_control_intent_operation(intent),
        request.agent_id.as_str(),
        request.player_id.as_str(),
        request.public_key.as_deref(),
        grant,
        required_signer_public_key,
        now_unix_ms,
    )
}

pub fn verify_hosted_prompt_control_rollback_strong_auth_grant(
    request: &PromptControlRollbackRequest,
    grant: &HostedStrongAuthGrant,
    required_signer_public_key: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    verify_hosted_prompt_control_strong_auth_grant(
        "prompt_control_rollback",
        request.agent_id.as_str(),
        request.player_id.as_str(),
        request.public_key.as_deref(),
        grant,
        required_signer_public_key,
        now_unix_ms,
    )
}

pub fn sign_agent_chat_auth_proof(
    request: &AgentChatRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    if nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let player_id =
        normalize_required_optional_field(request.player_id.as_deref(), "agent_chat player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "agent_chat public_key",
    )?;
    let signer_public_key =
        normalize_public_key_field(signer_public_key_hex, "agent_chat signer public key")?;
    if signer_public_key != request_public_key {
        return Err("agent_chat public_key does not match signer public key".to_string());
    }

    let signing_key =
        signing_key_from_hex(signer_private_key_hex, "agent_chat signer private key")?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "agent_chat signer public key",
    )?;

    let signing_payload = build_agent_chat_signing_payload(
        request,
        player_id.as_str(),
        request_public_key.as_str(),
        nonce,
    )?;
    sign_player_auth_proof(
        signing_key,
        player_id,
        signer_public_key,
        nonce,
        signing_payload,
    )
}

pub fn verify_agent_chat_auth_proof(
    request: &AgentChatRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_proof_scheme(proof)?;
    let request_player_id =
        normalize_required_optional_field(request.player_id.as_deref(), "agent_chat player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "agent_chat public_key",
    )?;
    let proof_player_id =
        normalize_required_field(proof.player_id.as_str(), "auth proof player_id")?;
    let proof_public_key =
        normalize_public_key_field(proof.public_key.as_str(), "auth proof public key")?;
    if request_player_id != proof_player_id {
        return Err("auth proof player_id does not match request player_id".to_string());
    }
    if request_public_key != proof_public_key {
        return Err("auth proof public_key does not match request public_key".to_string());
    }
    if proof.nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let signing_payload = build_agent_chat_signing_payload(
        request,
        proof_player_id.as_str(),
        proof_public_key.as_str(),
        proof.nonce,
    )?;
    verify_player_auth_signature(
        proof_public_key.as_str(),
        proof.signature.as_str(),
        signing_payload.as_slice(),
    )?;
    Ok(VerifiedPlayerAuth {
        player_id: proof_player_id,
        public_key: proof_public_key,
        nonce: proof.nonce,
    })
}

pub fn sign_gameplay_action_auth_proof(
    request: &GameplayActionRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    if nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let player_id =
        normalize_required_field(request.player_id.as_str(), "gameplay_action player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "gameplay_action public_key",
    )?;
    let signer_public_key =
        normalize_public_key_field(signer_public_key_hex, "gameplay_action signer public key")?;
    if signer_public_key != request_public_key {
        return Err("gameplay_action public_key does not match signer public key".to_string());
    }

    let signing_key =
        signing_key_from_hex(signer_private_key_hex, "gameplay_action signer private key")?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "gameplay_action signer public key",
    )?;

    let signing_payload = build_gameplay_action_signing_payload(
        request,
        player_id.as_str(),
        request_public_key.as_str(),
        nonce,
    )?;
    sign_player_auth_proof(
        signing_key,
        player_id,
        signer_public_key,
        nonce,
        signing_payload,
    )
}

pub fn verify_gameplay_action_auth_proof(
    request: &GameplayActionRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_proof_scheme(proof)?;
    let request_player_id =
        normalize_required_field(request.player_id.as_str(), "gameplay_action player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "gameplay_action public_key",
    )?;
    let proof_player_id =
        normalize_required_field(proof.player_id.as_str(), "auth proof player_id")?;
    let proof_public_key =
        normalize_public_key_field(proof.public_key.as_str(), "auth proof public key")?;
    if request_player_id != proof_player_id {
        return Err("auth proof player_id does not match request player_id".to_string());
    }
    if request_public_key != proof_public_key {
        return Err("auth proof public_key does not match request public_key".to_string());
    }
    if proof.nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let signing_payload = build_gameplay_action_signing_payload(
        request,
        proof_player_id.as_str(),
        proof_public_key.as_str(),
        proof.nonce,
    )?;
    verify_player_auth_signature(
        proof_public_key.as_str(),
        proof.signature.as_str(),
        signing_payload.as_slice(),
    )?;
    Ok(VerifiedPlayerAuth {
        player_id: proof_player_id,
        public_key: proof_public_key,
        nonce: proof.nonce,
    })
}

pub fn sign_session_register_auth_proof(
    request: &AuthoritativeSessionRegisterRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    if nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let player_id =
        normalize_required_field(request.player_id.as_str(), "session_register player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "session_register public_key",
    )?;
    let signer_public_key =
        normalize_public_key_field(signer_public_key_hex, "session_register signer public key")?;
    if signer_public_key != request_public_key {
        return Err("session_register public_key does not match signer public key".to_string());
    }

    let signing_key = signing_key_from_hex(
        signer_private_key_hex,
        "session_register signer private key",
    )?;
    verify_keypair_match(
        &signing_key,
        signer_public_key.as_str(),
        "session_register signer public key",
    )?;

    let signing_payload = build_session_register_signing_payload(
        request,
        player_id.as_str(),
        request_public_key.as_str(),
        nonce,
    )?;
    sign_player_auth_proof(
        signing_key,
        player_id,
        signer_public_key,
        nonce,
        signing_payload,
    )
}

pub fn verify_session_register_auth_proof(
    request: &AuthoritativeSessionRegisterRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_proof_scheme(proof)?;
    let request_player_id =
        normalize_required_field(request.player_id.as_str(), "session_register player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request.public_key.as_deref(),
        "session_register public_key",
    )?;
    let proof_player_id =
        normalize_required_field(proof.player_id.as_str(), "auth proof player_id")?;
    let proof_public_key =
        normalize_public_key_field(proof.public_key.as_str(), "auth proof public key")?;
    if request_player_id != proof_player_id {
        return Err("auth proof player_id does not match request player_id".to_string());
    }
    if request_public_key != proof_public_key {
        return Err("auth proof public_key does not match request public_key".to_string());
    }
    if proof.nonce == 0 {
        return Err("auth nonce must be greater than zero".to_string());
    }
    let signing_payload = build_session_register_signing_payload(
        request,
        proof_player_id.as_str(),
        proof_public_key.as_str(),
        proof.nonce,
    )?;
    verify_player_auth_signature(
        proof_public_key.as_str(),
        proof.signature.as_str(),
        signing_payload.as_slice(),
    )?;
    Ok(VerifiedPlayerAuth {
        player_id: proof_player_id,
        public_key: proof_public_key,
        nonce: proof.nonce,
    })
}

fn build_prompt_control_apply_signing_payload(
    intent: PromptControlAuthIntent,
    request: &PromptControlApplyRequest,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<Vec<u8>, String> {
    let payload = PromptControlApplySigningPayload {
        operation: prompt_control_intent_operation(intent),
        agent_id: request.agent_id.as_str(),
        player_id,
        public_key,
        nonce,
        expected_version: request.expected_version,
        updated_by: request.updated_by.as_deref(),
        system_prompt_override: prompt_field_patch(&request.system_prompt_override),
        short_term_goal_override: prompt_field_patch(&request.short_term_goal_override),
        long_term_goal_override: prompt_field_patch(&request.long_term_goal_override),
    };
    encode_signing_payload(payload)
}

fn build_prompt_control_rollback_signing_payload(
    request: &PromptControlRollbackRequest,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<Vec<u8>, String> {
    let payload = PromptControlRollbackSigningPayload {
        operation: "prompt_control_rollback",
        agent_id: request.agent_id.as_str(),
        player_id,
        public_key,
        nonce,
        to_version: request.to_version,
        expected_version: request.expected_version,
        updated_by: request.updated_by.as_deref(),
    };
    encode_signing_payload(payload)
}

fn build_agent_chat_signing_payload(
    request: &AgentChatRequest,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<Vec<u8>, String> {
    let intent_seq = match request.intent_seq {
        Some(0) => {
            return Err("agent_chat intent_seq must be greater than zero".to_string());
        }
        Some(value) => Some(value),
        None => None,
    };
    let payload = AgentChatSigningPayload {
        operation: "agent_chat",
        agent_id: request.agent_id.as_str(),
        player_id,
        public_key,
        nonce,
        message: request.message.as_str(),
        intent_tick: request.intent_tick,
        intent_seq,
    };
    encode_signing_payload(payload)
}

fn build_gameplay_action_signing_payload(
    request: &GameplayActionRequest,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<Vec<u8>, String> {
    let action_id =
        normalize_required_field(request.action_id.as_str(), "gameplay_action action_id")?;
    let target_agent_id = normalize_required_field(
        request.target_agent_id.as_str(),
        "gameplay_action target_agent_id",
    )?;
    let payload = GameplayActionSigningPayload {
        operation: "gameplay_action",
        action_id: action_id.as_str(),
        target_agent_id: target_agent_id.as_str(),
        player_id,
        public_key,
        nonce,
    };
    encode_signing_payload(payload)
}

fn build_session_register_signing_payload(
    request: &AuthoritativeSessionRegisterRequest,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<Vec<u8>, String> {
    let payload = SessionRegisterSigningPayload {
        operation: "session_register",
        player_id,
        public_key,
        nonce,
        requested_agent_id: request
            .requested_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        force_rebind: request.force_rebind,
    };
    encode_signing_payload(payload)
}

fn encode_signing_payload<T>(payload: T) -> Result<Vec<u8>, String>
where
    T: Serialize,
{
    let envelope = ViewerPlayerAuthSigningEnvelope {
        version: VIEWER_PLAYER_AUTH_PAYLOAD_VERSION,
        payload,
        actor: None,
    };
    serde_cbor::to_vec(&envelope).map_err(|err| format!("encode auth payload failed: {err}"))
}

fn build_hosted_prompt_control_strong_auth_grant_payload(
    operation: &'static str,
    player_id: &str,
    player_public_key: &str,
    agent_id: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
) -> Result<Vec<u8>, String> {
    let payload = HostedPromptControlStrongAuthGrantSigningPayload {
        operation,
        agent_id,
        player_id,
        player_public_key,
        issued_at_unix_ms,
        expires_at_unix_ms,
    };
    let envelope = HostedStrongAuthGrantSigningEnvelope {
        version: VIEWER_HOSTED_STRONG_AUTH_GRANT_PAYLOAD_VERSION,
        payload,
    };
    serde_cbor::to_vec(&envelope)
        .map_err(|err| format!("encode hosted strong-auth grant payload failed: {err}"))
}

fn prompt_control_intent_operation(intent: PromptControlAuthIntent) -> &'static str {
    match intent {
        PromptControlAuthIntent::Preview => "prompt_control_preview",
        PromptControlAuthIntent::Apply => "prompt_control_apply",
    }
}

fn prompt_field_patch(value: &Option<Option<String>>) -> PromptFieldPatch<'_> {
    match value {
        None => PromptFieldPatch {
            mode: PromptFieldMode::Unchanged,
            value: None,
        },
        Some(None) => PromptFieldPatch {
            mode: PromptFieldMode::Clear,
            value: None,
        },
        Some(Some(next)) => PromptFieldPatch {
            mode: PromptFieldMode::Set,
            value: Some(next.as_str()),
        },
    }
}

fn sign_player_auth_proof(
    signing_key: SigningKey,
    player_id: String,
    public_key: String,
    nonce: u64,
    signing_payload: Vec<u8>,
) -> Result<PlayerAuthProof, String> {
    let signature: Signature = signing_key.sign(signing_payload.as_slice());
    Ok(PlayerAuthProof {
        scheme: PlayerAuthScheme::Ed25519,
        player_id,
        public_key,
        nonce,
        signature: format!(
            "{VIEWER_PLAYER_AUTH_SIGNATURE_V1_PREFIX}{}",
            hex::encode(signature.to_bytes())
        ),
    })
}

fn verify_player_auth_signature(
    public_key_hex: &str,
    signature: &str,
    signing_payload: &[u8],
) -> Result<(), String> {
    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "auth public key")?;
    let signature_hex = signature
        .strip_prefix(VIEWER_PLAYER_AUTH_SIGNATURE_V1_PREFIX)
        .ok_or_else(|| "auth signature is not awviewauth:v1".to_string())?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "auth signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|err| format!("parse auth public key failed: {err}"))?;
    verifying_key
        .verify(signing_payload, &Signature::from_bytes(&signature_bytes))
        .map_err(|err| format!("verify auth signature failed: {err}"))
}

fn verify_proof_scheme(proof: &PlayerAuthProof) -> Result<(), String> {
    match proof.scheme {
        PlayerAuthScheme::Ed25519 => Ok(()),
    }
}

fn normalize_required_optional_field(raw: Option<&str>, label: &str) -> Result<String, String> {
    let Some(raw) = raw else {
        return Err(format!("{label} is required"));
    };
    normalize_required_field(raw, label)
}

fn normalize_required_optional_public_key(
    raw: Option<&str>,
    label: &str,
) -> Result<String, String> {
    let Some(raw) = raw else {
        return Err(format!("{label} is required"));
    };
    normalize_public_key_field(raw, label)
}

fn normalize_required_field(raw: &str, label: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{label} is empty"));
    }
    Ok(value.to_string())
}

fn normalize_public_key_field(raw: &str, label: &str) -> Result<String, String> {
    let normalized = normalize_required_field(raw, label)?;
    let bytes = decode_hex_array::<32>(normalized.as_str(), label)?;
    Ok(hex::encode(bytes))
}

fn normalize_prompt_control_grant_operation(raw: &str) -> Result<&'static str, String> {
    match raw.trim() {
        "prompt_control_preview" => Ok("prompt_control_preview"),
        "prompt_control_apply" => Ok("prompt_control_apply"),
        "prompt_control_rollback" => Ok("prompt_control_rollback"),
        _ => Err("unsupported hosted strong-auth action_id".to_string()),
    }
}

fn signing_key_from_hex(private_key_hex: &str, label: &str) -> Result<SigningKey, String> {
    let private_key_bytes = decode_hex_array::<32>(private_key_hex, label)?;
    Ok(SigningKey::from_bytes(&private_key_bytes))
}

fn verify_keypair_match(
    signing_key: &SigningKey,
    public_key_hex: &str,
    label: &str,
) -> Result<(), String> {
    let expected_public_key = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_key != public_key_hex {
        return Err(format!(
            "{label} does not match private key: expected={expected_public_key} actual={public_key_hex}"
        ));
    }
    Ok(())
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(raw).map_err(|err| format!("decode {label} failed: {err}"))?;
    if bytes.len() != N {
        return Err(format!(
            "{label} length mismatch: expected {N} bytes, got {}",
            bytes.len()
        ));
    }
    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(bytes.as_slice());
    Ok(fixed)
}

fn verify_hosted_prompt_control_strong_auth_grant(
    expected_action_id: &str,
    request_agent_id: &str,
    request_player_id: &str,
    request_public_key: Option<&str>,
    grant: &HostedStrongAuthGrant,
    required_signer_public_key: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if grant.version != VIEWER_HOSTED_STRONG_AUTH_GRANT_PAYLOAD_VERSION {
        return Err(format!(
            "hosted strong-auth grant version mismatch: expected={} actual={}",
            VIEWER_HOSTED_STRONG_AUTH_GRANT_PAYLOAD_VERSION, grant.version
        ));
    }
    let action_id = normalize_prompt_control_grant_operation(grant.action_id.as_str())?;
    if action_id != expected_action_id {
        return Err("hosted strong-auth grant action_id does not match request".to_string());
    }
    let request_agent_id =
        normalize_required_field(request_agent_id, "hosted strong-auth request agent_id")?;
    let request_player_id =
        normalize_required_field(request_player_id, "hosted strong-auth request player_id")?;
    let request_public_key = normalize_required_optional_public_key(
        request_public_key,
        "hosted strong-auth request public_key",
    )?;
    let grant_player_id = normalize_required_field(
        grant.player_id.as_str(),
        "hosted strong-auth grant player_id",
    )?;
    let grant_player_public_key = normalize_public_key_field(
        grant.player_public_key.as_str(),
        "hosted strong-auth grant player_public_key",
    )?;
    let grant_agent_id =
        normalize_required_field(grant.agent_id.as_str(), "hosted strong-auth grant agent_id")?;
    if request_player_id != grant_player_id {
        return Err("hosted strong-auth grant player_id does not match request".to_string());
    }
    if request_public_key != grant_player_public_key {
        return Err("hosted strong-auth grant public_key does not match request".to_string());
    }
    if request_agent_id != grant_agent_id {
        return Err("hosted strong-auth grant agent_id does not match request".to_string());
    }
    if grant.expires_at_unix_ms <= grant.issued_at_unix_ms {
        return Err(
            "hosted strong-auth grant expires_at_unix_ms must be greater than issued_at_unix_ms"
                .to_string(),
        );
    }
    if now_unix_ms > grant.expires_at_unix_ms {
        return Err("hosted strong-auth grant has expired".to_string());
    }
    let required_signer_public_key = normalize_public_key_field(
        required_signer_public_key,
        "hosted strong-auth required signer public key",
    )?;
    let grant_signer_public_key = normalize_public_key_field(
        grant.signer_public_key.as_str(),
        "hosted strong-auth grant signer public key",
    )?;
    if grant_signer_public_key != required_signer_public_key {
        return Err("hosted strong-auth grant signer is not allowlisted".to_string());
    }
    let signing_payload = build_hosted_prompt_control_strong_auth_grant_payload(
        action_id,
        grant_player_id.as_str(),
        grant_player_public_key.as_str(),
        grant_agent_id.as_str(),
        grant.issued_at_unix_ms,
        grant.expires_at_unix_ms,
    )?;
    verify_hosted_strong_auth_grant_signature(
        grant_signer_public_key.as_str(),
        grant.signature.as_str(),
        signing_payload.as_slice(),
    )
}

fn verify_hosted_strong_auth_grant_signature(
    public_key_hex: &str,
    signature: &str,
    signing_payload: &[u8],
) -> Result<(), String> {
    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "hosted strong-auth public key")?;
    let signature_hex = signature
        .strip_prefix(VIEWER_HOSTED_STRONG_AUTH_GRANT_SIGNATURE_V1_PREFIX)
        .ok_or_else(|| "hosted strong-auth signature is not awhostedgrant:v1".to_string())?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "hosted strong-auth signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|err| format!("parse hosted strong-auth public key failed: {err}"))?;
    verifying_key
        .verify(signing_payload, &Signature::from_bytes(&signature_bytes))
        .map_err(|err| format!("verify hosted strong-auth signature failed: {err}"))
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
