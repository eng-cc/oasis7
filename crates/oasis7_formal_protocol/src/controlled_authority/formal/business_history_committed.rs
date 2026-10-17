//! Ordinary-reader restoration beneath its own canonical result root.
//! No service checkpoint/provider assertion is accepted by this verifier.
use super::business_index::{
    BUSINESS_INDEX_DEPTH, BusinessIndexInclusionV1, BusinessIndexNodeV1, BusinessIndexRootV1,
};
use super::*;
/// Inclusion fact only; it cannot qualify arbitrary caller-supplied records.
pub(crate) struct VerifiedHistoricalCutInclusionV1 {
    cut: FormalHistoricalCutV1,
    authority: VerifiedAuthorityV1,
}
impl VerifiedHistoricalCutInclusionV1 {
    pub(crate) fn cut(&self) -> &FormalHistoricalCutV1 {
        &self.cut
    }
    pub(crate) fn authority(&self) -> &VerifiedAuthorityV1 {
        &self.authority
    }
}
/// An old published cut delegated by the reader's independently trusted current
/// canonical root under the profile2 exact-parent HistoricalCut contract.
pub struct VerifiedCommittedHistoricalCutV1 {
    commit: VerifiedFormalCommitV1,
    delegating_authority: VerifiedAuthorityV1,
    authority: VerifiedAuthorityV1,
    cut: FormalHistoricalCutV1,
    delegating_head: FormalHeadRefV1,
}
/// Inclusion of an exact historical cut beneath the reader's current signed
/// result root. This proves delegation of the leaf only; qualification of the
/// embedded historical proof must come from the reader's retained verified
/// commit lineage.
pub struct VerifiedHistoricalCutInclusionV2 {
    cut: FormalHistoricalCutV1,
    delegating_head: FormalHeadRefV1,
}
impl VerifiedHistoricalCutInclusionV2 {
    pub fn cut(&self) -> &FormalHistoricalCutV1 {
        &self.cut
    }
    pub fn delegating_head(&self) -> &FormalHeadRefV1 {
        &self.delegating_head
    }
    pub fn verify(
        current: &VerifiedFormalReadAnchorV1,
        descriptor: &BusinessIndexRootV1,
        inclusion: &BusinessIndexInclusionV1,
        expected_position: u64,
    ) -> Result<Self, FormalError> {
        verify_current_root_inclusion(current, descriptor, inclusion, expected_position)
    }
    /// Qualify a recovery result using an independently verified permission and
    /// its exact predecessor. Inclusion grants no recovery authority by itself.
    pub fn qualify_with_verified_recovery(
        &self,
        current: &VerifiedFormalReadAnchorV1,
        recovery: &VerifiedRecovery,
        predecessor: &VerifiedFormalCommitV1,
    ) -> Result<VerifiedCommittedHistoricalCutV1, FormalError> {
        let authority = recovery.successor();
        if self.delegating_head != current.head().head()
            || authority.definition() != current.authority().definition()
            || authority.policy() != current.authority().policy()
            || authority.activation_ref() != current.authority().activation_ref()
            || authority.context_digest()? != current.authority().context_digest()?
            || authority.definition().profile_version != 2
            || predecessor.head() != recovery.body().sealed_cut
            || self.cut.parent_record.world != authority.definition().world
            || self.cut.parent_qualified.decision.branch == FormalCompletionBranchV1::Normal
        {
            return Err(crypto::invalid(
                "historical recovery cut lacks exact verified active permission",
            ));
        }
        let commit = verify_formal_qualified(
            &self.cut.parent_qualified,
            recovery.original_authority(),
            Some(predecessor),
            Some(recovery),
        )?;
        if commit.record() != &self.cut.parent_record
            || commit.finalizing_authority_id() != authority.authority_id()?
        {
            return Err(crypto::invalid("historical recovery qualification binding"));
        }
        Ok(VerifiedCommittedHistoricalCutV1 {
            commit,
            delegating_authority: current.authority().clone(),
            authority: authority.clone(),
            cut: self.cut.clone(),
            delegating_head: self.delegating_head.clone(),
        })
    }
    /// Recovery permissions must already be independently verified from the
    /// external origin. Require the complete authority path to the current root.
    pub fn qualify_with_verified_recovery_lineage(
        &self,
        current: &VerifiedFormalReadAnchorV1,
        permissions: &[super::super::recovery_bootstrap::VerifiedRecoveryBootstrapV1],
    ) -> Result<VerifiedCommittedHistoricalCutV1, FormalError> {
        for pair in permissions.windows(2) {
            if let Some(prior) = pair[1].permission_evidence().prior_permission.as_deref() {
                let prior_bytes = serde_cbor::to_vec(prior)
                    .map_err(|error| crypto::invalid(error.to_string()))?;
                let previous_bytes = serde_cbor::to_vec(pair[0].permission_evidence())
                    .map_err(|error| crypto::invalid(error.to_string()))?;
                if prior_bytes != previous_bytes {
                    return Err(crypto::invalid(
                        "historical recovery exact prior permission binding",
                    ));
                }
            }
        }
        let contexts: Vec<_> = permissions
            .iter()
            .map(|p| (p.recovery(), p.cut()))
            .collect();
        self.qualify_with_verified_recovery_contexts(current, &contexts)
    }
    /// Reader-owned recovery tokens obtained from independent canonical replay.
    /// Each predecessor is qualified before replay installed its recovery. The
    /// complete checked authority path must terminate at the current anchor.
    pub fn qualify_with_verified_recovery_contexts(
        &self,
        current: &VerifiedFormalReadAnchorV1,
        contexts: &[(&VerifiedRecovery, &VerifiedFormalCommitV1)],
    ) -> Result<VerifiedCommittedHistoricalCutV1, FormalError> {
        if self.delegating_head != current.head().head() || contexts.is_empty() {
            return Err(crypto::invalid(
                "historical recovery missing verified lineage",
            ));
        }
        for (recovery, predecessor) in contexts {
            if predecessor.head() != recovery.body().sealed_cut {
                return Err(crypto::invalid(
                    "historical recovery predecessor context binding",
                ));
            }
        }
        for pair in contexts.windows(2) {
            if pair[1]
                .0
                .body()
                .retained_context
                .as_ref()
                .is_some_and(|retained| {
                    retained.previous_recovery_certificate_digest != pair[0].0.certificate_digest()
                })
            {
                return Err(crypto::invalid(
                    "historical recovery retained prior certificate binding",
                ));
            }
            if pair[0].0.successor().context_digest()?
                != pair[1].0.predecessor().context_digest()?
                || pair[0].0.successor().definition() != pair[1].0.predecessor().definition()
                || pair[0].0.successor().policy() != pair[1].0.predecessor().policy()
                || pair[0].0.successor().activation_ref()
                    != pair[1].0.predecessor().activation_ref()
            {
                return Err(crypto::invalid(
                    "historical recovery permission lineage fork",
                ));
            }
        }
        let latest = contexts.last().unwrap().0.successor();
        if latest.context_digest()? != current.authority().context_digest()?
            || latest.definition() != current.authority().definition()
            || latest.policy() != current.authority().policy()
            || latest.activation_ref() != current.authority().activation_ref()
        {
            return Err(crypto::invalid(
                "historical recovery lineage current binding",
            ));
        }
        if self
            .cut
            .parent_qualified
            .decision
            .primary
            .body
            .recovery_certificate_digest
            != self
                .cut
                .parent_qualified
                .decision
                .replica
                .body
                .recovery_certificate_digest
        {
            return Err(crypto::invalid(
                "historical recovery dual receipt certificate mismatch",
            ));
        }
        let permission = contexts
            .iter()
            .find(|permission| {
                self.cut
                    .parent_qualified
                    .decision
                    .primary
                    .body
                    .recovery_certificate_digest
                    .as_deref()
                    == Some(permission.0.certificate_digest())
            })
            .ok_or_else(|| crypto::invalid("historical recovery permission absent"))?;
        let recovery = permission.0;
        if recovery.successor().definition().profile_version != 2
            || self.cut.parent_record.world != current.head().record().world
            || self.cut.parent_qualified.decision.branch == FormalCompletionBranchV1::Normal
        {
            return Err(crypto::invalid("historical recovery canonical context"));
        }
        let commit = verify_formal_qualified(
            &self.cut.parent_qualified,
            recovery.original_authority(),
            Some(permission.1),
            Some(recovery),
        )?;
        if commit.record() != &self.cut.parent_record
            || commit.finalizing_authority_id() != recovery.successor().authority_id()?
        {
            return Err(crypto::invalid("historical recovery finalizer binding"));
        }
        Ok(VerifiedCommittedHistoricalCutV1 {
            commit,
            authority: recovery.successor().clone(),
            delegating_authority: current.authority().clone(),
            cut: self.cut.clone(),
            delegating_head: self.delegating_head.clone(),
        })
    }
    /// Qualify the exact included Normal proof with a locally verified
    /// authority. Recovery branches require their retained exact proof token.
    pub fn qualify_normal_with_verified_authority(
        &self,
        current: &VerifiedFormalReadAnchorV1,
        historical_authority: &VerifiedAuthorityV1,
    ) -> Result<VerifiedCommittedHistoricalCutV1, FormalError> {
        let authority_id = historical_authority.authority_id()?;
        if self.delegating_head != current.head().head()
            || historical_authority.definition().profile_version != 2
            || historical_authority.definition().world != current.head().record().world
            || historical_authority.activation_ref() != current.authority().activation_ref()
            || self.cut.parent_record.original_epoch != historical_authority.definition().epoch
            || self.cut.parent_record.original_authority_id != authority_id
            || self.cut.parent_qualified.decision.branch != FormalCompletionBranchV1::Normal
        {
            return Err(crypto::invalid(
                "historical cut is outside locally verified authority lineage",
            ));
        }
        let fact = VerifiedHistoricalCutInclusionV1 {
            cut: self.cut.clone(),
            authority: historical_authority.clone(),
        };
        let commit = super::super::proof::verify_historical_qualified_from_inclusion(&fact)?;
        Ok(VerifiedCommittedHistoricalCutV1 {
            commit,
            delegating_authority: current.authority().clone(),
            authority: historical_authority.clone(),
            cut: fact.cut,
            delegating_head: self.delegating_head.clone(),
        })
    }
}
impl VerifiedCommittedHistoricalCutV1 {
    pub fn delegating_authority(&self) -> &VerifiedAuthorityV1 {
        &self.delegating_authority
    }
    pub fn commit(&self) -> &VerifiedFormalCommitV1 {
        &self.commit
    }
    pub fn authority(&self) -> &VerifiedAuthorityV1 {
        &self.authority
    }
    pub fn proof(&self) -> &FormalQualifiedCommitProofV1 {
        &self.cut.parent_qualified
    }
    pub fn delegating_head(&self) -> &FormalHeadRefV1 {
        &self.delegating_head
    }
    pub fn verify(
        current: &VerifiedFormalReadAnchorV1,
        descriptor: &BusinessIndexRootV1,
        inclusion: &BusinessIndexInclusionV1,
        expected_position: u64,
    ) -> Result<Self, FormalError> {
        let delegated =
            verify_current_root_inclusion(current, descriptor, inclusion, expected_position)?;
        let cut = delegated.cut;
        let authority = current.authority();
        if cut.parent_record.original_epoch != authority.definition().epoch
            || cut.parent_record.original_authority_id != authority.authority_id()?
            || cut.parent_record.activation_ref != authority.activation_ref()
            || cut.parent_qualified.decision.branch != FormalCompletionBranchV1::Normal
        {
            return Err(crypto::invalid(
                "HistoryUnavailable: legal sameepoch Normal historical context required",
            ));
        }
        let fact = VerifiedHistoricalCutInclusionV1 {
            cut,
            authority: authority.clone(),
        };
        let commit = super::super::proof::verify_historical_qualified_from_inclusion(&fact)?;
        Ok(Self {
            commit,
            delegating_authority: current.authority().clone(),
            authority: fact.authority.clone(),
            cut: fact.cut,
            delegating_head: delegated.delegating_head,
        })
    }
    /// Verify a Normal historical proof under an authority already established
    /// by the caller's signed activation/recovery lineage. The provider's leaf
    /// supplies inclusion only; it cannot supply this authority token.
    pub fn verify_with_verified_authority(
        current: &VerifiedFormalReadAnchorV1,
        historical_authority: &VerifiedAuthorityV1,
        descriptor: &BusinessIndexRootV1,
        inclusion: &BusinessIndexInclusionV1,
        expected_position: u64,
    ) -> Result<Self, FormalError> {
        VerifiedHistoricalCutInclusionV2::verify(current, descriptor, inclusion, expected_position)?
            .qualify_normal_with_verified_authority(current, historical_authority)
    }
}

