use super::auth::verify_session_register_auth_proof_for_recovery;
use super::live::ViewerLiveDecisionMode;
use super::protocol::{
    AuthoritativeBatchFinality, AuthoritativeChallengeAck, AuthoritativeChallengeCommand,
    AuthoritativeChallengeError, AuthoritativeChallengeResolveRequest,
    AuthoritativeChallengeStatus, AuthoritativeChallengeSubmitRequest, AuthoritativeFinalityState,
    AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryAck, AuthoritativeRecoveryCommand,
    AuthoritativeRecoveryError, AuthoritativeRecoveryStatus, AuthoritativeRollbackReceipt,
    AuthoritativeRollbackRequest, AuthoritativeRollbackV2Request,
    AuthoritativeSessionRegisterRequest, AuthoritativeSessionRevokeRequest,
    AuthoritativeSessionRotateRequest, ControlCompletionAck, ControlCompletionStatus,
    GameplayActionError, RollbackAuthorizationEnvelope, RollbackIntent, VIEWER_PROTOCOL_VERSION,
    ViewerControl, ViewerControlProfile, ViewerEventKind, ViewerRequest, ViewerResponse,
    ViewerStream, viewer_event_kind_matches,
};
use crate::geometry::GeoPos;
use crate::observability::emit_stderr_or_event;
use crate::runtime::{
    Action as RuntimeAction, DomainEvent as RuntimeDomainEvent, Journal as RuntimeJournal,
    ReleaseSecurityPolicy, Snapshot as RuntimeSnapshot, World as RuntimeWorld,
    WorldError as RuntimeWorldError, WorldEventBody as RuntimeWorldEventBody, blake3_hex,
};
use crate::simulator::{
    AgentDecisionTrace, CHUNK_GENERATION_SCHEMA_VERSION, ChunkRuntimeConfig,
    PlayerGameplayRecentFeedback, RejectReason as SimulatorRejectReason, ResourceKind,
    RunnerMetrics, SNAPSHOT_VERSION, WorldConfig, WorldEvent, WorldInitConfig, WorldModel,
    WorldScenario, WorldSnapshot, build_world_model,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use tracing::Level;
mod authoritative;
mod branch_commitment;
#[path = "runtime_live/chain_link.rs"]
mod chain_link;
mod claim_snapshot;
#[path = "runtime_live/config.rs"]
mod config;
#[path = "runtime_live/control_blocking.rs"]
mod control_blocking;
#[path = "runtime_live/control_feedback.rs"]
mod control_feedback;
#[path = "runtime_live/control_plane.rs"]
mod control_plane;
#[path = "runtime_live/control_utils.rs"]
mod control_utils;
#[path = "runtime_live/decision_trace.rs"]
mod decision_trace;
mod gameplay_snapshot;
mod gameplay_snapshot_feedback;
mod gameplay_snapshot_helpers;
mod gameplay_snapshot_lane;
mod gameplay_validation_preview;
mod mapping;
mod player_gameplay;
mod recovery;
mod recovery_audit;
mod recovery_compensation;
mod recovery_persistence;
mod recovery_receipt;
mod recovery_rollback_v2;
mod recovery_session;
mod session_policy;
mod support;
#[cfg(test)]
mod tests;
use authoritative::{
    RuntimeAuthoritativeBatchRecord, RuntimeAuthoritativeChallengeRecord,
    RuntimeSettlementRankingGate, RuntimeStableCheckpoint,
};
use claim_snapshot::build_player_agent_claim_snapshot;
pub use config::{ChainLinkPolicy, ViewerRuntimeLiveServerConfig, ViewerRuntimeLiveServerError};
pub use control_plane::runtime_agent_chat_echo_enabled_from_env;
use control_plane::{RuntimeLlmSidecar, RuntimePlayerBindingPlan};
use control_utils::{control_mode_for_action, control_mode_label, runtime_control_error_details};
use decision_trace::{append_decision_upstream_trace, decision_trace_provider_error_retryable};
use gameplay_snapshot::{
    PlayerGameplayCausalitySignal, apply_runtime_snapshot_empty_entities_blocker,
    build_player_gameplay_snapshot, player_gameplay_causality_from_runtime_events,
    player_gameplay_feedback_from_control_ack,
};
use mapping::{map_runtime_event, runtime_state_to_simulator_model};
use session_policy::{
    RuntimeSessionPolicy, RuntimeSessionRevokeMetadata, location_id_for_pos,
    map_session_policy_error_code, normalize_optional_string, session_revoke_metadata_key,
};
pub use support::bootstrap_formal_release_runtime_world as viewer_bootstrap_formal_release_runtime_world;
pub use support::bootstrap_generated_sidecar_runtime_world as viewer_bootstrap_generated_sidecar_runtime_world;
use support::{
    FORMAL_RELEASE_DEFAULT_WORLD_ID, RuntimeLiveScript, RuntimeLiveSession,
    bootstrap_formal_release_runtime_world, bootstrap_generated_sidecar_runtime_world,
    bootstrap_runtime_world, is_expected_disconnect_error, is_timeout_error,
    latest_runtime_event_seq, lock_shared_server, runtime_metrics, send_response,
};

pub const VIEWER_FORMAL_RELEASE_DEFAULT_WORLD_ID: &str = FORMAL_RELEASE_DEFAULT_WORLD_ID;

const AUTHORITATIVE_BATCH_CONFIRM_DELAY_TICKS: u64 = 1;
const AUTHORITATIVE_BATCH_FINALITY_WINDOW_TICKS: u64 = 2;
const MAX_AUTHORITATIVE_BATCH_HISTORY: usize = 256;
const MAX_AUTHORITATIVE_CHALLENGE_HISTORY: usize = 512;
const MAX_AUTHORITATIVE_STABLE_CHECKPOINTS: usize = 64;
const LLM_GAMEPLAY_REQUIRED_HINT: &str =
    "enable --llm and configure a reachable LLM provider before retrying gameplay controls";
const LLM_PROVIDER_GATEWAY_TIMEOUT_HINT: &str = "local LLM provider gateway timed out; inspect output/local-letai-game-test/*/local-letai-provider-bridge.log, confirm proxy/upstream LetAI reachability, then rerun scripts/run-local-letai-game-test.sh or its chat probe/bridge smoke before retrying gameplay controls";
const RUNTIME_CONTROL_REQUIRED_HINT: &str = "inspect the reported runtime failure, repair the broken world/module state, then retry the control";
const BACKGROUND_PLAY_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(2);

pub struct ViewerRuntimeLiveServer {
    config: ViewerRuntimeLiveServerConfig,
    world: RuntimeWorld,
    initial_world_time: u64,
    auto_play_paused: bool,
    next_auto_play_step_at: Option<Instant>,
    last_chain_committed_height: u64,
    confirmed_player_gameplay_progress_time: Option<u64>,
    snapshot_config: WorldConfig,
    seed_model: Option<WorldModel>,
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
    rollback_readiness: BTreeMap<String, recovery_receipt::RuntimeRollbackReadinessRecord>,
    consumed_strict_audit_nonces: BTreeSet<String>,
    settlement_ranking_gate: RuntimeSettlementRankingGate,
    latest_player_gameplay_feedback: Option<PlayerGameplayRecentFeedback>,
    latest_player_gameplay_causality: Option<PlayerGameplayCausalitySignal>,
    runtime_action_players: BTreeMap<u64, String>,
    consumed_rollback_operator_nonces: BTreeSet<String>,
    authoritative_recovery_write_fence: Option<String>,
    #[cfg(test)]
    recovery_fault_injection: Option<recovery::RuntimeRecoveryFaultInjection>,
    #[cfg(test)]
    authoritative_recovery_dir_override: Option<PathBuf>,
}

const BACKGROUND_PLAY_TRANSIENT_FAILURE_BUDGET: u8 = 12;
const BACKGROUND_PLAY_TRANSIENT_FAILURE_RETRY_DELAY: Duration = Duration::from_secs(5);

fn should_emit_runtime_advance_snapshot(
    session: &mut RuntimeLiveSession,
    action: &str,
    emit_while_paused: bool,
) -> bool {
    let is_background_play = action == "play" && session.playing && !emit_while_paused;
    if !is_background_play {
        return true;
    }
    session.should_emit_background_snapshot(BACKGROUND_PLAY_SNAPSHOT_INTERVAL)
}

impl ViewerRuntimeLiveServer {
    pub fn new(
        config: ViewerRuntimeLiveServerConfig,
    ) -> Result<Self, ViewerRuntimeLiveServerError> {
        let (mut world, snapshot_config, seed_model) =
            if let Some(generated_world_dir) = config.generated_world_dir.as_deref() {
                let (world, snapshot_config, seed_model) =
                    bootstrap_generated_sidecar_runtime_world(generated_world_dir)
                        .map_err(ViewerRuntimeLiveServerError::Init)?;
                (world, snapshot_config, Some(seed_model))
            } else {
                match config.scenario {
                    Some(scenario) => {
                        let (world, snapshot_config) = bootstrap_runtime_world(scenario)
                            .map_err(ViewerRuntimeLiveServerError::Init)?;
                        (world, snapshot_config, None)
                    }
                    None if config.chain_status_bind.is_some() => (
                        RuntimeWorld::new_production_hardened(),
                        WorldConfig::default(),
                        None,
                    ),
                    None => {
                        let (world, snapshot_config) = bootstrap_formal_release_runtime_world()
                            .map_err(ViewerRuntimeLiveServerError::Init)?;
                        (world, snapshot_config, None)
                    }
                }
            };
        let mut recovered_generation = None;
        if let Some(recovery_dir) = config
            .generated_world_dir
            .as_deref()
            .map(|dir| dir.join("runtime-live-authoritative-recovery"))
        {
            std::fs::create_dir_all(&recovery_dir)?;
            let preflight_path = recovery_dir.join(format!(
                ".authoritative-recovery-preflight-{}",
                std::process::id()
            ));
            std::fs::write(&preflight_path, b"authoritative-recovery-preflight")?;
            std::fs::remove_file(&preflight_path)?;
            if let Some(committed) =
                RuntimeWorld::load_authoritative_recovery_generation(&recovery_dir)
                    .map_err(ViewerRuntimeLiveServerError::Runtime)?
            {
                let metadata = committed.recovery_metadata;
                let generation = serde_json::from_slice::<
                    recovery_receipt::RuntimeAuthoritativeRecoveryGeneration,
                >(&metadata)
                .or_else(|generation_error| {
                    serde_json::from_slice::<AuthoritativeRecoveryAck<u64>>(&metadata)
                        .map(
                            |ack| recovery_receipt::RuntimeAuthoritativeRecoveryGeneration {
                                schema_version: 0,
                                reorg_epoch: ack.reorg_epoch,
                                ack,
                                authoritative_batches: VecDeque::new(),
                                next_authoritative_batch_id: 1,
                                authoritative_challenges: VecDeque::new(),
                                next_authoritative_challenge_id: 1,
                                stable_checkpoints: VecDeque::new(),
                                session_policy: RuntimeSessionPolicy::default(),
                                session_revoke_metadata: Vec::new(),
                                rollback_readiness: BTreeMap::new(),
                                consumed_strict_audit_nonces: BTreeSet::new(),
                                runtime_action_players: BTreeMap::new(),
                                consumed_rollback_operator_nonces: BTreeSet::new(),
                                session_side_effects: Default::default(),
                            },
                        )
                        .map_err(|_| generation_error)
                })
                .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
                world = committed.world;
                if generation.ack.reorg_epoch != generation.reorg_epoch
                    || generation
                        .ack
                        .rollback_receipt
                        .as_ref()
                        .is_some_and(|receipt| {
                            world
                                .rollback_nonce_outcome(receipt.authorization_nonce.as_str())
                                .is_none_or(|outcome| {
                                    outcome.canonical_intent_hash != receipt.canonical_intent_digest
                                        || outcome.committed_reorg_epoch != generation.reorg_epoch
                                })
                        })
                {
                    return Err(ViewerRuntimeLiveServerError::Init(
                        "authoritative recovery generation metadata/world integrity mismatch"
                            .to_string(),
                    ));
                }
                recovered_generation = Some(generation);
            }
        }
        let initial_world_time = world.state().time;
        let mut llm_sidecar = match seed_model.as_ref() {
            Some(model) => {
                RuntimeLlmSidecar::new(config.decision_mode).with_runtime_seed_model(model)
            }
            None => RuntimeLlmSidecar::new(config.decision_mode),
        };
        if let Some(generation) = recovered_generation.as_ref() {
            Self::restore_persisted_session_side_effects(
                &mut llm_sidecar,
                &generation.session_side_effects,
            );
        }
        let next_virtual_event_id = latest_runtime_event_seq(&world).saturating_add(1).max(1);
        let mut server = Self {
            config,
            world,
            initial_world_time,
            auto_play_paused: false,
            next_auto_play_step_at: None,
            last_chain_committed_height: 0,
            confirmed_player_gameplay_progress_time: None,
            snapshot_config,
            seed_model,
            script: RuntimeLiveScript::default(),
            llm_sidecar,
            pending_virtual_events: recovered_generation
                .as_ref()
                .map(|generation| {
                    generation
                        .session_side_effects
                        .pending_virtual_events
                        .clone()
                })
                .unwrap_or_default(),
            next_virtual_event_id,
            authoritative_batches: recovered_generation
                .as_ref()
                .map(|generation| generation.authoritative_batches.clone())
                .unwrap_or_default(),
            next_authoritative_batch_id: recovered_generation
                .as_ref()
                .map(|generation| generation.next_authoritative_batch_id)
                .unwrap_or(1),
            authoritative_challenges: recovered_generation
                .as_ref()
                .map(|generation| generation.authoritative_challenges.clone())
                .unwrap_or_default(),
            next_authoritative_challenge_id: recovered_generation
                .as_ref()
                .map(|generation| generation.next_authoritative_challenge_id)
                .unwrap_or(1),
            stable_checkpoints: recovered_generation
                .as_ref()
                .map(|generation| generation.stable_checkpoints.clone())
                .unwrap_or_default(),
            reorg_epoch: recovered_generation
                .as_ref()
                .map(|generation| generation.reorg_epoch)
                .unwrap_or(0),
            session_policy: recovered_generation
                .as_ref()
                .map(|generation| generation.session_policy.clone())
                .unwrap_or_default(),
            session_revoke_metadata: recovered_generation
                .as_ref()
                .map(|generation| {
                    generation
                        .session_revoke_metadata
                        .iter()
                        .map(|entry| {
                            (
                                session_revoke_metadata_key(
                                    entry.player_id.as_str(),
                                    entry.session_pubkey.as_str(),
                                ),
                                entry.metadata.clone(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            rollback_readiness: recovered_generation
                .as_ref()
                .map(|generation| generation.rollback_readiness.clone())
                .unwrap_or_default(),
            consumed_strict_audit_nonces: recovered_generation
                .as_ref()
                .map(|generation| generation.consumed_strict_audit_nonces.clone())
                .unwrap_or_default(),
            settlement_ranking_gate: RuntimeSettlementRankingGate::default(),
            latest_player_gameplay_feedback: None,
            latest_player_gameplay_causality: None,
            runtime_action_players: recovered_generation
                .as_ref()
                .map(|generation| generation.runtime_action_players.clone())
                .unwrap_or_default(),
            consumed_rollback_operator_nonces: recovered_generation
                .as_ref()
                .map(|generation| generation.consumed_rollback_operator_nonces.clone())
                .unwrap_or_default(),
            authoritative_recovery_write_fence: None,
            #[cfg(test)]
            recovery_fault_injection: None,
            #[cfg(test)]
            authoritative_recovery_dir_override: None,
        };
        server.rebuild_settlement_ranking_gate();
        Ok(server)
    }

    pub fn run(self) -> Result<(), ViewerRuntimeLiveServerError> {
        let listener = TcpListener::bind(&self.config.bind_addr)?;
        let shared = Arc::new(Mutex::new(self));
        for incoming in listener.incoming() {
            let stream = incoming?;
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                if let Err(err) = Self::serve_shared_stream(shared, stream) {
                    emit_stderr_or_event(
                        Level::WARN,
                        format!("viewer runtime live server error: {err:?}").as_str(),
                        "viewer runtime live server error",
                    );
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

    fn chain_link_enabled(&self) -> bool {
        self.config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    }

    fn supports_agent_chat(&self) -> bool {
        self.llm_sidecar.supports_agent_chat() || self.config.agent_chat_echo_enabled
    }

    fn enable_auto_play_for_session_if_available(&mut self, session: &mut RuntimeLiveSession) {
        if !self.config.auto_play_on_connect {
            return;
        }
        session.playing = !self.auto_play_paused;
        session.next_play_step_at = None;
        session.transient_play_failures = 0;
    }

    fn pause_auto_play(&mut self, session: &mut RuntimeLiveSession) {
        self.auto_play_paused = true;
        session.playing = false;
        session.next_play_step_at = None;
    }

    fn resume_auto_play(&mut self, session: &mut RuntimeLiveSession) {
        self.auto_play_paused = false;
        session.playing = true;
        session.next_play_step_at = None;
        session.transient_play_failures = 0;
        self.next_auto_play_step_at = None;
    }

    fn should_advance_auto_play_step(&mut self) -> bool {
        if !self.config.auto_play_on_connect || self.auto_play_paused {
            self.next_auto_play_step_at = None;
            return false;
        }
        let now = Instant::now();
        if let Some(next_step_at) = self.next_auto_play_step_at {
            if now < next_step_at {
                return false;
            }
        }
        self.next_auto_play_step_at = Some(now + self.config.play_step_interval);
        true
    }

    fn defer_next_auto_play_step_after_completion(&mut self, interval: Duration) {
        if self.config.auto_play_on_connect && !self.auto_play_paused {
            self.next_auto_play_step_at = Some(Instant::now() + interval);
        }
    }

    fn emit_background_play_snapshot(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        if session.explicitly_subscribed_to(ViewerStream::Snapshot)
            && should_emit_runtime_advance_snapshot(session, "play", false)
        {
            let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        session.metrics = runtime_metrics(&self.world);
        if session.explicitly_subscribed_to(ViewerStream::Metrics) {
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

    fn drive_auto_play(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        if !self.config.auto_play_on_connect
            || self.auto_play_paused
            || !session.initial_snapshot_sent
        {
            return Ok(());
        }
        session.playing = true;
        if self.should_advance_auto_play_step() {
            let play_step_interval = self.config.play_step_interval;
            self.advance_runtime(session, writer, "play", 1, None, false)?;
            self.defer_next_auto_play_step_after_completion(play_step_interval);
        } else {
            self.emit_background_play_snapshot(session, writer)?;
        }
        Ok(())
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
        let mut session = RuntimeLiveSession::new_with_playing(false);

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

            let (write_fenced, chain_link_enabled, chain_poll_interval) = {
                let server = lock_shared_server(&shared)?;
                (
                    server.authoritative_recovery_write_fence.is_some(),
                    server.chain_link_enabled(),
                    server.config.chain_poll_interval,
                )
            };
            if !write_fenced
                && chain_link_enabled
                && session.initial_snapshot_sent
                && session.should_poll_chain(chain_poll_interval)
            {
                if let Err(err) = Self::sync_chain_linked_runtime_minimized_lock(
                    &shared,
                    &mut session,
                    &mut writer,
                ) {
                    emit_stderr_or_event(
                        Level::WARN,
                        format!("viewer runtime live: chain sync skipped: {err:?}").as_str(),
                        "viewer runtime live chain sync skipped",
                    );
                }
            }

            let mut server = lock_shared_server(&shared)?;
            if server.authoritative_recovery_write_fence.is_none() {
                server.drive_auto_play(&mut session, &mut writer)?;
            }
        }
    }

    fn serve_stream(&mut self, stream: TcpStream) -> Result<(), ViewerRuntimeLiveServerError> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);
        let mut writer = BufWriter::new(stream);
        let mut session = RuntimeLiveSession::new_with_playing(false);

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

            if self.authoritative_recovery_write_fence.is_none()
                && self.chain_link_enabled()
                && session.initial_snapshot_sent
                && session.should_poll_chain(self.config.chain_poll_interval)
            {
                if let Err(err) = self.sync_chain_linked_runtime(&mut session, &mut writer) {
                    emit_stderr_or_event(
                        Level::WARN,
                        format!("viewer runtime live: chain sync skipped: {err:?}").as_str(),
                        "viewer runtime live chain sync skipped",
                    );
                }
            }

            if self.authoritative_recovery_write_fence.is_none() {
                self.drive_auto_play(&mut session, &mut writer)?;
            }
        }
    }

    fn handle_request(
        &mut self,
        request: ViewerRequest,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        self.resolve_authoritative_recovery_write_fence()?;
        if self.authoritative_recovery_write_fence.is_some()
            && !matches!(
                &request,
                ViewerRequest::Hello { .. }
                    | ViewerRequest::HelloV2 { .. }
                    | ViewerRequest::Subscribe { .. }
                    | ViewerRequest::RequestSnapshot
            )
        {
            return Err(ViewerRuntimeLiveServerError::Init(
                "authoritative recovery commit status is unknown; runtime is read-only until durable generation readback resolves"
                    .to_string(),
            ));
        }
        match request {
            ViewerRequest::Hello { version: _, .. } => {
                session.negotiated_protocol =
                    crate::viewer::protocol::NegotiatedViewerProtocol::v1_without_capabilities();
                let capabilities = Vec::new();
                send_response(
                    writer,
                    &ViewerResponse::HelloAck {
                        server: "oasis7".to_string(),
                        version: VIEWER_PROTOCOL_VERSION,
                        min_version: 1,
                        max_version: VIEWER_PROTOCOL_VERSION,
                        capabilities,
                        world_id: self.config.world_id.clone(),
                        control_profile: ViewerControlProfile::Live,
                    },
                )?;
            }
            ViewerRequest::HelloV2 {
                version,
                capabilities: offered,
                ..
            } => {
                let selected = if version >= 2
                    && self.authoritative_recovery_dir().is_some()
                    && offered.iter().any(|capability| {
                        capability == crate::viewer::protocol::GOVERNED_ROLLBACK_REPLAY_CAPABILITY
                    }) {
                    vec![crate::viewer::protocol::GOVERNED_ROLLBACK_REPLAY_CAPABILITY.to_string()]
                } else {
                    Vec::new()
                };
                session.negotiated_protocol = crate::viewer::protocol::NegotiatedViewerProtocol {
                    version,
                    capabilities: selected.clone(),
                };
                send_response(
                    writer,
                    &ViewerResponse::HelloAck {
                        server: "oasis7".to_string(),
                        version: VIEWER_PROTOCOL_VERSION,
                        min_version: 1,
                        max_version: VIEWER_PROTOCOL_VERSION,
                        capabilities: selected,
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
                if self.chain_link_enabled() && !session.initial_snapshot_sent {
                    if let Err(err) = self.prime_chain_linked_runtime_for_snapshot() {
                        if self.config.chain_link_policy == ChainLinkPolicy::Enforcing {
                            return Err(err);
                        }
                        emit_stderr_or_event(
                            Level::WARN,
                            format!(
                                "viewer runtime live: initial chain sync skipped before snapshot: {err:?}"
                            )
                            .as_str(),
                            "viewer runtime live initial chain sync skipped",
                        );
                    }
                }
                if session.wants_initial_snapshot() {
                    let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                    send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                }
                if session.wants_initial_recovery_metadata() {
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
                                rollback_receipt: None,
                                acknowledged_at_tick: self.world.state().time,
                            },
                        },
                    )?;
                }
                if session.explicitly_subscribed_to(ViewerStream::Metrics) {
                    session.metrics = runtime_metrics(&self.world);
                    send_response(
                        writer,
                        &ViewerResponse::Metrics {
                            time: Some(self.world.state().time),
                            metrics: session.metrics.clone(),
                        },
                    )?;
                }
                if session.explicitly_subscribed_to(ViewerStream::Events) {
                    self.emit_authoritative_batch_snapshot(writer)?;
                    self.emit_authoritative_challenge_snapshot(writer)?;
                }
                session.initial_snapshot_sent = true;
                self.enable_auto_play_for_session_if_available(session);
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
            ViewerRequest::PromptControl { command } => {
                match self.handle_prompt_control(*command) {
                    Ok(ack) => {
                        send_response(writer, &ViewerResponse::PromptControlAck { ack })?;
                    }
                    Err(error) => {
                        send_response(writer, &ViewerResponse::PromptControlError { error })?;
                    }
                }
            }
            ViewerRequest::AgentChat { request } => match self.handle_agent_chat(request) {
                Ok(ack) => {
                    send_response(writer, &ViewerResponse::AgentChatAck { ack })?;
                    let provider_errors = self.enqueue_pending_provider_agent_chat_replies();
                    for error in provider_errors {
                        send_response(writer, &ViewerResponse::AgentChatError { error })?;
                    }
                    self.flush_pending_virtual_events(session, writer)?;
                }
                Err(error) => {
                    send_response(writer, &ViewerResponse::AgentChatError { error })?;
                }
            },
            ViewerRequest::GameplayAction { request } => match self.handle_gameplay_action(request)
            {
                Ok(ack) => {
                    let ack_player_id = ack.player_id.clone();
                    send_response(writer, &ViewerResponse::GameplayActionAck { ack })?;
                    if !ack_player_id.trim().is_empty() {
                        session.current_player_id = Some(ack_player_id);
                    }
                    if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
                        let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                        send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                    }
                }
                Err(error) => {
                    self.record_gameplay_action_rejection(&error);
                    send_response(writer, &ViewerResponse::GameplayActionError { error })?;
                    if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
                        let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                        send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                    }
                }
            },
            ViewerRequest::CollectData { command } => {
                self.handle_collect_data_protocol_request(command, session, writer)?;
            }
            ViewerRequest::QuoteRefineCompound { request } => send_response(
                writer,
                &self
                    .handle_refine_quote(request)
                    .map(|quote| ViewerResponse::RefineQuotePreflight { quote })
                    .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
            )?,
            ViewerRequest::QuoteProductValidation { request } => send_response(
                writer,
                &self
                    .handle_product_validation_quote(request)
                    .map(|quote| ViewerResponse::ProductValidationQuotePreflight { quote })
                    .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
            )?,
            ViewerRequest::QuotePowerSurvival { request } => self.quote_power(request, writer)?,
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
                match self.handle_authoritative_recovery_for_protocol(
                    command,
                    &session.negotiated_protocol,
                ) {
                    Ok((ack, emit_snapshot_after_ack)) => {
                        let ack_player_id = ack.player_id.clone();
                        let ack_status = ack.status;
                        send_response(writer, &ViewerResponse::AuthoritativeRecoveryAck { ack })?;
                        if let Some(player_id) = ack_player_id.as_deref() {
                            match ack_status {
                                AuthoritativeRecoveryStatus::SessionRevoked => {
                                    if session.current_player_id.as_deref() == Some(player_id) {
                                        session.current_player_id = None;
                                    }
                                }
                                _ => session.current_player_id = Some(player_id.to_string()),
                            }
                        }
                        if emit_snapshot_after_ack {
                            let snapshot =
                                self.compat_snapshot(session.current_player_id.as_deref());
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
                self.pause_auto_play(session);
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
            }
            ViewerControl::Play => {
                self.resume_auto_play(session);
            }
            ViewerControl::Step { count } => {
                self.pause_auto_play(session);
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
                self.advance_runtime(session, writer, "step", count.max(1), request_id, true)?;
            }
            ViewerControl::Seek { tick } => {
                self.pause_auto_play(session);
                session.next_play_step_at = None;
                session.transient_play_failures = 0;
                emit_stderr_or_event(
                    Level::INFO,
                    format!(
                        "viewer runtime live: ignore seek control in live mode (target_tick={tick})"
                    )
                    .as_str(),
                    "viewer runtime live ignored seek control in live mode",
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
        let mut runtime_events_for_feedback = Vec::new();

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
                    self.config.play_step_interval,
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
                            if session.explicitly_subscribed_to(ViewerStream::Events) {
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
                            let reason = append_decision_upstream_trace(reason, &trace);
                            if decision_trace_provider_error_retryable(&trace).unwrap_or(true) {
                                if self.tolerate_background_play_gameplay_block(
                                    session,
                                    writer,
                                    action,
                                    self.config.play_step_interval,
                                    "runtime play loop hit a transient LLM decision failure; will retry on the next play tick",
                                    reason.clone(),
                                    delta_logical_time,
                                    delta_event_seq,
                                )? {
                                    return Ok(());
                                }
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
            runtime_events_for_feedback.extend(new_events.iter().cloned());
            let mut mapped_events = Vec::new();
            for runtime_event in &new_events {
                let event = map_runtime_event(
                    runtime_event,
                    &self.snapshot_config,
                    self.seed_model.as_ref(),
                );
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
                if session.explicitly_subscribed_to(ViewerStream::Events) {
                    send_response(writer, &ViewerResponse::DecisionTrace { trace })?;
                }
            }

            if session.explicitly_subscribed_to(ViewerStream::Events)
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

            if session.explicitly_subscribed_to(ViewerStream::Snapshot)
                && should_emit_runtime_advance_snapshot(session, action, emit_while_paused)
            {
                let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
            }

            session.metrics = runtime_metrics(&self.world);
            if session.explicitly_subscribed_to(ViewerStream::Metrics) {
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
            let feedback = player_gameplay_feedback_from_control_ack(
                &control_mode_for_action(action, step_count),
                &ack,
            );
            let causality =
                player_gameplay_causality_from_runtime_events(&runtime_events_for_feedback);
            self.set_latest_player_gameplay_feedback_with_causality(feedback, causality);
            if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
                let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
            }
            send_response(writer, &ViewerResponse::ControlCompletionAck { ack })?;
        }

        Ok(())
    }
}
