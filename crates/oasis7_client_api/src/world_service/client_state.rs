//! Pure recovery cache schema and exact signed operation validation.
use super::task_context::{TaskContextPrepareV1, verify_prepare};
use super::{SignedReadRequest, SubmitIntentRequest};
use crate::world_service_correlation::derive_correlation_with_legacy;
use crate::world_service_wire::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientStateKind {
    Prepare,
    Prefix,
    Action,
    Outcome,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ClientStateV1<Payload = WorldServicePayloadV1> {
    ContextPrepare(Box<SignedReadRequest<TaskContextPrepareV1>>),
    PublicIntent(Box<SubmitIntentRequest<Payload>>),
}
#[expect(
    clippy::type_complexity,
    reason = "The shared wire profile preserves the execution adapter types while keeping the native recovery schema identical"
)]
pub fn validate_client_state<
    C: Serialize,
    P: Serialize,
    L: Serialize,
    CP: Serialize,
    ST: Serialize,
    R: Serialize,
    D: Serialize,
    X: Serialize,
>(
    record: &ClientStateV1<WorldServicePayloadProfileV1<C, P, L, CP, ST, R, D, X>>,
    kind: ClientStateKind,
) -> Result<(), String> {
    match record {
        ClientStateV1::ContextPrepare(proof) if kind == ClientStateKind::Prepare => {
            verify_prepare(proof)
        }
        ClientStateV1::PublicIntent(request) if kind != ClientStateKind::Prepare => {
            request.validate().map_err(|_| "invalid cached operation")?;
            let expected = derive_correlation_with_legacy(
                request.correlation.key.world.clone(),
                &request.signed_payload,
                |_| Err("unsupported cached browser operation".into()),
            )?;
            if expected != request.correlation {
                return Err("cached correlation mismatch".into());
            }
            match (&request.signed_payload, kind) {
                (WorldServicePayloadV1::Scheduler(s), ClientStateKind::Prefix)
                    if matches!(
                        s.request.operation,
                        SchedulerOperationV1::TaskProviderPrefix { .. }
                    ) =>
                {
                    Ok(())
                }
                (WorldServicePayloadV1::Scheduler(s), ClientStateKind::Outcome)
                    if matches!(
                        s.request.operation,
                        SchedulerOperationV1::TaskProviderFailure { .. }
                    ) =>
                {
                    Ok(())
                }
                (WorldServicePayloadV1::Cognition(s), ClientStateKind::Action)
                    if s.request.task_fence.is_some() =>
                {
                    Ok(())
                }
                _ => Err("cached operation kind mismatch".into()),
            }
        }
        _ => Err("cached operation kind mismatch".into()),
    }
}
