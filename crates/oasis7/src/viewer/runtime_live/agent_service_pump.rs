use super::agent_service_io::{AgentServiceIoJob, AgentServiceIoResult, AgentServiceProgress};
use super::*;
use std::sync::{Weak, mpsc};

const PUMP_INTERVAL: Duration = Duration::from_millis(50);

/// Connection presence is independent of playback: Pause is shared server policy.
pub(super) struct AgentServicePresence {
    server: Weak<Mutex<ViewerRuntimeLiveServer>>,
    registered: bool,
}

impl AgentServicePresence {
    pub(super) fn new(server: &Arc<Mutex<ViewerRuntimeLiveServer>>) -> Self {
        Self {
            server: Arc::downgrade(server),
            registered: false,
        }
    }

    pub(super) fn observe(
        &mut self,
        server: &mut ViewerRuntimeLiveServer,
        session: &RuntimeLiveSession,
    ) {
        if !self.registered && session.initial_snapshot_sent {
            server.agent_service_session_count =
                server.agent_service_session_count.saturating_add(1);
            self.registered = true;
        }
    }
}

impl Drop for AgentServicePresence {
    fn drop(&mut self) {
        if self.registered
            && let Some(shared) = self.server.upgrade()
            && let Ok(mut server) = shared.lock()
        {
            server.agent_service_session_count =
                server.agent_service_session_count.saturating_sub(1);
        }
    }
}

impl ViewerRuntimeLiveServer {
    fn agent_service_eligible(&self) -> bool {
        self.agent_service_session_count > 0
            && !self.auto_play_paused
            && self.authoritative_recovery_write_fence.is_none()
            && matches!(self.config.decision_mode, ViewerLiveDecisionMode::Llm)
    }

    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_agent_service_pump_status(&self) -> serde_json::Value {
        serde_json::json!({
            "session_count": self.agent_service_session_count,
            "play_enabled": !self.auto_play_paused,
            "llm_mode": matches!(self.config.decision_mode, ViewerLiveDecisionMode::Llm),
            "fenced": self.authoritative_recovery_write_fence.is_some(),
            "pump_started": self.agent_service_pump_started,
            "eligible": self.agent_service_eligible(),
            "known_service_commit": self.verified_world_view.as_ref().map(|view| &view.version().commit),
        })
    }

    fn record_agent_pump_error(&mut self, reason: &'static str) {
        if self.agent_service_pump_error == Some(reason) {
            return;
        }
        self.agent_service_pump_error = Some(reason);
        self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
            "agent_service", "blocked", "canonical Agent work remains pending",
            None, None, Some(reason.into()),
            Some("restore provider/service readiness; original requests remain queued for reconciliation".into()),
            0, 0,
        ));
    }

    fn observe_agent_pump_progress(&mut self, progress: &AgentServiceProgress) {
        if matches!(progress, AgentServiceProgress::Advanced) {
            self.agent_service_pump_error = None;
            if self
                .latest_player_gameplay_feedback
                .as_ref()
                .is_some_and(|feedback| feedback.action == "agent_service")
            {
                self.latest_player_gameplay_feedback = None;
            }
        }
    }

    fn agent_pump_worker_failed(&mut self) {
        self.agent_service_io_worker_failed();
        self.agent_service_pump_started = false;
        self.record_agent_pump_error(
            "Agent service executor stopped; pending work requires reconciliation",
        );
    }

    pub(super) fn start_agent_service_pump(
        shared: &Arc<Mutex<Self>>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        {
            let mut server = lock_shared_server(shared)?;
            if server.agent_service_pump_started || server.config.world_service.is_none() {
                return Ok(());
            }
            server.agent_service_pump_started = true;
        }
        let (jobs, incoming) = mpsc::sync_channel::<AgentServiceIoJob>(1);
        let (completed, results) = mpsc::sync_channel::<AgentServiceIoResult>(1);
        // The executor owns only immutable jobs. It never accesses the Viewer mutex.
        thread::spawn(move || {
            while let Ok(job) = incoming.recv() {
                if completed.send(job.execute()).is_err() {
                    break;
                }
            }
        });
        let weak = Arc::downgrade(shared);
        thread::spawn(move || {
            let mut in_flight = false;
            loop {
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let Ok(mut server) = shared.lock() else {
                    break;
                };
                let mut progress = None;
                if in_flight {
                    match results.try_recv() {
                        Ok(result) => {
                            in_flight = false;
                            match server.apply_agent_service_io(result) {
                                Ok(value) => {
                                    server.observe_agent_pump_progress(&value);
                                    progress = Some(value);
                                }
                                Err(_error) => {
                                    #[cfg(any(test, feature = "test_tier_required"))]
                                    if std::env::var("PRE2_RESUME_REJECTION_RECOVERY_TRACE")
                                        .as_deref()
                                        == Ok("1")
                                    {
                                        eprintln!("pre2_private_recovery_apply_error={_error}");
                                    }
                                    server.record_agent_pump_error("Agent service result could not be applied; original work remains pending");
                                }
                            }
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            server.agent_pump_worker_failed();
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                }
                if !in_flight {
                    let eligible = server.agent_service_eligible();
                    if progress.is_none()
                        || matches!(
                            progress,
                            Some(AgentServiceProgress::Idle | AgentServiceProgress::Advanced)
                        )
                    {
                        match server.prepare_agent_service_io(eligible) {
                            Ok(value) => {
                                server.observe_agent_pump_progress(&value);
                                progress = Some(value);
                            }
                            Err(_error) => {
                                #[cfg(any(test, feature = "test_tier_required"))]
                                if std::env::var("PRE2_RESUME_REJECTION_RECOVERY_TRACE").as_deref()
                                    == Ok("1")
                                {
                                    eprintln!("pre2_private_recovery_prepare_error={_error}");
                                }
                                server.record_agent_pump_error("Agent service preparation is blocked; original work remains pending");
                            }
                        }
                    }
                    if let Some(AgentServiceProgress::NeedsIo(job)) = progress {
                        // One owned in-flight job guarantees capacity; never wait under the mutex.
                        match jobs.try_send(*job) {
                            Ok(()) => in_flight = true,
                            Err(_) => {
                                server.agent_pump_worker_failed();
                                break;
                            }
                        }
                    }
                }
                drop(server);
                drop(shared);
                thread::sleep(PUMP_INTERVAL);
            }
        });
        Ok(())
    }
}
