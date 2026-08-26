//! Replay application for trusted capability authorization transitions.
//!
//! Authorization maps are part of the world state, but their mutations must
//! also be journal evidence.  This module applies those evidence records
//! during both normal append and stale-snapshot recovery.

use oasis7_wasm_abi::{
    CapabilityAudience, CapabilityCatalogEntry, CapabilityGrantV2, ModuleCallCaller, canonical_hash,
};
use serde::Serialize;

use super::super::capability_authorization::{
    CapabilityAuthorityRecord, CapabilityAuthorizationAuditReceipt,
    CapabilityAuthorizationNonceRecord, CapabilityBudgetAccount, CapabilityEffectReceiptLink,
    CapabilityInvocationContext,
};
use super::super::governance::{GovernanceFinalityCertificate, ProposalStatus};
use super::super::{CapabilityAuthorizationEvent, WorldError, WorldTime};
use super::World;
use super::capability_authorization::{
    deny, validate_authority_record, validate_invocation_context,
};
use super::capability_authorization_state::{capability_budget_key, validate_budget_account};

impl World {
    pub(super) fn verify_capability_authority_finality(
        &self,
        record: &CapabilityAuthorityRecord,
        certificate: &GovernanceFinalityCertificate,
    ) -> Result<(), WorldError> {
        let proposal = self
            .proposals
            .get(&certificate.proposal_id)
            .ok_or_else(|| deny("authority finality proposal is not known"))?;
        let proposal_manifest_hash = match &proposal.status {
            ProposalStatus::Approved { manifest_hash, .. }
            | ProposalStatus::Applied { manifest_hash } => manifest_hash,
            _ => return Err(deny("authority finality proposal is not approved")),
        };
        if proposal_manifest_hash != &certificate.manifest_hash {
            return Err(deny("authority finality proposal manifest hash mismatch"));
        }
        self.validate_governance_finality_certificate(
            certificate.proposal_id,
            certificate.manifest_hash.as_str(),
            certificate.epoch_id,
            certificate,
        )
        .map_err(|error| deny(format!("authority finality certificate: {error:?}")))?;
        if record.finality_epoch != certificate.epoch_id {
            return Err(deny("authority finality epoch does not match certificate"));
        }
        if !certificate.signatures.contains_key(&record.issuer_id) {
            return Err(deny(
                "authority issuer identity is not one of the finality signers",
            ));
        }
        let signer_public_key = self
            .node_identity_public_key(record.issuer_id.as_str())
            .ok_or_else(|| deny("authority issuer identity is not trusted"))?;
        if !signer_public_key.eq_ignore_ascii_case(record.public_key_hex.trim()) {
            return Err(deny(
                "authority issuer key does not match its finality signer identity",
            ));
        }
        Ok(())
    }

    pub(super) fn apply_capability_authorization_event(
        &mut self,
        event: &CapabilityAuthorizationEvent,
        time: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            CapabilityAuthorizationEvent::AuthorityInstalled { record } => {
                apply_authority_record(self, record)?;
            }
            CapabilityAuthorizationEvent::InvocationContextInstalled { key, context } => {
                apply_invocation_context(self, key, context)?;
            }
            CapabilityAuthorizationEvent::BudgetAccountInstalled { key, account } => {
                apply_budget_account(self, key, account)?;
            }
            CapabilityAuthorizationEvent::GrantRegistered { grant } => {
                apply_registered_grant(self, grant, time)?;
            }
            CapabilityAuthorizationEvent::CommandCommitted {
                budget_key,
                budget_account,
                grant,
                nonce_key,
                nonce_record,
                receipt,
                effect_receipt_links,
            } => {
                apply_command_commit(
                    self,
                    budget_key,
                    budget_account,
                    grant,
                    nonce_key,
                    nonce_record,
                    receipt,
                    effect_receipt_links,
                    time,
                )?;
            }
            CapabilityAuthorizationEvent::EffectReceiptCommitted {
                intent_id,
                authorization_receipt_id,
                effect_receipt_id,
            } => {
                apply_effect_receipt_commit(
                    self,
                    intent_id,
                    authorization_receipt_id,
                    effect_receipt_id,
                )?;
            }
        }
        self.refresh_capability_authorization_root()
    }
}

