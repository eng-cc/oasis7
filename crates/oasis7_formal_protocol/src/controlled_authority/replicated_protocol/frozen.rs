//! Complete replay inputs for the engineering execution protocol. No recovery
//! authorization or formal profile is conferred by a frozen certificate.
use super::*;
use ed25519_dalek::SigningKey;

const INPUT_DOMAIN: &str = "oasis7.replicated.frozen_input.v1";
const RECEIPT_DOMAIN: &str = "oasis7.replicated.frozen_receipt.v1";
const MAX_INPUT: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenInputBody {
    pub schema_version: u32,
    pub scope: String,
    pub trust: FixedTrust,
    pub parent: HeadAnchor,
    pub next_position: u64,
    pub request: LocalRequestIdentity,
    /// Exact closed record of the qualified predecessor, not a mutable cache.
    pub predecessor_record: ClosedRecord,
    pub execution_binding: String,
    #[serde(deserialize_with = "decode_input")]
    pub continuation_bytes: Vec<u8>,
    #[serde(deserialize_with = "decode_input")]
    pub action_bytes: Vec<u8>,
}
fn decode_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    package::bounded_vec::<D, u8, MAX_INPUT>(d)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedFrozenInput {
    pub body: FrozenInputBody,
    pub signature: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenReceiptBody {
    pub trust: FixedTrust,
    pub role: EndpointRole,
    pub endpoint_id: String,
    pub input_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenReceipt {
    pub body: FrozenReceiptBody,
    pub signature: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenCertificate {
    pub input: SignedFrozenInput,
    pub primary_receipt: FrozenReceipt,
    pub replica_receipt: FrozenReceipt,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenReservation {
    pub input: SignedFrozenInput,
    pub local_receipt: Option<FrozenReceipt>,
    pub certificate: Option<FrozenCertificate>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrozenStatus {
    NotRecorded,
    Reserved(Box<FrozenReservation>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrozenOutcome {
    Complete(Box<FrozenCertificate>),
    Unknown { request: LocalRequestIdentity },
}

impl FrozenInputBody {
    pub fn validate(&self, trust: &FixedTrust) -> Result<(), ProtocolError> {
        trust.validate()?;
        self.request
            .validate()
            .map_err(|e| invalid(e.to_string()))?;
        self.predecessor_record.validate()?;
        crypto::hash_hex(&self.execution_binding)?;
        crypto::hash_hex(&self.parent.decision_hash)?;
        if self.schema_version != 1
            || self.scope != EVIDENCE_SCOPE
            || &self.trust != trust
            || !self.parent.qualified
            || self.parent.position == 0
            || self.parent.position.checked_add(1) != Some(self.next_position)
            || self.continuation_bytes.is_empty()
            || self.continuation_bytes.len() > MAX_INPUT
            || self.action_bytes.is_empty()
            || self.action_bytes.len() > MAX_INPUT
        {
            return Err(invalid("frozen input identity/parent/closure bounds"));
        }
        Ok(())
    }
}
impl SignedFrozenInput {
    pub fn sign(body: FrozenInputBody, writer: &SigningKey) -> Result<Self, ProtocolError> {
        body.validate(&body.trust)?;
        if hex::encode(writer.verifying_key().to_bytes()) != body.trust.writer_key {
            return Err(invalid("frozen writer mismatch"));
        }
        let signature = crypto::sign(INPUT_DOMAIN, &body, writer)?;
        Ok(Self { body, signature })
    }
    pub fn verify(&self, trust: &FixedTrust) -> Result<String, ProtocolError> {
        self.body.validate(trust)?;
        crypto::verify(INPUT_DOMAIN, &self.body, &self.signature, &trust.writer_key)?;
        crypto::hash("oasis7.replicated.frozen_identity.v1", self)
    }
}
impl FrozenReceipt {
    pub fn sign(
        input: &SignedFrozenInput,
        role: EndpointRole,
        key: &SigningKey,
    ) -> Result<Self, ProtocolError> {
        let trust = &input.body.trust;
        if hex::encode(key.verifying_key().to_bytes()) != trust.endpoint(role).1 {
            return Err(invalid("frozen endpoint signer mismatch"));
        }
        let body = FrozenReceiptBody {
            trust: trust.clone(),
            role,
            endpoint_id: trust.endpoint(role).0.into(),
            input_digest: input.verify(trust)?,
        };
        let signature = crypto::sign(RECEIPT_DOMAIN, &body, key)?;
        Ok(Self { body, signature })
    }
    pub fn verify(
        &self,
        input: &SignedFrozenInput,
        role: EndpointRole,
    ) -> Result<(), ProtocolError> {
        let trust = &input.body.trust;
        if &self.body.trust != trust
            || self.body.role != role
            || self.body.endpoint_id != trust.endpoint(role).0
            || self.body.input_digest != input.verify(trust)?
        {
            return Err(invalid("frozen receipt binding mismatch"));
        }
        crypto::verify(
            RECEIPT_DOMAIN,
            &self.body,
            &self.signature,
            trust.endpoint(role).1,
        )
    }
}
impl FrozenCertificate {
    pub fn verify(&self, trust: &FixedTrust) -> Result<String, ProtocolError> {
        let digest = self.input.verify(trust)?;
        self.primary_receipt
            .verify(&self.input, EndpointRole::Primary)?;
        self.replica_receipt
            .verify(&self.input, EndpointRole::Replica)?;
        Ok(digest)
    }
    pub fn verify_candidate(&self, proposal: &SignedProposal) -> Result<(), ProtocolError> {
        let body = &proposal.body;
        let frozen = &self.input.body;
        let digest = self.verify(&body.trust)?;
        if body.frozen_input_digest.as_deref() != Some(&digest)
            || body.position != frozen.next_position
            || body.parent_hash != frozen.parent.decision_hash
            || body.request != frozen.request
            || body.record.before_state_root != frozen.predecessor_record.after_state_root
            || body.record.payload_digest != blake3::hash(&frozen.action_bytes).to_hex().as_str()
        {
            return Err(invalid("candidate differs from frozen reservation"));
        }
        Ok(())
    }
}
impl FrozenReservation {
    pub fn validate(&self, trust: &FixedTrust, role: EndpointRole) -> Result<(), ProtocolError> {
        self.input.verify(trust)?;
        if let Some(receipt) = &self.local_receipt {
            receipt.verify(&self.input, role)?;
        }
        if let Some(certificate) = &self.certificate {
            certificate.verify(trust)?;
            if certificate.input != self.input
                || self.local_receipt.as_ref()
                    != Some(if role == EndpointRole::Primary {
                        &certificate.primary_receipt
                    } else {
                        &certificate.replica_receipt
                    })
            {
                return Err(invalid("frozen local receipt/certificate mismatch"));
            }
        }
        Ok(())
    }
}
