//! Genuine signed Unknown before publication must reconcile only the original Act.
use super::*;
use oasis7::viewer::ViewerRuntimeLiveServer;

#[test]
fn real_tcp_hosted_act_unpublished_original_replays_exactly_once_canonically() {
    let fixture = Fixture::with_controlled_commits(true);
    let registration = fixture.delegation();
    fixture.client.submit(registration.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        2,
        Some(registration.clone()),
    );
    fixture.committed(&registration);
    let original = cognition_request(&fixture);
    let original_bytes = serde_json::to_vec(&original).unwrap();
    // No Submit has happened: this covers a durable checkpoint before network I/O.
    let response = ViewerRuntimeLiveServer::test_replay_original_hosted_act(
        fixture.client.clone(),
        original.clone(),
    )
    .expect("verified unpublished original must replay exact signed request");
    assert!(matches!(response, SubmitObservation::Response(r)
        if matches!(r.outcome, IntentOutcome::Received { .. })));
    // A volatile received admission still has no published canonical receipt.
    // Its genuine signed Unknown must also reconcile without inventing a new key.
    let response = ViewerRuntimeLiveServer::test_replay_original_hosted_act(
        fixture.client.clone(),
        original.clone(),
    )
    .expect("received but unpublished original must reconcile");
    assert!(matches!(response, SubmitObservation::Response(r)
        if matches!(r.outcome, IntentOutcome::Received { .. })));
    assert_eq!(serde_json::to_vec(&original).unwrap(), original_bytes);
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        3,
        Some(original.clone()),
    );
    let committed = fixture.committed(&original);
    let first = fixture
        .client
        .lookup(
            LookupIntentRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                key: original.correlation.key.clone(),
            },
            original.signed_payload.clone(),
        )
        .unwrap();
    first.validate(&original.correlation).unwrap();
    let IntentOutcome::Committed { commit, receipt } = first.outcome else {
        panic!("original Act must have genuine canonical receipt")
    };
    assert_eq!(*commit, committed);
    let canonical = fixture.driver.lock().unwrap().execution_world.clone();
    assert_eq!(canonical.agent_causal_receipts("agent-a").unwrap().len(), 1);
    let before = canonical.cognition().clone();
    let replay = fixture.client.submit(original.clone()).unwrap();
    let SubmitObservation::Response(replay) = replay else {
        panic!("completed replay must return original canonical result")
    };
    let IntentOutcome::Committed {
        receipt: replayed_receipt,
        ..
    } = replay.outcome
    else {
        panic!("completed replay must retain committed disposition")
    };
    assert_eq!(replayed_receipt, receipt);
    assert_eq!(
        fixture.driver.lock().unwrap().execution_world.cognition(),
        &before
    );
    println!(
        "PRE2_ACT_UNPUBLISHED_ORIGINAL_REPLAY_PASSED same_signed_bytes=true canonical_effects=1"
    );
}
