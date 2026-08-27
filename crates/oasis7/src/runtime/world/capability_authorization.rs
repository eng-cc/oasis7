//! The v2 trusted module-command boundary.
//!
//! Provider values are candidates only.  Every value is validated against the
//! live module registry and the governed issuer/revocation view before a
//! cloned world is allowed to call the sandbox.  The clone is published only
//! after the call and its output have passed all checks.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use oasis7_wasm_abi::{
    AgentCommandResponse, CapabilityCatalogSnapshot, CapabilityGrantV2, ModuleCallInput,
    ModuleCallOrigin, ModuleKind, ModuleSandbox, canonical_hash, capability_scope_hash,
    validate_module_command_declarations, validate_module_command_envelope,
};
use std::collections::{BTreeMap, BTreeSet};

use super::super::capability_authorization::{
    CapabilityAgentIdentity, CapabilityAuthorityRecord, CapabilityAuthorizationAuditReceipt,
    CapabilityAuthorizationNonceRecord, CapabilityBudgetAccount, CapabilityEffectReceiptLink,
    CapabilityInvocationContext,
};
use super::super::{
    CapabilityAuthorizationEvent, EffectIntent, EffectOrigin, PolicyDecisionRecord, WorldError,
    WorldEventBody,
};
use super::World;
use super::capability_authorization_state::{
    budget_reservation_units, capability_actual_units, capability_budget_key,
    validate_budget_account,
};

impl World {
    /// Legacy compatibility shim.  Authority cannot be installed from a
    /// process-local key/epoch pair; callers must provide finalized
    /// governance evidence through [`Self::install_capability_authority_record`].
    #[deprecated(note = "install a finalized CapabilityAuthorityRecord")]
    pub fn set_capability_trusted_issuer(
        &mut self,
        _issuer_id: &str,
        _public_key_hex: &str,
        _key_epoch: u64,
    ) -> Result<(), WorldError> {
        Err(deny(
            "direct issuer key registration is forbidden; install finalized authority evidence",
        ))
    }

    #[deprecated(note = "install a finalized CapabilityAuthorityRecord")]
    pub fn revoke_capability_grant_v2(&mut self, grant_id: &str) -> Result<(), WorldError> {
        let _ = grant_id;
        Err(deny(
            "direct grant revocation is forbidden; install finalized authority evidence",
        ))
    }

    #[deprecated(note = "install a finalized CapabilityAuthorityRecord")]
    pub fn set_capability_revocation_epoch(
        &mut self,
        _epoch: u64,
        _finalized_receipt_id: impl Into<String>,
    ) -> Result<(), WorldError> {
        Err(deny(
            "direct revocation epoch mutation is forbidden; install finalized authority evidence",
        ))
    }

    /// Reject the legacy unproven authority import path.
    ///
    /// Authority records are trust roots, so accepting a structurally valid
    /// record from a public API would let a caller choose its own issuer key.
    /// Use [`Self::install_capability_authority_record_with_finality`] with a
    /// certificate verified against the live governance signer set instead.
    #[deprecated(note = "install a CapabilityAuthorityRecord with finality proof")]
    pub fn install_capability_authority_record(
        &mut self,
        _record: CapabilityAuthorityRecord,
    ) -> Result<(), WorldError> {
        Err(deny(
            "authority installation requires a verified governance finality certificate",
        ))
    }

