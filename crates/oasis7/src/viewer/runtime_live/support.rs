use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::simulator::runtime_perf::unsupported_runtime_perf_snapshot;
use crate::simulator::{Location, RuntimePerfHealth, RuntimePerfSnapshot, WorldKernel, WorldModel};
use crate::viewer::gameplay_actions::formal_release_default_seed_model;

use super::*;

pub(crate) const FORMAL_RELEASE_DEFAULT_WORLD_ID: &str = "live-formal-release-default";
pub(crate) const FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID: &str = "starter-agent-0";

impl ViewerRuntimeLiveServerConfig {
    pub fn new(scenario: WorldScenario) -> Self {
        Self {
            bind_addr: "127.0.0.1:5010".to_string(),
            world_id: format!("live-runtime-{}", scenario.as_str()),
            scenario: Some(scenario),
            decision_mode: ViewerLiveDecisionMode::Script,
            play_step_interval: Duration::from_millis(800),
            chain_poll_interval: Duration::from_millis(200),
            auto_play_on_connect: false,
            hosted_public_join_mode: false,
            chain_status_bind: None,
            chain_submit_bind: None,
            chain_link_policy: ChainLinkPolicy::Enforcing,
            agent_chat_echo_enabled: control_plane::runtime_agent_chat_echo_enabled_from_env(),
            generated_world_dir: None,
        }
    }

