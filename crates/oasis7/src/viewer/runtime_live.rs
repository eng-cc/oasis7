use std::collections::{BTreeMap, VecDeque};
use std::io::{self, BufRead, BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::geometry::GeoPos;
use crate::runtime::{
    blake3_hex, Action as RuntimeAction, DomainEvent as RuntimeDomainEvent,
    Journal as RuntimeJournal, Snapshot as RuntimeSnapshot, World as RuntimeWorld,
    WorldError as RuntimeWorldError, WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{
    build_world_model, AgentDecisionTrace, ChunkRuntimeConfig, PlayerGameplayRecentFeedback,
    RejectReason as SimulatorRejectReason, ResourceKind, RunnerMetrics, WorldConfig, WorldEvent,
    WorldInitConfig, WorldScenario, WorldSnapshot, CHUNK_GENERATION_SCHEMA_VERSION,
    SNAPSHOT_VERSION,
};

use super::auth::verify_session_register_auth_proof;
use super::live::ViewerLiveDecisionMode;
use super::protocol::{
    viewer_event_kind_matches, AuthoritativeBatchFinality, AuthoritativeChallengeAck,
    AuthoritativeChallengeCommand, AuthoritativeChallengeError,
    AuthoritativeChallengeResolveRequest, AuthoritativeChallengeStatus,
    AuthoritativeChallengeSubmitRequest, AuthoritativeFinalityState,
    AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryAck, AuthoritativeRecoveryCommand,
    AuthoritativeRecoveryError, AuthoritativeRecoveryStatus, AuthoritativeRollbackRequest,
    AuthoritativeSessionRegisterRequest, AuthoritativeSessionRevokeRequest,
    AuthoritativeSessionRotateRequest, ControlCompletionAck, ControlCompletionStatus,
    ViewerControl, ViewerControlProfile, ViewerEventKind, ViewerRequest, ViewerResponse,
    ViewerStream, VIEWER_PROTOCOL_VERSION,
};
mod authoritative;
mod claim_snapshot;
#[path = "runtime_live/control_plane.rs"]
mod control_plane;
mod gameplay_snapshot;
mod mapping;
mod player_gameplay;
mod recovery;
mod session_policy;
mod support;
#[cfg(test)]
mod tests;

use authoritative::{
    RuntimeAuthoritativeBatchRecord, RuntimeAuthoritativeChallengeRecord,
    RuntimeSettlementRankingGate, RuntimeStableCheckpoint,
};
use claim_snapshot::build_player_agent_claim_snapshot;
use control_plane::RuntimeLlmSidecar;
use gameplay_snapshot::{
    build_player_gameplay_snapshot, player_gameplay_feedback_from_control_ack,
};
use mapping::{map_runtime_event, runtime_state_to_simulator_model};
use session_policy::{
    location_id_for_pos, map_session_policy_error_code, normalize_optional_string,
    session_revoke_metadata_key, RuntimeSessionPolicy, RuntimeSessionRevokeMetadata,
};
use support::{
    bootstrap_runtime_world, is_expected_disconnect_error, is_timeout_error,
    latest_runtime_event_seq, lock_shared_server, runtime_metrics, send_response,
    RuntimeLiveScript, RuntimeLiveSession,
};

const AUTHORITATIVE_BATCH_CONFIRM_DELAY_TICKS: u64 = 1;
const AUTHORITATIVE_BATCH_FINALITY_WINDOW_TICKS: u64 = 2;
const MAX_AUTHORITATIVE_BATCH_HISTORY: usize = 256;
const MAX_AUTHORITATIVE_CHALLENGE_HISTORY: usize = 512;
const MAX_AUTHORITATIVE_STABLE_CHECKPOINTS: usize = 64;

#[derive(Debug, Clone)]
pub struct ViewerRuntimeLiveServerConfig {
    pub bind_addr: String,
    pub scenario: WorldScenario,
    pub world_id: String,
    pub decision_mode: ViewerLiveDecisionMode,
    pub play_step_interval: Duration,
    pub hosted_public_join_mode: bool,
}

impl ViewerRuntimeLiveServerConfig {
    pub fn new(scenario: WorldScenario) -> Self {
        Self {
            bind_addr: "127.0.0.1:5010".to_string(),
            world_id: format!("live-runtime-{}", scenario.as_str()),
            scenario,
            decision_mode: ViewerLiveDecisionMode::Script,
            play_step_interval: Duration::from_millis(800),
            hosted_public_join_mode: false,
        }
    }

    pub fn with_bind_addr(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    pub fn with_world_id(mut self, world_id: impl Into<String>) -> Self {
        self.world_id = world_id.into();
        self
    }

    pub fn with_decision_mode(mut self, mode: ViewerLiveDecisionMode) -> Self {
        self.decision_mode = mode;
        self
    }

    pub fn with_llm_mode(mut self, enabled: bool) -> Self {
        self.decision_mode = if enabled {
            ViewerLiveDecisionMode::Llm
        } else {
            ViewerLiveDecisionMode::Script
        };
        self
    }

    pub fn with_play_step_interval(mut self, interval: Duration) -> Self {
        self.play_step_interval = interval.max(Duration::from_millis(50));
        self
    }

    pub fn with_hosted_public_join_mode(mut self, enabled: bool) -> Self {
        self.hosted_public_join_mode = enabled;
        self
    }
}

#[derive(Debug)]
pub enum ViewerRuntimeLiveServerError {
    Io(io::Error),
    Serde(String),
    Init(String),
    Runtime(RuntimeWorldError),
}

impl From<io::Error> for ViewerRuntimeLiveServerError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<RuntimeWorldError> for ViewerRuntimeLiveServerError {
    fn from(err: RuntimeWorldError) -> Self {
        Self::Runtime(err)
    }
}

pub struct ViewerRuntimeLiveServer {
    config: ViewerRuntimeLiveServerConfig,
    world: RuntimeWorld,
    snapshot_config: WorldConfig,
    script: RuntimeLiveScript,
    llm_sidecar: RuntimeLlmSidecar,
    pending_virtual_events: VecDeque<WorldEvent>,
    next_virtual_event_id: u64,
    authoritative_batches: VecDeque<RuntimeAuthoritativeBatchRecord>,
    next_authoritative_batch_id: u64,
    authoritative_challenges: VecDeque<RuntimeAuthoritativeChallengeRecord>,
    next_authoritative_challenge_id: u64,
    stable_checkpoints: VecDeque<RuntimeStableCheckpoint>,
    reorg_epoch: u64,
    session_policy: RuntimeSessionPolicy,
    session_revoke_metadata: BTreeMap<(String, String), RuntimeSessionRevokeMetadata>,
    settlement_ranking_gate: RuntimeSettlementRankingGate,
    latest_player_gameplay_feedback: Option<PlayerGameplayRecentFeedback>,
}

impl ViewerRuntimeLiveServer {
    pub fn new(
        config: ViewerRuntimeLiveServerConfig,
    ) -> Result<Self, ViewerRuntimeLiveServerError> {
        let (world, snapshot_config) =
            bootstrap_runtime_world(config.scenario).map_err(ViewerRuntimeLiveServerError::Init)?;
        let llm_sidecar = RuntimeLlmSidecar::new(config.decision_mode);
        let next_virtual_event_id = latest_runtime_event_seq(&world).saturating_add(1).max(1);
        Ok(Self {
            config,
            world,
            snapshot_config,
            script: RuntimeLiveScript::default(),
            llm_sidecar,
            pending_virtual_events: VecDeque::new(),
            next_virtual_event_id,
            authoritative_batches: VecDeque::new(),
            next_authoritative_batch_id: 1,
            authoritative_challenges: VecDeque::new(),
            next_authoritative_challenge_id: 1,
            stable_checkpoints: VecDeque::new(),
            reorg_epoch: 0,
            session_policy: RuntimeSessionPolicy::default(),
            session_revoke_metadata: BTreeMap::new(),
            settlement_ranking_gate: RuntimeSettlementRankingGate::default(),
            latest_player_gameplay_feedback: None,
        })
    }

    pub fn run(self) -> Result<(), ViewerRuntimeLiveServerError> {
        let listener = TcpListener::bind(&self.config.bind_addr)?;
        let shared = Arc::new(Mutex::new(self));
        for incoming in listener.incoming() {
            let stream = incoming?;
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                if let Err(err) = Self::serve_shared_stream(shared, stream) {
                    eprintln!("viewer runtime live server error: {err:?}");
                }
            });
        }
        Ok(())
    }

    pub fn run_once(&mut self) -> Result<(), ViewerRuntimeLiveServerError> {
        let listener = TcpListener::bind(&self.config.bind_addr)?;
        let (stream, _) = listener.accept()?;
        self.serve_stream(stream)
    }

    fn hosted_public_join_mode(&self) -> bool {
        self.config.hosted_public_join_mode
    }

    fn serve_shared_stream(
        shared: Arc<Mutex<Self>>,
        stream: TcpStream,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);
        let mut writer = BufWriter::new(stream);
        let mut session = RuntimeLiveSession::new();

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(()),
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(request) = serde_json::from_str::<ViewerRequest>(trimmed) {
                            let mut server = lock_shared_server(&shared)?;
                            server.handle_request(request, &mut session, &mut writer)?;
                        }
                    }
                }
                Err(err) if is_timeout_error(&err) => {}
                Err(err) if is_expected_disconnect_error(&err) => return Ok(()),
                Err(err) => return Err(ViewerRuntimeLiveServerError::Io(err)),
            }

            let play_step_interval = {
                let server = lock_shared_server(&shared)?;
                server.config.play_step_interval
            };
            if session.should_advance_play_step(play_step_interval) {
                let mut server = lock_shared_server(&shared)?;
                server.advance_runtime(&mut session, &mut writer, 1, None, false)?;
            }
        }
    }

    fn serve_stream(&mut self, stream: TcpStream) -> Result<(), ViewerRuntimeLiveServerError> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);
        let mut writer = BufWriter::new(stream);
        let mut session = RuntimeLiveSession::new();

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(()),
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(request) = serde_json::from_str::<ViewerRequest>(trimmed) {
                            self.handle_request(request, &mut session, &mut writer)?;
                        }
                    }
                }
                Err(err) if is_timeout_error(&err) => {}
                Err(err) if is_expected_disconnect_error(&err) => return Ok(()),
                Err(err) => return Err(ViewerRuntimeLiveServerError::Io(err)),
            }

            if session.should_advance_play_step(self.config.play_step_interval) {
                self.advance_runtime(&mut session, &mut writer, 1, None, false)?;
            }
        }
    }

    fn handle_request(
        &mut self,
        request: ViewerRequest,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        match request {
            ViewerRequest::Hello { .. } => {
                send_response(
                    writer,
                    &ViewerResponse::HelloAck {
                        server: "oasis7".to_string(),
                        version: VIEWER_PROTOCOL_VERSION,
                        world_id: self.config.world_id.clone(),
                        control_profile: ViewerControlProfile::Live,
                    },
                )?;
            }
            ViewerRequest::Subscribe {
                streams,
                event_kinds,
            } => {
                session.subscribed = streams.into_iter().collect();
                session.event_filters = if event_kinds.is_empty() {
                    None
                } else {
                    Some(event_kinds.into_iter().collect())
                };
            }
            ViewerRequest::RequestSnapshot => {
                if session.subscribed.is_empty()
                    || session.subscribed.contains(&ViewerStream::Snapshot)
                {
                    let snapshot = self.compat_snapshot();
                    send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                }
                if session.subscribed.is_empty()
                    || session.subscribed.contains(&ViewerStream::Snapshot)
                    || session.subscribed.contains(&ViewerStream::Events)
                {
                    let cursor = self.current_recovery_cursor()?;
                    send_response(
                        writer,
                        &ViewerResponse::AuthoritativeRecoveryAck {
                            ack: AuthoritativeRecoveryAck {
                                status: AuthoritativeRecoveryStatus::CatchUpReady,
                                reorg_epoch: self.reorg_epoch,
                                snapshot_height: cursor.snapshot_height,
                                snapshot_hash: cursor.snapshot_hash,
                                log_cursor: cursor.log_cursor,
                                stable_batch_id: cursor.stable_batch_id,
                                player_id: None,
                                agent_id: None,
                                session_pubkey: None,
                                replaced_by_pubkey: None,
                                session_epoch: None,
                                message: Some("snapshot_sync_metadata".to_string()),
                                revoke_reason: None,
                                revoked_by: None,
                                acknowledged_at_tick: self.world.state().time,
                            },
                        },
                    )?;
                }
                if session.subscribed.contains(&ViewerStream::Metrics) {
                    session.metrics = runtime_metrics(&self.world);
                    send_response(
                        writer,
                        &ViewerResponse::Metrics {
                            time: Some(self.world.state().time),
                            metrics: session.metrics.clone(),
                        },
                    )?;
                }
                if session.subscribed.contains(&ViewerStream::Events) {
                    self.emit_authoritative_batch_snapshot(writer)?;
                    self.emit_authoritative_challenge_snapshot(writer)?;
                }
            }
            ViewerRequest::PlaybackControl { mode, request_id } => {
                self.apply_control_mode(ViewerControl::from(mode), request_id, session, writer)?;
            }
            ViewerRequest::LiveControl { mode, request_id } => {
                self.apply_control_mode(ViewerControl::from(mode), request_id, session, writer)?;
            }
            ViewerRequest::Control { mode, request_id } => {
                self.apply_control_mode(mode, request_id, session, writer)?;
            }
            ViewerRequest::PromptControl { command } => match self.handle_prompt_control(command) {
                Ok(ack) => {
                    send_response(writer, &ViewerResponse::PromptControlAck { ack })?;
                }
                Err(error) => {
                    send_response(writer, &ViewerResponse::PromptControlError { error })?;
                }
            },
            ViewerRequest::AgentChat { request } => match self.handle_agent_chat(request) {
                Ok(ack) => {
                    send_response(writer, &ViewerResponse::AgentChatAck { ack })?;
                }
                Err(error) => {
                    send_response(writer, &ViewerResponse::AgentChatError { error })?;
                }
            },
            ViewerRequest::GameplayAction { request } => match self.handle_gameplay_action(request)
            {
                Ok(ack) => {
                    send_response(writer, &ViewerResponse::GameplayActionAck { ack })?;
                }
                Err(error) => {
                    send_response(writer, &ViewerResponse::GameplayActionError { error })?;
                }
            },
            ViewerRequest::AuthoritativeChallenge { command } => {
                match self.handle_authoritative_challenge(command) {
                    Ok((ack, maybe_batch_update)) => {
                        send_response(writer, &ViewerResponse::AuthoritativeChallengeAck { ack })?;
                        if let Some(batch) = maybe_batch_update {
                            send_response(writer, &ViewerResponse::AuthoritativeBatch { batch })?;
                        }
                    }
                    Err(error) => {
                        send_response(
                            writer,
                            &ViewerResponse::AuthoritativeChallengeError { error },
                        )?;
                    }
                }
            }
            ViewerRequest::AuthoritativeRecovery { command } => {
                match self.handle_authoritative_recovery(command) {
                    Ok((ack, emit_snapshot_after_ack)) => {
                        send_response(writer, &ViewerResponse::AuthoritativeRecoveryAck { ack })?;
                        if emit_snapshot_after_ack {
                            let snapshot = self.compat_snapshot();
                            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                            self.emit_authoritative_batch_snapshot(writer)?;
                            self.emit_authoritative_challenge_snapshot(writer)?;
                        }
                    }
                    Err(error) => {
                        send_response(
                            writer,
                            &ViewerResponse::AuthoritativeRecoveryError { error },
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_control_mode(
        &mut self,
        mode: ViewerControl,
        request_id: Option<u64>,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        match mode {
            ViewerControl::Pause => {
                session.playing = false;
                session.next_play_step_at = None;
            }
            ViewerControl::Play => {
                session.playing = true;
                session.next_play_step_at = None;
            }
            ViewerControl::Step { count } => {
                session.playing = false;
                session.next_play_step_at = None;
                self.advance_runtime(session, writer, count.max(1), request_id, true)?;
            }
            ViewerControl::Seek { tick } => {
                session.playing = false;
                session.next_play_step_at = None;
                eprintln!(
                    "viewer runtime live: ignore seek control in live mode (target_tick={tick})"
                );
            }
        }
        Ok(())
    }

    fn advance_runtime(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
        step_count: usize,
        request_id: Option<u64>,
        emit_while_paused: bool,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        let baseline_logical_time = self.world.state().time;
        let baseline_event_seq = latest_runtime_event_seq(&self.world);

        for _ in 0..step_count.max(1) {
            let mut decision_trace: Option<AgentDecisionTrace> = None;
            match self.config.decision_mode {
                ViewerLiveDecisionMode::Script => self.script.enqueue(&mut self.world),
                ViewerLiveDecisionMode::Llm => {
                    decision_trace = self.enqueue_llm_action_from_sidecar();
                }
            }
            let journal_start = self.world.journal().events.len();
            self.world.step()?;

            let new_events: Vec<_> = self.world.journal().events[journal_start..].to_vec();
            let mut mapped_events = Vec::new();
            for runtime_event in &new_events {
                let event = map_runtime_event(runtime_event, &self.snapshot_config);
                if matches!(runtime_event.body, RuntimeWorldEventBody::Domain(_)) {
                    self.llm_sidecar
                        .notify_action_result_if_needed(runtime_event, event.clone());
                }
                mapped_events.push(event);
            }
            mapped_events.extend(self.pending_virtual_events.drain(..));
            let pending_batch = self.register_authoritative_batch(mapped_events.as_slice())?;
            let batch_finality_updates =
                self.advance_authoritative_batch_finality(self.world.state().time)?;

            if let Some(trace) = decision_trace {
                if session.subscribed.contains(&ViewerStream::Events) {
                    send_response(writer, &ViewerResponse::DecisionTrace { trace })?;
                }
            }

            if session.subscribed.contains(&ViewerStream::Events)
                && (emit_while_paused || session.playing)
            {
                for event in &mapped_events {
                    if session.event_allowed(event) {
                        send_response(
                            writer,
                            &ViewerResponse::Event {
                                event: event.clone(),
                            },
                        )?;
                    }
                }
                send_response(
                    writer,
                    &ViewerResponse::AuthoritativeBatch {
                        batch: pending_batch,
                    },
                )?;
                for batch in batch_finality_updates {
                    send_response(writer, &ViewerResponse::AuthoritativeBatch { batch })?;
                }
            }

            if session.subscribed.contains(&ViewerStream::Snapshot) {
                let snapshot = self.compat_snapshot();
                send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
            }

            session.metrics = runtime_metrics(&self.world);
            if session.subscribed.contains(&ViewerStream::Metrics) {
                send_response(
                    writer,
                    &ViewerResponse::Metrics {
                        time: Some(self.world.state().time),
                        metrics: session.metrics.clone(),
                    },
                )?;
            }
        }

        if let Some(request_id) = request_id {
            let delta_logical_time = self.world.state().time.saturating_sub(baseline_logical_time);
            let delta_event_seq =
                latest_runtime_event_seq(&self.world).saturating_sub(baseline_event_seq);
            let status = if delta_logical_time > 0 || delta_event_seq > 0 {
                ControlCompletionStatus::Advanced
            } else {
                ControlCompletionStatus::TimeoutNoProgress
            };
            let ack = ControlCompletionAck {
                request_id,
                status,
                delta_logical_time,
                delta_event_seq,
            };
            self.latest_player_gameplay_feedback = Some(player_gameplay_feedback_from_control_ack(
                &ViewerControl::Step { count: step_count },
                &ack,
            ));
            send_response(writer, &ViewerResponse::ControlCompletionAck { ack })?;
        }

        Ok(())
    }

    fn compat_snapshot(&self) -> WorldSnapshot {
        let runtime_snapshot = self.world.snapshot();
        let runtime_journal_len = runtime_snapshot.journal_len;
        let next_event_id = runtime_snapshot.last_event_id.saturating_add(1).max(1);
        let next_action_id = runtime_snapshot.next_action_id.max(1);
        let primary_agent_claim = self
            .world
            .state()
            .agents
            .keys()
            .next()
            .and_then(|agent_id| {
                build_player_agent_claim_snapshot(
                    self.world.state(),
                    agent_id.as_str(),
                    self.world.governance_execution_policy().epoch_length_ticks,
                )
            });
        WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: self.world.state().time,
            config: self.snapshot_config.clone(),
            model: runtime_state_to_simulator_model(self.world.state(), &self.llm_sidecar),
            runtime_snapshot: Some(runtime_snapshot),
            player_gameplay: Some(build_player_gameplay_snapshot(
                self.world.state(),
                self.latest_player_gameplay_feedback.as_ref(),
                self.llm_sidecar.is_llm_mode() && self.llm_sidecar.supports_agent_chat(),
                primary_agent_claim,
            )),
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id,
            next_action_id,
            pending_actions: Vec::new(),
            journal_len: runtime_journal_len,
        }
    }
}