    /// Bind the invocation identity supplied by the trusted host.  This is
    /// persisted and immutable per grant/response nonce, so provider DTOs
    /// cannot choose a subject, presenter, or audience at execution time.
    pub fn install_capability_invocation_context(
        &mut self,
        context: CapabilityInvocationContext,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        validate_invocation_context(&context)?;
        let key =
            super::capability_authorization_events::capability_invocation_context_key(&context)?;
        if let Some(existing) = self.capability_invocation_contexts.get(&key)
            && existing != &context
        {
            return Err(deny("invocation context is immutable"));
        }
        if self.capability_invocation_contexts.get(&key) == Some(&context) {
            return Ok(());
        }
        if let oasis7_wasm_abi::CapabilitySubject::System { system_id, epoch } = &context.subject
            && self
                .capability_revocation_state
                .system_identities
                .get(system_id)
                != Some(epoch)
        {
            self.append_event(
                WorldEventBody::CapabilityAuthorization(
                    CapabilityAuthorizationEvent::SystemIdentityInstalled {
                        system_id: system_id.clone(),
                        epoch: *epoch,
                    },
                ),
                None,
            )?;
        }
        self.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::InvocationContextInstalled { key, context },
            ),
            None,
        )?;
        Ok(())
    }

    /// Install the durable logical budget for a subject/grant pair.  The
    /// account is immutable once consumed; governance may install a new grant
    /// and account rather than mutating historical spend.
    pub fn install_capability_budget_account(
        &mut self,
        account: CapabilityBudgetAccount,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        validate_budget_account(&account)?;
        let key = capability_budget_key(&account.subject, &account.grant_id)?;
        if let Some(existing) = self.capability_budget_accounts.get(&key)
            && existing != &account
        {
            return Err(deny("capability budget account is immutable"));
        }
        if self.capability_budget_accounts.get(&key) == Some(&account) {
            return Ok(());
        }
        self.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::BudgetAccountInstalled { key, account },
            ),
            None,
        )?;
        Ok(())
    }

    /// Insert an issuer-authenticated immutable grant into the durable view.
    /// Normal callers should use the executor, which performs this admission
    /// as part of its staged transaction; this method is provided for loading
    /// a governed grant before its first command.
    pub fn register_capability_grant_v2(
        &mut self,
        grant: CapabilityGrantV2,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
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
        if grant.expires_at_tick.is_none() {
            return Err(deny("grant must carry an explicit expiry"));
        }
        if grant.status != "verified" {
            return Err(deny(
                "only finalized and verified grants may enter durable authorization state",
            ));
        }
        if grant.issued_at_tick > self.state.time {
            return Err(deny("grant is not issued at the current logical tick"));
        }
        self.verify_issuer(&grant)?;
        self.verify_live_revocation(&grant)?;
        self.verify_parent_chain(&grant)?;
        if let oasis7_wasm_abi::CapabilitySubject::System { system_id, .. } = &grant.subject {
            let manifest = self
                .active_module_manifest(grant.scope.module_id.as_str())
                .map_err(|error| deny(format!("system subject module is not live: {error:?}")))?;
            let expected_system_id = grant
                .scope
                .module_id
                .strip_prefix("module.")
                .map(|suffix| format!("system-{suffix}"));
            if expected_system_id.as_deref() != Some(system_id.as_str())
                || manifest.version != grant.scope.module_version
            {
                return Err(deny("system subject is not bound to a live module system"));
            }
        }
        let encoded = serde_json::to_value(&grant)?;
        if let Some(existing) = self.capability_grants_v2.get(&grant.grant_id)
            && existing != &encoded
        {
            return Err(deny("immutable grant body changed"));
        }
        self.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::GrantRegistered { grant },
            ),
            None,
        )?;
        Ok(())
    }

    /// Validate one manifest capability against the live admission state.
    ///
    /// Legacy capabilities remain available for legacy module manifests.  A
    /// capability reference that is also present in the durable v2 registry is
    /// deliberately routed through the v2 checks instead of falling back to a
    /// legacy `allow_all` grant.  This keeps module activation aligned with
    /// the trusted executor's authority boundary.
    pub(super) fn validate_module_required_capability(
        &self,
        cap_ref: &str,
    ) -> Result<(), WorldError> {
        if self.capability_grants_v2.contains_key(cap_ref) {
            return self.validate_registered_v2_capability_for_admission(cap_ref);
        }

        let grant =
            self.capabilities
                .get(cap_ref)
                .ok_or_else(|| WorldError::ModuleChangeInvalid {
                    reason: format!("module cap missing {cap_ref}"),
                })?;
        if grant.is_expired(self.state.time) {
            return Err(WorldError::ModuleChangeInvalid {
                reason: format!("module cap expired {cap_ref}"),
            });
        }
        Ok(())
    }

    /// Check the durable v2 grant used by a module manifest during register or
    /// activation.  The manifest only establishes that the grant is a module
    /// dependency; command/effect scope is checked again at execution time.
    pub(super) fn validate_registered_v2_capability_for_admission(
        &self,
        cap_ref: &str,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        let encoded = self
            .capability_grants_v2
            .get(cap_ref)
            .ok_or_else(|| deny("v2 capability is not in the durable registry"))?;
        let grant: CapabilityGrantV2 = serde_json::from_value(encoded.clone())
            .map_err(|_| deny("v2 capability grant is malformed"))?;
        grant
            .validate()
            .map_err(|error| deny(format!("v2 capability grant validation: {error}")))?;
        if grant.grant_id != cap_ref
            || !grant
                .body_hash_matches()
                .map_err(|error| deny(format!("v2 capability body hash: {error}")))?
            || grant
                .expected_grant_id()
                .map_err(|error| deny(format!("v2 capability id hash: {error}")))?
                != grant.grant_id
        {
            return Err(deny("v2 capability canonical body hash or id mismatch"));
        }
        if grant.status != "verified" {
            return Err(deny("v2 capability is not finalized and verified"));
        }
        if grant.expires_at_tick.is_none()
            || grant
                .expires_at_tick
                .is_some_and(|expiry| self.state.time > expiry)
            || grant.issued_at_tick > self.state.time
        {
            return Err(deny("v2 capability lifetime is not currently valid"));
        }
        self.verify_issuer(&grant)?;
        self.verify_live_revocation(&grant)?;
        self.verify_parent_chain(&grant)
    }

    /// Execute one authenticated v2 command.  The executor argument is kept
    /// generic so the runtime remains buildable on wasm targets where the
    /// native executor dependency is unavailable; sandbox is the actual call
    /// boundary, as in the existing module runtime.
    pub fn execute_trusted_module_command<E>(
        &mut self,
        grant: CapabilityGrantV2,
        catalog: CapabilityCatalogSnapshot,
        response: AgentCommandResponse,
        _executor: &mut E,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<CapabilityAuthorizationAuditReceipt, WorldError> {
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
        self.verify_capability_authorization_root()?;
        catalog
            .validate()
            .map_err(|error| deny(format!("catalog validation: {error}")))?;
        response
            .validate()
            .map_err(|error| deny(format!("response validation: {error}")))?;
        let catalog_hash = catalog
            .canonical_hash()
            .map_err(|error| deny(format!("catalog hash: {error}")))?;
        if catalog.snapshot_id != catalog_hash {
            return Err(deny("catalog snapshot id is not its canonical hash"));
        }
        if !response.matches_catalog(&catalog) {
            return Err(deny("response does not match catalog snapshot"));
        }
        self.verify_invocation_context(&grant, &catalog, &response)?;
        if grant.issued_at_tick > self.state.time {
            return Err(deny("grant is not issued at the current logical tick"));
        }
        let request_hash = response
            .canonical_request_hash()
            .map_err(|error| deny(format!("request hash: {error}")))?;
        let nonce_key_hash = super::capability_authorization_events::authorization_nonce_key(
            &grant,
            &response.response_nonce,
        )?;

        if let Some(record) = self.capability_nonce_records.get(&nonce_key_hash) {
            if record.request_hash != request_hash {
                return Err(WorldError::CapabilityNonceConflict {
                    nonce_key_hash,
                    committed_request_hash: record.request_hash.clone(),
                    supplied_request_hash: request_hash,
                });
            }
            if let Some(receipt_id) = &record.committed_receipt_id
                && let Some(receipt) = self.capability_authorization_receipts.get(receipt_id)
            {
                let mut idempotent = receipt.clone();
                idempotent.decision = "idempotent".to_string();
                return Ok(idempotent);
            }
            return Err(deny("committed nonce has no durable receipt"));
        }

        if grant.status != "verified" {
            return Err(deny("grant is not finalized and verified"));
        }
        if grant.expires_at_tick.is_none() {
            return Err(deny("grant lifetime is not currently valid"));
        }
        if grant
            .expires_at_tick
            .is_some_and(|expiry| self.state.time > expiry)
        {
            return Err(deny("grant expired"));
        }
        if !super::capability_authorization_events::audience_matches(&grant, &catalog.audience)
            || grant.audience != response.audience
            || catalog.audience != response.audience
            || catalog.subject != response.subject
            || grant.subject != response.subject
            || catalog.presenter != response.presenter
        {
            return Err(deny("subject, presenter, or audience mismatch"));
        }
        if response.provider_id.as_deref() != Some(response.presenter.presenter_id.as_str()) {
            return Err(deny("provider identity is not the presented presenter"));
        }
        self.verify_live_capability_audience(&grant, &catalog, &response)?;

        self.verify_issuer(&grant)?;
        self.verify_live_revocation(&grant)?;
        self.verify_parent_chain(&grant)?;

        let manifest = self
            .active_module_manifest(response.selected_entry.module_id.as_str())
            .map_err(|error| deny(format!("active module: {error:?}")))?
            .clone();
        if manifest.version != response.selected_entry.module_version
            || manifest.module_id != grant.scope.module_id
            || manifest.version != grant.scope.module_version
        {
            return Err(deny("module identity or active version mismatch"));
        }
        validate_module_command_declarations(&manifest.abi_contract.declarations)
            .map_err(|error| deny(format!("module declaration: {error}")))?;
        validate_module_command_envelope(&response.envelope, &manifest.abi_contract.declarations)
            .map_err(|error| deny(format!("module envelope: {error}")))?;
        let entry = &response.selected_entry;
        if entry.module_id != manifest.module_id
            || entry.module_version != manifest.version
            || entry.namespace != response.envelope.namespace
            || entry.command != response.envelope.name
            || entry.schema_version != response.envelope.schema_version
            || entry.schema_hash != response.envelope.schema_hash
        {
            return Err(deny("selected declaration or envelope mismatch"));
        }
        let declaration = manifest
            .abi_contract
            .declarations
            .commands
            .iter()
            .find(|decl| {
                decl.namespace == entry.namespace
                    && decl.name == entry.command
                    && decl.schema_version == entry.schema_version
                    && decl.schema_hash == entry.schema_hash
            })
            .ok_or_else(|| deny("selected declaration is not active"))?;
        if entry.max_payload_bytes != declaration.max_payload_bytes {
            return Err(deny(
                "catalog payload bound does not match active declaration",
            ));
        }
        super::capability_authorization_events::validate_subject_for_manifest(
            self,
            &grant.subject,
            &manifest,
        )?;
        if !super::capability_authorization_events::scope_matches_command(
            &grant,
            entry,
            response.envelope.payload.as_slice(),
        ) {
            return Err(deny("grant scope does not exactly authorize command"));
        }
        let catalog_entry = catalog
            .entries
            .iter()
            .find(|candidate| {
                candidate.module_id == entry.module_id
                    && candidate.module_version == entry.module_version
                    && candidate.namespace == entry.namespace
                    && candidate.command == entry.command
                    && candidate.schema_version == entry.schema_version
                    && candidate.schema_hash == entry.schema_hash
                    && candidate.max_payload_bytes == entry.max_payload_bytes
            })
            .ok_or_else(|| deny("selected catalog entry is not present in the live catalog"))?;
        if catalog_entry.eligible_grant_ids.is_empty()
            || !catalog_entry
                .eligible_grant_ids
                .iter()
                .any(|id| id == &grant.grant_id)
        {
            return Err(deny("grant is not eligible for selected catalog entry"));
        }
        self.verify_catalog_freshness(&catalog, &manifest)?;

        let encoded_grant = serde_json::to_value(&grant)?;
        if let Some(existing) = self.capability_grants_v2.get(&grant.grant_id)
            && existing != &encoded_grant
        {
            return Err(deny("immutable grant body changed"));
        }

        let state_hash_before =
            canonical_hash(&self.state).map_err(|error| deny(format!("state hash: {error}")))?;
        let world_head_before = self
            .journal
            .events
            .last()
            .map(|event| event.id)
            .unwrap_or(0);
        let budget_key = capability_budget_key(&grant.subject, &grant.grant_id)?;
        let budget_before = self
            .capability_budget_accounts
            .get(&budget_key)
            .ok_or_else(|| deny("capability budget account is not available"))?
            .remaining_units;
        let budget_before_spent = self
            .capability_budget_accounts
            .get(&budget_key)
            .ok_or_else(|| deny("capability budget account is not available"))?
            .spent_units;
        let reservation_units =
            budget_reservation_units(response.envelope.payload.len(), &manifest.limits)?;
        if budget_before < reservation_units {
            return Err(deny("capability budget is insufficient before sandbox"));
        }
        let mut staged = self.clone();
        staged.reserve_capability_budget(&budget_key, reservation_units)?;
        staged.refresh_capability_authorization_root()?;
        let output =
            staged.execute_trusted_module_sandbox(&manifest, &response, &grant.subject, sandbox)?;
        let effect_intent_ids: Vec<String> = staged
            .journal
            .events
            .iter()
            .filter(|event| event.id > world_head_before)
            .filter_map(|event| match &event.body {
                WorldEventBody::EffectQueued(intent) => Some(intent.intent_id.clone()),
                _ => None,
            })
            .collect();
        let budget_after = staged.settle_capability_budget(
            &budget_key,
            reservation_units,
            capability_actual_units(response.envelope.payload.len(), &output)?,
        )?;
        staged.refresh_capability_authorization_root()?;
        staged.verify_live_authorization_before_commit(
            &grant,
            &catalog,
            &response,
            &manifest,
            world_head_before,
        )?;
        let state_hash_after = canonical_hash(&staged.state)
            .map_err(|error| deny(format!("staged state hash: {error}")))?;
        let result_hash =
            canonical_hash(&output).map_err(|error| deny(format!("result hash: {error}")))?;
        let receipt_id = format!("capability-authz-{request_hash}");
        let finality_block_hash = staged
            .capability_revocation_state
            .authority_records
            .get(&grant.issuer.issuer_id)
            .map(|record| record.finality_block_hash.clone());
        let receipt = CapabilityAuthorizationAuditReceipt {
            receipt_id: receipt_id.clone(),
            root_receipt_id: None,
            grant_id: Some(grant.grant_id.clone()),
            subject: serde_json::to_value(&grant.subject)?,
            presenter: Some(serde_json::to_value(&response.presenter)?),
            audience: serde_json::to_value(&grant.audience)?,
            scope_hash: capability_scope_hash(&grant.scope)
                .map_err(|error| deny(format!("scope hash: {error}")))?,
            module_id: Some(manifest.module_id.clone()),
            module_version: Some(manifest.version.clone()),
            manifest_hash: Some(
                canonical_hash(&manifest)
                    .map_err(|error| deny(format!("manifest hash: {error}")))?,
            ),
            catalog_snapshot_id: Some(catalog.snapshot_id.clone()),
            response_nonce: Some(response.response_nonce.clone()),
            authorization_nonce_key_hash: Some(nonce_key_hash.clone()),
            decision: "accepted".to_string(),
            denial_code: None,
            budget_before,
            budget_after: Some(budget_after),
            world_head_before,
            // `CommandCommitted` is the next journal event.  Capture its
            // allocated id before append_event applies and journals it so the
            // audit receipt's after-head includes the authorization commit,
            // not merely the sandbox/output events that precede it.
            world_head_after: Some(staged.next_event_id),
            branch_id: grant.audience.branch_id.clone(),
            finality_epoch: grant.audience.finality_epoch,
            finality_block_hash,
            finality_status: "verified".to_string(),
            state_hash_before,
            state_hash_after: Some(state_hash_after),
            committed_effect_receipt_id: None,
            committed_effect_receipt_ids: BTreeSet::new(),
            canonical_request_hash: request_hash.clone(),
            canonical_result_hash: result_hash.clone(),
        };
        let nonce_record = CapabilityAuthorizationNonceRecord {
            request_hash,
            outcome_hash: result_hash,
            committed_receipt_id: Some(receipt_id.clone()),
            state: "committed".to_string(),
        };
        let budget_account = staged
            .capability_budget_accounts
            .get(&budget_key)
            .cloned()
            .ok_or_else(|| deny("capability budget account disappeared before commit"))?;
        let mut effect_receipt_links = BTreeMap::new();
        for intent_id in effect_intent_ids {
            effect_receipt_links.insert(
                intent_id,
                CapabilityEffectReceiptLink {
                    authorization_receipt_id: receipt_id.clone(),
                },
            );
        }
        staged.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::CommandCommitted {
                    budget_key,
                    budget_before_remaining_units: budget_before,
                    budget_before_spent_units: budget_before_spent,
                    state_hash_before: receipt.state_hash_before.clone(),
                    receipt_hash:
                        super::capability_authorization_events::authorization_receipt_hash(
                            &receipt,
                        )?,
                    budget_account,
                    grant,
                    nonce_key: nonce_key_hash,
                    nonce_record,
                    receipt: receipt.clone(),
                    effect_receipt_links,
                },
            ),
            None,
        )?;
        *self = staged;
        Ok(receipt)
    }

    pub(super) fn verify_issuer(&self, grant: &CapabilityGrantV2) -> Result<(), WorldError> {
        let issuer = &grant.issuer;
        let authority = self
            .capability_revocation_state
            .authority_records
            .get(&issuer.issuer_id)
            .ok_or_else(|| deny("issuer has no finalized authority record"))?;
        validate_authority_record(authority)?;
        let proof = self
            .capability_revocation_state
            .authority_finality_proofs
            .get(&issuer.issuer_id)
            .ok_or_else(|| deny("issuer has no replayable finality proof"))?;
        self.verify_capability_authority_finality_proof(authority, proof)?;
        if issuer.issuer_kind != authority.issuer_kind
            || issuer.governance_epoch != authority.governance_epoch
            || issuer.finalized_receipt_id != authority.finalized_receipt_id
            || issuer.key_id != authority.key_id
            || issuer.issuer_key_epoch != authority.issuer_key_epoch
            || issuer.authority_rotation_receipt_id != authority.authority_rotation_receipt_id
            || grant.audience.world_id != authority.world_id
            || grant.audience.branch_id != authority.branch_id
            || grant.audience.finality_epoch != authority.finality_epoch
        {
            return Err(deny("grant issuer or governance finality binding mismatch"));
        }
        let key_bytes = hex::decode(authority.public_key_hex.trim())
            .map_err(|_| deny("issuer public key is invalid"))?;
        let key_bytes: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| deny("issuer public key length is invalid"))?;
        let key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| deny("issuer public key is invalid"))?;
        for encoded in [&issuer.signature, &grant.issuance_signature] {
            let signature_hex = encoded
                .strip_prefix("ed25519:")
                .ok_or_else(|| deny("issuer signature algorithm is not ed25519"))?;
            let bytes =
                hex::decode(signature_hex).map_err(|_| deny("issuer signature is invalid hex"))?;
            let bytes: [u8; 64] = bytes
                .try_into()
                .map_err(|_| deny("issuer signature length is invalid"))?;
            key.verify(
                grant
                    .canonical_body_bytes()
                    .map_err(|error| deny(format!("canonical grant body: {error}")))?
                    .as_slice(),
                &Signature::from_bytes(&bytes),
            )
            .map_err(|_| deny("issuer signature verification failed"))?;
        }
        Ok(())
    }

    fn verify_invocation_context(
        &self,
        grant: &CapabilityGrantV2,
        catalog: &CapabilityCatalogSnapshot,
        response: &AgentCommandResponse,
    ) -> Result<(), WorldError> {
        let key =
            super::capability_authorization_events::capability_invocation_context_key_for_values(
                grant.grant_id.as_str(),
                response.response_nonce.as_str(),
            )?;
        let context = self
            .capability_invocation_contexts
            .get(&key)
            // Snapshots written before the nonce-key migration used the grant
            // id alone.  Keep those records readable, but only when their
            // complete bound nonce still matches this response.
            .or_else(|| self.capability_invocation_contexts.get(&grant.grant_id))
            .ok_or_else(|| deny("trusted host invocation context is not bound"))?;
        if context.grant_id != grant.grant_id
            || context.subject != grant.subject
            || context.subject != catalog.subject
            || context.subject != response.subject
            || context.presenter != catalog.presenter
            || context.presenter != response.presenter
            || context.audience != grant.audience
            || context.audience != catalog.audience
            || context.audience != response.audience
            || context.catalog_snapshot_id != catalog.snapshot_id
            || context.module_id != response.selected_entry.module_id
            || context.module_version != response.selected_entry.module_version
            || context.response_nonce != response.response_nonce
        {
            return Err(deny("invocation fields do not match trusted host context"));
        }
        Ok(())
    }

    fn verify_live_authorization_before_commit(
        &self,
        grant: &CapabilityGrantV2,
        catalog: &CapabilityCatalogSnapshot,
        response: &AgentCommandResponse,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        world_head_before: u64,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        if self
            .journal
            .events
            .last()
            .map(|event| event.id)
            .unwrap_or(0)
            < world_head_before
        {
            return Err(deny("staged world head moved backwards"));
        }
        if self.state.time > catalog.valid_until_tick {
            return Err(deny("catalog expired before commit"));
        }
        let catalog_hash = catalog
            .canonical_hash()
            .map_err(|error| deny(format!("catalog hash: {error}")))?;
        if catalog.snapshot_id != catalog_hash || !response.matches_catalog(catalog) {
            return Err(deny("catalog or response changed before commit"));
        }
        let registry_hash = canonical_hash(&self.module_registry)
            .map_err(|error| deny(format!("module registry hash: {error}")))?;
        let policy_hash = canonical_hash(&self.policies)
            .map_err(|error| deny(format!("policy hash: {error}")))?;
        if catalog.module_registry_hash != registry_hash
            || catalog.policy_hash != policy_hash
            || catalog.revocation_epoch != self.capability_revocation_state.epoch
        {
            return Err(deny("live authorization changed before commit"));
        }
        self.verify_live_capability_audience(grant, catalog, response)?;
        if manifest.module_id != response.selected_entry.module_id
            || manifest.version != response.selected_entry.module_version
        {
            return Err(deny("active module changed before commit"));
        }
        let active_manifest = self
            .active_module_manifest(manifest.module_id.as_str())
            .map_err(|error| deny(format!("active module changed before commit: {error:?}")))?;
        if active_manifest != manifest {
            return Err(deny("active module manifest changed before commit"));
        }
        if grant.status != "verified" {
            return Err(deny("grant status changed before commit"));
        }
        self.verify_invocation_context(grant, catalog, response)?;
        self.verify_issuer(grant)?;
        self.verify_live_revocation(grant)?;
        self.verify_parent_chain(grant)
    }

    pub(super) fn verify_live_revocation(
        &self,
        grant: &CapabilityGrantV2,
    ) -> Result<(), WorldError> {
        let state = &self.capability_revocation_state;
        if state.epoch < grant.revocation_epoch {
            return Err(deny("revocation registry is stale"));
        }
        if state.revoked_grant_ids.contains(&grant.grant_id)
            || state.superseded_by.contains_key(&grant.grant_id)
        {
            return Err(deny("grant is revoked or superseded"));
        }
        Ok(())
    }

    fn verify_catalog_freshness(
        &self,
        catalog: &CapabilityCatalogSnapshot,
        manifest: &oasis7_wasm_abi::ModuleManifest,
    ) -> Result<(), WorldError> {
        let head = self
            .journal
            .events
            .last()
            .map(|event| event.id)
            .unwrap_or(0);
        if catalog.world_head != head || self.state.time > catalog.valid_until_tick {
            return Err(deny("catalog is stale"));
        }
        let registry_hash = canonical_hash(&self.module_registry)
            .map_err(|error| deny(format!("module registry hash: {error}")))?;
        let policy_hash = canonical_hash(&self.policies)
            .map_err(|error| deny(format!("policy hash: {error}")))?;
        if catalog.module_registry_hash != registry_hash || catalog.policy_hash != policy_hash {
            return Err(deny("catalog registry or policy hash is stale"));
        }
        if catalog.revocation_epoch != self.capability_revocation_state.epoch {
            return Err(deny("catalog revocation or subject binding is stale"));
        }
        if !manifest
            .abi_contract
            .declarations
            .commands
            .iter()
            .any(|decl| {
                catalog.entries.iter().any(|entry| {
                    decl.namespace == entry.namespace
                        && decl.name == entry.command
                        && decl.schema_version == entry.schema_version
                        && decl.schema_hash == entry.schema_hash
                })
            })
        {
            return Err(deny("catalog declaration is not active"));
        }
        Ok(())
    }

    fn execute_trusted_module_sandbox(
        &mut self,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        response: &AgentCommandResponse,
        subject: &oasis7_wasm_abi::CapabilitySubject,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<oasis7_wasm_abi::ModuleOutput, WorldError> {
        // The provider-supplied trace id is observability data, not an
        // authority binding.  Use the host-bound response nonce for the
        // durable trusted-command trace so replay can distinguish this
        // command's state update from an unrelated module tail event.
        let trace_id = format!("trusted-command-{}", response.response_nonce);
        let input = response
            .envelope
            .encode_canonical()
            .map_err(|error| deny(format!("command encoding: {error}")))?;
        let state = (manifest.kind == ModuleKind::Reducer).then(|| {
            self.state
                .module_states
                .get(&manifest.module_id)
                .cloned()
                .unwrap_or_default()
        });
        let call_input = ModuleCallInput {
            ctx: oasis7_wasm_abi::ModuleContext {
                v: "wasm-1".to_string(),
                module_id: manifest.module_id.clone(),
                trace_id: trace_id.clone(),
                time: self.state.time,
                origin: ModuleCallOrigin {
                    kind: "trusted_module_command".to_string(),
                    id: response.response_nonce.clone(),
                },
                caller: super::capability_authorization_events::module_call_caller(subject),
                limits: manifest.limits.clone(),
                stage: Some("trusted_module_command".to_string()),
                world_config_hash: Some(self.current_manifest_hash()?),
                manifest_hash: Some(super::super::util::hash_json(manifest)?),
                journal_height: Some(self.journal.events.len() as u64),
                module_version: Some(manifest.version.clone()),
                module_kind: Some(format!("{:?}", manifest.kind)),
                module_role: Some(format!("{:?}", manifest.role)),
            },
            event: None,
            action: Some(input),
            state,
        };
        let input_bytes = super::super::util::to_canonical_cbor(&call_input)?;
        self.check_module_runtime_resources(
            &manifest.module_id,
            &trace_id,
            manifest,
            input_bytes.len() as u64,
        )
        .map_err(|failure| deny(format!("runtime upper-bound charge: {}", failure.detail)))?;
        let output = self
            .call_module_raw(
                &manifest.module_id,
                &trace_id,
                input_bytes.clone(),
                manifest,
                sandbox,
            )
            .map_err(|failure| deny(format!("sandbox: {}", failure.detail)))?;
        if manifest.kind == ModuleKind::Pure && output.new_state.is_some() {
            return Err(deny("pure module returned state in trusted command"));
        }
        if output.tick_lifecycle.is_some() {
            return Err(deny(
                "trusted module commands cannot return tick lifecycle directives",
            ));
        }
        self.validate_module_output_limits(
            &manifest.module_id,
            &manifest.limits,
            output.effects.len(),
            output.emits.len(),
            output.output_bytes,
        )?;
        let mut intents = Vec::with_capacity(output.effects.len());
        for effect in &output.effects {
            let cap_ref = self.resolve_trusted_effect_cap_ref(manifest, effect)?;
            self.verify_trusted_effect_grant(&cap_ref, manifest, response, effect)?;
            let intent = EffectIntent {
                intent_id: format!("intent-{}", self.allocate_next_intent_seq()),
                kind: effect.kind.clone(),
                params: effect.params.clone(),
                cap_ref,
                origin: EffectOrigin::Module {
                    module_id: manifest.module_id.clone(),
                },
            };
            let decision = self.policies.decide(&intent);
            self.append_event(
                WorldEventBody::PolicyDecisionRecorded(PolicyDecisionRecord::from_intent(
                    &intent,
                    decision.clone(),
                )),
                None,
            )?;
            if !decision.is_allowed() {
                return Err(deny(format!(
                    "trusted effect policy denied {}",
                    decision
                        .reason()
                        .unwrap_or_else(|| "policy_deny".to_string())
                )));
            }
            intents.push(intent);
        }
        self.try_charge_module_runtime(
            &manifest.module_id,
            &trace_id,
            manifest,
            input_bytes.len() as u64,
            &output,
        )
        .map_err(|failure| deny(format!("runtime charge: {}", failure.detail)))?;
        if let Some(state) = &output.new_state {
            self.append_event(
                WorldEventBody::ModuleStateUpdated(oasis7_wasm_abi::ModuleStateUpdate {
                    module_id: manifest.module_id.clone(),
                    trace_id: trace_id.clone(),
                    state: state.clone(),
                }),
                None,
            )?;
        }
        for intent in intents {
            self.append_event(WorldEventBody::EffectQueued(intent), None)?;
        }
        for emit in &output.emits {
            self.append_event(
                WorldEventBody::ModuleEmitted(oasis7_wasm_abi::ModuleEmitEvent {
                    module_id: manifest.module_id.clone(),
                    trace_id: trace_id.clone(),
                    kind: emit.kind.clone(),
                    payload: emit.payload.clone(),
                }),
                None,
            )?;
        }
        Ok(output)
    }

    fn resolve_trusted_effect_cap_ref(
        &self,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        effect: &oasis7_wasm_abi::ModuleEffectIntent,
    ) -> Result<String, WorldError> {
        let cap_ref = if let Some(slot) = effect.cap_slot.as_deref() {
            let bound = manifest
                .abi_contract
                .cap_slots
                .get(slot)
                .ok_or_else(|| deny(format!("trusted effect cap slot is not bound: {slot}")))?;
            if !effect.cap_ref.trim().is_empty() && effect.cap_ref != *bound {
                return Err(deny("trusted effect cap slot conflicts with cap_ref"));
            }
            bound.clone()
        } else if effect.cap_ref.trim().is_empty() {
            return Err(deny("trusted effect cap_ref is empty"));
        } else {
            effect.cap_ref.clone()
        };
        if !manifest
            .required_caps
            .iter()
            .any(|required| required == &cap_ref)
        {
            return Err(deny("trusted effect cap_ref is not declared by manifest"));
        }
        Ok(cap_ref)
    }

    fn verify_trusted_effect_grant(
        &self,
        cap_ref: &str,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        response: &AgentCommandResponse,
        effect: &oasis7_wasm_abi::ModuleEffectIntent,
    ) -> Result<(), WorldError> {
        let encoded = self
            .capability_grants_v2
            .get(cap_ref)
            .ok_or_else(|| deny("trusted effect grant is not in the durable registry"))?;
        let grant: CapabilityGrantV2 = serde_json::from_value(encoded.clone())
            .map_err(|_| deny("trusted effect grant is malformed"))?;
        grant
            .validate()
            .map_err(|error| deny(format!("trusted effect grant validation: {error}")))?;
        if !grant
            .body_hash_matches()
            .map_err(|error| deny(format!("trusted effect grant body hash: {error}")))?
            || grant
                .expected_grant_id()
                .map_err(|error| deny(format!("trusted effect grant id hash: {error}")))?
                != grant.grant_id
        {
            return Err(deny("trusted effect grant canonical hash mismatch"));
        }
        if grant.grant_id != cap_ref
            || grant.status != "verified"
            || grant.subject != response.subject
            || grant.audience != response.audience
            || grant.expires_at_tick.is_none()
            || grant
                .expires_at_tick
                .is_some_and(|expiry| self.state.time > expiry)
            || grant.issued_at_tick > self.state.time
            || grant.scope.module_id != manifest.module_id
            || grant.scope.module_version != manifest.version
            || grant.scope.object_kind != "effect"
            || grant.scope.object_name != effect.kind
            || !matches!(grant.scope.operation.as_str(), "invoke" | "execute")
        {
            return Err(deny("trusted effect grant scope does not match output"));
        }
        if !super::capability_authorization_events::scope_selectors_match_json(
            &grant.scope,
            &effect.params,
        ) {
            return Err(deny("trusted effect output is outside the grant selectors"));
        }
        let params_bytes = serde_json::to_vec(&effect.params)?;
        if !grant
            .scope
            .max_payload_bytes
            .is_some_and(|limit| u64::try_from(params_bytes.len()).is_ok_and(|size| size <= limit))
        {
            return Err(deny("trusted effect payload exceeds grant bound"));
        }
        self.verify_issuer(&grant)?;
        self.verify_live_revocation(&grant)?;
        self.verify_parent_chain(&grant)
    }
}

