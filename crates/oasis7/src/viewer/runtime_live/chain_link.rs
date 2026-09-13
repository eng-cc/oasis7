use super::authoritative::compute_runtime_snapshot_hash;
use super::*;

use super::super::protocol::{CollectDataCommand, GameplayActionError, GameplayActionRequest};
use crate::runtime::{
    CognitionProvisioningRequestV1, MainTokenConfig, MainTokenSupplyState,
    WorldEvent as RuntimeWorldEvent, production_hardened_main_token_config,
};
use std::collections::BTreeSet;
use std::net::ToSocketAddrs;

const CHAIN_GAMEPLAY_SUBMIT_PATH: &str = "/v1/chain/gameplay/submit";
const CHAIN_LINK_TIMEOUT_MS: u64 = 1_500;
const MAX_CHAIN_LINK_HTTP_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Debug, serde::Deserialize)]
struct ChainStatusSyncSnapshot {
    consensus: ChainStatusConsensusSnapshot,
    execution_world_dir: PathBuf,
    release_security_policy: ReleaseSecurityPolicy,
}

#[derive(Debug, serde::Deserialize)]
struct ChainStatusConsensusSnapshot {
    committed_height: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct ChainGameplaySubmitResponse {
    ok: bool,
    #[serde(default)]
    pub(super) action_id: Option<u64>,
    #[serde(default)]
    submitted_at_unix_ms: Option<i64>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

struct PreparedChainLinkedRuntimeUpdate {
    committed_height: u64,
    world: RuntimeWorld,
}

struct ChainLinkedRuntimeDispatch {
    advanced: bool,
    responses: Vec<ViewerResponse>,
}

/// Verify that a chain-linked execution snapshot already contains the exact
/// authority/provisioning transaction supplied to the viewer. A chain-linked
/// viewer is an observer: it must never install authority or mutate the
/// chain writer's persistence directory. The chain writer owns publication;
/// this check only admits a snapshot after its durable records are visible.
fn verify_provider_backed_bootstrap_authorities(
    world: &RuntimeWorld,
    authorities: &[ProviderBackedBootstrapAuthorityV1],
) -> Result<(), String> {
    if authorities.is_empty() {
        return Ok(());
    }

    let binding = world
        .current_cognition_runtime_binding()
        .map_err(|error| format!("ProviderBacked authority binding unavailable: {error:?}"))?;
    let economy = world
        .cognition_economy()
        .map_err(|error| format!("ProviderBacked provisioning ledger unavailable: {error:?}"))?;
    let revocation = world.capability_revocation_state();
    let mut seen_agents = BTreeSet::new();
    let mut seen_provisions = BTreeSet::new();

    for authority in authorities {
        if !seen_agents.insert(authority.agent_id.clone())
            || !seen_provisions.insert(authority.provision_id.clone())
        {
            return Err(
                "ProviderBacked authority bootstrap input contains duplicate agent or provision id"
                    .to_string(),
            );
        }
        if authority.agent_id.trim().is_empty()
            || authority.owner_binding.trim().is_empty()
            || authority.owner_generation == 0
            || authority.world_id != binding.world_id
            || authority.branch_id != binding.branch_id
            || authority.reorg_epoch != binding.reorg_epoch
        {
            return Err(
                "ProviderBacked authority bootstrap input does not match live chain binding"
                    .to_string(),
            );
        }
        if !world.state().agents.contains_key(&authority.agent_id) {
            return Err(format!(
                "ProviderBacked authority bootstrap agent is not live: {}",
                authority.agent_id
            ));
        }

        let identity = revocation
            .agent_identities
            .get(&authority.agent_id)
            .ok_or_else(|| {
                format!(
                    "ProviderBacked authority identity is not durably installed: {}",
                    authority.agent_id
                )
            })?;
        if identity != &authority.identity
            || identity.owner_binding != authority.owner_binding
            || identity.generation != authority.owner_generation
        {
            return Err(format!(
                "ProviderBacked authority identity mismatch for agent {}",
                authority.agent_id
            ));
        }

        let persisted_record = revocation
            .authority_records
            .get(&authority.authority_record.issuer_id)
            .ok_or_else(|| {
                format!(
                    "ProviderBacked authority record is not durably installed for agent {}",
                    authority.agent_id
                )
            })?;
        let persisted_proof = revocation
            .authority_finality_proofs
            .get(&authority.authority_record.issuer_id)
            .ok_or_else(|| {
                format!(
                    "ProviderBacked authority finality proof is not durably installed for agent {}",
                    authority.agent_id
                )
            })?;
        if persisted_record != &authority.authority_record
            || persisted_proof != &authority.authority_finality_proof
        {
            return Err(format!(
                "ProviderBacked authority finality evidence mismatch for agent {}",
                authority.agent_id
            ));
        }

        let encoded_grant = serde_json::to_value(&authority.grant)
            .map_err(|error| format!("ProviderBacked grant cannot be encoded: {error}"))?;
        if world.capability_grants_v2().get(&authority.grant.grant_id) != Some(&encoded_grant) {
            return Err(format!(
                "ProviderBacked grant is not durably installed for agent {}",
                authority.agent_id
            ));
        }

        let persisted_context = world
            .capability_invocation_contexts()
            .values()
            .find(|context| {
                context.grant_id == authority.grant.grant_id
                    && context.subject == authority.invocation_context.subject
                    && context.presenter == authority.invocation_context.presenter
                    && context.audience == authority.invocation_context.audience
                    && context.module_id == authority.invocation_context.module_id
                    && context.module_version == authority.invocation_context.module_version
                    && context.response_nonce == authority.invocation_context.response_nonce
            })
            .ok_or_else(|| {
                format!(
                    "ProviderBacked invocation context is not durably installed for agent {}",
                    authority.agent_id
                )
            })?;
        if persisted_context.grant_id != authority.invocation_context.grant_id {
            return Err(format!(
                "ProviderBacked invocation context grant mismatch for agent {}",
                authority.agent_id
            ));
        }

        let expected_request = CognitionProvisioningRequestV1::new(
            authority.provision_id.clone(),
            authority.owner_binding.clone(),
            authority.owner_binding.clone(),
            authority.owner_generation,
            authority.world_id.clone(),
            authority.branch_id.clone(),
            authority.reorg_epoch,
            authority.allowance,
            authority.authority_context.clone(),
        );
        expected_request
            .validate()
            .map_err(|error| format!("ProviderBacked provisioning request invalid: {error}"))?;
        if expected_request.authority_digest != authority.authority_digest
            || expected_request.provisioning_digest != authority.provisioning_digest
        {
            return Err(format!(
                "ProviderBacked provisioning digest mismatch for agent {}",
                authority.agent_id
            ));
        }

        let provision = economy
            .provisions
            .get(&authority.provision_id)
            .ok_or_else(|| {
                format!(
                    "ProviderBacked provisioning is not durably committed for agent {}",
                    authority.agent_id
                )
            })?;
        if provision.request != expected_request {
            return Err(format!(
                "ProviderBacked provisioning request mismatch for agent {}",
                authority.agent_id
            ));
        }
        let receipt = economy
            .provision_receipts
            .get(&provision.receipt_id)
            .ok_or_else(|| {
                format!(
                    "ProviderBacked provisioning receipt is missing for agent {}",
                    authority.agent_id
                )
            })?;
        if receipt.request != expected_request
            || !economy.provision_journal.iter().any(|event| {
                event.request == expected_request && event.receipt_id == provision.receipt_id
            })
        {
            return Err(format!(
                "ProviderBacked provisioning receipt/journal mismatch for agent {}",
                authority.agent_id
            ));
        }
    }

    Ok(())
}

fn session_requests_runtime_feedback(session: &RuntimeLiveSession) -> bool {
    !session.uses_default_subscription()
}

impl ViewerRuntimeLiveServer {
    pub(super) fn sync_chain_linked_runtime(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let Some(chain_status_bind) = self
            .config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(false);
        };

        let prepared = match prepare_chain_linked_runtime_update(chain_status_bind) {
            Ok(prepared) => prepared,
            Err(err) => {
                if self
                    .config
                    .chain_link_policy
                    .records_player_facing_chain_failures()
                    && session_requests_runtime_feedback(session)
                {
                    self.record_chain_sync_failure(&err);
                }
                return Err(err);
            }
        };
        self.clear_chain_sync_failure_feedback();
        let dispatch = self.apply_chain_linked_runtime_update(prepared, session)?;
        for response in dispatch.responses {
            send_response(writer, &response)?;
        }
        Ok(dispatch.advanced)
    }

    pub(super) fn prime_chain_linked_runtime_for_snapshot(
        &mut self,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let Some(chain_status_bind) = self
            .config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(false);
        };

        let prepared = prepare_chain_linked_runtime_update(chain_status_bind)?;
        self.clear_chain_sync_failure_feedback();
        let mut silent_session = RuntimeLiveSession::new_with_playing(false);
        let dispatch = self.apply_chain_linked_runtime_update(prepared, &mut silent_session)?;
        Ok(dispatch.advanced)
    }

