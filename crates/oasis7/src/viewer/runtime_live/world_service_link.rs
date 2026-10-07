use super::super::protocol::{CollectDataCommand, GameplayActionError};
use super::chain_link::PreparedChainLinkedRuntimeUpdate;
use super::*;
use crate::world_service::client::{
    RemoteWorldServiceClient, WorldServiceClientConfig, WorldServicePort,
};
use crate::world_service::verified_view::VerifiedWorldView;
use oasis7_client_api::world_service::{
    ReadWorldChangesRequest, ReadWorldViewRequest, WORLD_SERVICE_CONTRACT_VERSION,
};

pub(super) struct PreparedWorldServiceSubmission {
    pub(super) correlation: oasis7_client_api::world_service::RequestCorrelation,
    pub(super) outcome:
        Result<oasis7_client_api::world_service::SubmitObservation<serde_json::Value>, String>,
    pub(super) admission_error: Option<GameplayActionError>,
}

impl ViewerRuntimeLiveServer {
    pub(super) fn prepare_shared_world_service_submission(
        shared: &Arc<Mutex<Self>>,
        request: &ViewerRequest,
    ) -> Result<Option<PreparedWorldServiceSubmission>, ViewerRuntimeLiveServerError> {
        let (config, query_state) = {
            let server = lock_shared_server(shared)?;
            if server.authoritative_recovery_write_fence.is_some() {
                return Ok(None);
            }
            (
                server.config.world_service.clone(),
                server.world_service_query_state.clone(),
            )
        };
        let Some(config) = config else {
            return Ok(None);
        };
        let bytes = match request {
            ViewerRequest::GameplayAction { request } => serde_json::to_vec(request),
            ViewerRequest::CollectData { command }
                if matches!(command, CollectDataCommand::Submit { .. }) =>
            {
                serde_json::to_vec(command)
            }
            _ => return Ok(None),
        }
        .map_err(|e| ViewerRuntimeLiveServerError::Serde(e.to_string()))?;
        let payload = crate::world_service::WorldServicePayloadV1::GameplayJson(bytes);
        let correlation =
            match crate::world_service::derive_correlation(config.expected_world.clone(), &payload)
            {
                Ok(correlation) => correlation,
                Err(_) => return Ok(None), // local admission returns the auth error
            };
        // Authenticate the Viewer session before crossing the canonical Submit
        // boundary. This lock protects admission only, never HTTP I/O.
        let admission = {
            use super::super::auth::{
                verify_collect_data_auth_proof, verify_gameplay_action_auth_proof,
            };
            let server = lock_shared_server(shared)?;
            let verified = match request {
                ViewerRequest::GameplayAction { request } => request
                    .auth
                    .as_ref()
                    .ok_or_else(|| "auth proof required".to_string())
                    .and_then(|auth| verify_gameplay_action_auth_proof(request, auth)),
                ViewerRequest::CollectData { command } => {
                    let CollectDataCommand::Submit { request } = command else {
                        unreachable!()
                    };
                    request
                        .auth
                        .as_ref()
                        .ok_or_else(|| "auth proof required".to_string())
                        .and_then(|auth| verify_collect_data_auth_proof(command, auth))
                }
                _ => unreachable!("only signed mutation requests were prepared"),
            };
            verified.and_then(|verified| {
                server
                    .session_policy
                    .validate_known_session_key(&verified.player_id, &verified.public_key)
                    .map(|_| ())
            })
        };
        if let Err(message) = admission {
            return Ok(Some(PreparedWorldServiceSubmission {
                correlation,
                outcome: Err("Viewer admission rejected before service submission".into()),
                admission_error: Some(GameplayActionError {
                    code: super::session_policy::map_session_policy_error_code(&message).into(),
                    message,
                    action_id: None,
                    target_agent_id: None,
                }),
            }));
        }
        let client = RemoteWorldServiceClient::new(config)
            .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?
            .with_query_state(query_state);
        let outcome = client
            .submit(oasis7_client_api::world_service::SubmitIntentRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                correlation: correlation.clone(),
                deadline_unix_ms: None,
                signed_payload: payload,
            })
            .map_err(|e| e.to_string());
        Ok(Some(PreparedWorldServiceSubmission {
            correlation,
            outcome,
            admission_error: None,
        }))
    }
    pub(super) fn submit_world_service_gameplay<T: serde::Serialize>(
        &mut self,
        original: &T,
    ) -> Result<u64, GameplayActionError> {
        use oasis7_client_api::world_service::{
            IntentOutcome, SubmitIntentRequest, SubmitObservation,
        };
        let error = |code: &str, message: String| GameplayActionError {
            code: code.into(),
            message,
            action_id: None,
            target_agent_id: None,
        };
        let client = self
            .world_service_client()
            .map_err(|e| error("world_service_unavailable", format!("{e:?}")))?
            .ok_or_else(|| {
                error(
                    "world_service_unavailable",
                    "service is not configured".into(),
                )
            })?;
        let bytes = serde_json::to_vec(original)
            .map_err(|e| error("world_service_invalid_request", e.to_string()))?;
        let payload = crate::world_service::WorldServicePayloadV1::GameplayJson(bytes);
        let correlation = crate::world_service::derive_correlation(
            client.config().expected_world.clone(),
            &payload,
        )
        .map_err(|e| error("world_service_invalid_request", e))?;
        let outcome = if let Some(prepared) = self.prepared_world_service_submission.take() {
            if prepared.correlation != correlation {
                return Err(error(
                    "world_service_invalid_request",
                    "prepared submission identity differs".into(),
                ));
            }
            if let Some(error) = prepared.admission_error {
                return Err(error);
            }
            prepared
                .outcome
                .map_err(|e| error("world_service_unavailable", e))?
        } else {
            client
                .submit(SubmitIntentRequest {
                    contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                    correlation: correlation.clone(),
                    deadline_unix_ms: None,
                    signed_payload: payload.clone(),
                })
                .map_err(|e| error("world_service_unavailable", e.to_string()))?
        };
        match outcome {
            SubmitObservation::OutcomeUnknown(_) => {
                self.pending_world_service_gameplay
                    .push((correlation, payload));
                Err(error("world_service_outcome_unknown",
                    "submission outcome unknown; query the original signed request, do not replace its nonce".into()))
            }
            SubmitObservation::Response(response) => match response.outcome {
                IntentOutcome::Committed { receipt, .. } => receipt
                    .get("action_id")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| {
                        error(
                            "world_service_invalid_receipt",
                            "receipt has no action reference".into(),
                        )
                    }),
                IntentOutcome::Received { .. } | IntentOutcome::Pending => {
                    self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
                        "world_service_intent",
                        "pending",
                        "intent received; no committed world effect yet",
                        None,
                        None,
                        None,
                        Some("wait for the original request outcome".into()),
                        0,
                        0,
                    ));
                    self.pending_world_service_gameplay
                        .push((correlation, payload));
                    // Legacy ACK has a scalar action ID; zero denotes no
                    // committed action reference, never execution success.
                    Ok(0)
                }
                other => Err(error(
                    "world_service_intent_unresolved",
                    format!("{other:?}"),
                )),
            },
        }
    }
}

