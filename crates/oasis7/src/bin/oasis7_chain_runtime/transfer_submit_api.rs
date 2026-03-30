use std::collections::{BTreeMap, VecDeque};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oasis7::consensus_action_payload::{
    decode_consensus_action_payload, encode_consensus_action_payload,
    verify_main_token_runtime_action_auth, ConsensusActionAuthEnvelope, ConsensusActionPayloadBody,
    ConsensusActionPayloadEnvelope, MainTokenActionAuthError, MainTokenActionAuthProof,
    MainTokenActionAuthScheme,
};
use oasis7::runtime::Action;
use oasis7_node::NodeRuntime;
use serde::{Deserialize, Serialize};

#[path = "transfer_submit_explorer_p1_api.rs"]
mod explorer_p1_api;

const TRANSFER_SUBMIT_PATH: &str = "/v1/chain/transfer/submit";
const TRANSFER_STATUS_PATH: &str = "/v1/chain/transfer/status";
const TRANSFER_HISTORY_PATH: &str = "/v1/chain/transfer/history";
const TRANSFER_ACCOUNTS_PATH: &str = "/v1/chain/transfer/accounts";
const EXPLORER_OVERVIEW_PATH: &str = "/v1/chain/explorer/overview";
const EXPLORER_TRANSACTIONS_PATH: &str = "/v1/chain/explorer/transactions";
const EXPLORER_TRANSACTION_PATH: &str = "/v1/chain/explorer/transaction";
const ACCOUNT_ID_MAX_LEN: usize = 128;
const MAX_TRACKED_TRANSFER_RECORDS: usize = 500;
const DEFAULT_HISTORY_LIMIT: usize = 50;
const MAX_HISTORY_LIMIT: usize = 200;
const TRANSFER_PENDING_AFTER_MS: i64 = 800;
const TRANSFER_TIMEOUT_MS: i64 = 30_000;
const TRANSFER_ERROR_INVALID_REQUEST: &str = "invalid_request";
const TRANSFER_ERROR_INTERNAL: &str = "internal_error";
const TRANSFER_ERROR_SUBMIT_FAILED: &str = "submit_failed";
const TRANSFER_ERROR_NOT_FOUND: &str = "not_found";
const TRANSFER_ERROR_INVALID_SIGNATURE: &str = "invalid_signature";
const TRANSFER_ERROR_ACCOUNT_AUTH_MISMATCH: &str = "account_auth_mismatch";

