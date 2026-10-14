//! Signed pure-protocol lineage tests; these do not claim a native install.
use super::*;
#[test]
fn healthy_endpoint_bootstrap_streams_beyond_one_page_and_rejects_forks_or_missing_pages() {
    let (_, old, initial_proof, initial_commit) = initial();
    let (mut signed, _, _, _) = transition_fixture();
    let mut current = initial_commit.clone();
    let mut history = Vec::new();
    // 128 preceding executions plus H. No counter substitution: every record
    // has real writer, Frozen, dual decision and ProofComplete signatures.
    for index in 0..128 {
        let freeze = unique_frozen(&old, &current, 90 + index);
        let record = execution(&old, &freeze);
        let qualified = proof(
            record,
            &old,
            FormalCompletionBranchV1::Normal,
            None,
            Some(freeze),
            4,
            5,
            6,
        );
        let next = verify_formal_qualified(&qualified, &old, Some(&current), None).unwrap();
        history.push(HealthyEndpointLineageStepV1::Commit(Box::new(qualified)));
        current = next;
    }
    signed.body.predecessor = current.head();
    signed.body.boundary_position = current.head().position + 1;
    signed.body.successor.valid_from_position = signed.body.boundary_position + 1;
    signed.body.independent_closure_root = signed.body.copy_contract_digest().unwrap();
    signed.writer_signature_hex = sign(
        normal_durable_set_transition_signing_bytes(&signed.body).unwrap(),
        4,
    );
    let freeze = unique_frozen(&old, &current, 218);
    let mut record = execution(&old, &freeze);
    record.schema_version = 2;
    record.durable_set_transition_digest = Some(signed.body.digest().unwrap());
    let boundary_proof = proof(
        record,
        &old,
        FormalCompletionBranchV1::Normal,
        None,
        Some(freeze),
        4,
        5,
        6,
    );
    let boundary = verify_formal_qualified(&boundary_proof, &old, Some(&current), None).unwrap();
    let transition =
        verify_normal_durable_set_transition(&signed, &old, &current, &boundary).unwrap();
    let (seals, installs) = signed_activation_fixture(&transition);
    let evidence = DurableSetSealedEvidenceV1 {
        transition: signed.clone(),
        primary_seal: seals[0].clone(),
        replica_seal: seals[1].clone(),
    };
    let pages = vec![
        HealthyEndpointLineagePageV1 {
            schema_version: 1,
            from: initial_commit.head(),
            steps: history,
            to: current.head(),
        },
        HealthyEndpointLineagePageV1 {
            schema_version: 1,
            from: current.head(),
            steps: vec![HealthyEndpointLineageStepV1::Commit(Box::new(
                boundary_proof,
            ))],
            to: boundary.head(),
        },
    ];
    let verified = verify_healthy_endpoint_bootstrap(
        &old,
        &initial_proof,
        &evidence,
        pages.clone().into_iter().map(Ok),
    )
    .unwrap();
    assert_eq!(verified.transition().boundary().head().position, 130);
    for page in &pages {
        let encoded = serde_cbor::to_vec(page).unwrap();
        assert_eq!(
            HealthyEndpointLineagePageV1::decode(&encoded).unwrap(),
            *page
        );
    }
    let mut oversized = pages[0].clone();
    oversized.steps.extend(pages[1].steps.clone());
    assert!(
        HealthyEndpointLineagePageV1::decode(&serde_cbor::to_vec(&oversized).unwrap()).is_err()
    );
    assert!(
        verify_healthy_endpoint_bootstrap(&old, &initial_proof, &evidence, [Ok(pages[0].clone())])
            .is_err()
    );
    let mut wrong = pages.clone();
    wrong[1].from.record_id = h(199);
    assert!(
        verify_healthy_endpoint_bootstrap(
            &old,
            &initial_proof,
            &evidence,
            wrong.into_iter().map(Ok)
        )
        .is_err()
    );
    let mut wrong = pages.clone();
    let HealthyEndpointLineageStepV1::Commit(last) = &mut wrong[1].steps[0] else {
        unreachable!()
    };
    last.decision.record.parent_record_id = h(198);
    **last = proof(
        last.decision.record.clone(),
        &old,
        FormalCompletionBranchV1::Normal,
        None,
        last.decision.frozen.clone(),
        4,
        5,
        6,
    );
    assert!(
        verify_healthy_endpoint_bootstrap(
            &old,
            &initial_proof,
            &evidence,
            wrong.into_iter().map(Ok)
        )
        .is_err()
    );
    let activation = verify_durable_set_activation(
        &transition,
        [&seals[0], &seals[1]],
        [&installs[0], &installs[1]],
    )
    .unwrap();
    activation
        .verify_evidence(
            &signed,
            [&seals[0], &seals[1]],
            [&installs[0], &installs[1]],
        )
        .unwrap();
    let mut wrong = signed;
    wrong.body.operation_id = h(197);
    wrong.writer_signature_hex = sign(
        normal_durable_set_transition_signing_bytes(&wrong.body).unwrap(),
        4,
    );
    assert!(
        activation
            .verify_evidence(&wrong, [&seals[0], &seals[1]], [&installs[0], &installs[1]])
            .is_err()
    );
}

fn unique_frozen(
    authority: &VerifiedAuthorityV1,
    parent: &VerifiedFormalCommitV1,
    request: u8,
) -> FormalFrozenCertificateV1 {
    let mut frozen = frozen(authority, parent, None);
    frozen.reservation.body.request_digest = h(request);
    frozen.reservation.signature_hex =
        sign(frozen_signing_bytes(&frozen.reservation.body).unwrap(), 4);
    let id = frozen.reservation.body.reservation_id().unwrap();
    frozen.primary.body.reservation_id = id.clone();
    frozen.replica.body.reservation_id = id;
    frozen.primary.signature_hex = sign(
        frozen_receipt_signing_bytes(&frozen.primary.body).unwrap(),
        5,
    );
    frozen.replica.signature_hex = sign(
        frozen_receipt_signing_bytes(&frozen.replica.body).unwrap(),
        6,
    );
    frozen
}