pub(super) fn validate_authority_record(
    record: &CapabilityAuthorityRecord,
) -> Result<(), WorldError> {
    for (field, value) in [
        ("issuer_id", record.issuer_id.as_str()),
        ("issuer_kind", record.issuer_kind.as_str()),
        ("key_id", record.key_id.as_str()),
        ("public_key_hex", record.public_key_hex.as_str()),
        ("finalized_receipt_id", record.finalized_receipt_id.as_str()),
        ("world_id", record.world_id.as_str()),
        ("branch_id", record.branch_id.as_str()),
        ("finality_block_hash", record.finality_block_hash.as_str()),
        ("finality_status", record.finality_status.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(deny(format!("authority record {field} is required")));
        }
    }
    if !matches!(
        record.issuer_kind.as_str(),
        "governance" | "system" | "kernel_migration"
    ) {
        return Err(deny("authority record issuer kind is invalid"));
    }
    if record.finality_status != "finalized" {
        return Err(deny("authority record is not finalized"));
    }
    if record
        .authority_rotation_receipt_id
        .as_deref()
        .is_some_and(|receipt| receipt.trim().is_empty())
    {
        return Err(deny(
            "capability authority rotation receipt cannot be empty",
        ));
    }
    let key_bytes = hex::decode(record.public_key_hex.trim())
        .map_err(|_| deny("authority record public key is invalid hex"))?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| deny("authority record public key length is invalid"))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| deny("authority record public key is invalid"))?;
    if record
        .revoked_grant_ids
        .iter()
        .any(|grant_id| grant_id.trim().is_empty())
        || record.superseded_by.iter().any(|(grant_id, replacement)| {
            grant_id == replacement
                || !is_canonical_grant_id(grant_id)
                || !is_canonical_grant_id(replacement)
                || record.revoked_grant_ids.contains(replacement)
        })
    {
        return Err(deny("authority record contains an invalid supersession id"));
    }
    for (grant_id, replacement_id) in &record.superseded_by {
        let mut cursor = replacement_id.as_str();
        let mut visited = BTreeSet::new();
        while let Some(next_target) = record.superseded_by.get(cursor) {
            if !visited.insert(cursor.to_string()) || next_target == grant_id {
                return Err(deny("authority record supersession graph contains a cycle"));
            }
            cursor = next_target.as_str();
        }
    }
    Ok(())
}