    pub(super) fn sync_chain_linked_runtime_minimized_lock(
        shared: &Arc<Mutex<Self>>,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let (chain_status_bind, records_player_facing_chain_failures) = {
            let server = lock_shared_server(shared)?;
            let chain_status_bind = server
                .config
                .chain_status_bind
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            (
                chain_status_bind,
                server
                    .config
                    .chain_link_policy
                    .records_player_facing_chain_failures(),
            )
        };
        let Some(chain_status_bind) = chain_status_bind else {
            return Ok(false);
        };

        let prepared = match prepare_chain_linked_runtime_update(chain_status_bind.as_str()) {
            Ok(prepared) => prepared,
            Err(err) => {
                if records_player_facing_chain_failures
                    && session_requests_runtime_feedback(session)
                {
                    let mut server = lock_shared_server(shared)?;
                    server.record_chain_sync_failure(&err);
                }
                return Err(err);
            }
        };
        let dispatch = {
            let mut server = lock_shared_server(shared)?;
            server.clear_chain_sync_failure_feedback();
            server.apply_chain_linked_runtime_update(prepared, session)?
        };
        for response in dispatch.responses {
            send_response(writer, &response)?;
        }

        Ok(dispatch.advanced)
    }

    fn apply_chain_linked_runtime_update(
        &mut self,
        prepared: PreparedChainLinkedRuntimeUpdate,
        session: &mut RuntimeLiveSession,
    ) -> Result<ChainLinkedRuntimeDispatch, ViewerRuntimeLiveServerError> {
        self.llm_sidecar
            .clear_stale_local_test_bindings_for_world(&prepared.world);
        let baseline_logical_time = self.world.state().time;
        let baseline_event_seq = latest_runtime_event_seq(&self.world);
        let baseline_snapshot = self.world.snapshot();
        let prepared_snapshot = prepared.world.snapshot();
        let baseline_snapshot_hash = compute_runtime_snapshot_hash(&baseline_snapshot)?;
        let prepared_snapshot_hash = compute_runtime_snapshot_hash(&prepared_snapshot)?;
        // Chain-linked viewers are observers. The chain writer must publish
        // authority and provisioning records in its committed world before
        // the viewer accepts that world; the viewer never mutates the loaded
        // execution directory and keeps retrying while publication is absent.
        verify_provider_backed_bootstrap_authorities(
            &prepared.world,
            &self.config.provider_backed_bootstrap_authorities,
        )
        .map_err(ViewerRuntimeLiveServerError::Init)?;
        let materially_different_world = prepared_snapshot_hash != baseline_snapshot_hash
            && chain_linked_runtime_has_playable_state(&prepared.world);
        if prepared.committed_height < self.last_chain_committed_height
            || (prepared.committed_height == self.last_chain_committed_height
                && !materially_different_world)
        {
            return Ok(ChainLinkedRuntimeDispatch {
                advanced: false,
                responses: Vec::new(),
            });
        }

        let delta_logical_time = prepared
            .world
            .state()
            .time
            .saturating_sub(baseline_logical_time);
        let delta_event_seq =
            latest_runtime_event_seq(&prepared.world).saturating_sub(baseline_event_seq);
        if delta_logical_time == 0 && delta_event_seq == 0 {
            if !materially_different_world {
                return Ok(ChainLinkedRuntimeDispatch {
                    advanced: false,
                    responses: Vec::new(),
                });
            }
        }

        if prepared.committed_height == 0
            && !chain_linked_runtime_has_playable_state(&prepared.world)
        {
            return Ok(ChainLinkedRuntimeDispatch {
                advanced: false,
                responses: Vec::new(),
            });
        }

        // Event IDs are a rolling sequence. Select the prepared journal by
        // its last full event position first so a new era (MAX -> 1) cannot
        // hide a completion behind a scalar ID comparison. If compaction or
        // a reorg removes the common tail, the helper falls back to the
        // era-aware journal cursor.
        let runtime_events = runtime_events_after_baseline(
            self.world.journal().events.as_slice(),
            prepared.world.journal().events.as_slice(),
            runtime_last_event_era(&baseline_snapshot),
            runtime_last_event_era(&prepared_snapshot),
        );
        self.world = prepared.world;
        self.last_chain_committed_height = prepared.committed_height;
        self.confirm_player_gameplay_progress();

        let mapped_events: Vec<_> = runtime_events
            .iter()
            .map(|runtime_event| {
                map_runtime_event(
                    runtime_event,
                    &self.snapshot_config,
                    self.seed_model.as_ref(),
                )
            })
            .collect();
        for (runtime_event, mapped_event) in runtime_events.iter().zip(mapped_events.iter()) {
            if matches!(runtime_event.body, RuntimeWorldEventBody::Domain(_)) {
                self.llm_sidecar
                    .notify_action_result_if_needed(runtime_event, mapped_event.clone());
            }
            self.llm_sidecar.notify_recipe_completion_with_binding(
                runtime_event,
                mapped_event.clone(),
                self.world.current_cognition_runtime_binding().ok(),
            );
        }
        let pending_batch = self.register_authoritative_batch(mapped_events.as_slice())?;
        let batch_finality_updates =
            self.advance_authoritative_batch_finality(self.world.state().time)?;

        let mut responses = Vec::new();
        if session.explicitly_subscribed_to(ViewerStream::Events) {
            for event in &mapped_events {
                if session.event_allowed(event) {
                    responses.push(ViewerResponse::Event {
                        event: event.clone(),
                    });
                }
            }
            responses.push(ViewerResponse::AuthoritativeBatch {
                batch: pending_batch,
            });
            for batch in batch_finality_updates {
                responses.push(ViewerResponse::AuthoritativeBatch { batch });
            }
        }

        if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
            responses.push(ViewerResponse::Snapshot { snapshot });
        }

