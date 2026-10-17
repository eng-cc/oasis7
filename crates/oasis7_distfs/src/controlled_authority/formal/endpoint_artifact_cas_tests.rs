//! Qualified closure objects are individually readable at their original hashes.
use super::*;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[test]
fn prepared_closed_artifacts_are_durable_raw_cas_and_hash_fail_closed() {
    let (fixture, initial_envelope) = super::business_tests::fixture();
    let mut coordinator = fixture.coordinator();
    let initial_material = fixture.initial.material.clone();
    let initial_proof = coordinator.submit(fixture.initial.clone()).unwrap();
    for object in &initial_material.objects {
        let (primary, replica) = coordinator.endpoints_mut();
        for endpoint in [primary, replica] {
            assert_eq!(
                endpoint.store.raw_object(&object.content_hash).unwrap(),
                Some(object.bytes.clone())
            );
        }
    }
    let parent = verify_formal_qualified(&initial_proof, &fixture.authority, None, None).unwrap();
    let (input, material) = fixture.frozen(&parent, 171);
    let frozen = coordinator.freeze(input, material).unwrap();
    let mut candidate = fixture.candidate(&frozen);
    let mut updates = vec![
        super::super::business_index::BusinessIndexLeafV1 {
            key_digest: super::super::business_index::metadata_key_digest(),
            kind: super::super::business_index::BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![171],
        },
        FormalHistoricalCutV1::from_proof(&initial_proof)
            .unwrap()
            .leaf()
            .unwrap(),
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    let delta = super::super::business_index::BusinessIndexDeltaV1::build(
        initial_envelope.delta.new,
        updates,
        &coordinator.endpoints_mut().0.store,
    )
    .unwrap();
    let envelope = super::business_tests::envelope(delta);
    super::business_tests::replace_result(&mut candidate.material, envelope.encode().unwrap());
    candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
    candidate.candidate.record.result_root = envelope.delta.new_root;
    candidate.candidate.writer_signature_hex = signed(
        canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
        4,
    );
    coordinator
        .endpoints_mut()
        .0
        .prepare(candidate.clone())
        .unwrap();
    for material in std::iter::once(&candidate.material).chain(candidate.frozen_material.iter()) {
        for object in &material.objects {
            assert_eq!(
                coordinator
                    .endpoints_mut()
                    .0
                    .store
                    .raw_object(&object.content_hash)
                    .unwrap(),
                Some(object.bytes.clone())
            );
        }
    }
    drop(coordinator);
    let reopened = fixture.endpoint(EndpointRole::Primary);
    for material in std::iter::once(&candidate.material).chain(candidate.frozen_material.iter()) {
        for object in &material.objects {
            assert_eq!(
                reopened.store.raw_object(&object.content_hash).unwrap(),
                Some(object.bytes.clone())
            );
        }
    }
    let object = &initial_material.objects[0];
    let canonical = fixture
        .pdir
        .join("formal-objects-v1")
        .join(".representation-v1")
        .join(&object.content_hash);
    let path = if canonical.exists() {
        canonical
    } else {
        fixture
            .pdir
            .join("formal-objects-v1")
            .join(".representation-v1")
            .join(&object.content_hash)
    };
    std::fs::remove_file(&path).unwrap();
    assert!(
        reopened
            .store
            .raw_object(&object.content_hash)
            .unwrap()
            .is_none()
    );
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut corrupt = options.open(&path).unwrap();
    corrupt
        .write_all(b"wrong immutable artifact bytes")
        .unwrap();
    corrupt.sync_all().unwrap();
    assert!(reopened.store.raw_object(&object.content_hash).is_err());
}

#[test]
fn closed_artifact_above_business_node_bound_uses_existing_raw_cas() {
    let (fixture, _) = super::business_tests::fixture();
    let endpoint = fixture.endpoint(EndpointRole::Primary);
    let bytes = vec![73; 5 * 1024 * 1024];
    let digest = blake3::hash(&bytes).to_hex().to_string();
    let write = super::super::business_index::BusinessIndexNodeWriteV1 {
        digest: digest.clone(),
        bytes: bytes.clone(),
    };
    endpoint
        .store
        .persist_closed_artifacts(std::slice::from_ref(&write))
        .unwrap();
    endpoint.store.persist_closed_artifacts(&[write]).unwrap();
    drop(endpoint);
    let reopened = fixture.endpoint(EndpointRole::Primary);
    assert_eq!(reopened.store.raw_object(&digest).unwrap(), Some(bytes));
}

#[test]
fn corrupt_existing_candidate_or_frozen_artifact_refuses_prepare_without_promotion() {
    for corrupt_frozen in [false, true] {
        let (fixture, initial_envelope) = super::business_tests::fixture();
        let mut coordinator = fixture.coordinator();
        let initial = coordinator.submit(fixture.initial.clone()).unwrap();
        let parent = verify_formal_qualified(&initial, &fixture.authority, None, None).unwrap();
        let (input, material) = fixture.frozen(&parent, 172);
        let frozen = coordinator.freeze(input, material).unwrap();
        let mut candidate = fixture.candidate(&frozen);
        let mut updates = vec![
            super::super::business_index::BusinessIndexLeafV1 {
                key_digest: super::super::business_index::metadata_key_digest(),
                kind: super::super::business_index::BusinessIndexLeafKindV1::CommitMetadata,
                canonical_value: vec![172],
            },
            FormalHistoricalCutV1::from_proof(&initial)
                .unwrap()
                .leaf()
                .unwrap(),
        ];
        updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
        let delta = super::super::business_index::BusinessIndexDeltaV1::build(
            initial_envelope.delta.new,
            updates,
            &coordinator.endpoints_mut().0.store,
        )
        .unwrap();
        let envelope = super::business_tests::envelope(delta);
        super::business_tests::replace_result(&mut candidate.material, envelope.encode().unwrap());
        candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
        candidate.candidate.record.result_root = envelope.delta.new_root;
        candidate.candidate.writer_signature_hex = signed(
            canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
            4,
        );
        let role = if corrupt_frozen {
            ArtifactRole::Snapshot
        } else {
            ArtifactRole::Result
        };
        let material = if corrupt_frozen {
            candidate.frozen_material.as_ref().unwrap()
        } else {
            &candidate.material
        };
        let hash = &material.roots[&role];
        let path = fixture.pdir.join("formal-objects-v1").join(hash);
        // Replace only this logical link. Do not mutate a shared pack inode.
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove test artifact link: {error}"),
        }
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&path).unwrap();
        file.write_all(b"corrupt already-present logical artifact")
            .unwrap();
        file.sync_all().unwrap();
        let endpoint = coordinator.endpoints_mut().0;
        let before_body = endpoint.store.body.clone();
        let before_checkpoint = endpoint.store.checkpoint_bytes().unwrap();
        let rejected = endpoint.prepare(candidate);
        assert!(
            rejected.is_err(),
            "no signed Prepare receipt for corrupt_frozen={corrupt_frozen}"
        );
        assert_eq!(
            endpoint.store.body, before_body,
            "no hot reservation/head promotion"
        );
        assert_eq!(
            endpoint.store.checkpoint_bytes().unwrap(),
            before_checkpoint,
            "no durable checkpoint promotion"
        );
        let pending: FormalRequestStateV1 = endpoint
            .store
            .get(before_body.pending.as_ref().unwrap())
            .unwrap();
        assert!(
            matches!(pending, FormalRequestStateV1::Frozen { .. }),
            "original remains Frozen rather than Prepared/Ready/Qualified"
        );
    }
}

