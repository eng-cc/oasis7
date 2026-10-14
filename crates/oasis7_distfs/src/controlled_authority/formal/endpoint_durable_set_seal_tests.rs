//! Real native stores and remote protected-signature protocol. The oracle's
//! keys/floor are local test simulations, not physical nonrollback evidence.
use super::*;
fn run_healthy_seal(fault: Option<u8>) {
    let fixture = Fixture::new();
    let po = Oracle::start(&fixture, EndpointRole::Primary);
    let ro = Oracle::start(&fixture, EndpointRole::Replica);
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
        open(EndpointRole::Primary, &po),
        open(EndpointRole::Replica, &ro),
        fixture.authority.clone(),
        fixture.authority.clone(),
        None,
        None,
    )
    .unwrap();
    let initial = coordinator.submit(fixture.initial.clone()).unwrap();
    let parent = verify_formal_qualified(&initial, &fixture.authority, None, None).unwrap();
    let mut successor = fixture.authority.definition().clone();
    successor.epoch = 2;
    successor.valid_from_position = 3;
    successor.predecessor_authority_id = Some(fixture.authority.authority_id().unwrap());
    successor.primary.instance.authority_epoch = 2;
    successor.replica.instance.authority_epoch = 2;
    successor.replica.instance.endpoint_public_key = public(12);
    successor.replica.instance.instance_id = h(120);
    successor.replica.instance.storage_id = h(121);
    successor.replica.instance.storage_fingerprint = h(122);
    successor.replica.instance.endpoint_id = "healthy-new-r".into();
    successor.replica.fault_domain_id = "healthy-new-r-domain".into();
    let mut body = NormalDurableSetTransitionBodyV1 {
        schema_version: 1,
        world: successor.world.clone(),
        predecessor_authority_id: fixture.authority.authority_id().unwrap(),
        predecessor: parent.head(),
        boundary_position: 2,
        operation_id: h(123),
        successor,
        independent_closure_root: h(124),
    };
    body.independent_closure_root = body.copy_contract_digest().unwrap();
    let signed_transition = SignedNormalDurableSetTransitionV1 {
        writer_signature_hex: signed(
            normal_durable_set_transition_signing_bytes(&body).unwrap(),
            4,
        ),
        body,
    };
    let (input, material) = fixture.frozen(&parent, 61);
    let executable = coordinator.freeze(input.clone(), material.clone()).unwrap();
    let mut candidate = fixture.candidate(&executable);
    candidate.candidate.record.schema_version = 2;
    candidate.candidate.record.durable_set_transition_digest =
        Some(signed_transition.body.digest().unwrap());
    candidate.candidate.writer_signature_hex = signed(
        canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
        4,
    );
    // Both native endpoints really persist Prepare/Decision/ProofComplete.
    // The Qualified envelope is valid, but neither local head is published yet.
    let (p, r) = coordinator.endpoints_mut();
    let pr = p.prepare(candidate.clone()).unwrap();
    let rr = r.prepare(candidate.clone()).unwrap();
    let request = candidate.candidate.record.request_digest.clone();
    let pf = p.decide(&request, pr.clone(), rr.clone()).unwrap();
    let rf = r.decide(&request, pr, rr).unwrap();
    let decision = FormalDecisionProofV1 {
        schema_version: 1,
        record: candidate.candidate.record.clone(),
        branch: candidate.branch,
        writer_signature_hex: candidate.candidate.writer_signature_hex.clone(),
        frozen: candidate.frozen.clone(),
        primary: pf,
        replica: rf,
    };
    let proof = FormalQualifiedCommitProofV1 {
        schema_version: 1,
        primary_complete: p.install_proof(decision.clone()).unwrap(),
        replica_complete: r.install_proof(decision.clone()).unwrap(),
        decision,
    };
    let boundary =
        verify_formal_qualified(&proof, &fixture.authority, Some(&parent), None).unwrap();
    let transition = verify_normal_durable_set_transition(
        &signed_transition,
        &fixture.authority,
        &parent,
        &boundary,
    )
    .unwrap();
    let (p, r) = coordinator.endpoints_mut();
    let unpublished_checkpoint = p.checkpoint().unwrap();
    assert!(
        p.verify_durable_set_transition_at_head(&signed_transition)
            .is_err()
    );
    assert!(p.query_durable_set(&transition).is_err());
    assert!(
        p.seal_durable_set(&transition, &unpublished_checkpoint)
            .is_err()
    );
    assert_eq!(p.head().unwrap(), parent.head());
    p.publish_qualified(proof.clone()).unwrap();
    r.publish_qualified(proof).unwrap();
    drop(coordinator);
    let mut primary = open(EndpointRole::Primary, &po);
    let resolved = primary
        .verify_durable_set_transition_at_head(&signed_transition)
        .unwrap();
    assert_eq!(resolved.digest(), transition.digest());
    assert_eq!(resolved.boundary().head(), transition.boundary().head());
    let mut altered = signed_transition.clone();
    altered.body.operation_id = h(125);
    assert!(
        primary
            .verify_durable_set_transition_at_head(&altered)
            .is_err()
    );
    assert!(primary.query_durable_set(&transition).unwrap().is_none());
    let expected = primary.checkpoint().unwrap();
    let mut stale = expected.clone();
    stale.revision = stale.revision.saturating_sub(1);
    assert!(primary.seal_durable_set(&transition, &stale).is_err());
    if fault.is_none() {
        verify_remote_seal_retry(
            primary,
            &fixture,
            signed_transition.clone(),
            expected.clone(),
            None,
        );
        primary = open(EndpointRole::Primary, &po);
    }
    primary.store.fault = fault;
    if fault.is_some() {
        assert!(primary.seal_durable_set(&transition, &expected).is_err());
        assert!(primary.checkpoint().is_err()); // poisoned; never resume old writes.
    } else {
        let seal = primary.seal_durable_set(&transition, &expected).unwrap();
        assert_eq!(seal.body.recovery_operation, transition.digest());
        assert!(seal.protected_signature.is_some());
    }
    drop(primary);
    let mut reopened = open(EndpointRole::Primary, &po);
    assert_eq!(
        reopened
            .verify_durable_set_transition_at_head(&signed_transition)
            .unwrap()
            .digest(),
        transition.digest()
    );
    let recovered = reopened.query_durable_set(&transition).unwrap().unwrap();
    if fault.is_none() {
        verify_remote_seal_retry(
            reopened,
            &fixture,
            signed_transition.clone(),
            expected.clone(),
            Some(&recovered),
        );
        reopened = open(EndpointRole::Primary, &po);
    }
    fencing::verify_epoch_seal(
        &recovered,
        &TrustedFencingPolicyV1 {
            provider_public_key: public(3),
            authorized_resource_bindings: fixture
                .authority
                .policy()
                .authorized_fencing_resource_bindings
                .clone(),
        },
    )
    .unwrap();
    // Replay a request whose response was discarded: immutable body and guard
    // checkpoint stay exactly the original; randomized attestation may differ.
    let replay = reopened.seal_durable_set(&transition, &expected).unwrap();
    assert_eq!(replay.body, recovered.body);
    assert_eq!(replay.provider.statement, recovered.provider.statement);
    let (next_input, next_material) = fixture.frozen(&boundary, 62);
    assert!(reopened.freeze(next_input, next_material).is_err());
    assert_eq!(reopened.head().unwrap(), boundary.head());
    assert!(reopened.seal_epoch(&h(125), 2, &expected).is_err());
}
#[test]
fn native_healthy_durable_set_protected_seal_retries_original_after_lost_reply() {
    run_healthy_seal(None);
}
#[test]
fn native_healthy_durable_set_protected_seal_reconciles_durable_intent_interruptions() {
    for fault in [1, 2] {
        run_healthy_seal(Some(fault));
    }
}

