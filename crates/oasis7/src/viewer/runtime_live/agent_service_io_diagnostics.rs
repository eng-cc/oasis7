//! Test-only lifecycle measurements; no credentials or request identities are retained.
use super::agent_service_io::{
    AgentServiceIoJob, AgentServiceIoOperation, AgentServiceIoResult, AgentServiceProgress,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub(super) struct IoDiagnostics {
    origin: Instant,
    state: AtomicU64,
    queued: AtomicU64,
    executing: AtomicU64,
    returned: AtomicU64,
    consumed: AtomicU64,
    applied: AtomicU64,
    execute_errors: AtomicU64,
    apply_errors: AtomicU64,
    current_operation: AtomicU64,
    started_us: AtomicU64,
    elapsed_us: AtomicU64,
    max_execute_us: AtomicU64,
    total_execute_us: AtomicU64,
    returned_us: AtomicU64,
    total_return_to_consume_us: AtomicU64,
    max_return_to_consume_us: AtomicU64,
    max_same_phase_launches: AtomicU64,
    phase_code: AtomicU64,
    same_phase_launches: AtomicU64,
}

impl Default for IoDiagnostics {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
            state: AtomicU64::new(0),
            queued: AtomicU64::new(0),
            executing: AtomicU64::new(0),
            returned: AtomicU64::new(0),
            consumed: AtomicU64::new(0),
            applied: AtomicU64::new(0),
            execute_errors: AtomicU64::new(0),
            apply_errors: AtomicU64::new(0),
            current_operation: AtomicU64::new(0),
            started_us: AtomicU64::new(0),
            elapsed_us: AtomicU64::new(0),
            max_execute_us: AtomicU64::new(0),
            total_execute_us: AtomicU64::new(0),
            returned_us: AtomicU64::new(0),
            total_return_to_consume_us: AtomicU64::new(0),
            max_return_to_consume_us: AtomicU64::new(0),
            max_same_phase_launches: AtomicU64::new(0),
            phase_code: AtomicU64::new(0),
            same_phase_launches: AtomicU64::new(0),
        }
    }
}