pub(super) fn prepare_world_service_update(
    config: WorldServiceClientConfig,
    previous: Option<VerifiedWorldView>,
    pending: Vec<(
        oasis7_client_api::world_service::RequestCorrelation,
        crate::world_service::WorldServicePayloadV1,
    )>,
    query_state: crate::world_service::client::WorldServiceQueryState,
) -> Result<PreparedChainLinkedRuntimeUpdate, ViewerRuntimeLiveServerError> {
    let client = RemoteWorldServiceClient::new(config.clone())
        .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?
        .with_query_state(query_state);
    let mut minimum = previous.as_ref().map(|view| view.version().commit.clone());
    let mut intent_results = Vec::new();
    for (correlation, payload) in pending.into_iter().take(32) {
        let result = client
            .lookup(
                oasis7_client_api::world_service::LookupIntentRequest {
                    contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                    key: correlation.key,
                },
                payload,
            )
            .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?;
        if let oasis7_client_api::world_service::IntentOutcome::Committed { commit, .. } =
            &result.outcome
        {
            if let Some(current) = &minimum {
                if commit
                    .satisfies_minimum(current)
                    .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?
                {
                    minimum = Some(commit.clone());
                }
            } else {
                minimum = Some(commit.clone());
            }
        }
        intent_results.push(result);
    }
    let view = client
        .read_view(ReadWorldViewRequest {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            world: config.expected_world,
            scope_id: config.scope_id,
            min_commit: minimum,
            fixed_commit: None,
            deadline_unix_ms: None,
        })
        .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?;
    let events = if let Some(previous) = previous {
        let mut cursor = previous.continuation().clone();
        let mut events = Vec::new();
        for _ in 0..8 {
            let changes = client
                .read_changes(ReadWorldChangesRequest {
                    contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                    cursor: cursor.clone(),
                    max_items: 256,
                    max_bytes: 1_048_576,
                })
                .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?;
            let target = view.continuation();
            if changes.next_cursor.stream_id != target.stream_id
                || changes.next_cursor.scope_id != target.scope_id
                || changes.next_cursor.era != target.era
                || changes.next_cursor.sequence > target.sequence
                || !view
                    .version()
                    .commit
                    .satisfies_minimum(&changes.next_cursor.commit)
                    .map_err(|e| ViewerRuntimeLiveServerError::Init(e.to_string()))?
            {
                return Err(ViewerRuntimeLiveServerError::Init(
                    "view and changes are not coherent; resynchronize".into(),
                ));
            }
            let stopped = changes.next_cursor == cursor;
            cursor = changes.next_cursor;
            for event in changes.changes {
                events.push(
                    serde_json::from_value(event.change)
                        .map_err(|e| ViewerRuntimeLiveServerError::Serde(e.to_string()))?,
                );
            }
            if cursor.sequence == target.sequence {
                break;
            }
            if stopped {
                return Err(ViewerRuntimeLiveServerError::Init(
                    "changes stopped before view cursor".into(),
                ));
            }
        }
        if cursor.sequence != view.continuation().sequence {
            return Err(ViewerRuntimeLiveServerError::Init(
                "changes catch-up exceeds bounded sync budget".into(),
            ));
        }
        events
    } else {
        view.projection().events.clone()
    };
    let world = RuntimeWorld::new_with_state(view.projection().state.clone());
    Ok(PreparedChainLinkedRuntimeUpdate {
        committed_height: view.version().commit.position,
        world,
        verified_view: Some(view),
        service_events: Some(events),
        intent_results,
    })
}
