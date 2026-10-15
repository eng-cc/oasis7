use super::*;
/// For this formal profile every legal append needs both endpoint receipts.
/// A trusted nonrollback, nonclonable seal of either necessary endpoint therefore
/// covers OLD WORLD APPENDS. This token does not fence outbox delivery, unrelated
/// signing paths or other profiles. It makes no inference from network failure.
#[derive(Debug, Clone)]
pub struct VerifiedAppendFencingCoverageV1 {
    pub(super) seal: FencingVerdict,
    pub(super) predecessor_authority_id: String,
    pub(super) cut: FormalHeadRefV1,
}
impl VerifiedAppendFencingCoverageV1 {
    pub fn derive_from_necessary_endpoint_seal(
        authority: &VerifiedAuthorityV1,
        cut: &VerifiedFormalCommitV1,
        seal: &FencingVerdict,
    ) -> Result<Self, FormalError> {
        let body = seal.seal_body();
        let expected = match body.instance.endpoint_role {
            crate::controlled_authority::replicated_protocol::EndpointRole::Primary => {
                &authority.definition().primary.instance
            }
            crate::controlled_authority::replicated_protocol::EndpointRole::Replica => {
                &authority.definition().replica.instance
            }
        };
        if seal.provider_public_key() != authority.policy().fencing_provider_public_key
            || !authority
                .policy()
                .authorized_fencing_resource_bindings
                .iter()
                .any(|resource| resource == seal.resource_binding())
        {
            return Err(FormalError::FencingUnproven);
        }
        if &body.instance != expected
            || cut.finalizing_authority_id() != authority.authority_id()?
            || cut.record().handoff.is_some()
            || body.highest_complete.position != cut.head().position
            || body.highest_complete.decision_hash != cut.head().record_id
        {
            return Err(invalid(
                "necessary-endpoint append fencing authority/current-cut mismatch",
            ));
        }
        Ok(Self {
            seal: seal.clone(),
            predecessor_authority_id: authority.authority_id()?,
            cut: cut.head(),
        })
    }
    pub fn evidence_digest(&self) -> Result<String, FormalError> {
        hash(
            "oasis7.formal.necessary_endpoint_append_fencing.v1",
            &(
                self.seal.seal_digest(),
                &self.predecessor_authority_id,
                &self.cut,
            ),
        )
    }
}
