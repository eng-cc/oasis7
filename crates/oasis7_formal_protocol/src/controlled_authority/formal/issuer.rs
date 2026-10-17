//! Recovery issuance is durable before propagation and depends on a separately
//! trusted, nonrollback witness. A local file lock is not that witness.
use super::{
    FormalError, RecoveryPolicyV1, SignedRecoveryCertificateV1, recovery_certificate_digest,
    recovery_signing_bytes,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const REQUEST_DOMAIN: &str = "oasis7.formal.issuer.request.v1";
const RECEIPT_DOMAIN: &str = "oasis7.formal.issuer.witness.v1";
const MAX_BYTES: usize = 256 * 1024;
#[path = "issuer_query.rs"]
mod query;
pub use query::*;
pub fn invalid(s: impl Into<String>) -> FormalError {
    FormalError::Invalid(s.into())
}
pub fn encoded<T: Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, FormalError> {
    let mut out = (domain.len() as u64).to_be_bytes().to_vec();
    out.extend(domain.as_bytes());
    out.extend(serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?);
    if out.len() > MAX_BYTES {
        return Err(invalid("issuer material byte bound"));
    }
    Ok(out)
}
pub fn digest<T: Serialize>(domain: &str, value: &T) -> Result<String, FormalError> {
    Ok(blake3::hash(&encoded(domain, value)?).to_hex().to_string())
}
pub fn canonical<const N: usize>(s: &str) -> Result<[u8; N], FormalError> {
    if s.len() != N * 2
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("issuer noncanonical hex"));
    }
    hex::decode(s)
        .map_err(|_| invalid("issuer hex"))?
        .try_into()
        .map_err(|_| invalid("issuer hex size"))
}
pub fn verify<T: Serialize>(
    domain: &str,
    value: &T,
    signature: &str,
    key: &str,
) -> Result<(), FormalError> {
    VerifyingKey::from_bytes(&canonical::<32>(key)?)
        .map_err(|_| invalid("issuer key"))?
        .verify_strict(
            &encoded(domain, value)?,
            &Signature::from_bytes(&canonical::<64>(signature)?),
        )
        .map_err(|_| invalid("issuer/witness signature"))
}
pub fn policy_binding(policy: &RecoveryPolicyV1) -> Result<String, FormalError> {
    digest("oasis7.formal.issuer.policy.v1", policy)
}
#[cfg(any(test, feature = "test-support"))]
pub fn test_policy_binding(policy: &RecoveryPolicyV1) -> Result<String, FormalError> {
    policy_binding(policy)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuerAnchorV1 {
    pub revision: u64,
    pub digest: String,
}
impl IssuerAnchorV1 {
    pub fn initial(policy: &RecoveryPolicyV1) -> Result<Self, FormalError> {
        Ok(Self {
            revision: 0,
            digest: policy_binding(policy)?,
        })
    }
    pub fn validate(&self) -> Result<(), FormalError> {
        canonical::<32>(&self.digest).map(|_| ())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuanceRequestBodyV1 {
    pub schema_version: u32,
    pub issuer_public_key: String,
    pub policy_binding: String,
    pub predecessor_authority_id: String,
    pub certificate_digest: String,
    pub observation_nonce: String,
    pub minimum_anchor: IssuerAnchorV1,
}
impl IssuanceRequestBodyV1 {
    pub fn validate(&self) -> Result<(), FormalError> {
        if self.schema_version != 1 {
            return Err(invalid("issuer request version"));
        }
        for s in [
            &self.issuer_public_key,
            &self.policy_binding,
            &self.predecessor_authority_id,
            &self.certificate_digest,
            &self.observation_nonce,
        ] {
            canonical::<32>(s)?;
        }
        self.minimum_anchor.validate()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIssuanceRequestV1 {
    pub body: IssuanceRequestBodyV1,
    pub signature_hex: String,
}
impl SignedIssuanceRequestV1 {
    pub fn verify(&self) -> Result<(), FormalError> {
        self.body.validate()?;
        verify(
            REQUEST_DOMAIN,
            &self.body,
            &self.signature_hex,
            &self.body.issuer_public_key,
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssuerWitnessCapabilityV1 {
    ControlledLocalTest,
    NonrollbackUniqueSuccessorV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuanceWitnessBodyV1 {
    pub schema_version: u32,
    pub capability: IssuerWitnessCapabilityV1,
    pub request: IssuanceRequestBodyV1,
    /// Stable revision of the original predecessor -> certificate grant.
    pub grant_revision: u64,
    /// Fresh current witness floor; old grants are queried without reissuing.
    pub current_anchor: IssuerAnchorV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIssuanceWitnessV1 {
    pub body: IssuanceWitnessBodyV1,
    pub signature_hex: String,
}
impl SignedIssuanceWitnessV1 {
    /// Signing is a provider assertion, never proof of its physical deployment.
    pub fn sign(body: IssuanceWitnessBodyV1, key: &SigningKey) -> Result<Self, FormalError> {
        body.request.validate()?;
        body.current_anchor.validate()?;
        let signature_hex = hex::encode(key.sign(&encoded(RECEIPT_DOMAIN, &body)?).to_bytes());
        Ok(Self {
            body,
            signature_hex,
        })
    }
}
/// Implementations authenticate the issuer and policy, permanently bind each
/// predecessor to one certificate, and retain original grants across retries.
/// The trusted key must be independent of the failed authority and protected
/// against cloning/rollback. Controlled local providers must label themselves.
pub trait IssuanceWitness: Send {
    fn query_predecessor(
        &mut self,
        query: &SignedIssuerGrantQueryV1,
    ) -> Result<SignedIssuerGrantQueryResultV1, FormalError>;
    fn authorize_or_query(
        &mut self,
        request: &SignedIssuanceRequestV1,
        certificate: &SignedRecoveryCertificateV1,
    ) -> Result<SignedIssuanceWitnessV1, FormalError>;
}
#[derive(Debug, Clone)]
pub struct VerifiedIssuance {
    certificate_digest: String,
    predecessor_authority_id: String,
    anchor_revision: u64,
    policy_binding: String,
    witness_public_key: String,
}
impl VerifiedIssuance {
    pub fn certificate_digest(&self) -> &str {
        &self.certificate_digest
    }
    pub fn predecessor_authority_id(&self) -> &str {
        &self.predecessor_authority_id
    }
    pub fn anchor_revision(&self) -> u64 {
        self.anchor_revision
    }
    pub fn verify_policy(&self, policy: &RecoveryPolicyV1) -> Result<(), FormalError> {
        if self.policy_binding != policy_binding(policy)?
            || self.witness_public_key != policy.issuer_anchor_public_key
        {
            return Err(FormalError::IssuerUnproven);
        }
        Ok(())
    }
}
/// Check an independently configured witness grant. Fresh query callers must
/// additionally compare the exact request including their nonce/minimum.
pub fn verify_issuance_witness(
    witness: &SignedIssuanceWitnessV1,
    certificate: &SignedRecoveryCertificateV1,
    policy: &RecoveryPolicyV1,
) -> Result<VerifiedIssuance, FormalError> {
    let b = &witness.body;
    b.request.validate()?;
    b.current_anchor.validate()?;
    if b.capability != IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1 {
        return Err(FormalError::IssuerUnproven);
    }
    verify(
        RECEIPT_DOMAIN,
        b,
        &witness.signature_hex,
        &policy.issuer_anchor_public_key,
    )?;
    let minimum = &b.request.minimum_anchor;
    if b.schema_version != 1
        || b.grant_revision == 0
        || b.grant_revision > b.current_anchor.revision
        || b.current_anchor.revision < minimum.revision
        || (minimum.revision == 0 && minimum != &IssuerAnchorV1::initial(policy)?)
        || (b.current_anchor.revision == minimum.revision
            && b.current_anchor.digest != minimum.digest)
        || b.request.issuer_public_key != policy.recovery_issuer_public_key
        || b.request.policy_binding != policy_binding(policy)?
        || b.request.predecessor_authority_id != certificate.body.predecessor_authority_id
        || b.request.certificate_digest != recovery_certificate_digest(certificate)?
    {
        return Err(FormalError::Rollback(
            "issuer witness floor or grant binding".into(),
        ));
    }
    VerifyingKey::from_bytes(&canonical::<32>(&policy.recovery_issuer_public_key)?)
        .map_err(|_| invalid("recovery issuer key"))?
        .verify_strict(
            &recovery_signing_bytes(&certificate.body)?,
            &Signature::from_bytes(&canonical::<64>(&certificate.signature_hex)?),
        )
        .map_err(|_| invalid("recovery issuer certificate signature"))?;
    Ok(VerifiedIssuance {
        certificate_digest: b.request.certificate_digest.clone(),
        predecessor_authority_id: b.request.predecessor_authority_id.clone(),
        anchor_revision: b.grant_revision,
        policy_binding: b.request.policy_binding.clone(),
        witness_public_key: policy.issuer_anchor_public_key.clone(),
    })
}
