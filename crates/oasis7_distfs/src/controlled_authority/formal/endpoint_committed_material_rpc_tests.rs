use super::*;
use crate::controlled_authority::formal::business_index::*;
#[test]
fn delegated_historical_material_rpc_reads_below_native_minimum_without_rewinding_head() {
    let (fixture, initial_envelope) = super::super::business_tests::fixture();
    let primary_oracle = Oracle::start(&fixture, EndpointRole::Primary);
    let replica_oracle = Oracle::start(&fixture, EndpointRole::Replica);
    let open = |role, oracle: &Oracle| {
        FileFormalEndpoint::open_protected(
            if role == EndpointRole::Primary {
                &fixture.pdir
            } else {
                &fixture.rdir
            },
            FormalEndpointAuthorizationV1 {
                authority: fixture.authority.clone(),
                original: fixture.authority.clone(),
                recovery: None,
                parent: None,
            },
            role,
            oracle.provider(&fixture),
            None,
        )
        .unwrap()
    };
    let mut coordinator = FormalCoordinator::new(
        open(EndpointRole::Primary, &primary_oracle),
        open(EndpointRole::Replica, &replica_oracle),
        fixture.authority.clone(),
        fixture.authority.clone(),
        None,
        None,
    )
    .unwrap();
    let first = coordinator.submit(fixture.initial.clone()).unwrap();
    let first_commit = verify_formal_qualified(&first, &fixture.authority, None, None).unwrap();
    let (input, material) = fixture.frozen(&first_commit, 124);
    let frozen = coordinator.freeze(input, material).unwrap();
    let mut candidate = fixture.candidate(&frozen);
    let mut updates = vec![
        BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![124],
        },
        FormalHistoricalCutV1::from_proof(&first)
            .unwrap()
            .leaf()
            .unwrap(),
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    let delta = BusinessIndexDeltaV1::build(
        initial_envelope.delta.new,
        updates,
        &coordinator.endpoints_mut().0.store,
    )
    .unwrap();
    let tree = delta.new.clone();
    let envelope = super::super::business_tests::envelope(delta);
    super::super::business_tests::replace_result(
        &mut candidate.material,
        envelope.encode().unwrap(),
    );
    candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
    candidate.candidate.record.result_root = envelope.delta.new_root;
    candidate.candidate.writer_signature_hex = signed(
        canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
        4,
    );
    let second = coordinator.submit(candidate).unwrap();
    let second_commit =
        verify_formal_qualified(&second, &fixture.authority, Some(&first_commit), None).unwrap();
    let body = fixture.authority.initial.clone().unwrap();
    let certificate = SignedInitialActivationV1 {
        signature_hex: signed(initial_activation_signing_bytes(&body).unwrap(), 9),
        body,
    };
    let trust = InitialActivationTrustV1 {
        issuer_public_key: public(9),
        world: fixture.authority.definition().world.clone(),
        initial_state_root: certificate.body.initial_state_root.clone(),
        governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
        recovery_policy_digest: fixture.authority.policy().digest().unwrap(),
    };
    let anchor = VerifiedFormalReadAnchorV1::from_initial(&certificate, &trust, &first)
        .unwrap()
        .after_verified_commit(fixture.authority.clone(), second_commit.clone())
        .unwrap();
    let historical_key =
        formal_historical_cut_key(&fixture.authority.definition().world, 1).unwrap();
    let inclusion = tree
        .prove(&historical_key, &coordinator.endpoints_mut().0.store)
        .unwrap();
    let cut = VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &inclusion, 1).unwrap();
    // Retain a genuinely dual-complete H+1 candidate before publication. The
    // server will legally publish it after serving restore's initial Head.
    let (input, material) = fixture.frozen(&second_commit, 125);
    let frozen = coordinator.freeze(input, material).unwrap();
    let mut candidate = fixture.candidate(&frozen);
    let mut updates = vec![
        BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![125],
        },
        FormalHistoricalCutV1::from_proof(&second)
            .unwrap()
            .leaf()
            .unwrap(),
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    let delta =
        BusinessIndexDeltaV1::build(tree, updates, &coordinator.endpoints_mut().0.store).unwrap();
    let envelope = super::super::business_tests::envelope(delta);
    super::super::business_tests::replace_result(
        &mut candidate.material,
        envelope.encode().unwrap(),
    );
    candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
    candidate.candidate.record.result_root = envelope.delta.new_root;
    candidate.candidate.writer_signature_hex = signed(
        canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
        4,
    );
    let (primary, replica) = coordinator.endpoints_mut();
    let primary_prepare = primary.prepare(candidate.clone()).unwrap();
    let replica_prepare = replica.prepare(candidate.clone()).unwrap();
    let request = &candidate.candidate.record.request_digest;
    let primary_final = primary
        .decide(request, primary_prepare.clone(), replica_prepare.clone())
        .unwrap();
    let replica_final = replica
        .decide(request, primary_prepare, replica_prepare)
        .unwrap();
    let decision = FormalDecisionProofV1 {
        schema_version: 1,
        record: candidate.candidate.record,
        branch: candidate.branch,
        writer_signature_hex: candidate.candidate.writer_signature_hex,
        frozen: candidate.frozen,
        primary: primary_final,
        replica: replica_final,
    };
    let third = FormalQualifiedCommitProofV1 {
        schema_version: 1,
        primary_complete: primary.install_proof(decision.clone()).unwrap(),
        replica_complete: replica.install_proof(decision.clone()).unwrap(),
        decision,
    };
    let third_commit =
        verify_formal_qualified(&third, &fixture.authority, Some(&second_commit), None).unwrap();
    replica.publish_qualified(third.clone()).unwrap();
    drop(coordinator);
    let primary = open(EndpointRole::Primary, &primary_oracle);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let server_stop = stop.clone();
    let server = std::thread::spawn(move || {
        let mut server = super::super::super::super::transport::FormalEndpointServer::new(
            primary,
            Duration::from_secs(3),
        )
        .unwrap();
        let mut forward = Some(third);
        while !server_stop.load(std::sync::atomic::Ordering::Acquire) {
            if let Err(error) = server.serve_one(&listener) {
                assert!(
                    server_stop.load(std::sync::atomic::Ordering::Acquire),
                    "{error:?}"
                );
                break;
            }
            if let Some(proof) = forward.take() {
                server.publish_qualified_for_test(proof).unwrap();
            }
        }
    });
    let mut client = super::super::super::super::transport::RemoteFormalEndpoint::new(
        address,
        // This is the entire cold restore budget, including every unique
        // sparse-index node, rather than a timeout renewed for each RPC.
        Duration::from_secs(30),
        fixture.authority.clone(),
        EndpointRole::Primary,
        key(4),
        second_commit.head(),
    )
    .unwrap();
    let historical = client
        .restore_native_at(
            &second_commit.head(),
            &fixture.authority,
            &fixture.authority,
            None,
        )
        .unwrap();
    assert!(
        client.last_native_restore_source_reads() < 64,
        "hash-closed target and parent deltas should avoid one RPC per embedded business node"
    );
    assert_eq!(historical.proof(), &second);
    assert_eq!(historical.current_head(), &second_commit.head());
    let restored = client.restore_committed_historical_cut(&cut).unwrap();
    assert_eq!(restored.proof(), &first);
    assert_eq!(restored.commit().head(), first_commit.head());
    assert_eq!(client.head().unwrap(), third_commit.head());
    assert_eq!(anchor.head().head(), second_commit.head());
    fs::remove_file(
        fixture
            .pdir
            .join("formal-objects-v1")
            .join(&initial_envelope.delta.node_writes[0].digest),
    )
    .unwrap();
    assert!(client.restore_committed_historical_cut(&cut).is_err());
    stop.store(true, std::sync::atomic::Ordering::Release);
    let _ = TcpStream::connect(address);
    server.join().unwrap();
}
