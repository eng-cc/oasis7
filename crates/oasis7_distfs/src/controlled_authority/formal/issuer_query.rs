use super::*;
const QUERY_DOMAIN: &str = "oasis7.formal.issuer.query.v1";
const QUERY_RESULT_DOMAIN: &str = "oasis7.formal.issuer.query-result.v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuerGrantQueryBodyV1 {
    pub schema_version: u32,
    pub issuer_public_key: String,
    pub policy_binding: String,
    pub predecessor_authority_id: String,
    pub observation_nonce: String,
    pub minimum_anchor: IssuerAnchorV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIssuerGrantQueryV1 {
    pub body: IssuerGrantQueryBodyV1,
    pub signature_hex: String,
}
impl SignedIssuerGrantQueryV1 {
    pub fn verify(&self) -> Result<(), FormalError> {
        if self.body.schema_version != 1 {
            return Err(invalid("issuer query version"));
        }
        for s in [
            &self.body.issuer_public_key,
            &self.body.policy_binding,
            &self.body.predecessor_authority_id,
            &self.body.observation_nonce,
        ] {
            canonical::<32>(s)?;
        }
        self.body.minimum_anchor.validate()?;
        verify(
            QUERY_DOMAIN,
            &self.body,
            &self.signature_hex,
            &self.body.issuer_public_key,
        )
    }
    pub(super) fn new(
        policy: &RecoveryPolicyV1,
        key: &SigningKey,
        predecessor: String,
        minimum: IssuerAnchorV1,
    ) -> Result<Self, FormalError> {
        let mut nonce = [0u8; 32];
        #[cfg(unix)]
        {
            File::open("/dev/urandom")
                .and_then(|mut f| f.read_exact(&mut nonce))
                .map_err(io)?;
        }
        #[cfg(not(unix))]
        {
            return Err(FormalError::IssuerUnproven);
        }
        let body = IssuerGrantQueryBodyV1 {
            schema_version: 1,
            issuer_public_key: policy.recovery_issuer_public_key.clone(),
            policy_binding: policy_binding(policy)?,
            predecessor_authority_id: predecessor,
            observation_nonce: hex::encode(nonce),
            minimum_anchor: minimum,
        };
        let signature_hex = hex::encode(key.sign(&encoded(QUERY_DOMAIN, &body)?).to_bytes());
        Ok(Self {
            body,
            signature_hex,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExistingIssuerGrantV1 {
    pub certificate: SignedRecoveryCertificateV1,
    pub grant_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuerGrantQueryResultBodyV1 {
    pub schema_version: u32,
    pub capability: IssuerWitnessCapabilityV1,
    pub query: IssuerGrantQueryBodyV1,
    pub current_anchor: IssuerAnchorV1,
    /// None asserts no prior grant in the provider's permanent unique index.
    pub grant: Option<ExistingIssuerGrantV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIssuerGrantQueryResultV1 {
    pub body: IssuerGrantQueryResultBodyV1,
    pub signature_hex: String,
}
impl SignedIssuerGrantQueryResultV1 {
    pub fn sign(body: IssuerGrantQueryResultBodyV1, key: &SigningKey) -> Result<Self, FormalError> {
        let signature_hex = hex::encode(key.sign(&encoded(QUERY_RESULT_DOMAIN, &body)?).to_bytes());
        Ok(Self {
            body,
            signature_hex,
        })
    }
    pub(super) fn verify(
        &self,
        query: &SignedIssuerGrantQueryV1,
        policy: &RecoveryPolicyV1,
    ) -> Result<(), FormalError> {
        query.verify()?;
        if self.body.capability != IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1 {
            return Err(FormalError::IssuerUnproven);
        }
        verify(
            QUERY_RESULT_DOMAIN,
            &self.body,
            &self.signature_hex,
            &policy.issuer_anchor_public_key,
        )?;
        self.body.current_anchor.validate()?;
        let minimum = &query.body.minimum_anchor;
        if self.body.schema_version != 1
            || self.body.query != query.body
            || query.body.issuer_public_key != policy.recovery_issuer_public_key
            || query.body.policy_binding != policy_binding(policy)?
            || (minimum.revision == 0 && minimum != &IssuerAnchorV1::initial(policy)?)
            || self.body.current_anchor.revision < minimum.revision
            || (self.body.current_anchor.revision == minimum.revision
                && self.body.current_anchor.digest != minimum.digest)
        {
            return Err(FormalError::Rollback(
                "issuer predecessor query challenge/policy/floor".into(),
            ));
        }
        if let Some(grant) = &self.body.grant {
            if grant.grant_revision == 0
                || grant.grant_revision > self.body.current_anchor.revision
                || grant.certificate.body.predecessor_authority_id
                    != query.body.predecessor_authority_id
            {
                return Err(invalid("issuer original grant identity/revision"));
            }
            VerifyingKey::from_bytes(&canonical::<32>(&policy.recovery_issuer_public_key)?)
                .map_err(|_| invalid("original grant issuer key"))?
                .verify_strict(
                    &recovery_signing_bytes(&grant.certificate.body)?,
                    &Signature::from_bytes(&canonical::<64>(&grant.certificate.signature_hex)?),
                )
                .map_err(|_| invalid("original grant certificate signature"))?;
        }
        Ok(())
    }
}
