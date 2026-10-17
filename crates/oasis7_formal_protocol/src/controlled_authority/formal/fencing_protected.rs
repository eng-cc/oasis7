//! Nonexportable endpoint signing under the SAME monotonic/exclusive lease.
//! Implementations must deny new mutation signatures after seal, and retain the
//! exact native metadata bound to their floor. Local file permissions are not
//! protected custody. These are independently pinned provider assertions.
use super::*;
const ATTEST: &str = "oasis7.formal.protected_endpoint_signature.v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectedEndpointSignaturePurposeV1 {
    NativeCheckpoint,
    FrozenReceipt,
    PrepareReceipt,
    FinalReceipt,
    ProofComplete,
    EpochSeal,
    RpcResponse,
}
impl ProtectedEndpointSignaturePurposeV1 {
    pub fn domain(&self) -> &'static str {
        match self {
            Self::NativeCheckpoint => "oasis7.formal.endpoint_checkpoint.sign.v1",
            Self::FrozenReceipt => "oasis7.formal.frozen_receipt.sign.v1",
            Self::PrepareReceipt => "oasis7.formal.prepare_receipt.sign.v1",
            Self::FinalReceipt => "oasis7.formal.final_receipt.sign.v1",
            Self::ProofComplete => "oasis7.formal.proof_complete.sign.v1",
            Self::EpochSeal => SEAL_DOMAIN,
            Self::RpcResponse => "oasis7.formal.endpoint.rpc.response.v1",
        }
    }
    pub fn mutation(&self) -> bool {
        !matches!(self, Self::EpochSeal | Self::RpcResponse)
    }
}
fn serialize_preimage<S: serde::Serializer>(
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(bytes)
}
fn deserialize_preimage<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Vec<u8>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded signing preimage byte string")
        }
        fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Vec<u8>, E> {
            if bytes.len() > 64 * 1024 {
                return Err(E::custom("protected signature preimage bound"));
            }
            Ok(bytes.to_vec())
        }
        fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<Vec<u8>, E> {
            if bytes.len() > 64 * 1024 {
                return Err(E::custom("protected signature preimage bound"));
            }
            Ok(bytes)
        }
    }
    decoder.deserialize_byte_buf(Visitor)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEndpointSigningRequestV1 {
    pub schema_version: u32,
    pub instance_digest: String,
    pub checkpoint: EndpointCheckpointBindingV1,
    pub challenge: String,
    pub purpose: ProtectedEndpointSignaturePurposeV1,
    #[serde(
        serialize_with = "serialize_preimage",
        deserialize_with = "deserialize_preimage"
    )]
    pub preimage: Vec<u8>,
}
impl ProtectedEndpointSigningRequestV1 {
    pub fn validate(&self) -> Result<(), FencingError> {
        canonical_hash(&self.instance_digest)?;
        canonical_hash(&self.challenge)?;
        self.checkpoint.validate()?;
        let domain = self.purpose.domain();
        let prefix = bytes(domain, &())?;
        let prefix = &prefix[..8 + domain.len()];
        if self.schema_version != 1
            || self.preimage.len() > 64 * 1024
            || !self.preimage.starts_with(prefix)
        {
            return Err(FencingError::FencingUnproven);
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String, FencingError> {
        self.validate()?;
        hash_value(self)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectedSignerCustodyV1 {
    NonexportableLeaseBoundV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEndpointSignatureBodyV1 {
    pub schema_version: u32,
    pub custody: ProtectedSignerCustodyV1,
    pub instance_digest: String,
    pub resource_binding: String,
    pub checkpoint: EndpointCheckpointBindingV1,
    pub request_digest: String,
    pub challenge: String,
    pub endpoint_signature: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEndpointSignatureV1 {
    pub body: ProtectedEndpointSignatureBodyV1,
    pub provider_signature: String,
}
pub fn protected_endpoint_signature_signing_bytes(
    body: &ProtectedEndpointSignatureBodyV1,
) -> Result<Vec<u8>, FencingError> {
    bytes(ATTEST, body)
}
pub fn verify_protected_endpoint_signature(
    signature: &ProtectedEndpointSignatureV1,
    request: &ProtectedEndpointSigningRequestV1,
    instance: &EndpointInstanceBindingV1,
    policy: &TrustedFencingPolicyV1,
) -> Result<String, FencingError> {
    request.validate()?;
    verify(
        ATTEST,
        &signature.body,
        &signature.provider_signature,
        &policy.provider_public_key,
    )?;
    let body = &signature.body;
    if body.schema_version != 1
        || body.custody != ProtectedSignerCustodyV1::NonexportableLeaseBoundV1
        || body.instance_digest != instance.digest()?
        || body.instance_digest != request.instance_digest
        || body.checkpoint != request.checkpoint
        || body.challenge != request.challenge
        || body.request_digest != request.digest()?
        || !policy
            .authorized_resource_bindings
            .contains(&body.resource_binding)
    {
        return Err(FencingError::FencingUnproven);
    }
    let public: [u8; 32] = hex::decode(&instance.endpoint_public_key)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("protected signer key"))?;
    let key = VerifyingKey::from_bytes(&public).map_err(|e| invalid(e.to_string()))?;
    let signature_bytes: [u8; 64] = hex::decode(&body.endpoint_signature)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("protected endpoint signature"))?;
    key.verify_strict(&request.preimage, &Signature::from_bytes(&signature_bytes))
        .map_err(|_| FencingError::FencingUnproven)?;
    Ok(body.endpoint_signature.clone())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEndpointRegistrationV1 {
    pub authority_id: String,
    pub activation_ref: String,
    pub unsigned_checkpoint_preimage: Vec<u8>,
    pub existing_checkpoint: Option<Vec<u8>>,
}
