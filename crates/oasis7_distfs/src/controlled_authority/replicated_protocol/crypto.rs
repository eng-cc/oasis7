use super::*;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

const PROPOSAL: &str = "oasis7.controlled_authority.replicated.proposal.v1";
fn receipt_domain(role: EndpointRole, stage: ReceiptStage) -> &'static str {
    match (role, stage) {
        (EndpointRole::Primary, ReceiptStage::Prepared) => {
            "oasis7.controlled_authority.replicated.primary_prepared.v1"
        }
        (EndpointRole::Replica, ReceiptStage::Prepared) => {
            "oasis7.controlled_authority.replicated.replica_prepared.v1"
        }
        (EndpointRole::Primary, ReceiptStage::DecisionDurable) => {
            "oasis7.controlled_authority.replicated.primary_decision.v1"
        }
        (EndpointRole::Replica, ReceiptStage::DecisionDurable) => {
            "oasis7.controlled_authority.replicated.replica_decision.v1"
        }
    }
}
pub(super) fn hash_hex(s: &str) -> Result<(), ProtocolError> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("canonical hash/key required"));
    }
    Ok(())
}
fn bytes<T: Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut result = (domain.len() as u64).to_be_bytes().to_vec();
    result.extend_from_slice(domain.as_bytes());
    result.extend(serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?);
    Ok(result)
}
pub(super) fn hash<T: Serialize>(domain: &str, value: &T) -> Result<String, ProtocolError> {
    Ok(blake3::hash(&bytes(domain, value)?).to_hex().to_string())
}
fn sign<T: Serialize>(domain: &str, value: &T, key: &SigningKey) -> Result<String, ProtocolError> {
    Ok(hex::encode(key.sign(&bytes(domain, value)?).to_bytes()))
}
fn verify<T: Serialize>(
    domain: &str,
    value: &T,
    signature: &str,
    key: &str,
) -> Result<(), ProtocolError> {
    hash_hex(key)?;
    if signature.len() != 128
        || !signature
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("canonical signature required"));
    }
    let key: [u8; 32] = hex::decode(key)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("key length"))?;
    let signature: [u8; 64] = hex::decode(signature)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("signature length"))?;
    VerifyingKey::from_bytes(&key)
        .map_err(|e| invalid(e.to_string()))?
        .verify_strict(&bytes(domain, value)?, &Signature::from_bytes(&signature))
        .map_err(|e| invalid(format!("receipt/proposal signature: {e}")))
}
impl FixedTrust {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.world_id.trim().is_empty()
            || self.chain_id.trim().is_empty()
            || self.authority_epoch == 0
            || self.primary_id.trim().is_empty()
            || self.replica_id.trim().is_empty()
            || [
                &self.world_id,
                &self.chain_id,
                &self.primary_id,
                &self.replica_id,
            ]
            .iter()
            .any(|s| s.len() > 256)
            || self.primary_id == self.replica_id
            || self.primary_key == self.replica_key
            || self.writer_key == self.primary_key
            || self.writer_key == self.replica_key
        {
            return Err(invalid(
                "fixed identity/positive epoch/distinct writer and endpoint keys/endpoint ids required",
            ));
        }
        for hash in [
            &self.genesis_digest,
            &self.writer_key,
            &self.primary_key,
            &self.replica_key,
        ] {
            hash_hex(hash)?;
        }
        for key in [&self.writer_key, &self.primary_key, &self.replica_key] {
            let bytes: [u8; 32] = hex::decode(key)
                .map_err(|e| invalid(e.to_string()))?
                .try_into()
                .map_err(|_| invalid("key size"))?;
            VerifyingKey::from_bytes(&bytes).map_err(|e| invalid(e.to_string()))?;
        }
        Ok(())
    }
    pub fn genesis_anchor(&self) -> Result<HeadAnchor, ProtocolError> {
        self.validate()?;
        Ok(HeadAnchor {
            position: 0,
            decision_hash: hash("oasis7.replicated.genesis.v1", self)?,
            qualified: false,
        })
    }
    pub(super) fn endpoint(&self, role: EndpointRole) -> (&str, &str) {
        match role {
            EndpointRole::Primary => (&self.primary_id, &self.primary_key),
            EndpointRole::Replica => (&self.replica_id, &self.replica_key),
        }
    }
}
pub(super) fn proposal_hash(p: &SignedProposal) -> Result<String, ProtocolError> {
    hash("oasis7.replicated.decision_hash.v1", p)
}
pub(super) fn sign_proposal(
    body: ProposalBody,
    key: &SigningKey,
) -> Result<SignedProposal, ProtocolError> {
    let signature_hex = sign(PROPOSAL, &body, key)?;
    Ok(SignedProposal {
        body,
        signature_hex,
    })
}
pub(super) fn verify_proposal(
    p: &SignedProposal,
    trust: &FixedTrust,
) -> Result<String, ProtocolError> {
    trust.validate()?;
    let b = &p.body;
    if b.schema_version != 1 || b.scope != EVIDENCE_SCOPE || &b.trust != trust || b.position == 0 {
        return Err(invalid("proposal identity/schema/scope"));
    }
    hash_hex(&b.parent_hash)?;
    b.request.validate().map_err(|e| invalid(e.to_string()))?;
    if [
        &b.request.verified_subject,
        &b.request.operation_domain,
        &b.request.nonce_scope,
        &b.request.request_id,
    ]
    .iter()
    .any(|s| s.len() > 256)
    {
        return Err(invalid("request field length"));
    }
    b.record.validate()?;
    if b.position == 1 && b.parent_hash != trust.genesis_anchor()?.decision_hash {
        return Err(invalid("false genesis parent"));
    }
    verify(PROPOSAL, b, &p.signature_hex, &trust.writer_key)?;
    proposal_hash(p)
}
pub(super) fn sign_receipt(
    trust: &FixedTrust,
    role: EndpointRole,
    stage: ReceiptStage,
    decision_hash: String,
    key: &SigningKey,
) -> Result<DurableReceipt, ProtocolError> {
    let body = ReceiptBody {
        schema_version: 1,
        scope: EVIDENCE_SCOPE.into(),
        trust: trust.clone(),
        role,
        endpoint_id: trust.endpoint(role).0.into(),
        stage,
        decision_hash,
    };
    let signature_hex = sign(receipt_domain(role, stage), &body, key)?;
    Ok(DurableReceipt {
        body,
        signature_hex,
    })
}
pub(super) fn verify_receipt(
    receipt: &DurableReceipt,
    trust: &FixedTrust,
    role: EndpointRole,
    stage: ReceiptStage,
    digest: &str,
) -> Result<(), ProtocolError> {
    let b = &receipt.body;
    if b.schema_version != 1
        || b.scope != EVIDENCE_SCOPE
        || &b.trust != trust
        || b.role != role
        || b.stage != stage
        || b.endpoint_id != trust.endpoint(role).0
        || b.decision_hash != digest
    {
        return Err(invalid("receipt role/stage/identity/digest"));
    }
    verify(
        receipt_domain(role, stage),
        b,
        &receipt.signature_hex,
        trust.endpoint(role).1,
    )
}
/// Trusted endpoint attestations only. Does not prove deployment fault domains,
/// execution correctness, player authorization or activation/formal finality.
pub fn verify_evidence(
    e: &DurabilityEvidence,
    trust: &FixedTrust,
    parent: &HeadAnchor,
) -> Result<HeadAnchor, ProtocolError> {
    let digest = verify_proposal(&e.proposal, trust)?;
    if parent.position == 0 && parent != &trust.genesis_anchor()? {
        return Err(invalid("false external genesis anchor"));
    }
    if parent.position.checked_add(1) != Some(e.proposal.body.position)
        || parent.decision_hash != e.proposal.body.parent_hash
    {
        return Err(invalid("evidence parent/position"));
    }
    verify_receipt(
        &e.replica_prepare,
        trust,
        EndpointRole::Replica,
        ReceiptStage::Prepared,
        &digest,
    )?;
    verify_receipt(
        &e.primary_receipt,
        trust,
        EndpointRole::Primary,
        ReceiptStage::DecisionDurable,
        &digest,
    )?;
    verify_receipt(
        &e.replica_receipt,
        trust,
        EndpointRole::Replica,
        ReceiptStage::DecisionDurable,
        &digest,
    )?;
    Ok(HeadAnchor {
        position: e.proposal.body.position,
        decision_hash: digest,
        qualified: true,
    })
}
