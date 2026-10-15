//! Parent evidence is derived inside native endpoint storage, never from a
//! caller's DTO, claimed root, or public service response.
use super::*;
pub struct VerifiedFormalBusinessParentV1 {
    record: CanonicalRecordV1,
    proof: FormalQualifiedCommitProofV1,
}
impl VerifiedFormalBusinessParentV1 {
    pub fn record(&self) -> &CanonicalRecordV1 {
        &self.record
    }
    pub fn proof(&self) -> &FormalQualifiedCommitProofV1 {
        &self.proof
    }
    pub fn cut_leaf(
        &self,
    ) -> Result<super::super::business_index::BusinessIndexLeafV1, FormalError> {
        super::super::business_history::FormalHistoricalCutV1::from_proof(&self.proof)?.leaf()
    }
    // Only endpoint descendants can call this, after checking the referenced
    // node against a fresh signed/provider checkpoint or their held native guard.
    pub(super) fn from_authenticated_parent(
        parent: &CheckpointParent,
        proof: FormalQualifiedCommitProofV1,
    ) -> Result<Self, FormalError> {
        if proof.decision.record != parent.record
            || proof.decision.primary.body.finalizing_authority_id != parent.finalizing_authority_id
            || proof.decision.replica.body.finalizing_authority_id != parent.finalizing_authority_id
        {
            return Err(invalid(
                "business parent exact authenticated qualified context",
            ));
        }
        Ok(Self {
            record: parent.record.clone(),
            proof,
        })
    }
    pub(super) fn from_endpoint(
        endpoint: &FileFormalEndpoint,
    ) -> Result<Option<Self>, FormalError> {
        let Some(parent) = endpoint.parent.as_ref() else {
            return Ok(None);
        };
        let node_id = endpoint
            .store
            .body
            .head_node
            .as_ref()
            .or(endpoint.store.body.imported_head_node.as_ref())
            .ok_or_else(|| invalid("business parent missing authenticated history root"))?;
        let node: HistoryNode = endpoint.store.get(node_id)?;
        if node.head != parent.head() || node.head != endpoint.store.body.head {
            return Err(invalid("business parent exact held checkpoint head"));
        }
        let FormalRequestStateV1::Qualified { proof, .. } = endpoint.store.get(&node.entry)? else {
            return Err(invalid("business parent archive not Qualified"));
        };
        Self::from_authenticated_parent(
            &CheckpointParent {
                record: parent.record().clone(),
                finalizing_authority_id: parent.finalizing_authority_id().into(),
            },
            proof,
        )
        .map(Some)
    }
}
