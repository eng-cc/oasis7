//! Governance observations from a verified formal projection or an explicitly
//! scoped engineering service assertion. Neither supplies runtime installation.
use super::verified_view::{ReadEvidencePolicy, VerifiedWorldView};
use super::*;
use crate::runtime::{GovernanceValidatorAdmissionStatus as Status, WorldGovernanceMembershipView};

pub fn public_response(
    view: WorldGovernanceMembershipView,
    scope: Option<String>,
) -> ReadMembershipResponse {
    let candidates = view
        .candidates
        .into_iter()
        .map(|(id, record)| {
            let status = match record.status {
                Status::Applied => MembershipCandidateStatus::Applied,
                Status::ApprovedCandidate => MembershipCandidateStatus::ApprovedCandidate,
                Status::ProbationReady => MembershipCandidateStatus::ProbationReady,
                Status::Active => MembershipCandidateStatus::Active,
                Status::Revoked => MembershipCandidateStatus::Revoked,
            };
            (
                id,
                PublicMembershipCandidate {
                    candidate_id: record.candidate_id,
                    node_id: record.node_id,
                    finality_signer_public_key: record.finality_signer_public_key,
                    stake: record.stake,
                    public_manifest_hash: record.public_manifest_hash,
                    requested_at_epoch: record.requested_at_epoch,
                    last_transition_tick: record.last_transition_tick,
                    approved_at_epoch: record.approved_at_epoch,
                    activation_epoch: record.activation_epoch,
                    revoked_at_epoch: record.revoked_at_epoch,
                    status,
                },
            )
        })
        .collect();
    ReadMembershipResponse {
        contract_version: WORLD_SERVICE_CONTRACT_VERSION,
        execution_evidence_scope: scope,
        commit: view.commit,
        state_root_hash: view.state_root_hash,
        governance_epoch: view.governance_epoch,
        history_profile: view.profile,
        candidates,
        selected_candidates: view
            .selected_records
            .into_iter()
            .map(|(node, record)| (node, record.candidate_id))
            .collect(),
        effective_registry: view
            .effective_registry
            .map(|registry| PublicMembershipRegistry {
                slot_id: registry.slot_id,
                threshold: registry.threshold,
                threshold_bps: registry.threshold_bps,
                signer_bindings: registry.signer_bindings,
                validator_stakes: registry.validator_stakes,
            }),
        effective_registry_digest: view.effective_registry_digest,
        supports_safe_runtime_update: false,
        applied_consensus_membership: None,
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedMembershipObservation {
    response: ReadMembershipResponse,
}
impl AuthenticatedMembershipObservation {
    /// Formal observations come exclusively from a privately verified public
    /// projection. A signed membership endpoint alone cannot create this value.
    pub(crate) fn from_formal_view(
        view: &VerifiedWorldView,
        request: &ReadMembershipRequest,
    ) -> Result<Self, String> {
        if view.formal_commit().is_none() || view.version().visibility_scope != "public" {
            return Err("formal membership requires independently proved public projection".into());
        }
        let body = view
            .projection()
            .formal_commit_header
            .as_ref()
            .and_then(|header| header.governance_membership.as_ref())
            .ok_or("formal governance body unavailable")?;
        if body.schema_version != 1
            || authority::request_digest(
                "governance-membership-registry-v1",
                &body.effective_registry,
            )? != body.effective_registry_digest
        {
            return Err("formal governance schema/registry commitment mismatch".into());
        }
        let response = ReadMembershipResponse {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            execution_evidence_scope: None,
            commit: view.version().commit.clone(),
            state_root_hash: body.state_root_hash.clone(),
            governance_epoch: body.governance_epoch,
            history_profile: "formal_controlled_single_authority_v1".into(),
            candidates: body.candidates.clone(),
            selected_candidates: body.selected_candidates.clone(),
            effective_registry: body.effective_registry.clone(),
            effective_registry_digest: body.effective_registry_digest.clone(),
            supports_safe_runtime_update: false,
            applied_consensus_membership: None,
        };
        // Exact minimum ancestry was already certified by the formal view
        // verifier; prototype binding equality cannot reject legal governance.
        if request
            .min_commit
            .as_ref()
            .is_some_and(|minimum| !view.formal_minimum_matches(minimum))
        {
            return Err("formal membership minimum was not certified by this exact view".into());
        }
        let mut shape = request.clone();
        shape.min_commit = None;
        response.validate(&shape).map_err(|e| e.to_string())?;
        Ok(Self { response })
    }
    /// Only the signed transport can construct this service assertion.
    pub(crate) fn from_signed_transport(
        response: ReadMembershipResponse,
        request: &ReadMembershipRequest,
        policy: ReadEvidencePolicy,
    ) -> Result<Self, String> {
        if !policy.accepts(response.execution_evidence_scope.as_deref()) {
            return Err("membership execution scope is not explicitly allowed".into());
        }
        let engineering_scope = response.execution_evidence_scope.as_deref()
            == Some(verified_view::CONTROLLED_PREREQUISITE_READ_SCOPE);
        let engineering_profile =
            response.history_profile == verified_view::CONTROLLED_PREREQUISITE_READ_SCOPE;
        // Formal membership has no authenticated profile contract in this
        // implementation. Admit one defined diagnostic pair, never arbitrary
        // unscoped strings by exclusion of known engineering labels.
        if !engineering_scope || !engineering_profile {
            return Err("membership history scope/profile pair is unsupported".into());
        }
        response
            .validate(request)
            .map_err(|error| error.to_string())?;
        if response.supports_safe_runtime_update || response.applied_consensus_membership.is_some()
        {
            return Err("membership runtime installation evidence is unsupported".into());
        }
        if authority::request_digest(
            "governance-membership-registry-v1",
            &response.effective_registry,
        )? != response.effective_registry_digest
        {
            return Err("membership registry digest mismatch".into());
        }
        Ok(Self { response })
    }
    pub fn response(&self) -> &ReadMembershipResponse {
        &self.response
    }
}

#[cfg(test)]
#[path = "membership_tests.rs"]
mod tests;
