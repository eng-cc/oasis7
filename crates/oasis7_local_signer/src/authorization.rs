use crate::error::SignerError;
use crate::identity::{operation_key, parse_sha256_hex, request_fingerprint, request_key};
use crate::protocol::{IpcRequest, SignContext};
use crate::rollback::{ValidatedRollbackPayload, validate_canonical_payload};
use crate::types::{BatchGrant, InstallationConfig, KeyBinding, Policy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedSign {
    pub installation_id: String,
    pub deployment_id: String,
    pub network_id: String,
    pub caller_uid: u32,
    pub request_id: String,
    pub purpose: String,
    pub signer_id: String,
    pub grant_id: String,
    pub operation_key: String,
    pub request_key: String,
    pub fingerprint: String,
    pub policy_revision: String,
    pub payload_sha256: String,
    pub public_key_sha256: String,
    pub max_distinct_requests: u32,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub context: SignContext,
    pub payload: Vec<u8>,
    pub rollback: ValidatedRollbackPayload,
}

pub fn authorize_sign(
    installation: &InstallationConfig,
    policy: &Policy,
    grant: &BatchGrant,
    caller_uid: u32,
    request: &IpcRequest,
    now_ms: u64,
) -> Result<AuthorizedSign, SignerError> {
    installation.validate().map_err(SignerError::InvalidInput)?;
    policy
        .validate(installation)
        .map_err(SignerError::InvalidInput)?;
    grant
        .validate(installation, policy)
        .map_err(SignerError::InvalidInput)?;

    let IpcRequest::Sign {
        schema_version: _,
        installation_id,
        request_id,
        purpose,
        provider_id,
        signer_id,
        grant_id,
        context,
        payload_base64,
    } = request
    else {
        return Err(SignerError::InvalidInput(
            "worker sign operation requires a sign command".to_owned(),
        ));
    };
    request.validate()?;

    if installation_id != &installation.installation_id
        || grant_id.0.as_deref() != Some(grant.grant_id.as_str())
        || caller_uid != grant.caller_uid
        || context.deployment_id != installation.deployment_id
        || context.deployment_id != grant.deployment_id
        || context.network_id != grant.network_id
        || context.task_uid != grant.task_uid
        || context.source_head_oid != grant.source_head_oid
        || purpose != "rollback_strict_audit_v1"
        || provider_id.0.is_some()
    {
        return Err(SignerError::AuthorizationDenied);
    }
    if !policy
        .enabled_purposes
        .iter()
        .any(|enabled| enabled == purpose)
    {
        return Err(SignerError::AuthorizationDenied);
    }
    if now_ms < grant.not_before || now_ms >= grant.expires_at {
        return Err(SignerError::AuthorizationDenied);
    }

    let payload = crate::protocol::decode_payload(payload_base64)?;
    if payload.len() as u64 > policy.limits.max_payload_bytes {
        return Err(SignerError::AuthorizationDenied);
    }
    let payload_digest_bytes = crate::identity::sha256(&payload);
    let payload_sha256 = hex::encode(payload_digest_bytes);
    let rollback = validate_canonical_payload(&payload, &context.protocol_context)
        .map_err(|_| SignerError::CryptoOrBindingInvalid)?;
    let not_before_ms = grant.not_before.max(rollback.issued_at_ms);
    let expires_at_ms = grant.expires_at.min(rollback.expires_at_ms);
    if now_ms < not_before_ms || now_ms >= expires_at_ms {
        return Err(SignerError::AuthorizationDenied);
    }

    let binding = unique_policy_key_binding(policy, purpose, signer_id)?;
    validate_key_binding_context(binding, context)?;
    let operation_key = operation_key(
        &installation.installation_id,
        &installation.deployment_id,
        &context.network_id,
        purpose,
        signer_id,
        &payload_digest_bytes,
    )?;
    let item = unique_grant_item(grant, &operation_key)?;
    if item.provider_id.0.as_deref() != provider_id.0.as_deref()
        || item.purpose != *purpose
        || item.signer_id != *signer_id
        || item.payload_sha256 != payload_sha256
        || item.protocol_bindings != context.protocol_context
        || item.public_key_sha256 != binding.public_key_sha256
    {
        return Err(SignerError::AuthorizationDenied);
    }

    let request_key = request_key(
        installation_id,
        purpose,
        caller_uid,
        provider_id.0.as_deref(),
        request_id,
    )?;
    let fingerprint = request_fingerprint(
        request,
        &grant.grant_id,
        &operation_key,
        &policy.policy_revision,
    )?;
    Ok(AuthorizedSign {
        installation_id: installation.installation_id.clone(),
        deployment_id: installation.deployment_id.clone(),
        network_id: context.network_id.clone(),
        caller_uid,
        request_id: request_id.clone(),
        purpose: purpose.clone(),
        signer_id: signer_id.clone(),
        grant_id: grant.grant_id.clone(),
        operation_key,
        request_key,
        fingerprint,
        policy_revision: policy.policy_revision.clone(),
        payload_sha256,
        public_key_sha256: binding.public_key_sha256.clone(),
        max_distinct_requests: item.max_distinct_requests,
        not_before_ms,
        expires_at_ms,
        context: context.clone(),
        payload,
        rollback,
    })
}

fn unique_policy_key_binding<'a>(
    policy: &'a Policy,
    purpose: &str,
    signer_id: &str,
) -> Result<&'a KeyBinding, SignerError> {
    let mut bindings = policy
        .key_bindings
        .iter()
        .filter(|binding| binding.purpose == purpose && binding.signer_id == signer_id);
    let binding = bindings.next().ok_or(SignerError::AuthorizationDenied)?;
    if bindings.next().is_some() {
        return Err(SignerError::AuthorizationDenied);
    }
    Ok(binding)
}

fn validate_key_binding_context(
    binding: &KeyBinding,
    context: &SignContext,
) -> Result<(), SignerError> {
    let protocol = &context.protocol_context;
    if !binding
        .protocol_authorities
        .iter()
        .any(|authority| authority == &protocol.authority_id)
        || !binding
            .deployment_ids
            .iter()
            .any(|deployment| deployment == &context.deployment_id)
        || !binding
            .network_ids
            .iter()
            .any(|network| network == &context.network_id)
    {
        return Err(SignerError::AuthorizationDenied);
    }
    Ok(())
}

fn unique_grant_item<'a>(
    grant: &'a BatchGrant,
    operation_key: &str,
) -> Result<&'a crate::types::GrantItem, SignerError> {
    let mut items = grant
        .items
        .iter()
        .filter(|item| item.operation_key == operation_key);
    let item = items.next().ok_or(SignerError::AuthorizationDenied)?;
    if items.next().is_some() {
        return Err(SignerError::AuthorizationDenied);
    }
    Ok(item)
}

pub fn parse_payload_sha256(value: &str) -> Result<[u8; 32], SignerError> {
    parse_sha256_hex(value)
}