fn apply_authority_record(
    world: &mut World,
    record: &CapabilityAuthorityRecord,
) -> Result<(), WorldError> {
    validate_authority_record(record)?;
    if world.chain_resource_manifest.world_id != "unbound"
        && world.chain_resource_manifest.world_id != record.world_id
    {
        return Err(deny("authority record world does not match live world"));
    }
    if let Some(existing) = world
        .capability_revocation_state
        .authority_records
        .get(&record.issuer_id)
        && existing != record
    {
        return Err(deny("authority record is immutable"));
    }
    world.capability_revocation_state.epoch = world
        .capability_revocation_state
        .epoch
        .max(record.revocation_epoch);
    world
        .capability_revocation_state
        .revoked_grant_ids
        .extend(record.revoked_grant_ids.iter().cloned());
    world
        .capability_revocation_state
        .superseded_by
        .extend(record.superseded_by.clone());
    world.capability_revocation_state.finalized_receipt_id =
        Some(record.finalized_receipt_id.clone());
    world
        .capability_revocation_state
        .authority_records
        .insert(record.issuer_id.clone(), record.clone());
    Ok(())
}

fn apply_invocation_context(
    world: &mut World,
    key: &str,
    context: &CapabilityInvocationContext,
) -> Result<(), WorldError> {
    validate_invocation_context(context)?;
    let expected_key = capability_invocation_context_key(context)?;
    if key != expected_key {
        return Err(deny(
            "invocation context journal key does not match context",
        ));
    }
    if let Some(existing) = world.capability_invocation_contexts.get(key)
        && existing != context
    {
        return Err(deny("invocation context is immutable"));
    }
    world
        .capability_invocation_contexts
        .insert(key.to_string(), context.clone());
    Ok(())
}

fn apply_budget_account(
    world: &mut World,
    key: &str,
    account: &CapabilityBudgetAccount,
) -> Result<(), WorldError> {
    validate_budget_account(account)?;
    if capability_budget_key(&account.subject, &account.grant_id)? != key {
        return Err(deny("capability budget journal key does not match account"));
    }
    if let Some(existing) = world.capability_budget_accounts.get(key)
        && existing != account
    {
        return Err(deny("capability budget account is immutable"));
    }
    world
        .capability_budget_accounts
        .insert(key.to_string(), account.clone());
    Ok(())
}