    pub fn formal_release_default() -> Self {
        Self {
            bind_addr: "127.0.0.1:5010".to_string(),
            world_id: FORMAL_RELEASE_DEFAULT_WORLD_ID.to_string(),
            scenario: None,
            decision_mode: ViewerLiveDecisionMode::Script,
            play_step_interval: Duration::from_millis(800),
            chain_poll_interval: Duration::from_millis(200),
            auto_play_on_connect: false,
            hosted_public_join_mode: false,
            chain_status_bind: None,
            chain_submit_bind: None,
            chain_link_policy: ChainLinkPolicy::Enforcing,
            agent_chat_echo_enabled: control_plane::runtime_agent_chat_echo_enabled_from_env(),
            generated_world_dir: None,
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

    pub fn with_optional_scenario(mut self, scenario: Option<WorldScenario>) -> Self {
        self.scenario = scenario;
        if self.world_id.trim().is_empty() {
            self.world_id = scenario
                .map(|value| format!("live-runtime-{}", value.as_str()))
                .unwrap_or_else(|| FORMAL_RELEASE_DEFAULT_WORLD_ID.to_string());
        }
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

    pub fn with_auto_play_on_connect(mut self, enabled: bool) -> Self {
        self.auto_play_on_connect = enabled;
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

    pub fn with_chain_submit_bind(mut self, addr: impl Into<String>) -> Self {
        self.chain_submit_bind = Some(addr.into());
        self
    }

    pub fn with_chain_link_policy(mut self, policy: ChainLinkPolicy) -> Self {
        self.chain_link_policy = policy;
        self
    }

    pub fn with_agent_chat_echo_enabled(mut self, enabled: bool) -> Self {
        self.agent_chat_echo_enabled = enabled;
        self
    }

    pub fn with_generated_world_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.generated_world_dir = Some(dir.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RuntimeLiveScript {
    phase: u8,
    move_direction: i64,
}

impl RuntimeLiveScript {
    pub(super) fn enqueue(&mut self, world: &mut RuntimeWorld) {
        let mut agent_ids: Vec<String> = world.state().agents.keys().cloned().collect();
        agent_ids.sort();

        if agent_ids.is_empty() {
            world.submit_action(RuntimeAction::RegisterAgent {
                agent_id: "runtime-agent-0".to_string(),
                pos: GeoPos::new(0, 0, 0),
            });
            world.submit_action(RuntimeAction::RegisterAgent {
                agent_id: "runtime-agent-1".to_string(),
                pos: GeoPos::new(0, 0, 0),
            });
            return;
        }

        let phase = self.phase;
        self.phase = self.phase.wrapping_add(1) % 4;

        match phase {
            0 => {
                let first = &agent_ids[0];
                let Some(from_pos) = world.state().agents.get(first).map(|cell| cell.state.pos)
                else {
                    return;
                };
                if self.move_direction == 0 {
                    self.move_direction = 1;
                } else {
                    self.move_direction = -self.move_direction;
                }
                let delta_cm = self.move_direction * 1_000;
                world.submit_action(RuntimeAction::MoveAgent {
                    agent_id: first.clone(),
                    to: GeoPos::new(from_pos.x_cm + delta_cm, from_pos.y_cm, from_pos.z_cm),
                });
            }
            1 => {
                if agent_ids.len() < 2 {
                    world.submit_action(RuntimeAction::MoveAgent {
                        agent_id: "missing-agent".to_string(),
                        to: GeoPos::new(0, 0, 0),
                    });
                    return;
                }
                let first = &agent_ids[0];
                let second = &agent_ids[1];
                let Some(target) = world.state().agents.get(first).map(|cell| cell.state.pos)
                else {
                    return;
                };
                world.submit_action(RuntimeAction::MoveAgent {
                    agent_id: second.clone(),
                    to: target,
                });
            }
            2 => {
                if agent_ids.len() < 2 {
                    world.submit_action(RuntimeAction::MoveAgent {
                        agent_id: "missing-agent".to_string(),
                        to: GeoPos::new(0, 0, 0),
                    });
                    return;
                }
                let from = &agent_ids[0];
                let to = &agent_ids[1];
                let _ = world.set_agent_resource_balance(from, ResourceKind::Electricity, 64);
                let _ = world.set_agent_resource_balance(to, ResourceKind::Electricity, 64);
                world.submit_action(RuntimeAction::EmitResourceTransfer {
                    from_agent_id: from.clone(),
                    to_agent_id: to.clone(),
                    kind: ResourceKind::Electricity,
                    amount: 1,
                });
            }
            _ => {
                world.submit_action(RuntimeAction::MoveAgent {
                    agent_id: "missing-agent".to_string(),
                    to: GeoPos::new(0, 0, 0),
                });
            }
        }
    }
}

pub(super) struct RuntimeLiveSession {
    pub(super) subscribed: HashSet<ViewerStream>,
    pub(super) event_filters: Option<HashSet<ViewerEventKind>>,
    pub(super) current_player_id: Option<String>,
    pub(super) playing: bool,
    pub(super) next_play_step_at: Option<Instant>,
    pub(super) next_background_snapshot_at: Option<Instant>,
    pub(super) next_chain_poll_at: Option<Instant>,
    pub(super) metrics: RunnerMetrics,
    pub(super) transient_play_failures: u8,
    pub(super) initial_snapshot_sent: bool,
    pub(super) negotiated_protocol: crate::viewer::protocol::NegotiatedViewerProtocol,
}

impl RuntimeLiveSession {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::new_with_playing(false)
    }

    pub(super) fn new_with_playing(playing: bool) -> Self {
        Self {
            subscribed: HashSet::new(),
            event_filters: None,
            current_player_id: None,
            playing,
            next_play_step_at: None,
            next_background_snapshot_at: None,
            next_chain_poll_at: None,
            metrics: RunnerMetrics::default(),
            transient_play_failures: 0,
            initial_snapshot_sent: false,
            negotiated_protocol:
                crate::viewer::protocol::NegotiatedViewerProtocol::v1_without_capabilities(),
        }
    }

    pub(super) fn event_allowed(&self, event: &WorldEvent) -> bool {
        match &self.event_filters {
            Some(filters) => filters
                .iter()
                .any(|filter| viewer_event_kind_matches(filter, &event.kind)),
            None => true,
        }
    }

    pub(super) fn explicitly_subscribed_to(&self, stream: ViewerStream) -> bool {
        self.subscribed.contains(&stream)
    }

    pub(super) fn uses_default_subscription(&self) -> bool {
        self.subscribed.is_empty()
    }

    pub(super) fn wants_initial_snapshot(&self) -> bool {
        self.uses_default_subscription() || self.explicitly_subscribed_to(ViewerStream::Snapshot)
    }

    pub(super) fn wants_initial_recovery_metadata(&self) -> bool {
        self.uses_default_subscription()
            || self.explicitly_subscribed_to(ViewerStream::Snapshot)
            || self.explicitly_subscribed_to(ViewerStream::Events)
    }

    pub(super) fn should_emit_background_snapshot(&mut self, interval: Duration) -> bool {
        let now = Instant::now();
        if let Some(next_snapshot_at) = self.next_background_snapshot_at {
            if now < next_snapshot_at {
                return false;
            }
        }
        self.next_background_snapshot_at = Some(now + interval);
        true
    }

    pub(super) fn should_poll_chain(&mut self, interval: Duration) -> bool {
        let now = Instant::now();
        if let Some(next_poll_at) = self.next_chain_poll_at {
            if now < next_poll_at {
                return false;
            }
        }
        self.next_chain_poll_at = Some(now + interval);
        true
    }
}

pub(super) fn bootstrap_runtime_world(
    scenario: WorldScenario,
) -> Result<(RuntimeWorld, WorldConfig), String> {
    let config = WorldConfig::default();
    let init = WorldInitConfig::from_scenario(scenario, &config);
    let (model, _) = build_world_model(&config, &init)
        .map_err(|err| format!("runtime live bootstrap build_world_model failed: {err:?}"))?;
    bootstrap_runtime_world_from_model(config, &model, "runtime live bootstrap")
}

pub fn bootstrap_generated_sidecar_runtime_world(
    generated_world_dir: &Path,
) -> Result<(RuntimeWorld, WorldConfig, WorldModel), String> {
    let sidecar_dir = generated_world_dir.join("generated-scenario-world");
    let provenance_path = generated_world_dir.join("world-generation-provenance.json");
    if !provenance_path.is_file() {
        return Err(format!(
            "generated world provenance missing: {}",
            provenance_path.display()
        ));
    }
    let kernel = WorldKernel::load_from_dir(&sidecar_dir).map_err(|err| {
        format!(
            "runtime live generated sidecar load failed dir={} err={err:?}",
            sidecar_dir.display()
        )
    })?;
    let snapshot = kernel.snapshot();
    let config = snapshot.config;
    let model = snapshot.model;
    let (world, config) =
        bootstrap_runtime_world_from_model(config, &model, "runtime live generated sidecar")?;
    Ok((world, config, model))
}

fn bootstrap_runtime_world_from_model(
    config: WorldConfig,
    model: &WorldModel,
    label: &str,
) -> Result<(RuntimeWorld, WorldConfig), String> {
    let mut world = RuntimeWorld::new_production_hardened();
    world.set_resource_balance(ResourceKind::Electricity, 400);
    for (material, amount) in [
        ("structural_frame", 40),
        ("circuit_board", 4),
        ("servo_motor", 2),
        ("heat_coil", 6),
        ("refractory_brick", 8),
        ("iron_ore", 60),
        ("carbon_fuel", 20),
        ("copper_ore", 60),
        ("silicate_ore", 20),
        ("hardware_part", 40),
    ] {
        world
            .set_material_balance(material, amount)
            .map_err(|err| {
                format!(
                    "{label} set material balance failed material={} err={err:?}",
                    material
                )
            })?;
    }
    let mut seed_agents: Vec<(String, GeoPos, i64, i64)> = model
        .agents
        .iter()
        .map(|(agent_id, agent)| {
            (
                agent_id.clone(),
                agent.pos,
                agent.resources.get(ResourceKind::Electricity),
                agent.resources.get(ResourceKind::Data),
            )
        })
        .collect();
    seed_agents.sort_by(|left, right| left.0.cmp(&right.0));

    if seed_agents.is_empty() {
        seed_agents.push(("runtime-agent-0".to_string(), GeoPos::new(0, 0, 0), 32, 8));
        seed_agents.push(("runtime-agent-1".to_string(), GeoPos::new(0, 0, 0), 32, 8));
    }

    for (agent_id, pos, _, _) in &seed_agents {
        world.submit_action(RuntimeAction::RegisterAgent {
            agent_id: agent_id.clone(),
            pos: *pos,
        });
    }

    if world.pending_actions_len() > 0 {
        world
            .step()
            .map_err(|err| format!("{label} register step failed: {err:?}"))?;
    }

    for (agent_id, electricity, data) in world
        .state()
        .agents
        .keys()
        .cloned()
        .map(|agent_id| {
            let maybe_seed = seed_agents
                .iter()
                .find(|entry| entry.0 == agent_id)
                .cloned();
            match maybe_seed {
                Some((_, _, electricity, data)) => (agent_id, electricity.max(32), data.max(8)),
                None => (agent_id, 32, 8),
            }
        })
        .collect::<Vec<_>>()
    {
        world
            .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, electricity)
            .map_err(|err| {
                format!(
                    "{label} set electricity failed agent={} err={err:?}",
                    agent_id
                )
            })?;
        world
            .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, data)
            .map_err(|err| format!("{label} set data failed agent={} err={err:?}", agent_id))?;
    }
    world
        .step()
        .map_err(|err| format!("{label} resource seed consensus step failed: {err:?}"))?;

    Ok((world, config))
}

pub fn bootstrap_formal_release_runtime_world() -> Result<(RuntimeWorld, WorldConfig), String> {
    let (config, seed_model) = formal_release_default_seed_model()?;
    let starter_agent = seed_model
        .agents
        .get(FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID)
        .ok_or_else(|| {
            format!(
                "formal release seed model missing bootstrap agent {}",
                FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID
            )
        })?;
    let mut world = RuntimeWorld::new_production_hardened();
    world.set_resource_balance(ResourceKind::Electricity, 400);
    for (material, amount) in [
        ("structural_frame", 40),
        ("circuit_board", 4),
        ("servo_motor", 2),
        ("heat_coil", 6),
        ("refractory_brick", 8),
        ("iron_ore", 60),
        ("carbon_fuel", 20),
        ("copper_ore", 60),
        ("silicate_ore", 20),
        ("hardware_part", 40),
    ] {
        world
            .set_material_balance(material, amount)
            .map_err(|err| {
                format!(
                    "formal release bootstrap set material balance failed material={} err={err:?}",
                    material
                )
            })?;
    }
    world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID.to_string(),
        pos: starter_agent.pos,
    });
    world
        .step()
        .map_err(|err| format!("formal release bootstrap register step failed: {err:?}"))?;
    world
        .set_agent_resource_balance(
            FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID,
            ResourceKind::Electricity,
            starter_agent
                .resources
                .get(ResourceKind::Electricity)
                .max(32),
        )
        .map_err(|err| {
            format!(
                "formal release bootstrap set electricity failed agent={} err={err:?}",
                FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID
            )
        })?;
    world
        .set_agent_resource_balance(
            FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID,
            ResourceKind::Data,
            starter_agent.resources.get(ResourceKind::Data).max(8),
        )
        .map_err(|err| {
            format!(
                "formal release bootstrap set data failed agent={} err={err:?}",
                FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID
            )
        })?;
    world.step().map_err(|err| {
        format!("formal release bootstrap resource consensus step failed: {err:?}")
    })?;
    Ok((world, config))
}

pub(super) fn formal_release_default_seed_location_for_pos(pos: GeoPos) -> Option<Location> {
    let (_, model) = formal_release_default_seed_model().ok()?;
    let location_id = model
        .agents
        .values()
        .find(|agent| agent.pos == pos)
        .map(|agent| agent.location_id.clone())?;
    model.locations.get(&location_id).cloned()
}

pub(super) fn runtime_metrics(world: &RuntimeWorld) -> RunnerMetrics {
    let total_ticks = world.state().time;
    let total_actions = world.journal().len() as u64;
    let action_rejected = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.body,
                RuntimeWorldEventBody::Domain(RuntimeDomainEvent::ActionRejected { .. })
            )
        })
        .count() as u64;

    RunnerMetrics {
        total_ticks,
        total_agents: world.state().agents.len(),
        agents_active: world.state().agents.len(),
        agents_quota_exhausted: 0,
        total_actions,
        total_decisions: 0,
        actions_per_tick: if total_ticks > 0 {
            total_actions as f64 / total_ticks as f64
        } else {
            0.0
        },
        decisions_per_tick: 0.0,
        success_rate: if total_actions > 0 {
            (total_actions.saturating_sub(action_rejected)) as f64 / total_actions as f64
        } else {
            0.0
        },
        runtime_perf: unsupported_runtime_live_perf_snapshot(),
    }
}

