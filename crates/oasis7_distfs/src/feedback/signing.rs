use ed25519_dalek::{Signer, SigningKey};

use super::{
    FEEDBACK_SIGNATURE_PAYLOAD_VERSION, FeedbackActionKind, FeedbackAppendRequest,
    FeedbackCreateRequest, FeedbackSignedPayload, FeedbackTombstoneRequest, WorldError, blake3_hex,
    decode_hex_array, feedback_create_content_hash, to_canonical_cbor,
};

pub fn public_key_hex_from_signing_key_hex(signing_key_hex: &str) -> Result<String, WorldError> {
    let signing_key = parse_signing_key(signing_key_hex)?;
    Ok(hex::encode(signing_key.verifying_key().to_bytes()))
}

pub fn sign_feedback_create_request(
    request: &FeedbackCreateRequest,
    signing_key_hex: &str,
) -> Result<String, WorldError> {
    let content_hash = feedback_create_content_hash(request)?;
    sign_feedback_request(
        FeedbackActionKind::Create,
        request.feedback_id.as_str(),
        request.author_public_key_hex.as_str(),
        content_hash.as_str(),
        request.nonce.as_str(),
        request.timestamp_ms,
        request.expires_at_ms,
        signing_key_hex,
    )
}

pub fn sign_feedback_append_request(
    request: &FeedbackAppendRequest,
    signing_key_hex: &str,
) -> Result<String, WorldError> {
    let content_hash = blake3_hex(request.content.as_bytes());
    sign_feedback_request(
        FeedbackActionKind::Append,
        request.feedback_id.as_str(),
        request.actor_public_key_hex.as_str(),
        content_hash.as_str(),
        request.nonce.as_str(),
        request.timestamp_ms,
        request.expires_at_ms,
        signing_key_hex,
    )
}

pub fn sign_feedback_tombstone_request(
    request: &FeedbackTombstoneRequest,
    signing_key_hex: &str,
) -> Result<String, WorldError> {
    let reason_hash = blake3_hex(request.reason.as_bytes());
    sign_feedback_request(
        FeedbackActionKind::Tombstone,
        request.feedback_id.as_str(),
        request.actor_public_key_hex.as_str(),
        reason_hash.as_str(),
        request.nonce.as_str(),
        request.timestamp_ms,
        request.expires_at_ms,
        signing_key_hex,
    )
}

fn sign_feedback_request(
    action: FeedbackActionKind,
    feedback_id: &str,
    actor_public_key_hex: &str,
    content_hash: &str,
    nonce: &str,
    timestamp_ms: i64,
    expires_at_ms: i64,
    signing_key_hex: &str,
) -> Result<String, WorldError> {
    let signing_key = parse_signing_key(signing_key_hex)?;
    let expected_public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_key_hex != actor_public_key_hex {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "signing key does not match actor public key: expected={} actual={}",
                expected_public_key_hex, actor_public_key_hex
            ),
        });
    }
    let payload = FeedbackSignedPayload {
        version: FEEDBACK_SIGNATURE_PAYLOAD_VERSION,
        action: action.as_str(),
        feedback_id,
        actor_public_key_hex,
        content_hash,
        nonce,
        timestamp_ms,
        expires_at_ms,
    };
    let payload_bytes = to_canonical_cbor(&payload)?;
    let signature = signing_key.sign(payload_bytes.as_slice());
    Ok(hex::encode(signature.to_bytes()))
}

fn parse_signing_key(signing_key_hex: &str) -> Result<SigningKey, WorldError> {
    let signing_key_bytes = decode_hex_array::<32>(signing_key_hex, "feedback signing key")?;
    Ok(SigningKey::from_bytes(&signing_key_bytes))
}