        session.metrics = runtime_metrics(&self.world);
        if session.explicitly_subscribed_to(ViewerStream::Metrics) {
            responses.push(ViewerResponse::Metrics {
                time: Some(self.world.state().time),
                metrics: session.metrics.clone(),
            });
        }

        Ok(ChainLinkedRuntimeDispatch {
            advanced: true,
            responses,
        })
    }
}

pub(super) fn runtime_events_after_baseline(
    baseline_events: &[RuntimeWorldEvent],
    prepared_events: &[RuntimeWorldEvent],
    baseline_event_era: u64,
    prepared_event_era: u64,
) -> Vec<RuntimeWorldEvent> {
    let baseline_event_seq = baseline_events
        .last()
        .map(|event| event.id)
        .unwrap_or_default();

    if let Some(last_baseline_event) = baseline_events.last()
        && let Some(position) = prepared_events
            .iter()
            .rposition(|event| event == last_baseline_event)
    {
        return prepared_events
            .get(position.saturating_add(1)..)
            .unwrap_or_default()
            .to_vec();
    }

    prepared_events
        .iter()
        .enumerate()
        .find(|(index, event)| {
            (
                runtime_event_era_at(prepared_events, *index, prepared_event_era),
                event.id,
            ) > (baseline_event_era, baseline_event_seq)
        })
        .map_or_else(Vec::new, |(index, _)| prepared_events[index..].to_vec())
}