pub(super) fn unsupported_runtime_live_perf_snapshot() -> RuntimePerfSnapshot {
    unsupported_runtime_perf_snapshot()
}

pub(super) fn latest_runtime_event_seq(world: &RuntimeWorld) -> u64 {
    world
        .journal()
        .events
        .last()
        .map(|event| event.id)
        .unwrap_or(0)
}

pub(super) fn send_response(
    writer: &mut BufWriter<TcpStream>,
    response: &ViewerResponse,
) -> Result<(), ViewerRuntimeLiveServerError> {
    let payload = serde_json::to_string(response)
        .map_err(|err| ViewerRuntimeLiveServerError::Serde(err.to_string()))?;
    writer.write_all(payload.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub(super) fn is_timeout_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

pub(super) fn lock_shared_server(
    shared: &Arc<Mutex<ViewerRuntimeLiveServer>>,
) -> Result<MutexGuard<'_, ViewerRuntimeLiveServer>, ViewerRuntimeLiveServerError> {
    shared.lock().map_err(|_| {
        ViewerRuntimeLiveServerError::Io(io::Error::other(
            "viewer runtime live shared state poisoned",
        ))
    })
}

pub(super) fn is_expected_disconnect_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::NotConnected
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_subscription_requests_initial_snapshot_and_recovery_metadata() {
        let session = RuntimeLiveSession::new();

        assert!(session.uses_default_subscription());
        assert!(session.wants_initial_snapshot());
        assert!(session.wants_initial_recovery_metadata());
        assert!(!session.explicitly_subscribed_to(ViewerStream::Snapshot));
        assert!(!session.explicitly_subscribed_to(ViewerStream::Events));
        assert!(!session.explicitly_subscribed_to(ViewerStream::Metrics));
    }

    #[test]
    fn explicit_subscriptions_drive_initial_request_semantics() {
        let mut snapshot_session = RuntimeLiveSession::new();
        snapshot_session.subscribed.insert(ViewerStream::Snapshot);
        assert!(!snapshot_session.uses_default_subscription());
        assert!(snapshot_session.wants_initial_snapshot());
        assert!(snapshot_session.wants_initial_recovery_metadata());

        let mut event_session = RuntimeLiveSession::new();
        event_session.subscribed.insert(ViewerStream::Events);
        assert!(!event_session.wants_initial_snapshot());
        assert!(event_session.wants_initial_recovery_metadata());

        let mut metrics_session = RuntimeLiveSession::new();
        metrics_session.subscribed.insert(ViewerStream::Metrics);
        assert!(!metrics_session.wants_initial_snapshot());
        assert!(!metrics_session.wants_initial_recovery_metadata());
    }

    #[test]
    fn bootstrap_runtime_world_defaults_to_production_release_policy() {
        let (world, _) =
            bootstrap_runtime_world(WorldScenario::Minimal).expect("bootstrap runtime live world");
        assert!(world.release_security_policy().is_production_hardened());
    }

    #[test]
    fn runtime_live_metrics_mark_runtime_perf_unknown_when_unavailable() {
        let perf = unsupported_runtime_live_perf_snapshot();
        assert_eq!(perf.health, RuntimePerfHealth::Unknown);
        assert_eq!(perf.tick.samples_total, 0);
        assert_eq!(perf.tick.samples_window, 0);
    }

    #[test]
    fn bootstrap_formal_release_runtime_world_uses_seeded_fragment_bootstrap_agent() {
        let (world, _) =
            bootstrap_formal_release_runtime_world().expect("formal release bootstrap");
        let agent = world
            .state()
            .agents
            .get(FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID)
            .expect("bootstrap agent should exist");
        let (_, seed_model) =
            formal_release_default_seed_model().expect("formal release seed model");
        let seed_agent = seed_model
            .agents
            .get(FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID)
            .expect("seed bootstrap agent should exist");
        let seed_location = seed_model
            .locations
            .get(&seed_agent.location_id)
            .expect("seed bootstrap location should exist");
        assert_eq!(world.state().agents.len(), 1);
        assert_eq!(agent.state.pos, seed_agent.pos);
        assert!(seed_agent.location_id.starts_with("frag-"));
        assert!(seed_location.fragment_budget.is_some());
        assert_eq!(agent.state.resources.get(ResourceKind::Electricity), 32);
        assert_eq!(agent.state.resources.get(ResourceKind::Data), 8);
    }
}