fn verify_current_root_inclusion(
    current: &VerifiedFormalReadAnchorV1,
    descriptor: &BusinessIndexRootV1,
    inclusion: &BusinessIndexInclusionV1,
    expected_position: u64,
) -> Result<VerifiedHistoricalCutInclusionV2, FormalError> {
    if current.authority().definition().profile_version != 2
        || descriptor
            .canonical_result_root()
            .map_err(crypto::invalid)?
            != current.head().record().result_root
        || inclusion.leaf_count != descriptor.leaf_count
        || inclusion.siblings.len() != BUSINESS_INDEX_DEPTH
    {
        return Err(crypto::invalid(
            "historical cut current canonical profile2 root/count/path",
        ));
    }
    let key: [u8; 32] = hex::decode(&inclusion.leaf.key_digest)
        .map_err(|_| crypto::invalid("historical cut key bytes"))?
        .try_into()
        .map_err(|_| crypto::invalid("historical cut key length"))?;
    let mut node = BusinessIndexNodeV1::leaf(inclusion.leaf.clone())
        .map_err(crypto::invalid)?
        .digest()
        .map_err(crypto::invalid)?;
    for (offset, sibling) in inclusion.siblings.iter().enumerate() {
        let depth = BUSINESS_INDEX_DEPTH - 1 - offset;
        let right = (key[depth / 8] >> (7 - depth % 8)) & 1 == 1;
        node = if right {
            BusinessIndexNodeV1::branch(depth, sibling.clone(), node)
        } else {
            BusinessIndexNodeV1::branch(depth, node, sibling.clone())
        }
        .map_err(crypto::invalid)?
        .digest()
        .map_err(crypto::invalid)?;
    }
    if node != descriptor.node_root {
        return Err(crypto::invalid(
            "historical cut inclusion under current signed resultRoot",
        ));
    }
    let cut = FormalHistoricalCutV1::decode_leaf(
        &inclusion.leaf,
        &current.head().record().world,
        expected_position,
    )?;
    if cut.parent_record.position >= current.head().record().position {
        return Err(crypto::invalid(
            "HistoryUnavailable: historical cut must delegate an earlier result",
        ));
    }
    Ok(VerifiedHistoricalCutInclusionV2 {
        cut,
        delegating_head: current.head().head(),
    })
}
