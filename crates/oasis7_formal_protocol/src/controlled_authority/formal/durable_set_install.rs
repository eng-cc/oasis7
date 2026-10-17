//! Sealed healthy-transition authorization. This token permits staging only;
//! it cannot be used as a normal-writer authority or substitute for installs.
use super::{crypto::*, *};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableSetSealedEvidenceV1 {
    pub transition: SignedNormalDurableSetTransitionV1,
    pub primary_seal: fencing::EpochSealReceiptV1,
    pub replica_seal: fencing::EpochSealReceiptV1,
}
pub struct VerifiedSealedDurableSetTransitionV1 {
    transition: VerifiedNormalDurableSetTransitionV1,
    evidence: DurableSetSealedEvidenceV1,
}
impl VerifiedSealedDurableSetTransitionV1 {
    pub fn transition(&self) -> &VerifiedNormalDurableSetTransitionV1 {
        &self.transition
    }
    pub fn evidence(&self) -> &DurableSetSealedEvidenceV1 {
        &self.evidence
    }
}
/// The caller supplies independently verified old authority and exact canonical
/// predecessor/boundary. Raw evidence supplies no new trust root.
pub fn verify_sealed_durable_set_transition(
    evidence: &DurableSetSealedEvidenceV1,
    old: &VerifiedAuthorityV1,
    predecessor: &VerifiedFormalCommitV1,
    boundary: &VerifiedFormalCommitV1,
) -> Result<VerifiedSealedDurableSetTransitionV1, FormalError> {
    let transition =
        verify_normal_durable_set_transition(&evidence.transition, old, predecessor, boundary)?;
    let policy = fencing::TrustedFencingPolicyV1 {
        provider_public_key: old.policy().fencing_provider_public_key.clone(),
        authorized_resource_bindings: old.policy().authorized_fencing_resource_bindings.clone(),
    };
    for (seal, instance) in [
        (&evidence.primary_seal, &old.definition().primary.instance),
        (&evidence.replica_seal, &old.definition().replica.instance),
    ] {
        fencing::verify_epoch_seal(seal, &policy).map_err(|e| invalid(e.to_string()))?;
        if &seal.body.instance != instance
            || seal.body.recovery_operation != transition.digest()
            || seal.body.next_epoch_floor != transition.body().successor.epoch
            || seal.body.highest_complete.position != boundary.head().position
            || seal.body.highest_complete.decision_hash != boundary.head().record_id
            || !seal.body.highest_complete.qualified
            || seal.body.checkpoint_before.head != seal.body.highest_complete
            || seal.body.tail != fencing::SealedTailV1::empty()
        {
            return Err(invalid(
                "healthy install requires dual exact sealed boundary and empty slot",
            ));
        }
    }
    Ok(VerifiedSealedDurableSetTransitionV1 {
        transition,
        evidence: evidence.clone(),
    })
}

/// Verifies one install without promoting an authority. Mutation permission
/// remains unavailable until verify_durable_set_activation checks both roles.
pub fn verify_one_durable_set_install(
    sealed: &VerifiedSealedDurableSetTransitionV1,
    role: crate::controlled_authority::replicated_protocol::EndpointRole,
    install: &DurableSetInstallReceiptV1,
) -> Result<(), FormalError> {
    use crate::controlled_authority::replicated_protocol::EndpointRole;
    let transition = sealed.transition();
    let (instance, old_seal) = match role {
        EndpointRole::Primary => (
            &transition.body().successor.primary.instance,
            &sealed.evidence().primary_seal,
        ),
        EndpointRole::Replica => (
            &transition.body().successor.replica.instance,
            &sealed.evidence().replica_seal,
        ),
    };
    let body = &install.body;
    hash_hex(&body.independent_inventory_digest)?;
    if body.schema_version != 1
        || &body.instance != instance
        || body.transition_digest != transition.digest()
        || body.successor_authority_id != transition.body().successor.authority_id()?
        || body.boundary != transition.boundary().head()
        || body.independent_closure_root != transition.body().independent_closure_root
        || body.checkpoint.head.position != body.boundary.position
        || body.checkpoint.head.decision_hash != body.boundary.record_id
        || !body.checkpoint.head.qualified
        || body.checkpoint.revision == 0
        || (role == EndpointRole::Primary
            && (body.checkpoint.revision <= old_seal.provider.statement.checkpoint.revision
                || install.provider_evidence.statement.resource_binding
                    != old_seal.provider.statement.resource_binding))
    {
        return Err(invalid(
            "healthy install exact role/boundary/floor/contract",
        ));
    }
    let policy = fencing::TrustedFencingPolicyV1 {
        provider_public_key: transition.old.policy().fencing_provider_public_key.clone(),
        authorized_resource_bindings: transition
            .old
            .policy()
            .authorized_fencing_resource_bindings
            .clone(),
    };
    fencing::verify_monotonic_checkpoint_evidence(
        &install.provider_evidence,
        instance,
        &body.checkpoint,
        &policy,
    )
    .map_err(|e| invalid(e.to_string()))?;
    verify(
        "oasis7.formal.durable_set_install.endpoint.v1",
        body,
        &install.endpoint_signature_hex,
        &instance.endpoint_public_key,
    )?;
    verify(
        "oasis7.formal.durable_set_install.provider.v1",
        body,
        &install.provider_signature_hex,
        &policy.provider_public_key,
    )
}
