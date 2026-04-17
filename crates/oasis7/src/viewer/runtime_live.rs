use std::collections::{BTreeMap, VecDeque};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::geometry::GeoPos;
use crate::runtime::{
    blake3_hex, Action as RuntimeAction, DomainEvent as RuntimeDomainEvent,
    Journal as RuntimeJournal, ReleaseSecurityPolicy, Snapshot as RuntimeSnapshot,
    World as RuntimeWorld, WorldError as RuntimeWorldError,
    WorldEventBody as RuntimeWorldEventBody,
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
#[path = "runtime_live/chain_link.rs"]
mod chain_link;
mod claim_snapshot;
#[path = "runtime_live/control_plane.rs"]
mod control_plane;
#[path = "runtime_live/control_utils.rs"]
mod control_utils;
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
use control_utils::{control_mode_for_action, control_mode_label, runtime_control_error_details};
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
const LLM_GAMEPLAY_REQUIRED_HINT: &str =
    "enable --llm and configure a reachable LLM provider before retrying gameplay controls";
const RUNTIME_CONTROL_REQUIRED_HINT: &str =
    "inspect the reported runtime failure, repair the broken world/module state, then retry the control";

#[derive(Debug, Clone)]
pub struct ViewerRuntimeLiveServerConfig {
    pub bind_addr: String,
    pub scenario: WorldScenario,
    pub world_id: String,
    pub decision_mode: ViewerLiveDecisionMode,
    pub play_step_interval: Duration,
    pub chain_poll_interval: Duration,
    pub hosted_public_join_mode: bool,
    pub chain_status_bind: Option<String>,
}

impl ViewerRuntimeLiveServerConfig {
    pub fn new(scenario: WorldScenario) -> Self {
        Self {
            bind_addr: "127.0.0.1:5010".to_string(),
            world_id: format!("live-runtime-{}", scenario.as_str()),
            scenario,
            decision_mode: ViewerLiveDecisionMode::Script,
            play_step_interval: Duration::from_millis(800),
            chain_poll_interval: Duration::from_millis(200),
            hosted_public_join_mode: false,
            chain_status_bind: None,
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

    pub fn with_chain_poll_interval(mut self, interval: Duration) -> Self {
        self.chain_poll_interval = interval.max(Duration::from_millis(50));
        self
    }

    pub fn with_hosted_public_join_mode(mut self, enabled: bool) -> Self {
        self.hosted_public_join_mode = enabled;
        self
    }

    pub fn with_chain_status_bind(mut self, addr: impl Into<String>) -> Self {
        self.chain_status_bind = Some(addr.into());
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
    initial_world_time: u64,
    last_chain_committed_height: u64,
    confirmed_player_gameplay_progress_time: Option<u64>,
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

const BACKGROUND_PLAY_TRANSIENT_FAILURE_BUDGET: u8 = 3;

impl ViewerRuntimeLiveServer {
    pub fn new(
        config: ViewerRuntimeLiveServerConfig,
    ) -> Result<Self, ViewerRuntimeLiveServerError> {
        let (world, snapshot_config) =
            bootstrap_runtime_world(config.scenario).map_err(ViewerRuntimeLiveServerError::Init)?;
        let initial_world_time = world.state().time;
        let llm_sidecar = RuntimeLlmSidecar::new(config.decision_mode);
        let next_virtual_event_id = latest_runtime_event_seq(&world).saturating_add(1).max(1);
        Ok(Self {
            config,
            world,
            initial_world_time,
            last_chain_committed_height: 0,
            confirmed_player_gameplay_progress_time: None,
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

    fn set_latest_player_gameplay_feedback(&mut self, feedback: PlayerGameplayRecentFeedback) {
        if feedback.delta_logical_time > 0 || feedback.delta_event_seq > 0 {
            self.confirm_player_gameplay_progress();
        }
        self.latest_player_gameplay_feedback = Some(feedback);
    }

    fn confirm_player_gameplay_progress(&mut self) {
        self.confirmed_player_gameplay_progress_time = Some(self.world.state().time);
    }

    pub fn run_once(&mut self) -> Result<(), ViewerRuntimeLiveServerError> {
        let listener = TcpListener::bind(&self.config.bind_addr)?;
        let (stream, _) = listener.accept()?;
        self.serve_stream(stream)
    }

    fn hosted_public_join_mode(&self) -> bool {
        self.config.hosted_public_join_mode
    }

    fn chain_link_enabled(&self) -> bool {
        self.config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
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

            let (chain_link_enabled, chain_poll_interval) = {
                let server = lock_shared_server(&shared)?;
                (
                    server.chain_link_enabled(),
                    server.config.chain_poll_interval,
                )
            };
            if chain_link_enabled && session.should_poll_chain(chain_poll_interval) {
                if let Err(err) = Self::sync_chain_linked_runtime_minimized_lock(
                    &shared,
                    &mut session,
                    &mut writer,
                ) {
                    eprintln!("viewer runtime live: chain sync skipped: {err:?}");
                }
            }

            let play_step_interval = {
                let server = lock_shared_server(&shared)?;
                server.config.play_step_interval
            };
            if session.should_advance_play_step(play_step_interval) {
                let mut server = lock_shared_server(&shared)?;
                server.advance_runtime(&mut session, &mut writer, "play", 1, None, false)?;
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

            if self.chain_link_enabled()
                && session.should_poll_chain(self.config.chain_poll_interval)
            {
                if let Err(err) = self.sync_chain_linked_runtime(&mut session, &mut writer) {
                    eprintln!("viewer runtime live: chain sync skipped: {err:?}");
                }
            }

            if session.should_advance_play_step(self.config.play_step_interval) {
                self.advance_runtime(&mut session, &mut writer, "play", 1, None, false)?;
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
                    if control_plane::runtime_agent_chat_echo_enabled_from_env() {
                        self.flush_pending_virtual_events(session, writer)?;
                    }
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
        if let Err(reason) = self.ensure_gameplay_ready_for_control(&mode) {
            return self.block_gameplay_control(
                session,
                writer,
                control_mode_label(&mode),
                "gameplay control rejected before world advance",
                reason,
                request_id,
                0,
                0,
                false,
            );
        }
        match mode {
            ViewerControl::Pause => {
                session.playing = false;
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
            }
            ViewerControl::Play => {
                session.playing = true;
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
            }
            ViewerControl::Step { count } => {
                session.playing = false;
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
                self.advance_runtime(session, writer, "step", count.max(1), request_id, true)?;
            }
            ViewerControl::Seek { tick } => {
                session.playing = false;
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
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
        action: &'static str,
        step_count: usize,
        request_id: Option<u64>,
        emit_while_paused: bool,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        let baseline_logical_time = self.world.state().time;
        let baseline_event_seq = latest_runtime_event_seq(&self.world);

        for _ in 0..step_count.max(1) {
            if let Err(reason) = self
                .llm_sidecar
                .ensure_gameplay_ready(&self.world, &self.snapshot_config)
            {
                let (delta_logical_time, delta_event_seq) =
                    self.control_completion_delta(baseline_logical_time, baseline_event_seq);
                if self.tolerate_background_play_gameplay_block(
                    session,
                    writer,
                    action,
                    "runtime play loop hit a transient LLM access failure; will retry on the next play tick",
                    reason.clone(),
                    delta_logical_time,
                    delta_event_seq,
                )? {
                    return Ok(());
                }
                return self.block_gameplay_control(
                    session,
                    writer,
                    action,
                    "runtime play loop stopped because active LLM access is no longer available",
                    reason,
                    request_id,
                    delta_logical_time,
                    delta_event_seq,
                    true,
                );
            }
            let mut decision_trace: Option<AgentDecisionTrace> = None;
            match self.config.decision_mode {
                ViewerLiveDecisionMode::Script => self.script.enqueue(&mut self.world),
                ViewerLiveDecisionMode::Llm => {
                    self.llm_sidecar.request_decision();
                    match self.enqueue_llm_action_from_sidecar() {
                        Ok(trace) => {
                            decision_trace = trace;
                        }
                        Err(trace) => {
                            if session.subscribed.contains(&ViewerStream::Events) {
                                send_response(
                                    writer,
                                    &ViewerResponse::DecisionTrace {
                                        trace: trace.clone(),
                                    },
                                )?;
                            }
                            let (delta_logical_time, delta_event_seq) = self
                                .control_completion_delta(
                                    baseline_logical_time,
                                    baseline_event_seq,
                                );
                            let reason = trace.llm_error.clone().unwrap_or_else(|| {
                                "gameplay requires a configured and reachable LLM provider"
                                    .to_string()
                            });
                            if self.tolerate_background_play_gameplay_block(
                                session,
                                writer,
                                action,
                                "runtime play loop hit a transient LLM decision failure; will retry on the next play tick",
                                reason.clone(),
                                delta_logical_time,
                                delta_event_seq,
                            )? {
                                return Ok(());
                            }
                            return self.block_gameplay_control(
                                session,
                                writer,
                                action,
                                "runtime play loop stopped because the LLM decision provider failed",
                                reason,
                                request_id,
                                delta_logical_time,
                                delta_event_seq,
                                true,
                            );
                        }
                    }
                }
            }
            let journal_start = self.world.journal().events.len();
            if let Err(error) = self.world.step() {
                let (delta_logical_time, delta_event_seq) =
                    self.control_completion_delta(baseline_logical_time, baseline_event_seq);
                return self.block_runtime_control(
                    session,
                    writer,
                    action,
                    "runtime step aborted because world advance failed",
                    ViewerRuntimeLiveServerError::Runtime(error),
                    request_id,
                    delta_logical_time,
                    delta_event_seq,
                    true,
                );
            }
            session.transient_play_failures = 0;
            if self.world.state().time > baseline_logical_time
                || latest_runtime_event_seq(&self.world) > baseline_event_seq
            {
                self.confirm_player_gameplay_progress();
            }

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
            let pending_batch = match self.register_authoritative_batch(mapped_events.as_slice()) {
                Ok(batch) => batch,
                Err(error) => {
                    let (delta_logical_time, delta_event_seq) =
                        self.control_completion_delta(baseline_logical_time, baseline_event_seq);
                    return self.block_runtime_control(
                        session,
                        writer,
                        action,
                        "runtime step aborted because authoritative batch registration failed",
                        error,
                        request_id,
                        delta_logical_time,
                        delta_event_seq,
                        true,
                    );
                }
            };
            let batch_finality_updates =
                match self.advance_authoritative_batch_finality(self.world.state().time) {
                    Ok(updates) => updates,
                    Err(error) => {
                        let (delta_logical_time, delta_event_seq) = self
                            .control_completion_delta(baseline_logical_time, baseline_event_seq);
                        return self.block_runtime_control(
                            session,
                            writer,
                            action,
                            "runtime step aborted because authoritative finality update failed",
                            error,
                            request_id,
                            delta_logical_time,
                            delta_event_seq,
                            true,
                        );
                    }
                };

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
            let delta_logical_time = self
                .world
                .state()
                .time
                .saturating_sub(baseline_logical_time);
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
                error_code: None,
                error_message: None,
            };
            self.set_latest_player_gameplay_feedback(player_gameplay_feedback_from_control_ack(
                &control_mode_for_action(action, step_count),
                &ack,
            ));
            send_response(writer, &ViewerResponse::ControlCompletionAck { ack })?;
        }

        Ok(())
    }

    fn tolerate_background_play_gameplay_block(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
        action: &str,
        effect: &str,
        reason: String,
        delta_logical_time: u64,
        delta_event_seq: u64,
    ) -> Result<bool, ViewerRuntimeLiveServerError> {
        let confirmed_runtime_progress = self.world.state().time > self.initial_world_time;
        let is_background_play = action == "play" && session.playing;
        if !is_background_play || !confirmed_runtime_progress {
            return Ok(false);
        }
        session.transient_play_failures = session.transient_play_failures.saturating_add(1);
        if session.transient_play_failures >= BACKGROUND_PLAY_TRANSIENT_FAILURE_BUDGET {
            return Ok(false);
        }
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: action.to_string(),
            stage: "blocked".to_string(),
            effect: effect.to_string(),
            reason: Some(reason),
            hint: Some(LLM_GAMEPLAY_REQUIRED_HINT.to_string()),
            delta_logical_time,
            delta_event_seq,
        });
        if session.subscribed.contains(&ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot();
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(true)
    }

    fn flush_pending_virtual_events(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        if self.pending_virtual_events.is_empty() {
            return Ok(());
        }
        let mapped_events: Vec<_> = self.pending_virtual_events.drain(..).collect();
        let pending_batch = self.register_authoritative_batch(mapped_events.as_slice())?;
        let batch_finality_updates =
            self.advance_authoritative_batch_finality(self.world.state().time)?;

        if session.subscribed.contains(&ViewerStream::Events) {
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

        Ok(())
    }

    fn compat_snapshot(&mut self) -> WorldSnapshot {
        let runtime_snapshot = self.world.snapshot();
        let runtime_journal_len = runtime_snapshot.journal_len;
        let next_event_id = runtime_snapshot.last_event_id.saturating_add(1).max(1);
        let next_action_id = runtime_snapshot.next_action_id.max(1);
        self.llm_sidecar.refresh_provider_check_snapshot();
        let gameplay_gate = self
            .llm_sidecar
            .ensure_gameplay_ready(&self.world, &self.snapshot_config)
            .err();
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
                self.confirmed_player_gameplay_progress_time.is_some(),
                self.latest_player_gameplay_feedback.as_ref(),
                gameplay_gate.is_none(),
                gameplay_gate.as_deref(),
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

    fn ensure_gameplay_ready_for_control(&mut self, mode: &ViewerControl) -> Result<(), String> {
        match mode {
            ViewerControl::Pause => Ok(()),
            ViewerControl::Play | ViewerControl::Step { .. } => self
                .llm_sidecar
                .ensure_gameplay_ready(&self.world, &self.snapshot_config),
            ViewerControl::Seek { .. } => Ok(()),
        }
    }

    fn ensure_gameplay_ready_for_action(
        &mut self,
        action: &str,
        action_id: Option<&str>,
        target_agent_id: Option<&str>,
    ) -> Result<(), (String, String)> {
        self.llm_sidecar
            .ensure_gameplay_ready(&self.world, &self.snapshot_config)
            .map_err(|message| {
                self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
                    action: action.to_string(),
                    stage: "blocked".to_string(),
                    effect: "gameplay action rejected before runtime submission".to_string(),
                    reason: Some(message.clone()),
                    hint: Some(LLM_GAMEPLAY_REQUIRED_HINT.to_string()),
                    delta_logical_time: 0,
                    delta_event_seq: 0,
                });
                let code = if self.llm_sidecar.is_llm_mode() {
                    "llm_init_failed"
                } else {
                    "llm_mode_required"
                };
                let detail = match (action_id, target_agent_id) {
                    (Some(action_id), Some(target_agent_id)) => format!(
                        "{message} (action_id={action_id}, target_agent_id={target_agent_id})"
                    ),
                    _ => message,
                };
                (code.to_string(), detail)
            })
    }

    fn control_completion_delta(
        &self,
        baseline_logical_time: u64,
        baseline_event_seq: u64,
    ) -> (u64, u64) {
        (
            self.world
                .state()
                .time
                .saturating_sub(baseline_logical_time),
            latest_runtime_event_seq(&self.world).saturating_sub(baseline_event_seq),
        )
    }

    fn gameplay_control_error(&self, reason: String) -> (String, String) {
        let code = if self.llm_sidecar.is_llm_mode() {
            "llm_init_failed"
        } else {
            "llm_mode_required"
        };
        (code.to_string(), reason)
    }

    fn block_gameplay_control(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
        action: &str,
        effect: &str,
        reason: String,
        request_id: Option<u64>,
        delta_logical_time: u64,
        delta_event_seq: u64,
        emit_snapshot: bool,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        let (error_code, error_message) = self.gameplay_control_error(reason.clone());
        session.playing = false;
        session.next_play_step_at = None;
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: action.to_string(),
            stage: "blocked".to_string(),
            effect: effect.to_string(),
            reason: Some(reason),
            hint: Some(LLM_GAMEPLAY_REQUIRED_HINT.to_string()),
            delta_logical_time,
            delta_event_seq,
        });
        if let Some(request_id) = request_id {
            let ack = ControlCompletionAck {
                request_id,
                status: ControlCompletionStatus::Blocked,
                delta_logical_time,
                delta_event_seq,
                error_code: Some(error_code),
                error_message: Some(error_message),
            };
            send_response(writer, &ViewerResponse::ControlCompletionAck { ack })?;
        }
        if emit_snapshot && session.subscribed.contains(&ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot();
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(())
    }

    fn block_runtime_control(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
        action: &str,
        effect: &str,
        error: ViewerRuntimeLiveServerError,
        request_id: Option<u64>,
        delta_logical_time: u64,
        delta_event_seq: u64,
        emit_snapshot: bool,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        let (error_code, error_message, hint) = runtime_control_error_details(&error);
        eprintln!("viewer runtime live: control {action} failed: {error_message} ({error:?})");
        session.playing = false;
        session.next_play_step_at = None;
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: action.to_string(),
            stage: "blocked".to_string(),
            effect: effect.to_string(),
            reason: Some(error_message.clone()),
            hint: Some(hint),
            delta_logical_time,
            delta_event_seq,
        });
        if let Some(request_id) = request_id {
            let ack = ControlCompletionAck {
                request_id,
                status: ControlCompletionStatus::Blocked,
                delta_logical_time,
                delta_event_seq,
                error_code: Some(error_code),
                error_message: Some(error_message),
            };
            send_response(writer, &ViewerResponse::ControlCompletionAck { ack })?;
        }
        if emit_snapshot && session.subscribed.contains(&ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot();
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(())
    }
}