impl IoDiagnostics {
    pub(super) fn execute(&self, job: AgentServiceIoJob) -> AgentServiceIoResult {
        self.executing();
        let result = job.execute();
        self.returned(result.response.is_err());
        result
    }
    pub(super) fn apply(
        &self,
        server: &mut super::ViewerRuntimeLiveServer,
        result: AgentServiceIoResult,
    ) -> Result<AgentServiceProgress, String> {
        self.consumed();
        let applied = server.apply_agent_service_io(result);
        self.applied(applied.is_err());
        applied
    }
    fn now_us(&self) -> u64 {
        self.origin.elapsed().as_micros().min(u64::MAX as u128) as u64
    }
    pub(super) fn queued(&self, job: &AgentServiceIoJob) {
        self.current_operation.store(
            match job.operation {
                AgentServiceIoOperation::Submit(_) => 1,
                AgentServiceIoOperation::Lookup { .. } => 2,
                AgentServiceIoOperation::View(_) => 3,
                AgentServiceIoOperation::Feedback { .. } => 4,
                AgentServiceIoOperation::Metadata(_) => 5,
            },
            Ordering::Relaxed,
        );
        let phase = match job.token.phase_digest.split(':').next().unwrap_or("") {
            "act" => 1,
            "act_view" => 2,
            "act_settled_view" => 3,
            "act_settle" => 4,
            "feedback" => 5,
            "feedback_ack" => 6,
            "feedback_ack_view" => 7,
            "wait" => 8,
            "resume" => 9,
            _ => 0,
        };
        if self.phase_code.swap(phase, Ordering::Relaxed) == phase {
            self.same_phase_launches.fetch_add(1, Ordering::Relaxed);
        } else {
            self.same_phase_launches.store(1, Ordering::Relaxed);
        }
        self.max_same_phase_launches.fetch_max(
            self.same_phase_launches.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.queued.fetch_add(1, Ordering::Relaxed);
        self.started_us.store(self.now_us(), Ordering::Relaxed);
        self.state.store(1, Ordering::Release);
    }
    pub(super) fn executing(&self) {
        self.executing.fetch_add(1, Ordering::Relaxed);
        self.started_us.store(self.now_us(), Ordering::Relaxed);
        self.state.store(2, Ordering::Release);
    }
    pub(super) fn returned(&self, failed: bool) {
        let elapsed = self
            .now_us()
            .saturating_sub(self.started_us.load(Ordering::Relaxed));
        self.elapsed_us.store(elapsed, Ordering::Relaxed);
        self.total_execute_us.fetch_add(elapsed, Ordering::Relaxed);
        self.returned_us.store(self.now_us(), Ordering::Relaxed);
        self.max_execute_us.fetch_max(elapsed, Ordering::Relaxed);
        self.execute_errors
            .fetch_add(u64::from(failed), Ordering::Relaxed);
        self.returned.fetch_add(1, Ordering::Relaxed);
        self.state.store(3, Ordering::Release);
    }
    pub(super) fn consumed(&self) {
        let elapsed = self
            .now_us()
            .saturating_sub(self.returned_us.load(Ordering::Relaxed));
        self.total_return_to_consume_us
            .fetch_add(elapsed, Ordering::Relaxed);
        self.max_return_to_consume_us
            .fetch_max(elapsed, Ordering::Relaxed);
        self.consumed.fetch_add(1, Ordering::Relaxed);
        self.state.store(4, Ordering::Release);
    }
    pub(super) fn applied(&self, failed: bool) {
        self.apply_errors
            .fetch_add(u64::from(failed), Ordering::Relaxed);
        self.applied.fetch_add(1, Ordering::Relaxed);
        self.state.store(5, Ordering::Release);
    }
    pub(super) fn snapshot(&self) -> serde_json::Value {
        let state = self.state.load(Ordering::Acquire);
        let operation = match self.current_operation.load(Ordering::Relaxed) {
            1 => "submit",
            2 => "lookup",
            3 => "view",
            4 => "feedback",
            5 => "metadata",
            _ => "absent",
        };
        let label = match state {
            1 => "queued",
            2 => "executing",
            3 => "returned",
            4 => "consumed",
            5 => "applied",
            _ => "absent",
        };
        serde_json::json!({
            "state": label, "operation": operation,
            "queued_count": self.queued.load(Ordering::Relaxed),
            "executing_count": self.executing.load(Ordering::Relaxed),
            "returned_count": self.returned.load(Ordering::Relaxed),
            "consumed_count": self.consumed.load(Ordering::Relaxed),
            "applied_count": self.applied.load(Ordering::Relaxed),
            "same_phase_launch_count": self.same_phase_launches.load(Ordering::Relaxed),
            "execute_error_count": self.execute_errors.load(Ordering::Relaxed),
            "apply_error_count": self.apply_errors.load(Ordering::Relaxed),
            "current_age_us": self.now_us().saturating_sub(self.started_us.load(Ordering::Relaxed)),
            "last_execute_us": self.elapsed_us.load(Ordering::Relaxed),
            "max_execute_us": self.max_execute_us.load(Ordering::Relaxed),
            "total_execute_us": self.total_execute_us.load(Ordering::Relaxed),
            "total_return_to_consume_us": self.total_return_to_consume_us.load(Ordering::Relaxed),
            "max_return_to_consume_us": self.max_return_to_consume_us.load(Ordering::Relaxed),
            "max_same_phase_launch_count": self.max_same_phase_launches.load(Ordering::Relaxed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::agent_service_io::AgentServiceIoToken;
    use super::*;
    use crate::world_service::client::{RemoteWorldServiceClient, WorldServiceClientConfig};
    use oasis7_client_api::world_service::{ReadWorldViewRequest, WorldIdentity};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex, mpsc},
        thread,
        time::Duration,
    };

    #[test]
    fn real_http_lifecycle_distinguishes_execution_from_viewer_lock_wait() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (observed, observation) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let transport = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 4096];
            assert!(socket.read(&mut bytes).unwrap() > 0);
            observed.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(2)).unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        });
        let world = WorldIdentity {
            world_id: "fixture".into(),
            genesis_digest: "fixture-genesis".into(),
        };
        let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        let client = RemoteWorldServiceClient::new(WorldServiceClientConfig {
            endpoint,
            trusted_service_public_key: hex::encode(key.verifying_key().to_bytes()),
            expected_world: world.clone(),
            scope_id: "public".into(),
            read_private_key_hex: hex::encode([7; 32]),
            timeout: Duration::from_secs(2),
            max_response_bytes: 4096,
        })
        .unwrap();
        let job = AgentServiceIoJob {
            token: AgentServiceIoToken {
                generation: 1,
                config_digest: "private-config".into(),
                phase_digest: "act_settle:private-request".into(),
            },
            client: Some(client),
            operation: AgentServiceIoOperation::View(ReadWorldViewRequest {
                contract_version: 1,
                world,
                scope_id: "public".into(),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            }),
        };
        let server = Arc::new(Mutex::new(
            super::super::ViewerRuntimeLiveServer::new(
                super::super::ViewerRuntimeLiveServerConfig::new(
                    crate::simulator::WorldScenario::Minimal,
                ),
            )
            .unwrap(),
        ));
        let held = server.lock().unwrap();
        let diagnostics = held.agent_service_io_diagnostics.clone();
        diagnostics.queued(&job);
        let executor = diagnostics.clone();
        let worker = thread::spawn(move || executor.execute(job));
        observation.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(diagnostics.snapshot()["state"], "executing");
        assert_eq!(diagnostics.snapshot()["returned_count"], 0);
        release.send(()).unwrap();
        let result = worker.join().unwrap();
        transport.join().unwrap();
        assert!(
            result.response.is_err(),
            "real 503 transport must remain an error"
        );
        assert_eq!(diagnostics.snapshot()["state"], "returned");
        assert_eq!(diagnostics.snapshot()["execute_error_count"], 1);
        let consumer_server = server.clone();
        let consumer_diagnostics = diagnostics.clone();
        let (waiting, wait) = mpsc::channel();
        let consumer = thread::spawn(move || {
            waiting.send(()).unwrap();
            consumer_diagnostics
                .apply(&mut consumer_server.lock().unwrap(), result)
                .is_err()
        });
        wait.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(diagnostics.snapshot()["state"], "returned");
        assert_eq!(
            diagnostics.snapshot()["consumed_count"],
            0,
            "View mutex prevents applying the returned result"
        );
        drop(held);
        assert!(
            consumer.join().unwrap(),
            "original mismatched token still fails closed"
        );
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot["state"], "applied");
        for field in [
            "queued_count",
            "executing_count",
            "returned_count",
            "consumed_count",
            "applied_count",
            "apply_error_count",
            "same_phase_launch_count",
        ] {
            assert_eq!(snapshot[field], 1, "{field}");
        }
        assert!(!snapshot.to_string().contains("private-"));
    }
}
