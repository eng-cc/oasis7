use std::collections::{BTreeMap, BTreeSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use super::super::api::{read_http_request, write_http_response, HttpRequest};

#[derive(Debug, Clone)]
pub(super) struct MockChainTx {
    pub(super) tx_hash: String,
    pub(super) action_id: u64,
    pub(super) from_account_id: String,
    pub(super) to_account_id: String,
    pub(super) amount: u64,
    pub(super) submitted_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
    pub(super) block_height: Option<u64>,
}

#[derive(Debug, Clone)]
pub(super) struct MockChainState {
    pub(super) committed_height: u64,
    pub(super) txs: Vec<MockChainTx>,
}

impl Default for MockChainState {
    fn default() -> Self {
        Self {
            committed_height: 0,
            txs: Vec::new(),
        }
    }
}

pub(super) struct MockChainServer {
    pub(super) base_url: String,
    state: Arc<Mutex<MockChainState>>,
    stop: Arc<AtomicBool>,
}

impl MockChainServer {
    pub(super) fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock chain server");
        listener
            .set_nonblocking(true)
            .expect("set mock chain server nonblocking");
        let base_url = format!("http://{}", listener.local_addr().expect("mock chain addr"));
        let state = Arc::new(Mutex::new(MockChainState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&state);
        let stop_for_thread = Arc::clone(&stop);
        thread::spawn(move || loop {
            if stop_for_thread.load(Ordering::Relaxed) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_http_request(&mut stream).expect("read mock chain request");
                    handle_mock_chain_request(&mut stream, &state_for_thread, request);
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("mock chain accept failed: {err}"),
            }
        });
        Self {
            base_url,
            state,
            stop,
        }
    }

    pub(super) fn set_state(&self, state: MockChainState) {
        *self.state.lock().expect("mock chain state lock") = state;
    }

    pub(super) fn set_committed_height(&self, committed_height: u64) {
        self.state
            .lock()
            .expect("mock chain state lock")
            .committed_height = committed_height;
    }
}

