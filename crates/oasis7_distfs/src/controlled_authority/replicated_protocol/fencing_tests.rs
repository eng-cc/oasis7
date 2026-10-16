use super::tests::{key, proposal, qualified, record, request, root, trust};
use super::*;
use crate::controlled_authority::fencing::*;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
fn binding(path: &std::path::Path, role: EndpointRole) -> EndpointInstanceBindingV1 {
    EndpointInstanceBindingV1 {
        schema_version: 1,
        world_id: trust().world_id,
        chain_id: trust().chain_id,
        genesis_digest: trust().genesis_digest,
        authority_epoch: 4,
        endpoint_role: role,
        endpoint_id: trust().endpoint(role).0.into(),
        endpoint_public_key: trust().endpoint(role).1.into(),
        instance_id: if role == EndpointRole::Primary {
            "1".repeat(64)
        } else {
            "2".repeat(64)
        },
        storage_id: if role == EndpointRole::Primary {
            "3".repeat(64)
        } else {
            "4".repeat(64)
        },
        storage_fingerprint: directory_fingerprint(path).unwrap(),
    }
}
fn endpoint(
    path: &std::path::Path,
    role: EndpointRole,
    provider: Arc<dyn MonotonicEndpointProvider>,
) -> (FileEndpoint, EndpointInstanceBindingV1) {
    let genesis = trust().genesis_anchor().unwrap();
    let n = if role == EndpointRole::Primary { 2 } else { 3 };
    drop(FileEndpoint::open(path, trust(), role, key(n), &genesis).unwrap());
    let instance = binding(path, role);
    let endpoint = FileEndpoint::open_with_fencing(
        path,
        trust(),
        role,
        key(n),
        &genesis,
        instance.clone(),
        provider,
    )
    .unwrap();
    (endpoint, instance)
}
#[test]
#[cfg(unix)]
fn native_seal_rejects_new_append_clone_and_snapshot_rollback_without_formal_claim() {
    let root = root();
    let provider = Arc::new(ControlledFileProvider::new(root.join("floor"), key(4)).unwrap());
    let (p, instance) = endpoint(&root.join("p"), EndpointRole::Primary, provider.clone());
    let (r, _) = endpoint(&root.join("r"), EndpointRole::Replica, provider.clone());
    let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    let genesis = trust().genesis_anchor().unwrap();
    let evidence = qualified(
        coordinator
            .submit(4, &genesis, request("committed"), record())
            .unwrap(),
    );
    let head = verify_evidence(&evidence, &trust(), &genesis).unwrap();
    let (mut p, r) = coordinator.into_endpoints();
    let original = std::fs::read(root.join("p/endpoint-hot-v2.json")).unwrap();
    let expected = p.checkpoint_binding().unwrap();
    let receipt = p.seal_epoch("a".repeat(64), 5, &expected).unwrap();
    assert_eq!(receipt.body.highest_complete, head);
    assert_eq!(receipt.body.tail.kind, SealedTailKindV1::Empty);
    assert_eq!(p.seal_epoch("a".repeat(64), 5, &expected).unwrap(), receipt);
    assert!(p.seal_epoch("b".repeat(64), 5, &expected).is_err());
    assert!(p.prepare(&proposal("next", &head)).is_err());
    assert_eq!(
        p.decide_primary(&evidence.proposal, &evidence.replica_prepare)
            .unwrap(),
        evidence.primary_receipt
    );
    assert_eq!(
        p.lookup(&request("committed")).unwrap(),
        EndpointStatus::Qualified(Box::new(evidence))
    );
    assert_eq!(
        verify_epoch_seal(
            &receipt,
            &TrustedFencingPolicyV1 {
                provider_public_key: provider.public_key(),
                authorized_resource_bindings: vec![provider.resource_binding().into()]
            }
        )
        .unwrap_err(),
        FencingError::FencingUnproven
    );
    drop((p, r));
    assert!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &head
        )
        .is_err()
    );
    std::fs::write(root.join("p/endpoint-hot-v2.json"), original).unwrap();
    assert!(
        FileEndpoint::open_with_fencing(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &head,
            instance.clone(),
            provider.clone()
        )
        .is_err()
    );
    let copied = root.join("copied");
    std::fs::create_dir(&copied).unwrap();
    assert!(instance.verify_directory(&copied).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
struct FaultProvider {
    inner: Arc<ControlledFileProvider>,
    fault: Arc<AtomicU8>,
}
struct FaultGuard {
    inner: Box<dyn MonotonicEndpointGuard>,
    fault: Arc<AtomicU8>,
}
impl MonotonicEndpointProvider for FaultProvider {
    fn acquire(
        &self,
        instance: &EndpointInstanceBindingV1,
        initial: &EndpointCheckpointBindingV1,
    ) -> Result<Box<dyn MonotonicEndpointGuard>, FencingError> {
        Ok(Box::new(FaultGuard {
            inner: self.inner.acquire(instance, initial)?,
            fault: self.fault.clone(),
        }))
    }
}
impl MonotonicEndpointGuard for FaultGuard {
    fn checkpoint(&self) -> &EndpointCheckpointBindingV1 {
        self.inner.checkpoint()
    }
    fn pending(&self) -> Option<&CheckpointIntentV1> {
        self.inner.pending()
    }
    fn seal(&self) -> Option<&EpochSealBodyV1> {
        self.inner.seal()
    }
    fn begin(&mut self, intent: CheckpointIntentV1) -> Result<(), FencingError> {
        self.inner.begin(intent)?;
        if self
            .fault
            .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Err(FencingError::Io("injected after durable intent".into()));
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), FencingError> {
        if self
            .fault
            .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Err(FencingError::Io(
                "injected before provider completion".into(),
            ));
        }
        self.inner.finish()
    }
    fn evidence(&self) -> Result<MonotonicProviderEvidenceV1, FencingError> {
        self.inner.evidence()
    }
}
#[test]
#[cfg(unix)]
fn provider_intent_windows_recover_exact_original_checkpoint_and_request() {
    for mode in [1, 2, 3] {
        let root = root();
        let fault = Arc::new(AtomicU8::new(0));
        let provider = Arc::new(FaultProvider {
            inner: Arc::new(ControlledFileProvider::new(root.join("floor"), key(4)).unwrap()),
            fault: fault.clone(),
        });
        let path = root.join("endpoint");
        let (mut endpoint, instance) = endpoint(&path, EndpointRole::Primary, provider.clone());
        let genesis = trust().genesis_anchor().unwrap();
        let proposed = proposal("original", &genesis);
        if mode == 3 {
            endpoint.inject_after(1, storage::Fault::Error(storage::Phase::Renamed));
        } else {
            fault.store(mode, Ordering::SeqCst);
        }
        assert!(endpoint.prepare(&proposed).is_err());
        assert!(matches!(
            endpoint.prepare(&proposed),
            Err(ProtocolError::Poisoned)
        ));
        drop(endpoint);
        let mut endpoint = FileEndpoint::open_with_fencing(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis,
            instance,
            provider,
        )
        .unwrap();
        assert_eq!(
            endpoint.lookup(&request("original")).unwrap(),
            EndpointStatus::Prepared(Box::new(proposed.clone()))
        );
        endpoint.prepare(&proposed).unwrap();
        assert!(endpoint.prepare(&proposal("different", &genesis)).is_err());
        drop(endpoint);
        std::fs::remove_dir_all(root).unwrap();
    }
}
