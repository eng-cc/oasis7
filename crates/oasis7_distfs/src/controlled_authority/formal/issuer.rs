//! Recovery issuance is durable before propagation and depends on a separately
//! trusted, nonrollback witness. A local file lock is not that witness.
use super::{
    FormalError, RecoveryPolicyV1, SignedRecoveryCertificateV1, VerifiedRecoveryIntent,
    recovery_certificate_digest, recovery_signing_bytes,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const REQUEST_DOMAIN: &str = "oasis7.formal.issuer.request.v1";
const RECEIPT_DOMAIN: &str = "oasis7.formal.issuer.witness.v1";
const MAX_BYTES: usize = 256 * 1024;
#[path = "issuer_query.rs"]
mod query;
pub use query::*;
fn invalid(s: impl Into<String>) -> FormalError {
    FormalError::Invalid(s.into())
}
fn io(e: std::io::Error) -> FormalError {
    FormalError::Io(e.to_string())
}
fn encoded<T: Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, FormalError> {
    let mut out = (domain.len() as u64).to_be_bytes().to_vec();
    out.extend(domain.as_bytes());
    out.extend(serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?);
    if out.len() > MAX_BYTES {
        return Err(invalid("issuer material byte bound"));
    }
    Ok(out)
}
fn digest<T: Serialize>(domain: &str, value: &T) -> Result<String, FormalError> {
    Ok(blake3::hash(&encoded(domain, value)?).to_hex().to_string())
}
fn canonical<const N: usize>(s: &str) -> Result<[u8; N], FormalError> {
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
fn verify<T: Serialize>(
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
fn policy_binding(policy: &RecoveryPolicyV1) -> Result<String, FormalError> {
    digest("oasis7.formal.issuer.policy.v1", policy)
}
#[cfg(test)]
pub(crate) fn test_policy_binding(policy: &RecoveryPolicyV1) -> Result<String, FormalError> {
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
    fn validate(&self) -> Result<(), FormalError> {
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
    fn validate(&self) -> Result<(), FormalError> {
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    policy: RecoveryPolicyV1,
    certificate: SignedRecoveryCertificateV1,
}
/// The store holds one immutable intent per predecessor. Its lock prevents
/// local races; all cross-instance uniqueness comes from IssuanceWitness.
pub struct RecoveryIssuer {
    directory: PathBuf,
    policy: RecoveryPolicyV1,
    key: SigningKey,
    minimum: IssuerAnchorV1,
    _lock: File,
    poisoned: bool,
}
impl RecoveryIssuer {
    pub fn open(
        directory: &Path,
        policy: RecoveryPolicyV1,
        key: SigningKey,
        minimum: IssuerAnchorV1,
    ) -> Result<Self, FormalError> {
        if hex::encode(key.verifying_key().to_bytes()) != policy.recovery_issuer_public_key {
            return Err(invalid("recovery issuer signer mismatch"));
        }
        minimum.validate()?;
        if minimum.revision == 0 && minimum != IssuerAnchorV1::initial(&policy)? {
            return Err(FormalError::Rollback(
                "issuer initial anchor differs from policy".into(),
            ));
        }
        canonical::<32>(&policy.issuer_anchor_public_key)?;
        if policy.issuer_anchor_public_key == policy.recovery_issuer_public_key {
            return Err(invalid("issuer witness must be independently keyed"));
        }
        private_directory(directory)?;
        let lock_path = directory.join("issuer.lock");
        if lock_path.exists() {
            private_regular(&lock_path)?;
        }
        let lock = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(io)?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: owns this open file descriptor for the store lifetime.
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(FormalError::Conflict("issuer store is locked".into()));
            }
        }
        #[cfg(not(unix))]
        {
            let _ = lock;
            return Err(FormalError::IssuerUnproven);
        }
        Ok(Self {
            directory: directory.into(),
            policy,
            key,
            minimum,
            _lock: lock,
            poisoned: false,
        })
    }
    pub fn issue(
        &mut self,
        intent: &VerifiedRecoveryIntent,
        witness: &mut dyn IssuanceWitness,
    ) -> Result<
        (
            SignedRecoveryCertificateV1,
            SignedIssuanceWitnessV1,
            VerifiedIssuance,
        ),
        FormalError,
    > {
        if self.poisoned {
            return Err(FormalError::Io(
                "issuer requires reopen and original reconciliation".into(),
            ));
        }
        if intent.policy() != &self.policy {
            return Err(invalid("issuer original policy changed"));
        }
        let body = intent.body().clone();
        canonical::<32>(&body.predecessor_authority_id)?;
        let signature_hex = hex::encode(self.key.sign(&recovery_signing_bytes(&body)?).to_bytes());
        let certificate = SignedRecoveryCertificateV1 {
            body,
            signature_hex,
        };
        let path = self.directory.join(format!(
            "{}.intent.json",
            certificate.body.predecessor_authority_id
        ));
        let query = SignedIssuerGrantQueryV1::new(
            &self.policy,
            &self.key,
            certificate.body.predecessor_authority_id.clone(),
            self.minimum.clone(),
        )?;
        let existing = witness.query_predecessor(&query)?;
        existing.verify(&query, &self.policy)?;
        self.minimum = existing.body.current_anchor.clone();
        if let Some(grant) = existing.body.grant {
            // An external original grant can repair a stale or absent local
            // intent. Preserve a conflicting ungranted intent as diagnostics.
            if path.exists() {
                let old: Intent = read_bounded(&path)?;
                if old.certificate != grant.certificate {
                    let diagnostic = self.directory.join(format!(
                        "{}.orphan-intent.json",
                        recovery_certificate_digest(&old.certificate)?
                    ));
                    let bytes = serde_json::to_vec(&old).map_err(|e| invalid(e.to_string()))?;
                    if let Err(error) = atomic(&self.directory, &diagnostic, &bytes) {
                        self.poisoned = true;
                        return Err(error);
                    }
                }
            }
            let bytes = serde_json::to_vec(&Intent {
                policy: self.policy.clone(),
                certificate: grant.certificate.clone(),
            })
            .map_err(|e| invalid(e.to_string()))?;
            if let Err(error) = atomic(&self.directory, &path, &bytes) {
                self.poisoned = true;
                return Err(error);
            }
            if grant.certificate != certificate {
                return Err(FormalError::Conflict("external witness retains the original recovery certificate; query and continue that operation".into()));
            }
        }
        if path.exists() {
            let old: Intent = read_bounded(&path)?;
            if old.policy != self.policy || old.certificate != certificate {
                return Err(FormalError::Conflict(
                    "predecessor already has another immutable recovery intent".into(),
                ));
            }
        } else {
            let bytes = serde_json::to_vec(&Intent {
                policy: self.policy.clone(),
                certificate: certificate.clone(),
            })
            .map_err(|e| invalid(e.to_string()))?;
            if let Err(error) = atomic(&self.directory, &path, &bytes) {
                self.poisoned = true;
                return Err(error);
            }
        }
        let mut nonce = [0; 32];
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
        let request_body = IssuanceRequestBodyV1 {
            schema_version: 1,
            issuer_public_key: self.policy.recovery_issuer_public_key.clone(),
            policy_binding: policy_binding(&self.policy)?,
            predecessor_authority_id: certificate.body.predecessor_authority_id.clone(),
            certificate_digest: recovery_certificate_digest(&certificate)?,
            observation_nonce: hex::encode(nonce),
            minimum_anchor: self.minimum.clone(),
        };
        let request = SignedIssuanceRequestV1 {
            signature_hex: hex::encode(
                self.key
                    .sign(&encoded(REQUEST_DOMAIN, &request_body)?)
                    .to_bytes(),
            ),
            body: request_body,
        };
        // Any lost reply is reconciled by the same immutable intent; a new nonce
        // is a fresh read challenge, never a new recovery operation identity.
        let receipt = witness.authorize_or_query(&request, &certificate)?;
        if receipt.body.request != request.body {
            return Err(invalid("stale or wrong issuer witness response"));
        }
        let verified = verify_issuance_witness(&receipt, &certificate, &self.policy)?;
        let result_path = self.directory.join(format!(
            "{}.result.json",
            certificate.body.predecessor_authority_id
        ));
        let bytes =
            serde_json::to_vec(&(&certificate, &receipt)).map_err(|e| invalid(e.to_string()))?;
        if let Err(error) = atomic(&self.directory, &result_path, &bytes) {
            self.poisoned = true;
            return Err(error);
        }
        self.minimum = receipt.body.current_anchor.clone();
        Ok((certificate, receipt, verified))
    }
    pub fn minimum_anchor(&self) -> &IssuerAnchorV1 {
        &self.minimum
    }
}
fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
}
fn private_regular(path: &Path) -> Result<(), FormalError> {
    let meta = fs::symlink_metadata(path).map_err(io)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(invalid("issuer regular private file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // SAFETY: geteuid only reads this process identity.
        if meta.uid() != unsafe { libc::geteuid() } || meta.permissions().mode() & 0o777 != 0o600 {
            return Err(invalid("issuer private file owner/mode"));
        }
    }
    Ok(())
}
fn private_directory(path: &Path) -> Result<(), FormalError> {
    if !path.is_absolute() {
        return Err(invalid("issuer directory must be absolute"));
    }
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(m) if m.file_type().is_symlink() => return Err(invalid("issuer directory symlink")),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io(e)),
        }
    }
    if !path.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(path).map_err(io)?;
        if let Some(parent) = path.parent() {
            File::open(parent).and_then(|f| f.sync_all()).map_err(io)?;
        }
    }
    let meta = fs::symlink_metadata(path).map_err(io)?;
    if !meta.is_dir() {
        return Err(invalid("issuer directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // SAFETY: geteuid only reads this process identity.
        if meta.uid() != unsafe { libc::geteuid() } || meta.permissions().mode() & 0o777 != 0o700 {
            return Err(invalid("issuer directory owner/mode"));
        }
    }
    Ok(())
}
fn read_bounded<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, FormalError> {
    private_regular(path)?;
    let mut bytes = Vec::new();
    private_options()
        .read(true)
        .open(path)
        .map_err(io)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid("issuer file byte bound"));
    }
    serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
}
fn atomic(directory: &Path, path: &Path, bytes: &[u8]) -> Result<(), FormalError> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid("issuer file byte bound"));
    }
    if path.exists() {
        private_regular(path)?;
    }
    let mut nonce = [0u8; 16];
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
    let tmp = directory.join(format!(
        ".issuer-{}-{}.tmp",
        std::process::id(),
        hex::encode(nonce)
    ));
    let mut file = private_options()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(io)?;
    let result = (|| {
        file.write_all(bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        fs::rename(&tmp, path).map_err(io)?;
        File::open(directory).and_then(|f| f.sync_all()).map_err(io)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
#[path = "issuer_tests.rs"]
mod tests;