fn runtime_last_event_era(snapshot: &RuntimeSnapshot) -> u64 {
    if snapshot.last_event_id == 0 {
        snapshot.event_id_era.saturating_sub(1)
    } else {
        snapshot.event_id_era
    }
}

fn runtime_event_era_at(
    events: &[RuntimeWorldEvent],
    event_index: usize,
    last_event_era: u64,
) -> u64 {
    let mut era = last_event_era;
    for pair in events[event_index..].windows(2).rev() {
        if pair[0].id == u64::MAX && pair[1].id == 1 {
            era = era.saturating_sub(1);
        }
    }
    era
}

pub(super) fn submit_chain_linked_gameplay_action(
    chain_submit_bind: &str,
    request: &GameplayActionRequest,
) -> Result<ChainGameplaySubmitResponse, GameplayActionError> {
    let response =
        post_chain_linked_gameplay_action(chain_submit_bind, request).map_err(|err| {
            gameplay_chain_submit_error(
                request,
                "chain_submit_unavailable",
                format!("chain gameplay submit transport failed: {err:?}"),
            )
        })?;

    if !response.ok {
        return Err(gameplay_chain_submit_error(
            request,
            response
                .error_code
                .clone()
                .unwrap_or_else(|| "chain_submit_failed".to_string()),
            response
                .error
                .clone()
                .unwrap_or_else(|| "chain gameplay submit was rejected".to_string()),
        ));
    }

    if response.action_id.is_none() {
        return Err(gameplay_chain_submit_error(
            request,
            "chain_submit_failed",
            "chain gameplay submit succeeded without consensus action id",
        ));
    }

    let _ = response.submitted_at_unix_ms;
    Ok(response)
}