fn apply_registered_grant(
    world: &mut World,
    grant: &CapabilityGrantV2,
    time: WorldTime,
) -> Result<(), WorldError> {
    validate_grant_body(grant, time)?;
    world.verify_issuer(grant)?;
    world.verify_live_revocation(grant)?;
    let encoded = serde_json::to_value(grant)?;
    if let Some(existing) = world.capability_grants_v2.get(&grant.grant_id)
        && existing != &encoded
    {
        return Err(deny("immutable grant body changed"));
    }
    world
        .capability_grants_v2
        .insert(grant.grant_id.clone(), encoded);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_command_commit(
    world: &mut World,
    budget_key: &str,
    budget_account: &CapabilityBudgetAccount,
    grant: &CapabilityGrantV2,
    nonce_key: &str,
    nonce_record: &CapabilityAuthorizationNonceRecord,
    receipt: &CapabilityAuthorizationAuditReceipt,
    effect_receipt_links: &std::collections::BTreeMap<String, CapabilityEffectReceiptLink>,
    time: WorldTime,
) -> Result<(), WorldError> {
    validate_grant_body(grant, time)?;
    world.verify_issuer(grant)?;
    world.verify_live_revocation(grant)?;
    validate_budget_account(budget_account)?;
    if capability_budget_key(&budget_account.subject, &budget_account.grant_id)? != budget_key
        || budget_account.subject != grant.subject
        || budget_account.grant_id != grant.grant_id
    {
        return Err(deny("capability command budget binding is invalid"));
    }
    if nonce_record.state != "committed"
        || nonce_record.request_hash.trim().is_empty()
        || nonce_record.outcome_hash.trim().is_empty()
        || nonce_record.committed_receipt_id.as_deref() != Some(receipt.receipt_id.as_str())
        || nonce_record.request_hash != receipt.canonical_request_hash
        || nonce_record.outcome_hash != receipt.canonical_result_hash
    {
        return Err(deny("capability command nonce journal record is invalid"));
    }
    let response_nonce = receipt
        .response_nonce
        .as_deref()
        .ok_or_else(|| deny("capability command receipt nonce is required"))?;
    if authorization_nonce_key(grant, response_nonce)? != nonce_key {
        return Err(deny("capability command nonce key is not canonical"));
    }
    if receipt.grant_id.as_deref() != Some(grant.grant_id.as_str())
        || receipt.authorization_nonce_key_hash.as_deref() != Some(nonce_key)
        || receipt.decision != "accepted"
        || receipt.receipt_id.trim().is_empty()
        || receipt.canonical_request_hash.trim().is_empty()
        || receipt.canonical_result_hash.trim().is_empty()
        || receipt.subject != serde_json::to_value(&grant.subject)?
        || receipt.audience != serde_json::to_value(&grant.audience)?
    {
        return Err(deny(
            "capability command receipt journal binding is invalid",
        ));
    }
    let encoded = serde_json::to_value(grant)?;
    if let Some(existing) = world.capability_grants_v2.get(&grant.grant_id)
        && existing != &encoded
    {
        return Err(deny("immutable grant body changed"));
    }
    world
        .capability_grants_v2
        .insert(grant.grant_id.clone(), encoded);

    if let Some(existing) = world.capability_nonce_records.get(nonce_key)
        && existing != nonce_record
    {
        return Err(deny("capability nonce journal record changed"));
    }
    world
        .capability_nonce_records
        .insert(nonce_key.to_string(), nonce_record.clone());

    if let Some(existing) = world
        .capability_authorization_receipts
        .get(&receipt.receipt_id)
        && existing != receipt
    {
        return Err(deny("capability authorization receipt changed"));
    }
    world
        .capability_authorization_receipts
        .insert(receipt.receipt_id.clone(), receipt.clone());

    if let Some(existing) = world.capability_budget_accounts.get(budget_key)
        && (existing.remaining_units < budget_account.remaining_units
            || existing.spent_units > budget_account.spent_units)
    {
        return Err(deny("capability budget journal transition regressed"));
    }
    world
        .capability_budget_accounts
        .insert(budget_key.to_string(), budget_account.clone());

    for (intent_id, link) in effect_receipt_links {
        if intent_id.trim().is_empty() || link.authorization_receipt_id != receipt.receipt_id {
            return Err(deny("capability effect receipt journal binding is invalid"));
        }
        if let Some(existing) = world.capability_effect_receipt_links.get(intent_id)
            && existing != link
        {
            return Err(deny("capability effect receipt link changed"));
        }
        world
            .capability_effect_receipt_links
            .insert(intent_id.clone(), link.clone());
    }
    Ok(())
}

fn apply_effect_receipt_commit(
    world: &mut World,
    intent_id: &str,
    authorization_receipt_id: &str,
    effect_receipt_id: &str,
) -> Result<(), WorldError> {
    if intent_id.trim().is_empty()
        || authorization_receipt_id.trim().is_empty()
        || effect_receipt_id.trim().is_empty()
    {
        return Err(deny(
            "effect receipt authorization journal binding is required",
        ));
    }
    let Some(link) = world.capability_effect_receipt_links.get(intent_id) else {
        if world
            .capability_authorization_receipts
            .get(authorization_receipt_id)
            .and_then(|receipt| receipt.committed_effect_receipt_id.as_deref())
            == Some(effect_receipt_id)
        {
            return Ok(());
        }
        return Err(deny("effect receipt authorization link is missing"));
    };
    if link.authorization_receipt_id != authorization_receipt_id {
        return Err(deny("effect receipt authorization link does not match"));
    }
    let audit = world
        .capability_authorization_receipts
        .get_mut(authorization_receipt_id)
        .ok_or_else(|| deny("effect receipt authorization link has no audit receipt"))?;
    if let Some(existing) = &audit.committed_effect_receipt_id
        && existing != effect_receipt_id
    {
        return Err(deny("authorization receipt effect binding changed"));
    }
    audit.committed_effect_receipt_id = Some(effect_receipt_id.to_string());
    world.capability_effect_receipt_links.remove(intent_id);
    Ok(())
}

fn validate_grant_body(grant: &CapabilityGrantV2, time: WorldTime) -> Result<(), WorldError> {
    grant
        .validate()
        .map_err(|error| deny(format!("grant validation: {error}")))?;
    if !grant
        .body_hash_matches()
        .map_err(|error| deny(format!("grant body hash: {error}")))?
        || grant
            .expected_grant_id()
            .map_err(|error| deny(format!("grant id hash: {error}")))?
            != grant.grant_id
    {
        return Err(deny("grant canonical body hash or id mismatch"));
    }
    if grant.issued_at_tick > time {
        return Err(deny("grant is not issued at the current logical tick"));
    }
    Ok(())
}

pub(super) fn module_call_caller(subject: &oasis7_wasm_abi::CapabilitySubject) -> ModuleCallCaller {
    match subject {
        oasis7_wasm_abi::CapabilitySubject::Agent { agent_id, .. } => ModuleCallCaller::Agent {
            agent_id: agent_id.clone(),
        },
        oasis7_wasm_abi::CapabilitySubject::Module { module_id, .. } => ModuleCallCaller::Module {
            module_id: module_id.clone(),
        },
        oasis7_wasm_abi::CapabilitySubject::System { system_id, .. } => ModuleCallCaller::System {
            system_id: system_id.clone(),
        },
    }
}

pub(super) fn validate_subject_for_manifest(
    subject: &oasis7_wasm_abi::CapabilitySubject,
    manifest: &oasis7_wasm_abi::ModuleManifest,
) -> Result<(), WorldError> {
    if let oasis7_wasm_abi::CapabilitySubject::Module {
        module_id,
        module_version,
        ..
    } = subject
        && (module_id != &manifest.module_id || module_version != &manifest.version)
    {
        return Err(deny(
            "module subject identity does not match the active module",
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct InvocationContextKey<'a> {
    grant_id: &'a str,
    response_nonce: &'a str,
}

#[derive(Serialize)]
struct AuthorizationNonceKey<'a> {
    subject: &'a oasis7_wasm_abi::CapabilitySubject,
    issuer_id: &'a str,
    issuer_key_epoch: u64,
    grant_id: &'a str,
    audience: &'a CapabilityAudience,
    branch_id: &'a str,
    finality_epoch: u64,
    nonce: &'a str,
}

pub(super) fn authorization_nonce_key(
    grant: &CapabilityGrantV2,
    response_nonce: &str,
) -> Result<String, WorldError> {
    if response_nonce.trim().is_empty() {
        return Err(deny("authorization response nonce is required"));
    }
    canonical_hash(&AuthorizationNonceKey {
        subject: &grant.subject,
        issuer_id: &grant.issuer.issuer_id,
        issuer_key_epoch: grant.issuer.issuer_key_epoch,
        grant_id: &grant.grant_id,
        audience: &grant.audience,
        branch_id: &grant.audience.branch_id,
        finality_epoch: grant.audience.finality_epoch,
        nonce: response_nonce,
    })
    .map_err(|error| deny(format!("authorization nonce key: {error}")))
}

pub(super) fn capability_invocation_context_key(
    context: &CapabilityInvocationContext,
) -> Result<String, WorldError> {
    capability_invocation_context_key_for_values(
        context.grant_id.as_str(),
        context.response_nonce.as_str(),
    )
}

pub(super) fn capability_invocation_context_key_for_values(
    grant_id: &str,
    response_nonce: &str,
) -> Result<String, WorldError> {
    if grant_id.trim().is_empty() || response_nonce.trim().is_empty() {
        return Err(deny(
            "invocation context grant id and response nonce are required",
        ));
    }
    canonical_hash(&InvocationContextKey {
        grant_id,
        response_nonce,
    })
    .map_err(|error| deny(format!("invocation context key: {error}")))
}

pub(super) fn audience_matches(grant: &CapabilityGrantV2, audience: &CapabilityAudience) -> bool {
    grant.audience == *audience
}

pub(super) fn scope_matches_command(
    grant: &CapabilityGrantV2,
    entry: &CapabilityCatalogEntry,
    payload_len: usize,
) -> bool {
    grant.scope.module_id == entry.module_id
        && grant.scope.module_version == entry.module_version
        && grant.scope.namespace == entry.namespace
        && grant.scope.object_kind == "command"
        && grant.scope.object_name == entry.command
        && grant.scope.operation == "execute"
        // The command/effect payload is opaque to this runtime.  A
        // selector-bearing grant is therefore not executable until a
        // schema-aware target binding is part of the ABI.
        && grant.scope.entity_selector.is_none()
        && grant.scope.resource_selector.is_none()
        && grant.scope.max_payload_bytes.is_some_and(|max| {
            u64::try_from(payload_len).is_ok_and(|payload_len| payload_len <= max)
        })
}