fn verify_remote_seal_retry(
    endpoint: FileFormalEndpoint,
    fixture: &Fixture,
    transition: SignedNormalDurableSetTransitionV1,
    checkpoint: EndpointCheckpointBindingV1,
    original: Option<&EpochSealReceiptV1>,
) {
    use crate::controlled_authority::formal::transport::{
        FormalEndpointServer, RemoteFormalEndpoint,
    };
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = stop.clone();
    let server = std::thread::spawn(move || {
        let mut server = FormalEndpointServer::new(endpoint, Duration::from_secs(3)).unwrap();
        while !server_stop.load(Ordering::Acquire) {
            let result = server.serve_one(&listener);
            if server_stop.load(Ordering::Acquire) {
                break;
            }
            result.unwrap();
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut client = RemoteFormalEndpoint::new(
            address,
            Duration::from_secs(3),
            fixture.authority.clone(),
            EndpointRole::Primary,
            key(4),
            endpoint_head(&checkpoint),
        )
        .unwrap();
        let queried = client.query_durable_set(transition.clone()).unwrap();
        assert_eq!(
            queried.as_ref().map(|seal| &seal.body),
            original.map(|seal| &seal.body)
        );
        let retried = client
            .seal_durable_set(transition.clone(), checkpoint.clone())
            .unwrap();
        if let Some(original) = original {
            assert_eq!(retried.body, original.body);
            assert_eq!(retried.provider.statement, original.provider.statement);
        }
        assert_eq!(
            client.query_durable_set(transition).unwrap().unwrap().body,
            retried.body
        );
    }));
    stop.store(true, Ordering::Release);
    let _ = TcpStream::connect(address);
    server.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

fn endpoint_head(checkpoint: &EndpointCheckpointBindingV1) -> FormalHeadRefV1 {
    FormalHeadRefV1 {
        position: checkpoint.head.position,
        record_id: checkpoint.head.decision_hash.clone(),
    }
}
