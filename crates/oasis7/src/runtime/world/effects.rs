use super::super::util::sha256_hex;
use serde_json::Value as JsonValue;

use super::super::{
    CausedBy, EffectIntent, EffectOrigin, EffectReceipt, PolicyDecisionRecord, WorldError,
    WorldEventBody, WorldEventId,
};
use super::World;

impl World {
    // ---------------------------------------------------------------------
    // Effect handling
    // ---------------------------------------------------------------------

    pub fn take_next_effect(&mut self) -> Option<EffectIntent> {
        if self.inflight_effect_capacity_reached() {
            self.record_inflight_effect_dispatch_blocked();
            return None;
        }
        let intent = self.pending_effects.pop_front()?;
        self.inflight_effects
            .insert(intent.intent_id.clone(), intent.clone());
        Some(intent)
    }

    pub fn emit_effect(
        &mut self,
        kind: impl Into<String>,
        params: JsonValue,
        cap_ref: impl Into<String>,
        origin: EffectOrigin,
    ) -> Result<String, WorldError> {
        let intent = self.build_effect_intent(kind, params, cap_ref, origin)?;
        let intent_id = intent.intent_id.clone();
        self.append_event(WorldEventBody::EffectQueued(intent), None)?;
        Ok(intent_id)
    }

    pub(super) fn build_effect_intent(
        &mut self,
        kind: impl Into<String>,
        params: JsonValue,
        cap_ref: impl Into<String>,
        origin: EffectOrigin,
    ) -> Result<EffectIntent, WorldError> {
        let kind = kind.into();
        let cap_ref = cap_ref.into();
        let intent_id = format!("intent-{}", self.allocate_next_intent_seq());

        let intent = EffectIntent {
            intent_id: intent_id.clone(),
            kind: kind.clone(),
            params,
            cap_ref: cap_ref.clone(),
            origin,
        };

        let grant =
            self.capabilities
                .get(&cap_ref)
                .ok_or_else(|| WorldError::CapabilityMissing {
                    cap_ref: cap_ref.clone(),
                })?;

        if grant.is_expired(self.state.time) {
            return Err(WorldError::CapabilityExpired { cap_ref });
        }

        if !grant.allows(&kind) {
            return Err(WorldError::CapabilityNotAllowed { cap_ref, kind });
        }

        let decision = self.policies.decide(&intent);
        let record = PolicyDecisionRecord::from_intent(&intent, decision.clone());
        self.append_event(WorldEventBody::PolicyDecisionRecorded(record), None)?;

        if !decision.is_allowed() {
            return Err(WorldError::PolicyDenied {
                intent_id,
                reason: decision
                    .reason()
                    .unwrap_or_else(|| "policy_deny".to_string()),
            });
        }

        Ok(intent)
    }

    pub fn ingest_receipt(&mut self, receipt: EffectReceipt) -> Result<WorldEventId, WorldError> {
        // Receipt ingestion also closes the durable authorization link for a
        // trusted command effect.  Stage both the effect acknowledgement and
        // the audit update so a malformed receipt cannot leave half a commit.
        let mut staged = self.clone();
        let event_id = staged.ingest_receipt_staged(receipt)?;
        *self = staged;
        Ok(event_id)
    }

    fn ingest_receipt_staged(
        &mut self,
        mut receipt: EffectReceipt,
    ) -> Result<WorldEventId, WorldError> {
        let known = self.inflight_effects.contains_key(&receipt.intent_id)
            || self
                .pending_effects
                .iter()
                .any(|intent| intent.intent_id == receipt.intent_id);
        if !known {
            return Err(WorldError::ReceiptUnknownIntent {
                intent_id: receipt.intent_id,
            });
        }

        self.finalize_receipt_signature(&mut receipt)?;
        let event_id = self.append_event(
            WorldEventBody::ReceiptAppended(receipt.clone()),
            Some(CausedBy::Effect {
                intent_id: receipt.intent_id.clone(),
            }),
        )?;

        if let Some(link) = self
            .capability_effect_receipt_links
            .remove(&receipt.intent_id)
        {
            let audit = self
                .capability_authorization_receipts
                .get_mut(&link.authorization_receipt_id)
                .ok_or_else(|| WorldError::CapabilityAuthorizationDenied {
                    reason: "effect receipt authorization link has no audit receipt".to_string(),
                })?;
            if audit.committed_effect_receipt_id.is_none() {
                // `intent_id` is the stable identifier of the actual
                // EffectReceipt in the existing effect protocol.
                audit.committed_effect_receipt_id = Some(receipt.intent_id);
            }
            self.refresh_capability_authorization_root()?;
        }
        Ok(event_id)
    }

    fn finalize_receipt_signature(&self, receipt: &mut EffectReceipt) -> Result<(), WorldError> {
        let Some(signer) = &self.receipt_signer else {
            return Ok(());
        };
        let consensus_height = self.next_receipt_consensus_height();
        let receipts_root = self.compute_next_receipts_root(receipt, consensus_height)?;

        if let Some(signature) = &receipt.signature {
            signer.verify(
                receipt,
                signature,
                &self.state.node_identity_bindings,
                consensus_height,
                receipts_root.as_str(),
            )?;
        } else {
            let signature = signer.sign(receipt, consensus_height, receipts_root.as_str())?;
            receipt.signature = Some(signature);
        }

        Ok(())
    }

    fn next_receipt_consensus_height(&self) -> u64 {
        self.journal.events.len() as u64 + 1
    }

    fn compute_next_receipts_root(
        &self,
        receipt: &EffectReceipt,
        consensus_height: u64,
    ) -> Result<String, WorldError> {
        let mut root = "0".repeat(64);
        for (idx, event) in self.journal.events.iter().enumerate() {
            let WorldEventBody::ReceiptAppended(existing_receipt) = &event.body else {
                continue;
            };
            let leaf_hash = receipt_leaf_hash(existing_receipt)?;
            root = advance_receipts_root(root.as_str(), idx as u64 + 1, leaf_hash.as_str());
        }
        let next_leaf_hash = receipt_leaf_hash(receipt)?;
        Ok(advance_receipts_root(
            root.as_str(),
            consensus_height,
            next_leaf_hash.as_str(),
        ))
    }
}

fn advance_receipts_root(previous_root: &str, consensus_height: u64, leaf_hash: &str) -> String {
    let payload = format!("receipts-root:v1|{previous_root}|{consensus_height}|{leaf_hash}");
    sha256_hex(payload.as_bytes())
}

fn receipt_leaf_hash(receipt: &EffectReceipt) -> Result<String, WorldError> {
    #[derive(serde::Serialize)]
    struct ReceiptLeaf<'a> {
        intent_id: &'a str,
        status: &'a str,
        payload: &'a JsonValue,
        cost_cents: Option<u64>,
    }

    let leaf = ReceiptLeaf {
        intent_id: &receipt.intent_id,
        status: &receipt.status,
        payload: &receipt.payload,
        cost_cents: receipt.cost_cents,
    };
    let bytes = serde_json::to_vec(&leaf)?;
    Ok(sha256_hex(bytes.as_slice()))
}
