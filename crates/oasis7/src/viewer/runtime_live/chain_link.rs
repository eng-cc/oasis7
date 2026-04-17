use super::*;

use super::super::protocol::{GameplayActionError, GameplayActionRequest};
use std::net::ToSocketAddrs;

const CHAIN_GAMEPLAY_SUBMIT_PATH: &str = "/v1/chain/gameplay/submit";
const CHAIN_LINK_TIMEOUT_MS: u64 = 300;

#[derive(Debug, serde::Deserialize)]
struct ChainStatusSyncSnapshot {
    consensus: ChainStatusConsensusSnapshot,
    execution_world_dir: PathBuf,
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

        let prepared = prepare_chain_linked_runtime_update(chain_status_bind)?;
        let dispatch = self.apply_chain_linked_runtime_update(prepared, session)?;
        for response in dispatch.responses {
            send_response(writer, &response)?;
        }
        Ok(dispatch.advanced)
    }

    pub(super) fn sync_chain_linked_runtime_minimized_lock(
        shared: &Arc<Mutex<Self>>,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let chain_status_bind = {
            let server = lock_shared_server(shared)?;
            server
                .config
                .chain_status_bind
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let Some(chain_status_bind) = chain_status_bind else {
            return Ok(false);
        };

        let prepared = prepare_chain_linked_runtime_update(chain_status_bind.as_str())?;
        let dispatch = {
            let mut server = lock_shared_server(shared)?;
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
        if prepared.committed_height <= self.last_chain_committed_height {
            return Ok(ChainLinkedRuntimeDispatch {
                advanced: false,
                responses: Vec::new(),
            });
        }

        let baseline_logical_time = self.world.state().time;
        let baseline_event_seq = latest_runtime_event_seq(&self.world);
        let delta_logical_time = prepared
            .world
            .state()
            .time
            .saturating_sub(baseline_logical_time);
        let delta_event_seq =
            latest_runtime_event_seq(&prepared.world).saturating_sub(baseline_event_seq);
        if delta_logical_time == 0 && delta_event_seq == 0 {
            return Ok(ChainLinkedRuntimeDispatch {
                advanced: false,
                responses: Vec::new(),
            });
        }

        self.world = prepared.world;
        self.last_chain_committed_height = prepared.committed_height;
        self.confirm_player_gameplay_progress();

        let mapped_events: Vec<_> = self
            .world
            .journal()
            .events
            .iter()
            .filter(|event| event.id > baseline_event_seq)
            .map(|runtime_event| map_runtime_event(runtime_event, &self.snapshot_config))
            .collect();
        let pending_batch = self.register_authoritative_batch(mapped_events.as_slice())?;
        let batch_finality_updates =
            self.advance_authoritative_batch_finality(self.world.state().time)?;

        let mut responses = Vec::new();
        if session.subscribed.contains(&ViewerStream::Events) {
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

        if session.subscribed.contains(&ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot();
            responses.push(ViewerResponse::Snapshot { snapshot });
        }

        session.metrics = runtime_metrics(&self.world);
        if session.subscribed.contains(&ViewerStream::Metrics) {
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

pub(super) fn submit_chain_linked_gameplay_action(
    chain_status_bind: &str,
    request: &GameplayActionRequest,
) -> Result<ChainGameplaySubmitResponse, GameplayActionError> {
    let response =
        post_chain_linked_gameplay_action(chain_status_bind, request).map_err(|err| {
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

fn prepare_chain_linked_runtime_update(
    chain_status_bind: &str,
) -> Result<PreparedChainLinkedRuntimeUpdate, ViewerRuntimeLiveServerError> {
    let chain_status = fetch_chain_status_snapshot(chain_status_bind)?;
    let world = load_chain_execution_world(chain_status.execution_world_dir.as_path())?;
    Ok(PreparedChainLinkedRuntimeUpdate {
        committed_height: chain_status.consensus.committed_height,
        world,
    })
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

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
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
    chain_status_bind: &str,
    request: &GameplayActionRequest,
) -> Result<ChainGameplaySubmitResponse, ViewerRuntimeLiveServerError> {
    let mut stream = connect_chain_status_stream(
        chain_status_bind,
        Duration::from_millis(CHAIN_LINK_TIMEOUT_MS),
    )?;
    let payload = serde_json::to_vec(request)
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    let request_head = format!(
        "POST {CHAIN_GAMEPLAY_SUBMIT_PATH} HTTP/1.1\r\nHost: {chain_status_bind}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(request_head.as_bytes())?;
    stream.write_all(payload.as_slice())?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
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

fn load_chain_execution_world(
    execution_world_dir: &Path,
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
            world.with_release_security_policy(ReleaseSecurityPolicy::production_hardened())
        })
        .map_err(ViewerRuntimeLiveServerError::Runtime)
}
