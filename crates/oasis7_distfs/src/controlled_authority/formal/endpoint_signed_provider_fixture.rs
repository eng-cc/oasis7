//! Signed provider-contract simulation for native endpoint tests. This owns
//! fixture keys and checks the actual local ledger/lease, but makes no claim of
//! physical fault independence or deployed nonexportable custody.
use super::*;
use fencing::{MonotonicEndpointGuard, MonotonicEndpointProvider};

pub(super) struct SignedTestProvider {
    inner: fencing::ControlledFileProvider,
    key: SigningKey,
}
impl SignedTestProvider {
    pub(super) fn new(path: impl AsRef<Path>, key: SigningKey) -> Self {
        Self {
            inner: fencing::ControlledFileProvider::new(path, key.clone()).unwrap(),
            key,
        }
    }
    pub(super) fn resource_binding(&self) -> &str {
        self.inner.resource_binding()
    }
}
struct SignedTestGuard {
    inner: Box<dyn MonotonicEndpointGuard>,
    key: SigningKey,
}
impl MonotonicEndpointProvider for SignedTestProvider {
    fn acquire(
        &self,
        instance: &fencing::EndpointInstanceBindingV1,
        initial: &fencing::EndpointCheckpointBindingV1,
    ) -> Result<Box<dyn MonotonicEndpointGuard>, fencing::FencingError> {
        Ok(Box::new(SignedTestGuard {
            inner: self.inner.acquire(instance, initial)?,
            key: self.key.clone(),
        }))
    }
}
impl MonotonicEndpointGuard for SignedTestGuard {
    fn checkpoint(&self) -> &fencing::EndpointCheckpointBindingV1 {
        self.inner.checkpoint()
    }
    fn pending(&self) -> Option<&fencing::CheckpointIntentV1> {
        self.inner.pending()
    }
    fn seal(&self) -> Option<&fencing::EpochSealBodyV1> {
        self.inner.seal()
    }
    fn begin(&mut self, intent: fencing::CheckpointIntentV1) -> Result<(), fencing::FencingError> {
        self.inner.begin(intent)
    }
    fn finish(&mut self) -> Result<(), fencing::FencingError> {
        self.inner.finish()
    }
    fn evidence(&self) -> Result<fencing::MonotonicProviderEvidenceV1, fencing::FencingError> {
        let mut evidence = self.inner.evidence()?;
        evidence.statement.capability =
            fencing::ProviderCapabilityV1::MonotonicExclusiveSignerStorageV1;
        evidence.signature =
            fencing::sign_fencing_body(fencing::PROVIDER_DOMAIN, &evidence.statement, &self.key)?;
        Ok(evidence)
    }
}
