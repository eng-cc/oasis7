use std::collections::BTreeMap;
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oasis7::consensus_action_payload::{
    ConsensusActionPayloadEnvelope, encode_consensus_action_payload,
};
use oasis7::viewer::{
    CollectDataCommand, GameplayActionRequest, build_runtime_action_from_gameplay_request,
    verify_collect_data_auth_proof, verify_gameplay_action_auth_proof,
};
use oasis7_node::NodeRuntime;
use serde::{Deserialize, Serialize};

const GAMEPLAY_SUBMIT_PATH: &str = "/v1/chain/gameplay/submit";
const GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST: &str = "invalid_request";
const GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH: &str = "invalid_auth";
const GAMEPLAY_SUBMIT_ERROR_NONCE_REPLAY: &str = "auth_nonce_replay";
const GAMEPLAY_SUBMIT_ERROR_INTERNAL: &str = "internal_error";
const GAMEPLAY_SUBMIT_ERROR_SUBMIT_FAILED: &str = "submit_failed";
const GAMEPLAY_NONCE_LEDGER_FILE: &str = "gameplay-auth-nonces.json";

static NEXT_GAMEPLAY_ACTION_ID: AtomicU64 = AtomicU64::new(1);
static GAMEPLAY_NONCE_LEDGER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ChainGameplaySubmitRequest {
    CollectData(CollectDataCommand),
    Gameplay(GameplayActionRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainGameplaySubmitResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) action_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) submitted_at_unix_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainGameplaySubmitResponse {
    fn success(action_id: u64, submitted_at_unix_ms: i64) -> Self {
        Self {
            ok: true,
            action_id: Some(action_id),
            submitted_at_unix_ms: Some(submitted_at_unix_ms),
            error_code: None,
            error: None,
        }
    }

    fn error(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            action_id: None,
            submitted_at_unix_ms: None,
            error_code: Some(error_code.into()),
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct GameplayNonceLedger {
    #[serde(default)]
    last_nonce_by_player_key: BTreeMap<String, BTreeMap<String, u64>>,
}

#[derive(Debug)]
enum GameplayNonceRecordError {
    Replay(String),
    Internal(String),
}

impl GameplayNonceLedger {
    fn record_nonce(
        &mut self,
        player_id: &str,
        public_key: &str,
        nonce: u64,
    ) -> Result<(), String> {
        if nonce == 0 {
            return Err("auth nonce must be greater than zero".to_string());
        }
        let last_nonce = self
            .last_nonce_by_player_key
            .entry(player_id.to_string())
            .or_default()
            .entry(public_key.to_string())
            .or_default();
        if nonce <= *last_nonce {
            return Err(format!(
                "auth nonce replay for player {} key {}: expected nonce > {}, received {}",
                player_id, public_key, last_nonce, nonce
            ));
        }
        *last_nonce = nonce;
        Ok(())
    }
}

pub(super) fn maybe_handle_gameplay_submit_request(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    method: &str,
    path: &str,
    execution_world_dir: &Path,
) -> Result<bool, String> {
    if path != GAMEPLAY_SUBMIT_PATH {
        return Ok(false);
    }
    if !method.eq_ignore_ascii_case("POST") {
        write_gameplay_submit_error(
            stream,
            405,
            GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST,
            format!("method {method} is not allowed for {GAMEPLAY_SUBMIT_PATH}").as_str(),
        )?;
        return Ok(true);
    }
    handle_gameplay_submit(stream, request_bytes, runtime, execution_world_dir)?;
    Ok(true)
}

fn handle_gameplay_submit(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    execution_world_dir: &Path,
) -> Result<(), String> {
    let body = match super::feedback_submit_api::extract_http_json_body(request_bytes) {
        Ok(body) => body,
        Err(err) => {
            write_gameplay_submit_error(
                stream,
                400,
                GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST,
                err.as_str(),
            )?;
            return Ok(());
        }
    };
    let request = match parse_chain_gameplay_submit_request(body) {
        Ok(request) => request,
        Err(err) => {
            write_gameplay_submit_error(
                stream,
                400,
                GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST,
                err.as_str(),
            )?;
            return Ok(());
        }
    };
    let (verified, runtime_action) =
        match authorize_chain_gameplay_submit(request, execution_world_dir) {
            Ok(result) => result,
            Err((status, code, message)) => {
                write_gameplay_submit_error(stream, status, code, message.as_str())?;
                return Ok(());
            }
        };

    if let Err(err) = record_gameplay_nonce(
        execution_world_dir,
        verified.player_id.as_str(),
        verified.public_key.as_str(),
        verified.nonce,
    ) {
        let (status, code, message) = match err {
            GameplayNonceRecordError::Replay(message) => {
                (409, GAMEPLAY_SUBMIT_ERROR_NONCE_REPLAY, message)
            }
            GameplayNonceRecordError::Internal(message) => {
                (503, GAMEPLAY_SUBMIT_ERROR_INTERNAL, message)
            }
        };
        write_gameplay_submit_error(stream, status, code, message.as_str())?;
        return Ok(());
    }

    let payload = match build_gameplay_submit_action_payload(runtime_action) {
        Ok(payload) => payload,
        Err(err) => {
            write_gameplay_submit_error(stream, 502, GAMEPLAY_SUBMIT_ERROR_INTERNAL, err.as_str())?;
            return Ok(());
        }
    };
    let action_id = match next_gameplay_action_id() {
        Ok(action_id) => action_id,
        Err(err) => {
            write_gameplay_submit_error(stream, 502, GAMEPLAY_SUBMIT_ERROR_INTERNAL, err.as_str())?;
            return Ok(());
        }
    };
    if let Err(err) = runtime
        .lock()
        .map_err(|_| "failed to lock node runtime for gameplay submit".to_string())?
        // The HTTP endpoint has already verified the browser player's gameplay proof.
        // The node still submits the consensus envelope on its own transport lane.
        .submit_consensus_action_payload(action_id, payload)
    {
        write_gameplay_submit_error(
            stream,
            502,
            GAMEPLAY_SUBMIT_ERROR_SUBMIT_FAILED,
            format!("gameplay submit failed: {err}").as_str(),
        )?;
        return Ok(());
    }

    let response = ChainGameplaySubmitResponse::success(action_id, super::now_unix_ms());
    write_gameplay_submit_json_response(stream, 200, &response)
}

pub(super) fn parse_gameplay_submit_request(body: &[u8]) -> Result<GameplayActionRequest, String> {
    serde_json::from_slice(body).map_err(|err| format!("invalid gameplay submit request: {err}"))
}

fn parse_chain_gameplay_submit_request(body: &[u8]) -> Result<ChainGameplaySubmitRequest, String> {
    serde_json::from_slice(body).map_err(|err| format!("invalid gameplay submit request: {err}"))
}

fn authorize_chain_gameplay_submit(
    request: ChainGameplaySubmitRequest,
    execution_world_dir: &Path,
) -> Result<
    (oasis7::viewer::VerifiedPlayerAuth, oasis7::runtime::Action),
    (u16, &'static str, String),
> {
    match request {
        ChainGameplaySubmitRequest::Gameplay(request) => {
            let auth = request.auth.as_ref().ok_or_else(|| {
                (
                    401,
                    GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH,
                    "gameplay submit requires auth proof".to_string(),
                )
            })?;
            let verified = verify_gameplay_action_auth_proof(&request, auth)
                .map_err(|err| (401, GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH, err))?;
            let action = build_runtime_action_from_gameplay_request(&request)
                .map_err(|err| (400, GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST, err.message))?;
            Ok((verified, action))
        }
        ChainGameplaySubmitRequest::CollectData(command) => {
            let CollectDataCommand::Submit { request } = &command else {
                return Err((
                    400,
                    GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST,
                    "chain gameplay submit accepts only collect_data mode=submit".to_string(),
                ));
            };
            let auth = request.auth.as_ref().ok_or_else(|| {
                (
                    401,
                    GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH,
                    "collect_data submit requires auth proof".to_string(),
                )
            })?;
            let verified = verify_collect_data_auth_proof(&command, auth)
                .map_err(|err| (401, GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH, err))?;
            let world = super::execution_bridge::load_execution_world(execution_world_dir)
                .map_err(|err| (503, GAMEPLAY_SUBMIT_ERROR_INTERNAL, err))?;
            let matching_claims = world
                .state()
                .starter_oc_claims
                .values()
                .filter(|claim| {
                    claim.player_id == verified.player_id
                        && claim.public_key.as_deref() == Some(verified.public_key.as_str())
                })
                .collect::<Vec<_>>();
            let [claim] = matching_claims.as_slice() else {
                return Err((
                    403,
                    GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH,
                    format!(
                        "collect_data requires exactly one authoritative player/key Agent binding; found {}",
                        matching_claims.len()
                    ),
                ));
            };
            Ok((
                verified,
                oasis7::runtime::Action::CollectData {
                    collector_agent_id: claim.agent_id.clone(),
                    electricity_cost: request.electricity_cost,
                    data_amount: request.data_amount,
                },
            ))
        }
    }
}

fn build_gameplay_submit_action_payload(
    action: oasis7::runtime::Action,
) -> Result<Vec<u8>, String> {
    let envelope = ConsensusActionPayloadEnvelope::from_runtime_action(action);
    encode_consensus_action_payload(&envelope)
}

fn next_gameplay_action_id() -> Result<u64, String> {
    let action_id = NEXT_GAMEPLAY_ACTION_ID.fetch_add(1, Ordering::Relaxed);
    if action_id == 0 {
        return Err("gameplay action id allocator exhausted".to_string());
    }
    Ok(action_id)
}

fn write_gameplay_submit_error(
    stream: &mut TcpStream,
    status_code: u16,
    error_code: &str,
    error: &str,
) -> Result<(), String> {
    let payload = ChainGameplaySubmitResponse::error(error_code, error);
    write_gameplay_submit_json_response(stream, status_code, &payload)
}

fn write_gameplay_submit_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    payload: &ChainGameplaySubmitResponse,
) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(payload)
        .map_err(|err| format!("failed to encode gameplay submit payload: {err}"))?;
    super::write_json_response(stream, status_code, body.as_slice(), false)
        .map_err(|err| format!("failed to write gameplay submit json response: {err}"))
}

fn gameplay_nonce_ledger_lock() -> &'static Mutex<()> {
    GAMEPLAY_NONCE_LEDGER_LOCK.get_or_init(|| Mutex::new(()))
}

