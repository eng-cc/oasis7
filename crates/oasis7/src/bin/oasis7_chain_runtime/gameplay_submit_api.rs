use std::collections::{BTreeMap, VecDeque};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oasis7::consensus_action_payload::{
    encode_consensus_action_payload, ConsensusActionPayloadEnvelope,
};
use oasis7::viewer::{
    build_runtime_action_from_gameplay_request, verify_gameplay_action_auth_proof,
    GameplayActionRequest,
};
use oasis7_node::NodeRuntime;
use serde::{Deserialize, Serialize};

const GAMEPLAY_SUBMIT_PATH: &str = "/v1/chain/gameplay/submit";
const GAMEPLAY_SUBMIT_ERROR_INVALID_REQUEST: &str = "invalid_request";
const GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH: &str = "invalid_auth";
const GAMEPLAY_SUBMIT_ERROR_NONCE_REPLAY: &str = "auth_nonce_replay";
const GAMEPLAY_SUBMIT_ERROR_INTERNAL: &str = "internal_error";
const GAMEPLAY_SUBMIT_ERROR_SUBMIT_FAILED: &str = "submit_failed";
const MAX_TRACKED_GAMEPLAY_NONCES: usize = 4096;

static NEXT_GAMEPLAY_ACTION_ID: AtomicU64 = AtomicU64::new(1);
static GAMEPLAY_NONCE_TRACKER: OnceLock<Mutex<GameplayNonceTracker>> = OnceLock::new();

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

#[derive(Debug, Default)]
struct GameplayNonceTracker {
    by_player: BTreeMap<String, VecDeque<u64>>,
    order: VecDeque<(String, u64)>,
}

impl GameplayNonceTracker {
    fn record_nonce(&mut self, player_id: &str, nonce: u64) -> Result<(), String> {
        if nonce == 0 {
            return Err("auth nonce must be greater than zero".to_string());
        }
        let history = self.by_player.entry(player_id.to_string()).or_default();
        if history.iter().any(|existing| *existing == nonce) {
            return Err(format!(
                "auth nonce replay detected for player {} nonce {}",
                player_id, nonce
            ));
        }
        history.push_back(nonce);
        self.order.push_back((player_id.to_string(), nonce));
        self.prune();
        Ok(())
    }

    fn prune(&mut self) {
        while self.order.len() > MAX_TRACKED_GAMEPLAY_NONCES {
            let Some((player_id, nonce)) = self.order.pop_front() else {
                break;
            };
            let mut remove_player = false;
            if let Some(history) = self.by_player.get_mut(player_id.as_str()) {
                if let Some(index) = history.iter().position(|existing| *existing == nonce) {
                    history.remove(index);
                }
                remove_player = history.is_empty();
            }
            if remove_player {
                self.by_player.remove(player_id.as_str());
            }
        }
    }
}

pub(super) fn maybe_handle_gameplay_submit_request(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    method: &str,
    path: &str,
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
    handle_gameplay_submit(stream, request_bytes, runtime)?;
    Ok(true)
}

fn handle_gameplay_submit(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
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
    let request = match parse_gameplay_submit_request(body) {
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
    let Some(auth) = request.auth.as_ref() else {
        write_gameplay_submit_error(
            stream,
            401,
            GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH,
            "gameplay submit requires auth proof",
        )?;
        return Ok(());
    };

    let verified = match verify_gameplay_action_auth_proof(&request, auth) {
        Ok(verified) => verified,
        Err(err) => {
            write_gameplay_submit_error(
                stream,
                401,
                GAMEPLAY_SUBMIT_ERROR_INVALID_AUTH,
                err.as_str(),
            )?;
            return Ok(());
        }
    };

    if let Err(err) = with_gameplay_nonce_tracker(|tracker| {
        tracker.record_nonce(verified.player_id.as_str(), verified.nonce)
    }) {
        write_gameplay_submit_error(
            stream,
            409,
            GAMEPLAY_SUBMIT_ERROR_NONCE_REPLAY,
            err.as_str(),
        )?;
        return Ok(());
    }

    let runtime_action = match build_runtime_action_from_gameplay_request(&request) {
        Ok(action) => action,
        Err(err) => {
            write_gameplay_submit_error(stream, 400, err.code.as_str(), err.message.as_str())?;
            return Ok(());
        }
    };
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
        .submit_consensus_action_payload_as_player(verified.player_id, action_id, payload)
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

fn gameplay_nonce_tracker() -> &'static Mutex<GameplayNonceTracker> {
    GAMEPLAY_NONCE_TRACKER.get_or_init(|| Mutex::new(GameplayNonceTracker::default()))
}

fn with_gameplay_nonce_tracker<T>(
    f: impl FnOnce(&mut GameplayNonceTracker) -> Result<T, String>,
) -> Result<T, String> {
    let mut tracker = gameplay_nonce_tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut tracker)
}

#[cfg(test)]
pub(super) fn reset_gameplay_submit_state_for_tests() {
    NEXT_GAMEPLAY_ACTION_ID.store(1, Ordering::Relaxed);
    let mut tracker = gameplay_nonce_tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    tracker.by_player.clear();
    tracker.order.clear();
}

#[cfg(test)]
#[path = "gameplay_submit_api_tests.rs"]
mod tests;