impl Drop for MockChainServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_mock_chain_request(
    stream: &mut TcpStream,
    state: &Arc<Mutex<MockChainState>>,
    request: HttpRequest,
) {
    let path = request
        .path
        .split('?')
        .next()
        .unwrap_or(request.path.as_str());
    match (request.method.as_str(), path) {
        ("GET", "/v1/chain/explorer/overview") => {
            let payload = {
                let state = state.lock().expect("mock chain state lock");
                json!({
                    "ok": true,
                    "committed_height": state.committed_height,
                })
            };
            respond_json(stream, 200, &payload);
        }
        ("GET", "/v1/chain/explorer/txs") => {
            let account_id = query_param(request.path.as_str(), "account_id")
                .expect("mock chain account_id query");
            let payload = {
                let state = state.lock().expect("mock chain state lock");
                let items = state
                    .txs
                    .iter()
                    .filter(|tx| tx.to_account_id == account_id)
                    .map(|tx| {
                        json!({
                            "tx_hash": tx.tx_hash,
                            "action_id": tx.action_id,
                            "from_account_id": tx.from_account_id,
                            "to_account_id": tx.to_account_id,
                            "amount": tx.amount,
                            "submitted_at_unix_ms": tx.submitted_at_unix_ms,
                            "updated_at_unix_ms": tx.updated_at_unix_ms,
                            "block_height": tx.block_height,
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "ok": true,
                    "items": items,
                })
            };
            respond_json(stream, 200, &payload);
        }
        _ => respond_json(stream, 404, &json!({"ok": false, "error": "not found"})),
    }
}

#[derive(Debug, Clone)]
struct MockProjectRecord {
    external_project_id: String,
    platform_project_id: String,
    platform_user_id: String,
    project_name: String,
    token_key: String,
}

#[derive(Debug, Clone, Default)]
struct MockLetaiState {
    next_user_seq: u64,
    next_project_seq: u64,
    user_ids_by_external: BTreeMap<String, String>,
    project_by_user_and_external: BTreeMap<(String, String), MockProjectRecord>,
    topup_requests: Vec<Value>,
    topup_records: Vec<(String, String, u64)>,
    fail_first_topup_requests: usize,
    fail_first_user_upsert_requests: usize,
    omit_logs_for_orders: BTreeSet<String>,
}

pub(super) struct MockLetaiServer {
    pub(super) base_url: String,
    state: Arc<Mutex<MockLetaiState>>,
    stop: Arc<AtomicBool>,
}

impl MockLetaiServer {
    pub(super) fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock letai server");
        listener
            .set_nonblocking(true)
            .expect("set mock letai server nonblocking");
        let base_url = format!("http://{}", listener.local_addr().expect("mock letai addr"));
        let state = Arc::new(Mutex::new(MockLetaiState {
            next_user_seq: 1,
            next_project_seq: 1,
            ..MockLetaiState::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&state);
        let stop_for_thread = Arc::clone(&stop);
        thread::spawn(move || loop {
            if stop_for_thread.load(Ordering::Relaxed) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_http_request(&mut stream).expect("read mock letai request");
                    handle_mock_letai_request(&mut stream, &state_for_thread, request);
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("mock letai accept failed: {err}"),
            }
        });
        Self {
            base_url,
            state,
            stop,
        }
    }

    pub(super) fn fail_first_topup_requests(&self, count: usize) {
        self.state
            .lock()
            .expect("mock letai state lock")
            .fail_first_topup_requests = count;
    }

    pub(super) fn fail_first_user_upsert_requests(&self, count: usize) {
        self.state
            .lock()
            .expect("mock letai state lock")
            .fail_first_user_upsert_requests = count;
    }

    pub(super) fn omit_logs_for_order(&self, external_order_id: &str) {
        self.state
            .lock()
            .expect("mock letai state lock")
            .omit_logs_for_orders
            .insert(external_order_id.to_string());
    }

    pub(super) fn recorded_topup_requests(&self) -> Vec<Value> {
        self.state
            .lock()
            .expect("mock letai state lock")
            .topup_requests
            .clone()
    }
}

impl Drop for MockLetaiServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_mock_letai_request(
    stream: &mut TcpStream,
    state: &Arc<Mutex<MockLetaiState>>,
    request: HttpRequest,
) {
    let path = request
        .path
        .split('?')
        .next()
        .unwrap_or(request.path.as_str());
    match (request.method.as_str(), path) {
        ("POST", "/api/platform/open/users/upsert") => {
            let payload: Value =
                serde_json::from_slice(request.body.as_slice()).expect("user upsert payload");
            let external_user_id = payload
                .get("external_user_id")
                .and_then(Value::as_str)
                .expect("external_user_id")
                .to_string();
            {
                let mut state = state.lock().expect("mock letai state lock");
                if state.fail_first_user_upsert_requests > 0 {
                    state.fail_first_user_upsert_requests -= 1;
                    respond_json(
                        stream,
                        500,
                        &json!({"success": false, "message": "temporary user upsert failure"}),
                    );
                    return;
                }
            }
            let platform_user_id = {
                let mut state = state.lock().expect("mock letai state lock");
                if let Some(existing) = state.user_ids_by_external.get(external_user_id.as_str()) {
                    existing.clone()
                } else {
                    let next = format!("platform-user-{:06}", state.next_user_seq);
                    state.next_user_seq = state.next_user_seq.saturating_add(1);
                    state
                        .user_ids_by_external
                        .insert(external_user_id.clone(), next.clone());
                    next
                }
            };
            respond_json(
                stream,
                200,
                &json!({
                    "success": true,
                    "data": {
                        "platform_user_id": platform_user_id,
                        "external_user_id": external_user_id,
                    }
                }),
            );
        }
        ("POST", _) if path.ends_with("/projects/upsert") => {
            let payload: Value =
                serde_json::from_slice(request.body.as_slice()).expect("project ensure payload");
            let platform_user_id = path
                .trim_start_matches("/api/platform/open/users/")
                .trim_end_matches("/projects/upsert")
                .trim_end_matches('/')
                .to_string();
            let external_project_id = payload
                .get("external_project_id")
                .and_then(Value::as_str)
                .expect("external_project_id")
                .to_string();
            let project_name = payload
                .get("external_project_name")
                .and_then(Value::as_str)
                .unwrap_or("default-project")
                .to_string();
            let record = {
                let mut state = state.lock().expect("mock letai state lock");
                let key = (platform_user_id.clone(), external_project_id.clone());
                if let Some(existing) = state.project_by_user_and_external.get(&key) {
                    existing.clone()
                } else {
                    let seq = state.next_project_seq;
                    state.next_project_seq = state.next_project_seq.saturating_add(1);
                    let record = MockProjectRecord {
                        external_project_id: external_project_id.clone(),
                        platform_project_id: format!("platform-project-{:06}", seq),
                        platform_user_id: platform_user_id.clone(),
                        project_name,
                        token_key: format!("token-key-{:06}", seq),
                    };
                    state
                        .project_by_user_and_external
                        .insert(key, record.clone());
                    record
                }
            };
            respond_json(
                stream,
                200,
                &json!({
                    "success": true,
                    "data": {
                        "platform_project_id": record.platform_project_id.as_str(),
                        "platform_user_id": record.platform_user_id.as_str(),
                        "external_project_id": record.external_project_id.as_str(),
                        "external_project_name": record.project_name.as_str(),
                        "local_user_id": 105,
                        "token_id": 205,
                        "token_name": record.project_name.as_str(),
                        "token_key": record.token_key.as_str(),
                        "group": "default",
                        "status": 1,
                        "token_status": 1,
                        "quota_limited_by": "user",
                        "quota_used": 0,
                        "user_quota_remaining": 2_000_000,
                        "unlimited_quota": true,
                        "expires_at": -1,
                        "metadata": "{\"env\":\"test\"}",
                    }
                }),
            );
        }
        ("POST", _) if path.ends_with("/topups") => {
            let platform_user_id = path
                .trim_start_matches("/api/platform/open/users/")
                .trim_end_matches("/topups")
                .trim_end_matches('/')
                .to_string();
            let payload: Value =
                serde_json::from_slice(request.body.as_slice()).expect("topup payload");
            let mut state = state.lock().expect("mock letai state lock");
            state.topup_requests.push(payload.clone());
            if state.fail_first_topup_requests > 0 {
                state.fail_first_topup_requests -= 1;
                respond_json(
                    stream,
                    500,
                    &json!({"ok": false, "error": "temporary topup failure"}),
                );
                return;
            }
            let external_order_id = payload
                .get("external_order_id")
                .and_then(Value::as_str)
                .expect("external_order_id")
                .to_string();
            let quota = payload.get("quota").and_then(Value::as_u64).expect("quota");
            state
                .topup_records
                .push((platform_user_id.clone(), external_order_id.clone(), quota));
            respond_json(
                stream,
                200,
                &json!({
                    "ok": true,
                    "data": {
                        "platform_user_id": platform_user_id,
                        "external_order_id": external_order_id,
                        "quota": quota,
                    }
                }),
            );
        }
        ("GET", _)
            if path.starts_with("/api/platform/open/users/")
                && !path.ends_with("/logs")
                && !path.ends_with("/projects/upsert") =>
        {
            let platform_user_id = path
                .trim_start_matches("/api/platform/open/users/")
                .trim_end_matches('/')
                .to_string();
            let total_quota = {
                let state = state.lock().expect("mock letai state lock");
                state
                    .topup_records
                    .iter()
                    .filter(|(user_id, _, _)| user_id == &platform_user_id)
                    .map(|(_, _, quota)| *quota)
                    .sum::<u64>()
            };
            respond_json(
                stream,
                200,
                &json!({
                    "ok": true,
                    "data": {
                        "platform_user_id": platform_user_id,
                        "quota_balance": total_quota,
                    }
                }),
            );
        }
        ("GET", _)
            if path.starts_with("/api/platform/open/projects/") && path.ends_with("/summary") =>
        {
            let platform_project_id = path
                .trim_start_matches("/api/platform/open/projects/")
                .trim_end_matches("/summary")
                .trim_end_matches('/')
                .to_string();
            let project = {
                let state = state.lock().expect("mock letai state lock");
                state
                    .project_by_user_and_external
                    .iter()
                    .find(|(_, record)| record.platform_project_id == platform_project_id)
                    .map(|(_, record)| {
                        json!({
                            "platform_project_id": record.platform_project_id,
                            "platform_user_id": record.platform_user_id,
                            "external_project_id": record.external_project_id,
                            "external_project_name": record.project_name,
                            "local_user_id": 105,
                            "token_id": 205,
                            "token_name": record.project_name,
                            "group": "default",
                            "status": 1,
                            "token_status": 1,
                            "quota_limited_by": "user",
                            "quota_used": 0,
                            "user_quota_remaining": 2_000_000,
                            "unlimited_quota": true,
                            "expires_at": -1,
                            "metadata": "{\"env\":\"test\"}",
                        })
                    })
                    .unwrap_or_else(|| json!({}))
            };
            respond_json(stream, 200, &json!({"success": true, "data": project}));
        }
        ("GET", _)
            if path.starts_with("/api/platform/open/projects/") && path.ends_with("/logs") =>
        {
            let platform_project_id = path
                .trim_start_matches("/api/platform/open/projects/")
                .trim_end_matches("/logs")
                .trim_end_matches('/')
                .to_string();
            let external_order_id =
                query_param(request.path.as_str(), "external_order_id").unwrap_or_default();
            let items = {
                let state = state.lock().expect("mock letai state lock");
                if state
                    .omit_logs_for_orders
                    .contains(external_order_id.as_str())
                {
                    Vec::new()
                } else {
                    state
                        .topup_records
                        .iter()
                        .enumerate()
                        .filter(|(_, (user_id, order_id, _))| {
                            order_id == &external_order_id
                                && state.project_by_user_and_external.values().any(|record| {
                                    record.platform_project_id == platform_project_id
                                        && record.platform_user_id == *user_id
                                })
                        })
                        .map(|(index, (_, order_id, quota))| {
                            json!({
                                "id": index + 1,
                                "external_order_id": order_id,
                                "quota": quota,
                            })
                        })
                        .collect::<Vec<_>>()
                }
            };
            respond_json(stream, 200, &json!({"ok": true, "items": items}));
        }
        _ => respond_json(stream, 404, &json!({"ok": false, "error": "not found"})),
    }
}

pub(super) fn assert_http_status_line(status_code: u16, expected_prefix: &str) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status listener");
    let addr = listener.local_addr().expect("status listener addr");
    let expected_prefix = expected_prefix.to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status connection");
        write_http_response(
            &mut stream,
            status_code,
            "application/json; charset=utf-8",
            b"{}",
            false,
        )
        .expect("write response");
        let _ = stream.shutdown(Shutdown::Both);
    });

    let mut client = TcpStream::connect(addr).expect("connect status listener");
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("write request");
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set timeout");
    let mut response = String::new();
    client.read_to_string(&mut response).expect("read response");
    server.join().expect("join server");
    assert!(response.starts_with(expected_prefix.as_str()));
}

fn respond_json(stream: &mut TcpStream, status_code: u16, payload: &Value) {
    let bytes = serde_json::to_vec(payload).expect("encode mock payload");
    write_http_response(
        stream,
        status_code,
        "application/json; charset=utf-8",
        bytes.as_slice(),
        false,
    )
    .expect("write mock response");
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (candidate_key, value) = pair.split_once('=')?;
        if candidate_key == key {
            Some(percent_decode(value))
        } else {
            None
        }
    })
}

fn percent_decode(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hi = bytes[index + 1];
                let lo = bytes[index + 2];
                if let (Some(hi), Some(lo)) = (hex_value(hi), hex_value(lo)) {
                    output.push((hi * 16 + lo) as char);
                    index += 3;
                } else {
                    output.push('%');
                    index += 1;
                }
            }
            other => {
                output.push(other as char);
                index += 1;
            }
        }
    }
    output
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