#[test]
fn independent_published_closure_never_uses_inline_material_or_other_fault_domain() {
    let fixture = Fixture::new();
    let mut coordinator = fixture.coordinator();
    let candidate = fixture.initial.clone();
    let digest = blake3::hash(&serde_cbor::to_vec(&candidate.candidate).unwrap())
        .to_hex()
        .to_string();
    let request = &candidate.candidate.record.request_digest;
    assert!(
        coordinator
            .endpoints_mut()
            .0
            .verify_published_closure(request, &digest)
            .is_err()
    );
    coordinator.submit(candidate.clone()).unwrap();
    let (p, r) = coordinator.endpoints_mut();
    let primary = p.verify_published_closure(request, &digest).unwrap();
    let replica = r.verify_published_closure(request, &digest).unwrap();
    assert_eq!(primary, replica);
    assert!(primary.logical_objects > 0);
    assert!(p.verify_published_closure(request, &h(241)).is_err());
    let object = &candidate.material.objects[0];
    let root = fixture.pdir.join("formal-objects-v1");
    let selected = root.join(".representation-v1").join(&object.content_hash);
    let path = if selected.exists() {
        selected
    } else {
        root.join(&object.content_hash)
    };
    fs::remove_file(path).unwrap();
    // The authenticated Qualified entry still contains complete inline bytes.
    assert!(matches!(
        p.query(request).unwrap(),
        FormalQueryOutcomeV1::Recorded(_)
    ));
    assert!(p.verify_published_closure(request, &digest).is_err());
    assert_eq!(
        r.verify_published_closure(request, &digest).unwrap(),
        replica
    );
}

