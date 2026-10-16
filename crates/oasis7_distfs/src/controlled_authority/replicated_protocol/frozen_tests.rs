use super::tests::{coordinator, qualified, record, request, root, trust};
use super::*;

#[test]
#[cfg(unix)]
fn completed_frozen_slot_binds_candidate_and_survives_publication_and_reopen() {
    let directory = root();
    let mut c = coordinator(&directory);
    let genesis = trust().genesis_anchor().unwrap();
    let predecessor = qualified(c.submit(4, &genesis, request("before"), record()).unwrap());
    let parent = verify_evidence(&predecessor, &trust(), &genesis).unwrap();
    let input = &predecessor.proposal.body.record;
    let body = FrozenInputBody {
        schema_version: 1,
        scope: EVIDENCE_SCOPE.into(),
        trust: trust(),
        parent: parent.clone(),
        next_position: parent.position + 1,
        request: request("frozen"),
        predecessor_record: input.clone(),
        execution_binding: "a".repeat(64),
        continuation_bytes: b"original continuation".to_vec(),
        action_bytes: input
            .objects
            .iter()
            .find(|o| o.content_hash == input.payload_digest)
            .unwrap()
            .bytes
            .clone(),
    };
    let FrozenOutcome::Complete(certificate) = c.freeze_input(body.clone()).unwrap() else {
        panic!("not frozen");
    };
    let FrozenOutcome::Complete(retry) = c.freeze_input(body.clone()).unwrap() else {
        panic!("not frozen");
    };
    assert_eq!(retry, certificate);
    let mut competing = body.clone();
    competing.request = request("competitor");
    assert!(c.freeze_input(competing).is_err());
    let mut candidate = record();
    candidate.before_state_root = input.after_state_root.clone();
    let evidence = qualified(
        c.submit(4, &parent, request("frozen"), candidate.clone())
            .unwrap(),
    );
    assert_eq!(
        evidence.frozen_certificate.as_ref(),
        Some(certificate.as_ref())
    );
    assert_eq!(evidence.proposal.body.schema_version, 2);
    let mut removed = evidence.clone();
    removed.frozen_certificate = None;
    assert!(verify_evidence(&removed, &trust(), &parent).is_err());
    let head = verify_evidence(&evidence, &trust(), &parent).unwrap();
    let FrozenOutcome::Complete(archived_retry) = c.freeze_input(body.clone()).unwrap() else {
        panic!("archived frozen retry changed");
    };
    assert_eq!(archived_retry, certificate);
    drop(c);
    let (p, r) = super::tests::endpoints(&directory, &head, &head);
    let mut reopened = ReplicatedCoordinator::new(p, r, super::tests::key(1)).unwrap();
    assert_eq!(
        qualified(
            reopened
                .submit(4, &parent, request("frozen"), candidate)
                .unwrap()
        ),
        evidence
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(unix)]
fn partial_freeze_cannot_execute_or_accept_an_unbound_candidate() {
    let directory = root();
    let mut c = coordinator(&directory);
    let genesis = trust().genesis_anchor().unwrap();
    let before = qualified(c.submit(4, &genesis, request("before"), record()).unwrap());
    let head = verify_evidence(&before, &trust(), &genesis).unwrap();
    let body = FrozenInputBody {
        schema_version: 1,
        scope: EVIDENCE_SCOPE.into(),
        trust: trust(),
        parent: head.clone(),
        next_position: 2,
        request: request("frozen"),
        predecessor_record: before.proposal.body.record.clone(),
        execution_binding: "a".repeat(64),
        continuation_bytes: vec![1],
        action_bytes: vec![2],
    };
    let signed = frozen::SignedFrozenInput::sign(body.clone(), &super::tests::key(1)).unwrap();
    let (mut p, mut r) = c.into_endpoints();
    p.freeze_input(&signed).unwrap();
    let mut proposal = super::tests::proposal("frozen", &head);
    proposal.body.schema_version = 2;
    proposal.body.frozen_input_digest = Some(signed.verify(&trust()).unwrap());
    proposal = crypto::sign_proposal(proposal.body, &super::tests::key(1)).unwrap();
    assert!(p.prepare(&proposal).is_err());
    let replica_receipt = r.freeze_input(&signed).unwrap();
    let certificate = FrozenCertificate {
        primary_receipt: p.freeze_input(&signed).unwrap(),
        input: signed,
        replica_receipt,
    };
    p.complete_frozen(&certificate).unwrap();
    r.complete_frozen(&certificate).unwrap();
    // Frozen action bytes differ from the candidate's input; dual acknowledgements
    // cannot authorize replacing the original action or predecessor.
    assert!(p.prepare(&proposal).is_err());
    assert!(r.prepare(&proposal).is_err());
    assert_eq!(p.head(), head);
    assert_eq!(r.head(), head);
    drop((p, r));
    std::fs::remove_dir_all(directory).unwrap();
}
