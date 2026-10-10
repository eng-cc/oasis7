use super::*;

#[test]
fn target_actor_memory_intents_require_matching_committed_runtime_receipt_exactly_once() {
    let mut store = MemoryWriteStore::default();
    for status in ["pending", "rejected", "failed"] {
        assert_non_committed_status_does_not_write(status);
    }

    let mut runner = provider_backed_runner();
    runner
        .start_turn_with_context(AGENT_ID, host_context())
        .unwrap();
    let rejected_outcome = completed_turn(&mut runner);
    let mut rejected = rejected_outcome
        .feedback_for_runtime_status("rejected", None)
        .unwrap();
    rejected.provenance = "runtime_authoritative".into();
    rejected.reject_reason = Some("stale_base".into());
    let mut wrong_terminal = rejected.clone();
    wrong_terminal.decision_request_id = "wrong-terminal-request".into();
    assert!(
        runner
            .consume_runtime_feedback_with_lineage(AGENT_ID, wrong_terminal, None, &mut store)
            .is_err()
    );
    runner
        .consume_runtime_feedback_with_lineage(AGENT_ID, rejected.clone(), None, &mut store)
        .unwrap();
    runner
        .consume_runtime_feedback_with_lineage(AGENT_ID, rejected.clone(), None, &mut store)
        .unwrap();
    let mut rejected_collision = rejected;
    rejected_collision.reject_reason = Some("different terminal reason".into());
    let collision = runner
        .consume_runtime_feedback_with_lineage(AGENT_ID, rejected_collision, None, &mut store)
        .expect_err("released feedback ID cannot be replayed with a changed envelope");
    assert!(collision.to_string().contains("feedback_id_conflict"));
    assert!(
        store.entries().is_empty(),
        "terminal feedback cannot write memory"
    );
    let mut replacement_context = host_context();
    replacement_context.agent_turn_id = "replacement-turn".into();
    replacement_context.decision_request_id = "replacement-request".into();
    let turn_id = runner
        .start_turn_with_context(AGENT_ID, replacement_context)
        .expect("open provider-backed target turn");
    let outcome = completed_turn(&mut runner);
    assert_eq!(outcome.turn_id, turn_id);
    assert_eq!(outcome.memory_write_intents.len(), 1);

    let mut committed = outcome
        .feedback_for_runtime_status("committed", Some("receipt-live-harness-1"))
        .expect("build correlated committed feedback");
    committed.candidate_action_id = Some(7);
    committed.feedback_seq = 2;
    committed.provenance = "runtime_authoritative".to_string();
    let receipt = runtime_receipt_for_feedback(&committed);
    runner
        .consume_runtime_feedback_with_lineage(
            AGENT_ID,
            committed.clone(),
            Some(&receipt),
            &mut store,
        )
        .expect("matching Runtime committed receipt writes memory");
    runner
        .consume_runtime_feedback_with_lineage(
            AGENT_ID,
            committed.clone(),
            Some(&receipt),
            &mut store,
        )
        .expect("same receipt replay is exactly-once");
    let mut mismatched_receipt = receipt.clone();
    mismatched_receipt.receipt_digest =
        "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into();
    assert!(
        runner
            .consume_runtime_feedback_with_lineage(
                AGENT_ID,
                committed,
                Some(&mismatched_receipt),
                &mut store,
            )
            .expect_err("same feedback envelope with a different receipt lineage is rejected")
            .to_string()
            .contains("replay receipt lineage mismatch")
    );
    assert_eq!(store.entries().len(), 1);
    assert_eq!(
        store.entries()[0]["provenance"],
        "runtime_authoritative",
        "provider must not self-assign authoritative provenance"
    );
}

pub(super) struct VerifiedWorldReadback;

impl RuntimeReceiptReadbackVerifier for VerifiedWorldReadback {
    fn verify_world_readback(
        &self,
        _context: &ContinuousAgentTurnContextV1,
        _feedback: &FeedbackEnvelopeV1,
        receipt: &RuntimeReceiptLineageV1,
        response_identity: &ResponseArtifactIdentityV1,
    ) -> Result<RuntimeReceiptReadbackHandleV1, crate::simulator::AsyncAgentRunnerError> {
        Ok(RuntimeReceiptReadbackHandleV1::from_verified(
            receipt,
            response_identity,
        ))
    }
}

#[test]
fn committed_feedback_can_cross_only_the_world_readback_identity_seam() {
    let mut runner = provider_backed_runner();
    let _turn_id = runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open provider-backed target turn");
    let outcome = completed_turn(&mut runner);
    let mut committed = outcome
        .feedback_for_runtime_status("committed", Some("receipt-readback-1"))
        .expect("build correlated committed feedback");
    committed.candidate_action_id = Some(7);
    committed.provenance = "runtime_authoritative".to_string();
    let receipt = runtime_receipt_for_feedback(&committed);
    let response_identity = outcome
        .prepared_response_context
        .as_ref()
        .expect("response context retained")
        .response_artifact_identity();
    let mut store = MemoryWriteStore::default();
    runner
        .consume_runtime_feedback_with_world_readback(
            AGENT_ID,
            committed.clone(),
            &receipt,
            &response_identity,
            &VerifiedWorldReadback,
            &mut store,
        )
        .expect("Runtime readback verifier admits committed memory");
    assert_eq!(store.entries().len(), 1);
    runner
        .consume_runtime_feedback_with_lineage(
            AGENT_ID,
            committed.clone(),
            Some(&receipt),
            &mut store,
        )
        .expect("legacy lineaged replay remains idempotent after world-readback completion");
    let mut changed_receipt = receipt.clone();
    changed_receipt.envelope_digest =
        "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into();
    assert!(
        runner
            .consume_runtime_feedback_with_lineage(
                AGENT_ID,
                committed,
                Some(&changed_receipt),
                &mut store,
            )
            .expect_err("post-release replay must retain the previously accepted receipt lineage")
            .to_string()
            .contains("replay receipt lineage mismatch")
    );
    assert_eq!(store.entries().len(), 1);
}
