//! Public governance observations. Authentication and evidence-policy checks
//! belong to the transport; these contracts never confer consensus voting power.
use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadMembershipRequest {
    pub contract_version: u32,
    pub world: WorldIdentity,
    pub min_commit: Option<CommitRef>,
    pub fixed_commit: Option<CommitRef>,
}
impl ReadMembershipRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_version(self.contract_version)?;
        self.world.validate()?;
        for commit in [&self.min_commit, &self.fixed_commit].into_iter().flatten() {
            commit.validate()?;
            if commit.world != self.world {
                return Err(ContractError("membership request world mismatch"));
            }
        }
        if let (Some(minimum), Some(fixed)) = (&self.min_commit, &self.fixed_commit)
            && !fixed.satisfies_minimum(minimum)?
        {
            return Err(ContractError("fixed membership commit precedes minimum"));
        }
        Ok(())
    }
}

/// The transport supplies fresh OS randomness for each read. Both actor and
/// service signatures cover this envelope, preventing an old signed snapshot
/// from satisfying a later identical list/status/watch query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipReadEnvelope {
    pub query: ReadMembershipRequest,
    pub observation_nonce: String,
}
impl MembershipReadEnvelope {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.query.validate()?;
        if self.observation_nonce.len() != 64
            || !self
                .observation_nonce
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ContractError("invalid membership observation nonce"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipCandidateStatus {
    Applied,
    ApprovedCandidate,
    ProbationReady,
    Active,
    Revoked,
}

/// Deliberate public allowlist: operator owner and free-form revocation reason
/// remain excluded, even if future admission storage acquires private evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicMembershipCandidate {
    pub candidate_id: String,
    pub node_id: String,
    pub finality_signer_public_key: String,
    pub stake: u64,
    pub public_manifest_hash: String,
    pub requested_at_epoch: u64,
    pub last_transition_tick: u64,
    pub approved_at_epoch: Option<u64>,
    pub activation_epoch: Option<u64>,
    pub revoked_at_epoch: Option<u64>,
    pub status: MembershipCandidateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicMembershipRegistry {
    pub slot_id: String,
    pub threshold: u16,
    pub threshold_bps: u16,
    pub signer_bindings: BTreeMap<String, String>,
    pub validator_stakes: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadMembershipResponse {
    pub contract_version: u32,
    pub execution_evidence_scope: Option<String>,
    pub commit: CommitRef,
    pub state_root_hash: String,
    pub governance_epoch: u64,
    pub history_profile: String,
    pub candidates: BTreeMap<String, PublicMembershipCandidate>,
    pub selected_candidates: BTreeMap<String, String>,
    pub effective_registry: Option<PublicMembershipRegistry>,
    pub effective_registry_digest: String,
    pub supports_safe_runtime_update: bool,
    /// Current implementation supplies no installation acknowledgement.
    pub applied_consensus_membership: Option<CommitRef>,
}
impl ReadMembershipResponse {
    pub fn validate(&self, request: &ReadMembershipRequest) -> Result<(), ContractError> {
        request.validate()?;
        validate_version(self.contract_version)?;
        self.commit.validate()?;
        required(&self.state_root_hash)?;
        required(&self.history_profile)?;
        required(&self.effective_registry_digest)?;
        if self.commit.world != request.world
            || request
                .fixed_commit
                .as_ref()
                .is_some_and(|fixed| fixed != &self.commit)
        {
            return Err(ContractError("membership response commit mismatch"));
        }
        if let Some(minimum) = &request.min_commit
            && !self.commit.satisfies_minimum(minimum)?
        {
            return Err(ContractError("membership response precedes minimum"));
        }
        for (id, record) in &self.candidates {
            required(id)?;
            required(&record.node_id)?;
            if id != &record.candidate_id {
                return Err(ContractError("membership candidate key mismatch"));
            }
        }
        for (node, candidate) in &self.selected_candidates {
            if self
                .candidates
                .get(candidate)
                .is_none_or(|record| &record.node_id != node)
            {
                return Err(ContractError("selected membership candidate mismatch"));
            }
        }
        Ok(())
    }
}
