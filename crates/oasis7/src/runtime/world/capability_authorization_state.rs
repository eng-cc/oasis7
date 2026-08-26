//! Durable state accounting for the v2 trusted authorization lane.
//!
//! This module owns the budget transitions and deterministic authorization
//! root.  The command executor remains in `capability_authorization`; these
//! helpers are kept separate so persistence and accounting changes have one
//! focused implementation surface.

use oasis7_wasm_abi::canonical_hash;
use serde::Serialize;

use super::super::WorldError;
use super::super::capability_authorization::{
    CapabilityAuthorizationAuditReceipt, CapabilityAuthorizationNonceRecord,
    CapabilityBudgetAccount, CapabilityEffectReceiptLink, CapabilityInvocationContext,
    CapabilityRevocationState,
};
use super::World;
use super::capability_authorization::deny;

impl World {
    pub(super) fn reserve_capability_budget(
        &mut self,
        key: &str,
        reservation_units: i64,
    ) -> Result<(), WorldError> {
        let account = self
            .capability_budget_accounts
            .get_mut(key)
            .ok_or_else(|| deny("capability budget account is not available"))?;
        if reservation_units < 0 || account.remaining_units < reservation_units {
            return Err(deny("capability budget is insufficient before sandbox"));
        }
        account.remaining_units -= reservation_units;
        account.reserved_units = account
            .reserved_units
            .checked_add(reservation_units)
            .ok_or_else(|| deny("capability budget reservation overflow"))?;
        Ok(())
    }

    pub(super) fn settle_capability_budget(
        &mut self,
        key: &str,
        reservation_units: i64,
        actual_units: i64,
    ) -> Result<i64, WorldError> {
        if actual_units < 0 || actual_units > reservation_units {
            return Err(deny("capability budget actual cost exceeds reservation"));
        }
        let account = self
            .capability_budget_accounts
            .get_mut(key)
            .ok_or_else(|| deny("capability budget account is not available"))?;
        account.reserved_units = account
            .reserved_units
            .checked_sub(reservation_units)
            .ok_or_else(|| deny("capability budget reservation underflow"))?;
        account.remaining_units = account
            .remaining_units
            .checked_add(reservation_units - actual_units)
            .ok_or_else(|| deny("capability budget release overflow"))?;
        account.spent_units = account
            .spent_units
            .checked_add(actual_units)
            .ok_or_else(|| deny("capability budget spend overflow"))?;
        Ok(account.remaining_units)
    }

    pub(super) fn refresh_capability_authorization_root(&mut self) -> Result<(), WorldError> {
        self.capability_authorization_root = self.compute_capability_authorization_root()?;
        Ok(())
    }

    pub(super) fn verify_capability_authorization_root(&self) -> Result<(), WorldError> {
        let expected = self.compute_capability_authorization_root()?;
        if self.capability_authorization_root != expected {
            return Err(deny(
                "capability authorization root does not match durable state",
            ));
        }
        Ok(())
    }

    fn compute_capability_authorization_root(&self) -> Result<String, WorldError> {
        canonical_hash(&CapabilityAuthorizationRootBody {
            grants: &self.capability_grants_v2,
            revocation: &self.capability_revocation_state,
            invocation_contexts: &self.capability_invocation_contexts,
            budget_accounts: &self.capability_budget_accounts,
            nonce_records: &self.capability_nonce_records,
            receipts: &self.capability_authorization_receipts,
            effect_receipt_links: &self.capability_effect_receipt_links,
        })
        .map_err(|error| deny(format!("authorization root: {error}")))
    }
}

#[derive(Serialize)]
struct CapabilityAuthorizationRootBody<'a> {
    grants: &'a std::collections::BTreeMap<String, serde_json::Value>,
    revocation: &'a CapabilityRevocationState,
    invocation_contexts: &'a std::collections::BTreeMap<String, CapabilityInvocationContext>,
    budget_accounts: &'a std::collections::BTreeMap<String, CapabilityBudgetAccount>,
    nonce_records: &'a std::collections::BTreeMap<String, CapabilityAuthorizationNonceRecord>,
    receipts: &'a std::collections::BTreeMap<String, CapabilityAuthorizationAuditReceipt>,
    effect_receipt_links: &'a std::collections::BTreeMap<String, CapabilityEffectReceiptLink>,
}

#[derive(Serialize)]
struct CapabilityBudgetKey<'a> {
    subject: &'a oasis7_wasm_abi::CapabilitySubject,
    grant_id: &'a str,
}

pub(super) fn capability_budget_key(
    subject: &oasis7_wasm_abi::CapabilitySubject,
    grant_id: &str,
) -> Result<String, WorldError> {
    if grant_id.trim().is_empty() {
        return Err(deny("capability budget grant id is required"));
    }
    canonical_hash(&CapabilityBudgetKey { subject, grant_id })
        .map_err(|error| deny(format!("capability budget key: {error}")))
}

pub(super) fn validate_budget_account(account: &CapabilityBudgetAccount) -> Result<(), WorldError> {
    if account.grant_id.trim().is_empty() {
        return Err(deny("capability budget grant id is required"));
    }
    account
        .subject
        .validate()
        .map_err(|error| deny(format!("capability budget subject: {error}")))?;
    if account.remaining_units < 0 || account.reserved_units < 0 || account.spent_units < 0 {
        return Err(deny("capability budget values must be non-negative"));
    }
    Ok(())
}

fn budget_units_for_bytes(bytes: u64) -> Result<i64, WorldError> {
    if bytes == 0 {
        return Ok(0);
    }
    let rounded = bytes
        .checked_add(1_023)
        .ok_or_else(|| deny("capability budget byte cost overflow"))?
        / 1_024;
    i64::try_from(rounded).map_err(|_| deny("capability budget byte cost is too large"))
}

pub(super) fn budget_reservation_units(
    payload_len: usize,
    limits: &oasis7_wasm_abi::ModuleLimits,
) -> Result<i64, WorldError> {
    let payload_units = budget_units_for_bytes(
        u64::try_from(payload_len).map_err(|_| deny("capability payload length is too large"))?,
    )?;
    let output_units = budget_units_for_bytes(limits.max_output_bytes)?;
    let effects = i64::from(limits.max_effects)
        .checked_mul(2)
        .ok_or_else(|| deny("capability budget effect cost overflow"))?;
    let emits = i64::from(limits.max_emits);
    payload_units
        .checked_add(output_units)
        .and_then(|value| value.checked_add(effects))
        .and_then(|value| value.checked_add(emits))
        .ok_or_else(|| deny("capability budget reservation overflow"))
}

pub(super) fn capability_actual_units(
    payload_len: usize,
    output: &oasis7_wasm_abi::ModuleOutput,
) -> Result<i64, WorldError> {
    let payload_units = budget_units_for_bytes(
        u64::try_from(payload_len).map_err(|_| deny("capability payload length is too large"))?,
    )?;
    let output_units = budget_units_for_bytes(output.output_bytes)?;
    let effects = i64::try_from(output.effects.len())
        .map_err(|_| deny("capability budget effect count is too large"))?
        .checked_mul(2)
        .ok_or_else(|| deny("capability budget effect cost overflow"))?;
    let emits = i64::try_from(output.emits.len())
        .map_err(|_| deny("capability budget emit count is too large"))?;
    payload_units
        .checked_add(output_units)
        .and_then(|value| value.checked_add(effects))
        .and_then(|value| value.checked_add(emits))
        .ok_or_else(|| deny("capability budget actual cost overflow"))
}