pub(super) fn submit_chain_linked_collect_data(
    chain_submit_bind: &str,
    command: &CollectDataCommand,
) -> Result<ChainGameplaySubmitResponse, GameplayActionError> {
    let response = post_chain_linked_submit_payload(chain_submit_bind, command).map_err(|err| {
        collect_data_chain_submit_error(
            "chain_submit_unavailable",
            format!("chain collect_data submit transport failed: {err:?}"),
        )
    })?;
    if !response.ok {
        return Err(collect_data_chain_submit_error(
            response
                .error_code
                .clone()
                .unwrap_or_else(|| "chain_submit_failed".to_string()),
            response
                .error
                .clone()
                .unwrap_or_else(|| "chain collect_data submit was rejected".to_string()),
        ));
    }
    if response.action_id.is_none() {
        return Err(collect_data_chain_submit_error(
            "chain_submit_failed",
            "chain collect_data submit succeeded without consensus action id",
        ));
    }
    let _ = response.submitted_at_unix_ms;
    Ok(response)
}

fn prepare_chain_linked_runtime_update(
    chain_status_bind: &str,
) -> Result<PreparedChainLinkedRuntimeUpdate, ViewerRuntimeLiveServerError> {
    let chain_status = fetch_chain_status_snapshot(chain_status_bind)?;
    let world = match load_chain_execution_world(
        chain_status.execution_world_dir.as_path(),
        chain_status.release_security_policy,
    ) {
        Ok(world) => world,
        Err(err) if chain_status.consensus.committed_height == 0 => {
            let _ = err;
            RuntimeWorld::new_production_hardened()
        }
        Err(err) => return Err(err),
    };
    let sync_watermark =
        chain_linked_runtime_sync_watermark(chain_status.consensus.committed_height, &world);
    Ok(PreparedChainLinkedRuntimeUpdate {
        committed_height: sync_watermark,
        world,
    })
}

fn chain_linked_runtime_sync_watermark(committed_height: u64, world: &RuntimeWorld) -> u64 {
    committed_height
        .max(world.state().time)
        .max(latest_runtime_event_seq(world))
}

fn chain_linked_runtime_has_playable_state(world: &RuntimeWorld) -> bool {
    let state = world.state();
    !state.agents.is_empty() || !state.resources.is_empty() || !state.factories.is_empty()
}

fn fetch_chain_status_snapshot(
    chain_status_bind: &str,
) -> Result<ChainStatusSyncSnapshot, ViewerRuntimeLiveServerError> {
    let mut stream = connect_chain_status_stream(
        chain_status_bind,
        Duration::from_millis(CHAIN_LINK_TIMEOUT_MS),
    )?;
    let request = format!(
        "GET /v1/chain/status HTTP/1.1\r\nHost: {chain_status_bind}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let response = read_chain_link_http_response(&mut stream)?;
    let (status_code, payload): (u16, ChainStatusSyncSnapshot) =
        parse_http_json_response(response.as_slice(), "chain status")?;
    if status_code != 200 {
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "chain status request returned non-200 response: HTTP {status_code}"
        )));
    }
    Ok(payload)
}

