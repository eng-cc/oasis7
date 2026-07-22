use super::*;
use crate::simulator::WorldKernel;

impl ViewerRuntimeLiveServer {
    pub(super) fn tolerate_background_play_gameplay_block(
        &mut self,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
        action: &str,
        play_step_interval: Duration,
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
        self.defer_next_auto_play_step_after_completion(
            play_step_interval.max(BACKGROUND_PLAY_TRANSIENT_FAILURE_RETRY_DELAY),
        );
        let hint = Self::llm_gameplay_hint_for_reason(&reason);
        self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
            action,
            "blocked",
            effect,
            Some("continue advancing the live world".to_string()),
            None,
            Some(reason),
            Some(hint),
            delta_logical_time,
            delta_event_seq,
        ));
        if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(true)
    }

    pub(super) fn flush_pending_virtual_events(
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

        if session.explicitly_subscribed_to(ViewerStream::Events) {
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

        if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
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

    pub(super) fn compat_snapshot(&mut self, current_player_id: Option<&str>) -> WorldSnapshot {
        let runtime_snapshot = self.world.snapshot();
        let runtime_state = &runtime_snapshot.state;
        let runtime_journal_len = runtime_snapshot.journal_len;
        let next_event_id = runtime_snapshot.last_event_id.saturating_add(1).max(1);
        let next_action_id = runtime_snapshot.next_action_id.max(1);
        self.llm_sidecar.refresh_provider_check_snapshot();
        let gameplay_gate = if self.llm_sidecar.is_llm_mode() {
            None
        } else {
            Some("gameplay requires runtime live server running with --llm".to_string())
        };
        let snapshot_player_id = current_player_id
            .map(str::trim)
            .filter(|player_id| !player_id.is_empty());
        let snapshot_bound_agent_id = snapshot_player_id
            .and_then(|player_id| self.llm_sidecar.bound_agent_for_player(player_id));
        let primary_agent_claim = snapshot_bound_agent_id.and_then(|agent_id| {
            build_player_agent_claim_snapshot(
                runtime_state,
                agent_id,
                self.world.governance_execution_policy().epoch_length_ticks,
            )
        });
        let first_agent_claim_target_bound = runtime_state
            .agents
            .contains_key(crate::viewer::FIRST_AGENT_CLAIM_TARGET_AGENT_ID)
            && self
                .llm_sidecar
                .agent_player_bindings
                .contains_key(crate::viewer::FIRST_AGENT_CLAIM_TARGET_AGENT_ID);
        let first_agent_claim_target_available = snapshot_player_id.is_some()
            && snapshot_bound_agent_id.is_none()
            && !first_agent_claim_target_bound;
        let model = runtime_state_to_simulator_model(
            runtime_state,
            &self.llm_sidecar,
            self.seed_model.as_ref(),
        );
        let micro_depot_facilities =
            WorldKernel::micro_depot_player_facility_snapshots_from_model(&model);
        let mut player_gameplay = build_player_gameplay_snapshot(
            runtime_state,
            snapshot_bound_agent_id,
            self.confirmed_player_gameplay_progress_time.is_some(),
            self.latest_player_gameplay_feedback.as_ref(),
            self.latest_player_gameplay_causality.as_ref(),
            gameplay_gate.is_none(),
            gameplay_gate.as_deref(),
            self.llm_sidecar.is_llm_mode() && self.supports_agent_chat(),
            first_agent_claim_target_available,
            primary_agent_claim,
        );
        player_gameplay.micro_depot_facilities = micro_depot_facilities;
        if snapshot_player_id.is_some() && snapshot_bound_agent_id.is_none() {
            player_gameplay.available_actions.retain(|action| {
                action.target_agent_id.is_none()
                    || action.action_id == crate::viewer::ACTION_CLAIM_FIRST_AGENT
            });
            if first_agent_claim_target_available
                && !player_gameplay
                    .available_actions
                    .iter()
                    .any(|action| action.action_id == crate::viewer::ACTION_CLAIM_FIRST_AGENT)
            {
                player_gameplay
                    .available_actions
                    .push(crate::simulator::PlayerGameplayAction {
                        action_id: crate::viewer::ACTION_CLAIM_FIRST_AGENT.to_string(),
                        label: "Claim first Agent".to_string(),
                        protocol_action: "gameplay_action.submit".to_string(),
                        target_agent_id: Some(
                            crate::viewer::FIRST_AGENT_CLAIM_TARGET_AGENT_ID.to_string(),
                        ),
                        disabled_reason: None,
                    });
            }
        }
        apply_runtime_snapshot_empty_entities_blocker(
            &mut player_gameplay,
            model.agents.is_empty(),
            model.locations.is_empty(),
        );
        let chain_resource_manifest = runtime_snapshot.chain_resource_manifest.clone();
        let latest_chain_resource_delta = runtime_snapshot
            .latest_chain_resource_delta
            .clone()
            .unwrap_or_default();
        WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: runtime_state.time,
            config: self.snapshot_config.clone(),
            model,
            runtime_snapshot: Some(runtime_snapshot),
            player_gameplay: Some(player_gameplay),
            chain_resource_manifest,
            latest_chain_resource_delta,
            chunk_runtime: ChunkRuntimeConfig::default(),
            intel_ttl_ticks: 0,
            next_event_id,
            next_action_id,
            pending_actions: Vec::new(),
            journal_len: runtime_journal_len,
        }
    }

    pub(super) fn ensure_gameplay_ready_for_control(
        &mut self,
        mode: &ViewerControl,
    ) -> Result<(), String> {
        match mode {
            ViewerControl::Pause => Ok(()),
            ViewerControl::Play | ViewerControl::Step { .. } => self
                .llm_sidecar
                .ensure_gameplay_ready(&self.world, &self.snapshot_config),
            ViewerControl::Seek { .. } => Ok(()),
        }
    }

    pub(super) fn ensure_gameplay_ready_for_action(
        &mut self,
        action: &str,
        action_id: Option<&str>,
        target_agent_id: Option<&str>,
    ) -> Result<(), (String, String)> {
        self.llm_sidecar
            .ensure_gameplay_ready(&self.world, &self.snapshot_config)
            .map_err(|message| {
                self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
                    action,
                    "blocked",
                    "gameplay action rejected before runtime submission",
                    Some("submit a gameplay action".to_string()),
                    target_agent_id.map(ToOwned::to_owned),
                    Some(message.clone()),
                    Some(LLM_GAMEPLAY_REQUIRED_HINT.to_string()),
                    0,
                    0,
                ));
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

    pub(super) fn control_completion_delta(
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

    pub(super) fn gameplay_control_error(&self, reason: String) -> (String, String) {
        let code = if self.llm_sidecar.is_llm_mode() {
            "llm_init_failed"
        } else {
            "llm_mode_required"
        };
        (code.to_string(), reason)
    }

    pub(super) fn llm_gameplay_hint_for_reason(reason: &str) -> String {
        let normalized = reason.to_ascii_lowercase();
        if normalized.contains("provider_gateway_unreachable")
            || normalized.contains("provider_http_502")
            || normalized.contains("provider_http_503")
            || normalized.contains("provider_http_504")
            || normalized.contains("provider bridge returned http 502")
            || normalized.contains("provider bridge returned http 503")
            || normalized.contains("provider bridge returned http 504")
            || normalized.contains("read operation timed out")
            || normalized.contains("operation timed out")
            || normalized.contains("timed out")
        {
            LLM_PROVIDER_GATEWAY_TIMEOUT_HINT.to_string()
        } else {
            LLM_GAMEPLAY_REQUIRED_HINT.to_string()
        }
    }

    pub(super) fn block_gameplay_control(
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
        self.pause_auto_play(session);
        session.next_play_step_at = None;
        let hint = Self::llm_gameplay_hint_for_reason(&reason);
        self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
            action,
            "blocked",
            effect,
            Some("advance the live world".to_string()),
            None,
            Some(reason),
            Some(hint),
            delta_logical_time,
            delta_event_seq,
        ));
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
        if emit_snapshot && session.explicitly_subscribed_to(ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(())
    }

    pub(super) fn block_runtime_control(
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
        emit_stderr_or_event(
            Level::WARN,
            format!("viewer runtime live: control {action} failed: {error_message} ({error:?})")
                .as_str(),
            "viewer runtime live control failed",
        );
        self.pause_auto_play(session);
        session.next_play_step_at = None;
        self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
            action,
            "blocked",
            effect,
            Some("advance the live world".to_string()),
            None,
            Some(error_message.clone()),
            Some(hint),
            delta_logical_time,
            delta_event_seq,
        ));
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
        if emit_snapshot && session.explicitly_subscribed_to(ViewerStream::Snapshot) {
            let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
            send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
        }
        Ok(())
    }
}