#[test]
fn storage_mark_protects_frozen_and_history_and_rejects_missing_pending_entry() {
    let fixture = Fixture::new();
    let mut coordinator = fixture.coordinator();
    let proof = coordinator.submit(fixture.initial.clone()).unwrap();
    let parent = verify_formal_qualified(&proof, &fixture.authority, None, None).unwrap();
    let (input, material) = fixture.frozen(&parent, 231);
    let frozen_objects: Vec<_> = material
        .objects
        .iter()
        .map(|o| o.content_hash.clone())
        .collect();
    coordinator.freeze(input, material).unwrap();
    let endpoint = coordinator.endpoints_mut().0;
    let keep = endpoint
        .store
        .verify_storage_roots_durable(1, &endpoint.public_key)
        .unwrap();
    let pending = endpoint.store.body.pending.clone().unwrap();
    assert!(keep.contains(&pending));
    assert!(keep.contains(endpoint.store.body.head_node.as_ref().unwrap()));
    for object in frozen_objects {
        assert!(keep.contains(&object));
    }
    // A well-formed extra object is not a canonical checkpoint root. The
    // independent append-only retention registry still preserves its alias.
    let extra = endpoint
        .store
        .put(&"unreferenced phase/cache fixture")
        .unwrap();
    let keep = endpoint
        .store
        .verify_storage_roots_durable(1, &endpoint.public_key)
        .unwrap();
    assert!(!keep.contains(&extra));
    fs::remove_file(
        endpoint
            .store
            .path
            .join("formal-objects-v1")
            .join(".representation-v1")
            .join(pending),
    )
    .unwrap();
    assert!(
        endpoint
            .store
            .verify_storage_roots_durable(1, &endpoint.public_key)
            .is_err()
    );
}

#[test]
fn pending_published_closure_pin_allows_new_publications_but_blocks_retirement() {
    let fixture = Fixture::new();
    let mut coordinator = fixture.coordinator();
    let candidate = fixture.initial.clone();
    let proof = coordinator.submit(candidate.clone()).unwrap();
    let digest = blake3::hash(&serde_cbor::to_vec(&candidate.candidate).unwrap())
        .to_hex()
        .to_string();
    let proof_digest = blake3::hash(&serde_cbor::to_vec(&proof).unwrap())
        .to_hex()
        .to_string();
    let (primary, _) = coordinator.endpoints_mut();
    let job = super::super::store::published_closure::ClosureJob::new(
        &primary.store,
        &candidate,
        1,
        &digest,
        &proof_digest,
    )
    .unwrap();
    assert!(
        !job.complete(),
        "candidate readback has not completed its first page"
    );
    let stage = (
        "new immutable metadata while older closure is pending",
        418u64,
    );
    let id = primary.store.put(&stage).unwrap();
    assert_eq!(
        primary.store.raw_object(&id).unwrap().unwrap(),
        serde_cbor::to_vec(&stage).unwrap()
    );
    let bytes = b"new independent small CAS node during pinned readback".to_vec();
    let node_id = blake3::hash(&bytes).to_hex().to_string();
    crate::controlled_authority::formal::business_pack::persist(
        &fixture.pdir.join("formal-objects-v1"),
        &[
            crate::controlled_authority::formal::business_index::BusinessIndexNodeWriteV1 {
                digest: node_id.clone(),
                bytes: bytes.clone(),
            },
        ],
    )
    .unwrap();
    assert_eq!(primary.store.raw_object(&node_id).unwrap().unwrap(), bytes);
    primary
        .store
        .verify_published_raw_aliases(&candidate)
        .unwrap();
    assert!(
        matches!(
            primary.collect_physical_orphans(),
            Err(FormalError::Conflict(_))
        ),
        "retirement cannot invalidate this still-live verification cursor"
    );
    drop(job);
    primary.collect_physical_orphans().unwrap();
    assert_eq!(primary.store.raw_object(&node_id).unwrap().unwrap(), bytes);
}