static NEXT_TRANSFER_ACTION_ID: AtomicU64 = AtomicU64::new(1);
static TRANSFER_TRACKER: OnceLock<Mutex<TransferTracker>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TransferLifecycleStatus {
    Accepted,
    Pending,
    Confirmed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChainTransferSubmitRequest {
    pub(super) from_account_id: String,
    pub(super) to_account_id: String,
    pub(super) amount: u64,
    pub(super) nonce: u64,
    pub(super) public_key: String,
    pub(super) signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferSubmitResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) action_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) submitted_at_unix_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) lifecycle_status: Option<TransferLifecycleStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainTransferSubmitResponse {
    pub(super) fn success(action_id: u64, submitted_at_unix_ms: i64) -> Self {
        Self {
            ok: true,
            action_id: Some(action_id),
            submitted_at_unix_ms: Some(submitted_at_unix_ms),
            lifecycle_status: Some(TransferLifecycleStatus::Accepted),
            error_code: None,
            error: None,
        }
    }

    pub(super) fn error(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            action_id: None,
            submitted_at_unix_ms: None,
            lifecycle_status: None,
            error_code: Some(error_code.into()),
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferRecord {
    pub(super) action_id: u64,
    pub(super) from_account_id: String,
    pub(super) to_account_id: String,
    pub(super) amount: u64,
    pub(super) nonce: u64,
    pub(super) status: TransferLifecycleStatus,
    pub(super) submitted_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferStatusResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) action_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) status: Option<ChainTransferRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainTransferStatusResponse {
    fn success(action_id: u64, status: ChainTransferRecord) -> Self {
        Self {
            ok: true,
            observed_at_unix_ms: super::now_unix_ms(),
            action_id,
            status: Some(status),
            error_code: None,
            error: None,
        }
    }

    fn error(action_id: u64, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            observed_at_unix_ms: super::now_unix_ms(),
            action_id,
            status: None,
            error_code: Some(code.into()),
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferHistoryResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) account_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) status_filter: Option<TransferLifecycleStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) action_filter: Option<u64>,
    pub(super) limit: usize,
    pub(super) total: usize,
    pub(super) items: Vec<ChainTransferRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainTransferHistoryResponse {
    fn success(
        account_filter: Option<String>,
        status_filter: Option<TransferLifecycleStatus>,
        action_filter: Option<u64>,
        limit: usize,
        total: usize,
        items: Vec<ChainTransferRecord>,
    ) -> Self {
        Self {
            ok: true,
            observed_at_unix_ms: super::now_unix_ms(),
            account_filter,
            status_filter,
            action_filter,
            limit,
            total,
            items,
            error_code: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferAccountEntry {
    pub(super) account_id: String,
    pub(super) liquid_balance: u64,
    pub(super) vested_balance: u64,
    pub(super) restricted_starter_claim_balance: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_transfer_nonce: Option<u64>,
    pub(super) next_nonce_hint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainTransferAccountsResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) node_id: String,
    pub(super) world_id: String,
    pub(super) accounts: Vec<ChainTransferAccountEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainTransferAccountsResponse {
    fn success(node_id: &str, world_id: &str, accounts: Vec<ChainTransferAccountEntry>) -> Self {
        Self {
            ok: true,
            observed_at_unix_ms: super::now_unix_ms(),
            node_id: node_id.to_string(),
            world_id: world_id.to_string(),
            accounts,
            error_code: None,
            error: None,
        }
    }

    fn error(
        node_id: &str,
        world_id: &str,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            ok: false,
            observed_at_unix_ms: super::now_unix_ms(),
            node_id: node_id.to_string(),
            world_id: world_id.to_string(),
            accounts: Vec::new(),
            error_code: Some(code.into()),
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChainExplorerOverviewResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) node_id: String,
    pub(super) world_id: String,
    pub(super) latest_height: u64,
    pub(super) committed_height: u64,
    pub(super) network_committed_height: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_block_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_execution_block_hash: Option<String>,
    pub(super) tracked_records: usize,
    pub(super) transfer_total: usize,
    pub(super) transfer_accepted: usize,
    pub(super) transfer_pending: usize,
    pub(super) transfer_confirmed: usize,
    pub(super) transfer_failed: usize,
    pub(super) transfer_timeout: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ChainExplorerOverviewResponse {
    fn success(
        node_id: &str,
        world_id: &str,
        snapshot: &oasis7_node::NodeSnapshot,
        counters: TransferLifecycleCounters,
    ) -> Self {
        Self {
            ok: true,
            observed_at_unix_ms: super::now_unix_ms(),
            node_id: node_id.to_string(),
            world_id: world_id.to_string(),
            latest_height: snapshot.consensus.latest_height,
            committed_height: snapshot.consensus.committed_height,
            network_committed_height: snapshot.consensus.network_committed_height,
            last_block_hash: snapshot.consensus.last_block_hash.clone(),
            last_execution_block_hash: snapshot.consensus.last_execution_block_hash.clone(),
            tracked_records: counters.total,
            transfer_total: counters.total,
            transfer_accepted: counters.accepted,
            transfer_pending: counters.pending,
            transfer_confirmed: counters.confirmed,
            transfer_failed: counters.failed,
            transfer_timeout: counters.timeout,
            error_code: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TransferLifecycleCounters {
    total: usize,
    accepted: usize,
    pending: usize,
    confirmed: usize,
    failed: usize,
    timeout: usize,
}

#[derive(Debug, Clone)]
struct TrackedTransfer {
    action_id: u64,
    from_account_id: String,
    to_account_id: String,
    amount: u64,
    nonce: u64,
    status: TransferLifecycleStatus,
    submitted_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    error_code: Option<String>,
    error: Option<String>,
}

impl TrackedTransfer {
    fn to_record(&self) -> ChainTransferRecord {
        ChainTransferRecord {
            action_id: self.action_id,
            from_account_id: self.from_account_id.clone(),
            to_account_id: self.to_account_id.clone(),
            amount: self.amount,
            nonce: self.nonce,
            status: self.status,
            submitted_at_unix_ms: self.submitted_at_unix_ms,
            updated_at_unix_ms: self.updated_at_unix_ms,
            error_code: self.error_code.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct TransferTracker {
    by_action_id: BTreeMap<u64, TrackedTransfer>,
    action_order: VecDeque<u64>,
}

impl TransferTracker {
    fn record_accepted(
        &mut self,
        action_id: u64,
        request: &ChainTransferSubmitRequest,
        now_ms: i64,
    ) {
        let tracked = TrackedTransfer {
            action_id,
            from_account_id: request.from_account_id.clone(),
            to_account_id: request.to_account_id.clone(),
            amount: request.amount,
            nonce: request.nonce,
            status: TransferLifecycleStatus::Accepted,
            submitted_at_unix_ms: now_ms,
            updated_at_unix_ms: now_ms,
            error_code: None,
            error: None,
        };
        self.by_action_id.insert(action_id, tracked);
        self.action_order.push_back(action_id);
        self.prune();
    }

    fn upsert_confirmed(
        &mut self,
        action_id: u64,
        from_account_id: String,
        to_account_id: String,
        amount: u64,
        nonce: u64,
        committed_at_unix_ms: i64,
    ) {
        match self.by_action_id.get_mut(&action_id) {
            Some(existing) => {
                existing.status = TransferLifecycleStatus::Confirmed;
                existing.updated_at_unix_ms = committed_at_unix_ms;
                existing.error_code = None;
                existing.error = None;
            }
            None => {
                self.by_action_id.insert(
                    action_id,
                    TrackedTransfer {
                        action_id,
                        from_account_id,
                        to_account_id,
                        amount,
                        nonce,
                        status: TransferLifecycleStatus::Confirmed,
                        submitted_at_unix_ms: committed_at_unix_ms,
                        updated_at_unix_ms: committed_at_unix_ms,
                        error_code: None,
                        error: None,
                    },
                );
                self.action_order.push_back(action_id);
                self.prune();
            }
        }
    }

    fn refresh_lifecycle_by_time(&mut self, now_ms: i64) {
        for item in self.by_action_id.values_mut() {
            match item.status {
                TransferLifecycleStatus::Accepted => {
                    if now_ms.saturating_sub(item.submitted_at_unix_ms) >= TRANSFER_TIMEOUT_MS {
                        item.status = TransferLifecycleStatus::Timeout;
                        item.updated_at_unix_ms = now_ms;
                    } else if now_ms.saturating_sub(item.submitted_at_unix_ms)
                        >= TRANSFER_PENDING_AFTER_MS
                    {
                        item.status = TransferLifecycleStatus::Pending;
                        item.updated_at_unix_ms = now_ms;
                    }
                }
                TransferLifecycleStatus::Pending => {
                    if now_ms.saturating_sub(item.submitted_at_unix_ms) >= TRANSFER_TIMEOUT_MS {
                        item.status = TransferLifecycleStatus::Timeout;
                        item.updated_at_unix_ms = now_ms;
                    }
                }
                TransferLifecycleStatus::Confirmed
                | TransferLifecycleStatus::Failed
                | TransferLifecycleStatus::Timeout => {}
            }
        }
    }

    fn get_record(&self, action_id: u64) -> Option<ChainTransferRecord> {
        self.by_action_id
            .get(&action_id)
            .map(TrackedTransfer::to_record)
    }

    fn query_history(
        &self,
        account_filter: Option<&str>,
        status_filter: Option<TransferLifecycleStatus>,
        action_filter: Option<u64>,
        limit: usize,
    ) -> (usize, Vec<ChainTransferRecord>) {
        let mut items = self
            .action_order
            .iter()
            .rev()
            .filter_map(|action_id| self.by_action_id.get(action_id))
            .filter(|item| {
                if let Some(action_id) = action_filter {
                    return item.action_id == action_id;
                }
                if let Some(account) = account_filter {
                    if item.from_account_id != account && item.to_account_id != account {
                        return false;
                    }
                }
                if let Some(status) = status_filter {
                    if item.status != status {
                        return false;
                    }
                }
                true
            })
            .map(TrackedTransfer::to_record)
            .collect::<Vec<_>>();

        items.sort_by(|left, right| {
            right
                .submitted_at_unix_ms
                .cmp(&left.submitted_at_unix_ms)
                .then_with(|| right.action_id.cmp(&left.action_id))
        });

        let total = items.len();
        items.truncate(limit);
        (total, items)
    }

    fn lifecycle_counters(&self) -> TransferLifecycleCounters {
        let mut counters = TransferLifecycleCounters::default();
        for item in self.by_action_id.values() {
            counters.total += 1;
            match item.status {
                TransferLifecycleStatus::Accepted => counters.accepted += 1,
                TransferLifecycleStatus::Pending => counters.pending += 1,
                TransferLifecycleStatus::Confirmed => counters.confirmed += 1,
                TransferLifecycleStatus::Failed => counters.failed += 1,
                TransferLifecycleStatus::Timeout => counters.timeout += 1,
            }
        }
        counters
    }

    fn prune(&mut self) {
        while self.action_order.len() > MAX_TRACKED_TRANSFER_RECORDS {
            if let Some(action_id) = self.action_order.pop_front() {
                self.by_action_id.remove(&action_id);
            }
        }
    }
}

pub(super) fn maybe_handle_transfer_submit_request(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    method: &str,
    path: &str,
    node_id: &str,
    world_id: &str,
    execution_world_dir: &Path,
) -> Result<bool, String> {
    super::explorer_p0_api::configure_persistence_root(execution_world_dir);

    if method.eq_ignore_ascii_case("POST") && path == TRANSFER_SUBMIT_PATH {
        return handle_transfer_submit(stream, request_bytes, runtime, execution_world_dir);
    }

    if !(method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD")) {
        return Ok(false);
    }

    if path == TRANSFER_STATUS_PATH {
        return handle_transfer_status(stream, request_bytes, runtime, head_only(method));
    }
    if path == TRANSFER_HISTORY_PATH {
        return handle_transfer_history(stream, request_bytes, runtime, head_only(method));
    }
    if path == TRANSFER_ACCOUNTS_PATH {
        return handle_transfer_accounts(
            stream,
            runtime,
            node_id,
            world_id,
            execution_world_dir,
            head_only(method),
        );
    }
    if path == EXPLORER_OVERVIEW_PATH {
        return handle_explorer_overview(stream, runtime, node_id, world_id, head_only(method));
    }
    if path == EXPLORER_TRANSACTIONS_PATH {
        return handle_explorer_transactions(stream, request_bytes, runtime, head_only(method));
    }
    if path == EXPLORER_TRANSACTION_PATH {
        return handle_explorer_transaction(stream, request_bytes, runtime, head_only(method));
    }

    if explorer_p1_api::maybe_handle_explorer_p1_request(
        stream,
        request_bytes,
        runtime,
        path,
        execution_world_dir,
        head_only(method),
    )? {
        return Ok(true);
    }

    if super::explorer_p0_api::maybe_handle_explorer_p0_request(
        stream,
        request_bytes,
        path,
        head_only(method),
    )? {
        return Ok(true);
    }

    Ok(false)
}

fn head_only(method: &str) -> bool {
    method.eq_ignore_ascii_case("HEAD")
}

fn handle_transfer_submit(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    execution_world_dir: &Path,
) -> Result<bool, String> {
    let body = match super::feedback_submit_api::extract_http_json_body(request_bytes) {
        Ok(body) => body,
        Err(err) => {
            write_transfer_submit_error(stream, 400, TRANSFER_ERROR_INVALID_REQUEST, err.as_str())?;
            return Ok(true);
        }
    };
    let submit_request = match parse_transfer_submit_request(body) {
        Ok(request) => request,
        Err(err) => {
            write_transfer_submit_error(stream, 400, TRANSFER_ERROR_INVALID_REQUEST, err.as_str())?;
            return Ok(true);
        }
    };
    if let Err((code, message)) = verify_transfer_submit_request_auth(&submit_request) {
        write_transfer_submit_error(stream, 400, code.as_str(), message.as_str())?;
        return Ok(true);
    }

    if let Err((code, message)) =
        preflight_validate_transfer_request(execution_world_dir, &submit_request)
    {
        write_transfer_submit_error(stream, 400, code.as_str(), message.as_str())?;
        return Ok(true);
    }

    let action_id = match next_transfer_action_id() {
        Ok(action_id) => action_id,
        Err(err) => {
            write_transfer_submit_error(stream, 502, TRANSFER_ERROR_INTERNAL, err.as_str())?;
            return Ok(true);
        }
    };
    let payload = match build_transfer_submit_action_payload(&submit_request) {
        Ok(payload) => payload,
        Err(err) => {
            write_transfer_submit_error(stream, 502, TRANSFER_ERROR_INTERNAL, err.as_str())?;
            return Ok(true);
        }
    };
    if let Err(err) = runtime
        .lock()
        .map_err(|_| "failed to lock node runtime for transfer submit".to_string())?
        .submit_consensus_action_payload(action_id, payload)
    {
        write_transfer_submit_error(
            stream,
            502,
            TRANSFER_ERROR_SUBMIT_FAILED,
            format!("transfer submit failed: {err}").as_str(),
        )?;
        return Ok(true);
    }

    let now_ms = super::now_unix_ms();
    with_transfer_tracker(|tracker| tracker.record_accepted(action_id, &submit_request, now_ms));
    super::explorer_p0_api::record_transfer_accepted(action_id, &submit_request, now_ms);

    let response = ChainTransferSubmitResponse::success(action_id, now_ms);
    write_transfer_json_response(stream, 200, &response, false)
        .map_err(|err| format!("failed to write transfer submit response: {err}"))?;
    Ok(true)
}

fn handle_transfer_status(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    head_only: bool,
) -> Result<bool, String> {
    let target = parse_http_target(request_bytes)?;
    let params = parse_query_params(target.as_str());
    let action_id = match params
        .get("action_id")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        Some(action_id) => action_id,
        None => {
            let response = ChainTransferStatusResponse::error(
                0,
                TRANSFER_ERROR_INVALID_REQUEST,
                "query parameter action_id must be a positive integer",
            );
            write_transfer_json_response(stream, 400, &response, head_only)?;
            return Ok(true);
        }
    };

    let mut tracker = lock_transfer_tracker();
    sync_tracker_from_runtime(runtime, &mut tracker)?;
    tracker.refresh_lifecycle_by_time(super::now_unix_ms());

    let response = match tracker.get_record(action_id) {
        Some(status) => ChainTransferStatusResponse::success(action_id, status),
        None => ChainTransferStatusResponse::error(
            action_id,
            TRANSFER_ERROR_NOT_FOUND,
            format!("transfer action_id not found: {action_id}"),
        ),
    };
    write_transfer_json_response(stream, 200, &response, head_only)?;
    Ok(true)
}

fn handle_transfer_history(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    head_only: bool,
) -> Result<bool, String> {
    let target = parse_http_target(request_bytes)?;
    let params = parse_query_params(target.as_str());

    let account_filter = params
        .get("account_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let action_filter = params
        .get("action_id")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    let limit = params
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(1, MAX_HISTORY_LIMIT))
        .unwrap_or(DEFAULT_HISTORY_LIMIT);

    let mut tracker = lock_transfer_tracker();
    sync_tracker_from_runtime(runtime, &mut tracker)?;
    tracker.refresh_lifecycle_by_time(super::now_unix_ms());

    let (total, items) =
        tracker.query_history(account_filter.as_deref(), None, action_filter, limit);
    let response = ChainTransferHistoryResponse::success(
        account_filter,
        None,
        action_filter,
        limit,
        total,
        items,
    );
    write_transfer_json_response(stream, 200, &response, head_only)?;
    Ok(true)
}

fn handle_transfer_accounts(
    stream: &mut TcpStream,
    runtime: &Arc<Mutex<NodeRuntime>>,
    node_id: &str,
    world_id: &str,
    execution_world_dir: &Path,
    head_only: bool,
) -> Result<bool, String> {
    let mut tracker = lock_transfer_tracker();
    sync_tracker_from_runtime(runtime, &mut tracker)?;
    tracker.refresh_lifecycle_by_time(super::now_unix_ms());

    let response = build_transfer_accounts_response(node_id, world_id, execution_world_dir);
    write_transfer_json_response(stream, 200, &response, head_only)?;
    Ok(true)
}

fn handle_explorer_overview(
    stream: &mut TcpStream,
    runtime: &Arc<Mutex<NodeRuntime>>,
    node_id: &str,
    world_id: &str,
    head_only: bool,
) -> Result<bool, String> {
    let mut tracker = lock_transfer_tracker();
    sync_tracker_from_runtime(runtime, &mut tracker)?;
    tracker.refresh_lifecycle_by_time(super::now_unix_ms());

    let snapshot = runtime
        .lock()
        .map_err(|_| "failed to lock node runtime for explorer overview".to_string())?
        .snapshot();
    let counters = tracker.lifecycle_counters();
    let response = ChainExplorerOverviewResponse::success(node_id, world_id, &snapshot, counters);
    write_transfer_json_response(stream, 200, &response, head_only)?;
    Ok(true)
}

fn handle_explorer_transactions(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    head_only: bool,
) -> Result<bool, String> {
    let target = parse_http_target(request_bytes)?;
    let params = parse_query_params(target.as_str());

    let account_filter = params
        .get("account_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let action_filter = params
        .get("action_id")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    let status_filter = match params
        .get("status")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(raw) => match parse_transfer_lifecycle_status(raw) {
            Some(status) => Some(status),
            None => {
                let response = ChainTransferHistoryResponse {
                    ok: false,
                    observed_at_unix_ms: super::now_unix_ms(),
                    account_filter: account_filter.clone(),
                    status_filter: None,
                    action_filter,
                    limit: 0,
                    total: 0,
                    items: Vec::new(),
                    error_code: Some(TRANSFER_ERROR_INVALID_REQUEST.to_string()),
                    error: Some(format!(
                        "query parameter status must be one of: accepted,pending,confirmed,failed,timeout; got `{raw}`"
                    )),
                };
                write_transfer_json_response(stream, 400, &response, head_only)?;
                return Ok(true);
            }
        },
        None => None,
    };
    let limit = params
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(1, MAX_HISTORY_LIMIT))
        .unwrap_or(DEFAULT_HISTORY_LIMIT);

    let mut tracker = lock_transfer_tracker();
    sync_tracker_from_runtime(runtime, &mut tracker)?;
    tracker.refresh_lifecycle_by_time(super::now_unix_ms());
    let (total, items) = tracker.query_history(
        account_filter.as_deref(),
        status_filter,
        action_filter,
        limit,
    );
    let response = ChainTransferHistoryResponse::success(
        account_filter,
        status_filter,
        action_filter,
        limit,
        total,
        items,
    );
    write_transfer_json_response(stream, 200, &response, head_only)?;
    Ok(true)
}

fn handle_explorer_transaction(
    stream: &mut TcpStream,
    request_bytes: &[u8],
    runtime: &Arc<Mutex<NodeRuntime>>,
    head_only: bool,
) -> Result<bool, String> {
    handle_transfer_status(stream, request_bytes, runtime, head_only)
}

fn write_transfer_json_response<T: Serialize>(
    stream: &mut TcpStream,
    status_code: u16,
    payload: &T,
    head_only: bool,
) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(payload)
        .map_err(|err| format!("failed to encode transfer API response: {err}"))?;
    super::write_json_response(stream, status_code, body.as_slice(), head_only)
        .map_err(|err| format!("failed to write transfer API response: {err}"))
}

fn build_transfer_accounts_response(
    node_id: &str,
    world_id: &str,
    execution_world_dir: &Path,
) -> ChainTransferAccountsResponse {
    let world = match super::execution_bridge::load_execution_world(execution_world_dir) {
        Ok(world) => world,
        Err(err) => {
            return ChainTransferAccountsResponse::error(
                node_id,
                world_id,
                TRANSFER_ERROR_INTERNAL,
                err,
            );
        }
    };

    let mut accounts = world
        .main_token_account_balances()
        .into_iter()
        .map(|balance| {
            let last_nonce = world.main_token_last_transfer_nonce(balance.account_id.as_str());
            let next_nonce_hint = last_nonce.unwrap_or(0).saturating_add(1);
            ChainTransferAccountEntry {
                account_id: balance.account_id,
                liquid_balance: balance.liquid_balance,
                vested_balance: balance.vested_balance,
                restricted_starter_claim_balance: balance.restricted_starter_claim_balance,
                last_transfer_nonce: last_nonce,
                next_nonce_hint,
            }
        })
        .collect::<Vec<_>>();

    if let Some(node_account) = world.node_main_token_account(node_id) {
        let exists = accounts
            .iter()
            .any(|account| account.account_id == node_account);
        if !exists {
            let balance = world
                .main_token_account_balance(node_account)
                .cloned()
                .unwrap_or_default();
            let last_nonce = world.main_token_last_transfer_nonce(node_account);
            accounts.push(ChainTransferAccountEntry {
                account_id: node_account.to_string(),
                liquid_balance: balance.liquid_balance,
                vested_balance: balance.vested_balance,
                restricted_starter_claim_balance: balance.restricted_starter_claim_balance,
                last_transfer_nonce: last_nonce,
                next_nonce_hint: last_nonce.unwrap_or(0).saturating_add(1),
            });
        }
    }

    accounts.sort_by(|left, right| left.account_id.cmp(&right.account_id));
    ChainTransferAccountsResponse::success(node_id, world_id, accounts)
}

fn preflight_validate_transfer_request(
    execution_world_dir: &Path,
    request: &ChainTransferSubmitRequest,
) -> Result<(), (String, String)> {
    let world = match super::execution_bridge::load_execution_world(execution_world_dir) {
        Ok(world) => world,
        Err(err) => {
            return Err((
                TRANSFER_ERROR_INTERNAL.to_string(),
                format!("load execution world failed: {err}"),
            ));
        }
    };

    if let Some(from_account) = world.main_token_account_balance(request.from_account_id.as_str()) {
        if from_account.liquid_balance < request.amount {
            return Err((
                "insufficient_balance".to_string(),
                format!(
                    "insufficient balance: account={} transferable_balance={} restricted_starter_claim_balance={} amount={}",
                    request.from_account_id,
                    from_account.liquid_balance,
                    from_account.restricted_starter_claim_balance,
                    request.amount
                ),
            ));
        }
    }

    let last_nonce = world
        .main_token_last_transfer_nonce(request.from_account_id.as_str())
        .unwrap_or(0);
    if last_nonce > 0 && request.nonce <= last_nonce {
        return Err((
            "nonce_replay".to_string(),
            format!(
                "nonce replay detected: account={} nonce={} last_nonce={}",
                request.from_account_id, request.nonce, last_nonce
            ),
        ));
    }

    Ok(())
}

fn sync_tracker_from_runtime(
    runtime: &Arc<Mutex<NodeRuntime>>,
    tracker: &mut TransferTracker,
) -> Result<(), String> {
    let batches = runtime
        .lock()
        .map_err(|_| "failed to lock node runtime for transfer tracker sync".to_string())?
        .drain_committed_action_batches();
    super::explorer_p0_api::ingest_committed_batches(batches.as_slice());

    for batch in batches {
        for committed_action in batch.actions {
            let action_id = committed_action.action_id;
            let decoded =
                match decode_consensus_action_payload(committed_action.payload_cbor.as_slice()) {
                    Ok(decoded) => decoded,
                    Err(_) => continue,
                };
            let ConsensusActionPayloadBody::RuntimeAction { action } = decoded else {
                continue;
            };
            let Action::TransferMainToken {
                from_account_id,
                to_account_id,
                amount,
                nonce,
            } = action
            else {
                continue;
            };
            tracker.upsert_confirmed(
                action_id,
                from_account_id,
                to_account_id,
                amount,
                nonce,
                batch.committed_at_unix_ms,
            );
        }
    }

    Ok(())
}

pub(super) fn parse_transfer_submit_request(
    body: &[u8],
) -> Result<ChainTransferSubmitRequest, String> {
    let mut request: ChainTransferSubmitRequest = serde_json::from_slice(body)
        .map_err(|err| format!("invalid transfer submit payload: {err}"))?;

    request.from_account_id =
        normalize_account_id(request.from_account_id.as_str(), "from_account_id")?;
    request.to_account_id = normalize_account_id(request.to_account_id.as_str(), "to_account_id")?;
    request.public_key =
        normalize_public_key_field(request.public_key.as_str(), "transfer public_key")?;
    request.signature =
        normalize_signature_field(request.signature.as_str(), "transfer signature")?;

    if request.from_account_id == request.to_account_id {
        return Err("transfer from_account_id and to_account_id cannot be the same".to_string());
    }
    if request.amount == 0 {
        return Err("transfer amount must be > 0".to_string());
    }
    if request.nonce == 0 {
        return Err("transfer nonce must be > 0".to_string());
    }
    Ok(request)
}

fn verify_transfer_submit_request_auth(
    request: &ChainTransferSubmitRequest,
) -> Result<(), (String, String)> {
    let action = build_transfer_submit_action(request);
    let proof = build_transfer_submit_auth_proof(request);
    verify_main_token_runtime_action_auth(&action, &proof)
        .map(|_| ())
        .map_err(map_transfer_auth_error)
}

#[path = "transfer_submit_api_support.rs"]
mod transfer_submit_api_support;

use transfer_submit_api_support::*;

#[cfg(test)]
#[path = "transfer_submit_api_tests.rs"]
mod tests;
