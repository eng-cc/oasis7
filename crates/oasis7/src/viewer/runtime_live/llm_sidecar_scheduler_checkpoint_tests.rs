//! Retry admission checks the original signed bytes before any network operation.
use super::*;
use crate::world_service::{authority::sign_read_request, wire::*};
use oasis7_client_api::world_service::WorldIdentity;
fn checkpoint() -> (
    WorldIdentity,
    lineage_persistence::PendingProviderSchedulerIntent,
) {
    let world = WorldIdentity {
        world_id: "world".into(),
        genesis_digest: "0".repeat(64),
    };
    let payload = WorldServicePayloadV1::Scheduler(
        sign_read_request(
            "scheduler",
            SchedulerIntentV1 {
                agent_id: "agent-1".into(),
                request_id: "original-reject-predecessor".into(),
                delegation_generation: 7,
                captured_base_binding: crate::runtime::RuntimeCognitionBaseBindingV1 {
                    world_id: "world".into(),
                    branch_id: "main".into(),
                    finality_epoch: 0,
                    finality_block_hash: None,
                    finality_status: "pending".into(),
                    base_tick: 12,
                    base_world_hash: "0".repeat(64),
                    reorg_epoch: 0,
                    runtime_manifest_hash: "0".repeat(64),
                },
                operation: SchedulerOperationV1::TransitionContinuation {
                    continuation_id: "predecessor-1".into(),
                    to: crate::runtime::ContinuationStatusV1::Rejected,
                    logical_tick: 12,
                },
            },
            &"11".repeat(32),
        )
        .unwrap(),
    );
    let correlation = crate::world_service::derive_correlation(world.clone(), &payload).unwrap();
    (
        world,
        lineage_persistence::PendingProviderSchedulerIntent {
            resume_context: None,
            resume_current_context: None,
            correlation,
            payload,
        },
    )
}
#[test]
fn scheduler_retry_accepts_exact_signed_checkpoint_without_replacing_bytes() {
    let (world, pending) = checkpoint();
    let before = serde_json::to_vec(&pending).unwrap();
    RuntimeLlmSidecar::validate_scheduler_checkpoint_integrity(
        &world,
        "original-reject-predecessor",
        &pending,
    )
    .unwrap();
    assert_eq!(serde_json::to_vec(&pending).unwrap(), before);
}
#[test]
fn scheduler_retry_fences_signature_tick_correlation_and_world_tampering() {
    for case in 0..5 {
        let (mut world, mut pending) = checkpoint();
        match case {
            0 => {
                let WorldServicePayloadV1::Scheduler(signed) = &mut pending.payload else {
                    unreachable!()
                };
                signed.signature_hex = "00".repeat(64);
            }
            1 => {
                let WorldServicePayloadV1::Scheduler(signed) = &mut pending.payload else {
                    unreachable!()
                };
                let SchedulerOperationV1::TransitionContinuation { logical_tick, .. } =
                    &mut signed.request.operation
                else {
                    unreachable!()
                };
                *logical_tick += 1;
            }
            2 => {
                pending.correlation.key.world.genesis_digest = "1".repeat(64);
            }
            3 => {
                world.world_id = "other-world".into();
            }
            _ => {
                let WorldServicePayloadV1::Scheduler(signed) = &pending.payload else {
                    unreachable!()
                };
                let mut request = signed.request.clone();
                request.request_id = "different-valid-signed-request".into();
                pending.payload = WorldServicePayloadV1::Scheduler(
                    sign_read_request("scheduler", request, &"11".repeat(32)).unwrap(),
                );
                pending.correlation =
                    crate::world_service::derive_correlation(world.clone(), &pending.payload)
                        .unwrap();
            }
        }
        let before = serde_json::to_vec(&pending).unwrap();
        assert!(
            RuntimeLlmSidecar::validate_scheduler_checkpoint_integrity(
                &world,
                "original-reject-predecessor",
                &pending
            )
            .is_err(),
            "case {case}"
        );
        assert_eq!(serde_json::to_vec(&pending).unwrap(), before, "case {case}");
    }
}