fn post_chain_linked_gameplay_action(
    chain_submit_bind: &str,
    request: &GameplayActionRequest,
) -> Result<ChainGameplaySubmitResponse, ViewerRuntimeLiveServerError> {
    post_chain_linked_submit_payload(chain_submit_bind, request)
}

fn post_chain_linked_submit_payload(
    chain_submit_bind: &str,
    payload: &impl serde::Serialize,
) -> Result<ChainGameplaySubmitResponse, ViewerRuntimeLiveServerError> {
    let mut stream = connect_chain_status_stream(
        chain_submit_bind,
        Duration::from_millis(CHAIN_LINK_TIMEOUT_MS),
    )?;
    let payload = serde_json::to_vec(payload)
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    let request_head = format!(
        "POST {CHAIN_GAMEPLAY_SUBMIT_PATH} HTTP/1.1\r\nHost: {chain_submit_bind}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(request_head.as_bytes())?;
    stream.write_all(payload.as_slice())?;
    stream.flush()?;

    let response = read_chain_link_http_response(&mut stream)?;
    let (status_code, payload): (u16, ChainGameplaySubmitResponse) =
        parse_http_json_response(response.as_slice(), "chain gameplay submit")?;
    if !(200..=299).contains(&status_code) && payload.ok {
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "chain gameplay submit returned HTTP {status_code} with invalid success payload"
        )));
    }
    Ok(payload)
}

fn connect_chain_status_stream(
    chain_status_bind: &str,
    timeout: Duration,
) -> Result<TcpStream, ViewerRuntimeLiveServerError> {
    let mut addrs = chain_status_bind.to_socket_addrs()?;
    let first_addr = addrs.next().ok_or_else(|| {
        ViewerRuntimeLiveServerError::Serde(format!(
            "chain status bind resolved to no addresses: {chain_status_bind}"
        ))
    })?;

    let mut connected = None;
    let mut last_err = None;
    for addr in std::iter::once(first_addr).chain(addrs) {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(err) => {
                last_err = Some(err);
            }
        }
    }
    let stream = connected.ok_or_else(|| {
        ViewerRuntimeLiveServerError::Io(
            last_err.expect("last_err must be set after connect attempts"),
        )
    })?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    Ok(stream)
}

fn parse_http_json_response<T: serde::de::DeserializeOwned>(
    response: &[u8],
    label: &str,
) -> Result<(u16, T), ViewerRuntimeLiveServerError> {
    let Some(body_start) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "{label} response missing HTTP body"
        )));
    };
    let header = std::str::from_utf8(&response[..body_start])
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    let status_code = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|token| token.parse::<u16>().ok())
        .ok_or_else(|| {
            ViewerRuntimeLiveServerError::Serde(format!(
                "{label} response missing HTTP status code"
            ))
        })?;
    let payload = serde_json::from_slice::<T>(&response[(body_start + 4)..])
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    Ok((status_code, payload))
}

fn read_chain_link_http_response(
    stream: &mut TcpStream,
) -> Result<Vec<u8>, ViewerRuntimeLiveServerError> {
    let mut response = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    let started_at = Instant::now();
    let timeout = Duration::from_millis(CHAIN_LINK_TIMEOUT_MS);

    loop {
        let bytes = match stream.read(&mut chunk) {
            Ok(bytes) => bytes,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) && started_at.elapsed() < timeout =>
            {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(err) => return Err(ViewerRuntimeLiveServerError::Io(err)),
        };
        if bytes == 0 {
            return Ok(response);
        }
        response.extend_from_slice(&chunk[..bytes]);
        if response.len() > MAX_CHAIN_LINK_HTTP_RESPONSE_BYTES {
            return Err(ViewerRuntimeLiveServerError::Serde(format!(
                "chain HTTP response exceeds {MAX_CHAIN_LINK_HTTP_RESPONSE_BYTES} byte limit"
            )));
        }
        if chain_link_http_response_is_complete(response.as_slice())? {
            return Ok(response);
        }
    }
}