fn record_gameplay_nonce(
    execution_world_dir: &Path,
    player_id: &str,
    public_key: &str,
    nonce: u64,
) -> Result<(), GameplayNonceRecordError> {
    let _guard = gameplay_nonce_ledger_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = execution_world_dir.join(GAMEPLAY_NONCE_LEDGER_FILE);
    let mut ledger = if path.exists() {
        let bytes = std::fs::read(path.as_path()).map_err(|err| {
            GameplayNonceRecordError::Internal(format!(
                "read gameplay nonce ledger {} failed: {err}",
                path.display()
            ))
        })?;
        serde_json::from_slice(&bytes).map_err(|err| {
            GameplayNonceRecordError::Internal(format!(
                "parse gameplay nonce ledger {} failed: {err}",
                path.display()
            ))
        })?
    } else {
        GameplayNonceLedger::default()
    };
    ledger
        .record_nonce(player_id, public_key, nonce)
        .map_err(GameplayNonceRecordError::Replay)?;
    let bytes = serde_json::to_vec_pretty(&ledger).map_err(|err| {
        GameplayNonceRecordError::Internal(format!("encode gameplay nonce ledger failed: {err}"))
    })?;
    super::write_bytes_atomic(path.as_path(), bytes.as_slice())
        .map_err(GameplayNonceRecordError::Internal)
}

#[cfg(test)]
pub(super) fn reset_gameplay_submit_state_for_tests() {
    NEXT_GAMEPLAY_ACTION_ID.store(1, Ordering::Relaxed);
}

#[cfg(test)]
#[path = "gameplay_submit_api_tests.rs"]
mod tests;
