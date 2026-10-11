use super::{dto::*, protocol};
use crate::{Error, Result};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
pub const FRAME_LIMIT: usize = 1024 * 1024;
pub const REQUEST_LIMIT: usize = 32;
pub struct Shared {
    pub snapshot: RuntimeSnapshot,
    pub replies: HashMap<String, mpsc::SyncSender<std::result::Result<Value, String>>>,
    pub raw_requests: HashMap<String, Value>,
    pub seen_requests: HashSet<String>,
}
pub struct Transport {
    pub shared: Arc<Mutex<Shared>>,
    pub writer: mpsc::SyncSender<Vec<u8>>,
    pub child: Child,
    pub baseline: Vec<super::lifecycle::ProcessIdentity>,
    shutdown_complete: bool,
}
impl Transport {
    pub fn spawn(
        binary: &str,
        root: &Path,
        snapshot: RuntimeSnapshot,
        database: &Path,
    ) -> Result<Self> {
        let version = crate::process::run(binary, &["--version"], root)?;
        let version = std::str::from_utf8(&version)
            .map_err(|_| Error::Invalid("invalid Codex version".into()))?
            .trim();
        if !["codex-cli 0.137.0", "codex 0.137.0"].contains(&version) {
            return Err(Error::Invalid(
                "experimental connector requires exact Codex 0.137.0".into(),
            ));
        }
        let mut cmd = Command::new(binary);
        cmd.arg("app-server")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd.spawn()?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let errors = child.stderr.take().unwrap();
        let shared = Arc::new(Mutex::new(Shared {
            snapshot,
            replies: HashMap::new(),
            raw_requests: HashMap::new(),
            seen_requests: HashSet::new(),
        }));
        let (writer, rx) = mpsc::sync_channel::<Vec<u8>>(64);
        let state = shared.clone();
        std::thread::spawn(move || {
            let mut input = input;
            while let Ok(bytes) = rx.recv() {
                if bytes.is_empty() {
                    break;
                } // Explicit idle shutdown closes stdin.
                if input.write_all(&bytes).and_then(|_| input.flush()).is_err() {
                    degrade(&state, "stdio write failed");
                    break;
                }
            }
        });
        let (events, event_rx) = mpsc::sync_channel::<(RuntimeSnapshot, i64)>(8);
        let event_state = shared.clone();
        let database = database.to_path_buf();
        std::thread::spawn(move || {
            let store = match super::store::Store::writer(&database) {
                Ok(s) => s,
                Err(_) => {
                    degrade(&event_state, "event storage unavailable");
                    return;
                }
            };
            while let Ok((snapshot, sequence)) = event_rx.recv() {
                let scope = format!("{}:{}", snapshot.project_id, snapshot.environment);
                if store.save_at(&scope, &snapshot, sequence, true).is_err() {
                    degrade(
                        &event_state,
                        "critical event persistence failed; new operations disabled",
                    );
                    break;
                }
            }
        });
        let state = shared.clone();
        let tx = writer.clone();
        std::thread::spawn(move || read_loop(output, state, tx, events));
        // Drain diagnostics without retaining credentials or arbitrary stderr content.
        std::thread::spawn(move || {
            let mut reader = errors;
            let mut buf = [0; 8192];
            while matches!(reader.read(&mut buf),Ok(n) if n>0) {}
        });
        Ok(Self {
            shared,
            writer,
            child,
            baseline: Vec::new(),
            shutdown_complete: false,
        })
    }
    pub fn request(
        &self,
        id: &str,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, String> {
        if ![
            "initialize",
            "thread/start",
            "thread/resume",
            "thread/read",
            "turn/start",
            "turn/steer",
            "turn/interrupt",
        ]
        .contains(&method)
        {
            return Err("unsupported client method".into());
        }
        let (tx, rx) = mpsc::sync_channel(1);
        {
            let mut shared = self
                .shared
                .lock()
                .map_err(|_| "runtime state unavailable")?;
            if shared.replies.len() >= REQUEST_LIMIT {
                return Err("request queue exhausted".into());
            }
            shared.replies.insert(id.into(), tx);
        }
        let frame = json!({"id":id,"method":method,"params":params});
        if let Err(e) = self.send(frame) {
            self.shared.lock().unwrap().replies.remove(id);
            return Err(e);
        }
        let result = rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "response not confirmed".to_string())
            .and_then(|r| r);
        self.shared.lock().unwrap().replies.remove(id);
        result
    }
    pub fn send(&self, frame: Value) -> std::result::Result<(), String> {
        send(&self.writer, &frame)
    }
    pub fn observation(&self) -> (RuntimeSnapshot, i64) {
        let state = self.shared.lock().unwrap();
        (state.snapshot.clone(), super::store::sequence())
    }
    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.shared.lock().unwrap().snapshot.clone()
    }
    pub fn shutdown(&mut self) -> Result<()> {
        if self.shutdown_complete {
            return Ok(());
        }
        // A reaped child PID can be reused. Never signal its old group or discover new owners.
        if self.child.try_wait()?.is_some() {
            return Err(Error::Invalid(
                "runtime already exited; owned descendant shutdown unconfirmed".into(),
            ));
        }
        let mut owned = super::lifecycle::capture_owned(self.child.id())?;
        self.writer.try_send(Vec::new()).map_err(|_| {
            Error::Invalid(
                "stdin shutdown queue unavailable; keep original instance managed".into(),
            )
        })?;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(current) = super::lifecycle::capture_owned(self.child.id()) {
                for process in current {
                    if !owned.iter().any(|old| old.pid == process.pid) {
                        owned.push(process);
                    }
                }
            }
            if self.child.try_wait()?.is_some() || std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Verified idle service descendants may use distinct groups; stop only captured births.
        super::lifecycle::stop_verified(&owned)?;
        self.child.wait()?;
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while !super::lifecycle::survivors(&owned)?.is_empty() {
            if std::time::Instant::now() >= deadline {
                return Err(Error::Invalid(
                    "owned descendant shutdown remains unconfirmed".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        self.shutdown_complete = true;
        Ok(())
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
fn send(writer: &mpsc::SyncSender<Vec<u8>>, frame: &Value) -> std::result::Result<(), String> {
    let mut bytes = serde_json::to_vec(frame).map_err(|_| "frame encoding failed")?;
    if bytes.len() > FRAME_LIMIT {
        return Err("frame exceeds limit".into());
    }
    bytes.push(b'\n');
    writer
        .try_send(bytes)
        .map_err(|_| "control queue unavailable".into())
}
pub fn degrade(shared: &Arc<Mutex<Shared>>, reason: &str) {
    if let Ok(mut state) = shared.lock() {
        state.snapshot.state = "degraded".into();
        state.snapshot.error = Some(reason.into());
        for session in &mut state.snapshot.sessions {
            if session.owned {
                session.state = "unknown".into();
                session.background_state = "unknown".into();
            }
        }
        for reply in state.replies.values() {
            let _ = reply.try_send(Err(reason.into()));
        }
    }
}
fn read_loop(
    output: impl Read,
    shared: Arc<Mutex<Shared>>,
    writer: mpsc::SyncSender<Vec<u8>>,
    events: mpsc::SyncSender<(RuntimeSnapshot, i64)>,
) {
    let mut reader = BufReader::new(output);
    loop {
        let mut line = Vec::new();
        let result = reader
            .by_ref()
            .take((FRAME_LIMIT + 2) as u64)
            .read_until(b'\n', &mut line);
        if !matches!(result,Ok(n) if n>0) {
            degrade(&shared, "connection ended; execution state unknown");
            break;
        }
        if line.len() > FRAME_LIMIT || !line.ends_with(b"\n") {
            degrade(&shared, "protocol frame exceeded bound");
            break;
        }
        let frame: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) => {
                degrade(&shared, "malformed protocol frame");
                break;
            }
        };
        let mut state = match shared.lock() {
            Ok(s) => s,
            Err(_) => break,
        };
        if frame.get("method").is_none() {
            if let Some(id) = frame.get("id").and_then(Value::as_str)
                && let Some(reply) = state.replies.remove(id)
            {
                let result = if frame.get("error").is_some() {
                    Err("sourceConfirmedFailure".into())
                } else if let Some(value) = frame.get("result") {
                    Ok(value.clone())
                } else {
                    Err("malformed response".into())
                };
                let _ = reply.try_send(result);
            }
            continue;
        }
        let method = frame["method"].as_str().unwrap_or("");
        if let Some(id) = frame.get("id") {
            if !(id.is_string() || id.is_i64() || id.is_u64()) {
                state.snapshot.state = "degraded".into();
                state.snapshot.error = Some("invalid server request identity".into());
                continue;
            }
            let key = id.to_string();
            if let Some(old) = state.raw_requests.get(&key) {
                if old["params"]["threadId"] != frame["params"]["threadId"]
                    || old["params"]["turnId"] != frame["params"]["turnId"]
                {
                    state.snapshot.state = "degraded".into();
                    state.snapshot.error =
                        Some("request ID reused for another target while pending".into());
                }
                continue;
            }
            if state.snapshot.requests.iter().any(|r| {
                r.request_id == key
                    && r.thread_id == frame["params"]["threadId"].as_str().unwrap_or("")
                    && r.turn_id == frame["params"]["turnId"].as_str().unwrap_or("")
            }) {
                continue;
            }
            let fingerprint = crate::index::digest(
                serde_json::to_string(&serde_json::json!([
                    key,
                    frame["params"]["threadId"],
                    frame["params"]["turnId"]
                ]))
                .unwrap()
                .as_bytes(),
            );
            if state.seen_requests.contains(&fingerprint) {
                continue;
            }
            if state.seen_requests.len() >= 4096 {
                state.snapshot.state = "degraded".into();
                state.snapshot.error =
                    Some("request identity history bound reached; new work disabled".into());
                continue;
            }
            match protocol::server_request(&frame, &state.snapshot) {
                Ok(request) => {
                    if state.raw_requests.len() >= 128 {
                        state.snapshot.state = "degraded".into();
                        state.snapshot.error =
                            Some("critical request queue overflow; new operations disabled".into());
                        continue;
                    }
                    state.raw_requests.insert(key, frame.clone());
                    state.seen_requests.insert(fingerprint);
                    for session in &mut state.snapshot.sessions {
                        if session.thread_id == request.thread_id {
                            session.state = "waiting".into();
                            session.turn_id = Some(request.turn_id.clone());
                        }
                    }
                    state.snapshot.requests.push(request);
                    let mut terminal = 0;
                    state.snapshot.requests.reverse();
                    state.snapshot.requests.retain(|r| {
                        if r.state == "resolved" {
                            terminal += 1;
                            terminal <= 64
                        } else {
                            true
                        }
                    });
                    state.snapshot.requests.reverse();
                    persist_event(&mut state, &events);
                }
                Err(reason) => {
                    let _ = send(
                        &writer,
                        &json!({"id":id,"error":{"code":-32601,"message":"unsupported or unowned request"}}),
                    );
                    state.snapshot.error = Some(reason);
                    state.snapshot.state = "degraded".into();
                }
            }
            continue;
        }
        protocol::notification(method, &frame["params"], &mut state.snapshot);
        if method == "serverRequest/resolved" {
            let id = frame["params"]["requestId"].to_string();
            state.raw_requests.remove(&id);
        }
        if [
            "turn/started",
            "turn/completed",
            "thread/status/changed",
            "serverRequest/resolved",
            "item/started",
            "item/completed",
        ]
        .contains(&method)
        {
            persist_event(&mut state, &events);
        }
        if state.snapshot.logs.len() >= 128 {
            state.snapshot.logs.remove(0);
            state.snapshot.logs_truncated = true;
        }
        state.snapshot.logs.push(method.chars().take(120).collect());
    }
}

fn persist_event(state: &mut Shared, events: &mpsc::SyncSender<(RuntimeSnapshot, i64)>) {
    if events
        .try_send((state.snapshot.clone(), super::store::sequence()))
        .is_err()
    {
        state.snapshot.state = "degraded".into();
        state.snapshot.error =
            Some("critical event persistence queue overflow; new effects disabled".into());
        for session in &mut state.snapshot.sessions {
            session.state = "unknown".into();
            session.background_state = "unknown".into();
        }
    }
}