pub(super) fn chain_link_http_response_is_complete(
    response: &[u8],
) -> Result<bool, ViewerRuntimeLiveServerError> {
    let Some(header_end) = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
    else {
        return Ok(false);
    };
    let header = std::str::from_utf8(&response[..header_end])
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    let Some(content_length) = chain_link_http_content_length(header)? else {
        return Ok(false);
    };
    let expected = header_end.checked_add(content_length).ok_or_else(|| {
        ViewerRuntimeLiveServerError::Serde(
            "chain HTTP response Content-Length overflow".to_string(),
        )
    })?;
    if expected > MAX_CHAIN_LINK_HTTP_RESPONSE_BYTES {
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "chain HTTP response exceeds {MAX_CHAIN_LINK_HTTP_RESPONSE_BYTES} byte limit"
        )));
    }
    Ok(response.len() >= expected)
}

pub(super) fn chain_link_http_content_length(
    header: &str,
) -> Result<Option<usize>, ViewerRuntimeLiveServerError> {
    for line in header.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            let length = value.trim().parse::<usize>().map_err(|_| {
                ViewerRuntimeLiveServerError::Serde(
                    "chain HTTP response Content-Length must be a non-negative integer".to_string(),
                )
            })?;
            return Ok(Some(length));
        }
    }
    Ok(None)
}

fn gameplay_chain_submit_error(
    request: &GameplayActionRequest,
    code: impl Into<String>,
    message: impl Into<String>,
) -> GameplayActionError {
    GameplayActionError {
        code: code.into(),
        message: message.into(),
        action_id: Some(request.action_id.clone()),
        target_agent_id: Some(request.target_agent_id.clone()),
    }
}

fn collect_data_chain_submit_error(
    code: impl Into<String>,
    message: impl Into<String>,
) -> GameplayActionError {
    GameplayActionError {
        code: code.into(),
        message: message.into(),
        action_id: Some("collect_data".to_string()),
        target_agent_id: None,
    }
}

pub(super) fn load_chain_execution_world(
    execution_world_dir: &Path,
    release_security_policy: ReleaseSecurityPolicy,
) -> Result<RuntimeWorld, ViewerRuntimeLiveServerError> {
    let snapshot_path = execution_world_dir.join("snapshot.json");
    let journal_path = execution_world_dir.join("journal.json");
    if !snapshot_path.exists() || !journal_path.exists() {
        let mut missing_files = Vec::new();
        if !snapshot_path.exists() {
            missing_files.push(snapshot_path.display().to_string());
        }
        if !journal_path.exists() {
            missing_files.push(journal_path.display().to_string());
        }
        return Err(ViewerRuntimeLiveServerError::Serde(format!(
            "execution world is not ready; missing persistence file(s): {}",
            missing_files.join(", ")
        )));
    }

    RuntimeWorld::load_from_dir(execution_world_dir)
        .map(|world| {
            let mut world = world.with_release_security_policy(release_security_policy.clone());
            normalize_chain_execution_world_main_token_config(&mut world, release_security_policy);
            world
        })
        .map_err(ViewerRuntimeLiveServerError::Runtime)
}

fn normalize_chain_execution_world_main_token_config(
    world: &mut RuntimeWorld,
    release_security_policy: ReleaseSecurityPolicy,
) {
    if release_security_policy.is_production_hardened() {
        if world.main_token_config() == &MainTokenConfig::default() {
            world.set_main_token_config(production_hardened_main_token_config());
        }
        return;
    }

    let state = world.state();
    let pristine_main_token_state = state.main_token_supply == MainTokenSupplyState::default()
        && state.main_token_balances.is_empty()
        && state.main_token_genesis_buckets.is_empty()
        && state.main_token_epoch_issuance_records.is_empty()
        && state.main_token_treasury_balances.is_empty()
        && state.main_token_claim_nonces.is_empty()
        && state.main_token_transfer_nonces.is_empty()
        && state.main_token_scheduled_policy_updates.is_empty()
        && state.main_token_treasury_distribution_records.is_empty()
        && state.main_token_node_points_bridge_records.is_empty()
        && state
            .restricted_starter_claim_liveops_pool_top_up_records
            .is_empty();
    if pristine_main_token_state
        && world.main_token_config() == &production_hardened_main_token_config()
    {
        world.set_main_token_config(MainTokenConfig::default());
    }
}