fn is_canonical_grant_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn validate_agent_identity(
    agent_id: &str,
    identity: &CapabilityAgentIdentity,
) -> Result<(), WorldError> {
    if agent_id.trim().is_empty() || identity.owner_binding.trim().is_empty() {
        return Err(deny("capability agent identity fields are required"));
    }
    if identity.generation == 0 {
        return Err(deny(
            "capability agent identity generation must be positive",
        ));
    }
    Ok(())
}

pub(super) fn validate_invocation_context(
    context: &CapabilityInvocationContext,
) -> Result<(), WorldError> {
    for (field, value) in [
        ("grant_id", context.grant_id.as_str()),
        ("catalog_snapshot_id", context.catalog_snapshot_id.as_str()),
        ("module_id", context.module_id.as_str()),
        ("module_version", context.module_version.as_str()),
        ("response_nonce", context.response_nonce.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(deny(format!("invocation context {field} is required")));
        }
    }
    context
        .subject
        .validate()
        .map_err(|error| deny(format!("invocation context subject: {error}")))?;
    context
        .presenter
        .validate()
        .map_err(|error| deny(format!("invocation context presenter: {error}")))?;
    context
        .audience
        .validate()
        .map_err(|error| deny(format!("invocation context audience: {error}")))?;
    Ok(())
}

pub(super) fn deny(reason: impl Into<String>) -> WorldError {
    WorldError::CapabilityAuthorizationDenied {
        reason: reason.into(),
    }
}
