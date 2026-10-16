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
    let path = fixture
        .pdir
        .join("formal-objects-v1")
        .join(&object.content_hash);
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
