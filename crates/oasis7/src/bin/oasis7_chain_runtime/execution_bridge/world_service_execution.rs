use oasis7::{
    runtime::{CausedBy, DomainEvent, World, WorldEventBody},
    world_service::*,
};
use oasis7_node::NodeExecutionCommitContext;

pub(super) struct StagedResult {
    result: CanonicalIntentResultV1,
    runtime_action_id: Option<u64>,
}

pub(super) fn apply_intents(
    world: &mut World,
    context: &NodeExecutionCommitContext,
    expected_world: Option<&WorldIdentity>,
    intents: Vec<(u64, SubmitIntentRequest<WorldServicePayloadV1>)>,
) -> Result<Vec<StagedResult>, String> {
    let mut staged = Vec::new();
    for (action_id, request) in intents {
        request.validate().map_err(|e| e.to_string())?;
        let correlation = derive_correlation(
            request.correlation.key.world.clone(),
            &request.signed_payload,
        )?;
        if request.correlation != correlation
            || correlation.key.world.world_id != context.world_id
            || expected_world != Some(&correlation.key.world)
        {
            return Err("canonical service correlation/world mismatch".into());
        }
        let key = correlation::key_digest(&correlation.key)?;
        if let Some(existing) = world
            .capability_revocation_state()
            .world_service_results
            .get(&key)
        {
            let existing: CanonicalIntentResultV1 =
                serde_json::from_value(existing.clone()).map_err(|e| e.to_string())?;
            if existing.request.correlation != correlation {
                return Err("canonical service idempotency conflict".into());
            }
            continue;
        }
        // Within-height duplicates also resolve to one canonical execution.
        if let Some(existing) = staged
            .iter()
            .find(|entry: &&StagedResult| entry.result.request.correlation.key == correlation.key)
        {
            if existing.result.request.correlation != correlation {
                return Err("within-block service idempotency conflict".into());
            }
            continue;
        }
        if matches!(&request.signed_payload, WorldServicePayloadV1::GameplayJson(bytes)
            if gameplay::consumed_collect_data_nonce(world, bytes)?)
        {
            // A legacy commit may win after HTTP admission. Do not turn its
            // consumed nonce into a new rejection under the original key.
            // Without a correlated record, Lookup truthfully remains Unknown.
            continue;
        }
        let mut candidate = world.clone();
        let mut runtime_action_id = None;
        let applied = match request.signed_payload.clone() {
            WorldServicePayloadV1::GameplayJson(bytes) => {
                gameplay::authenticated_action(&candidate, &bytes).map(|action| {
                    let id = candidate.submit_action(action);
                    runtime_action_id = Some(id);
                    serde_json::json!({"action_id": action_id, "runtime_action_id": id})
                })
            }
            WorldServicePayloadV1::AgentChat(request) => {
                let proof = request
                    .auth
                    .as_ref()
                    .ok_or("canonical chat proof required")?;
                let staged_nonce_consumed = staged.iter().any(|entry: &StagedResult| {
                    matches!(&entry.result.request.signed_payload, WorldServicePayloadV1::AgentChat(previous)
                        if previous.auth.as_ref().is_some_and(|old| old.public_key == proof.public_key && old.nonce >= proof.nonce))
                });
                if staged_nonce_consumed {
                    Err("canonical chat nonce did not advance within block".into())
                } else {
                    candidate.apply_authenticated_agent_chat(&request)
                }
            }
            WorldServicePayloadV1::Cognition(signed) => {
                candidate.commit_authenticated_cognition(signed)
            }
            WorldServicePayloadV1::FeedbackAck(signed) => {
                candidate.apply_authenticated_feedback_ack(signed)
            }
            WorldServicePayloadV1::Delegation(signed) => candidate
                .apply_agent_signer_delegation(&signed)
                .map(|()| serde_json::json!({"delegation": signed.request})),
            WorldServicePayloadV1::Scheduler(signed) => {
                candidate.apply_authenticated_scheduler(signed)
            }
        };
        let (receipt, rejected) = match applied {
            Ok(value) => {
                *world = candidate;
                (value, None)
            }
            Err(reason) => (serde_json::Value::Null, Some(reason)),
        };
        staged.push(StagedResult {
            result: CanonicalIntentResultV1 {
                request,
                action_id,
                committed_height: context.height,
                receipt,
                rejected,
            },
            runtime_action_id,
        });
    }
    Ok(staged)
}

pub(super) fn finalize_intents(world: &mut World, staged: Vec<StagedResult>) -> Result<(), String> {
    for mut entry in staged {
        if let Some(id) = entry.runtime_action_id {
            let events: Vec<_> = world
                .journal()
                .events
                .iter()
                .filter(|event| event.caused_by == Some(CausedBy::Action(id)))
                .cloned()
                .collect();
            if let Some(reason) = events.iter().find_map(|event| match &event.body {
                WorldEventBody::Domain(DomainEvent::ActionRejected { action_id, reason })
                    if *action_id == id =>
                {
                    Some(format!("{reason:?}"))
                }
                _ => None,
            }) {
                entry.result.rejected = Some(reason);
            }
            // No terminal evidence cannot be represented as a successful receipt.
            if events.is_empty() {
                return Err(
                    "service gameplay execution produced no correlated event evidence".into(),
                );
            }
            entry.result.receipt["events"] =
                serde_json::to_value(events).map_err(|e| e.to_string())?;
        }
        world.record_world_service_result(entry.result)?;
    }
    Ok(())
}
