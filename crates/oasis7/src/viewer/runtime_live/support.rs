use std::collections::HashSet;
use std::io::Write;

use super::*;

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
                pos: GeoPos::new(0.0, 0.0, 0.0),
            });
            world.submit_action(RuntimeAction::RegisterAgent {
                agent_id: "runtime-agent-1".to_string(),
                pos: GeoPos::new(0.0, 0.0, 0.0),
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
                let delta_cm = (self.move_direction * 1_000) as f64;
                world.submit_action(RuntimeAction::MoveAgent {
                    agent_id: first.clone(),
                    to: GeoPos::new(from_pos.x_cm + delta_cm, from_pos.y_cm, from_pos.z_cm),
                });
            }
            1 => {
                if agent_ids.len() < 2 {
                    world.submit_action(RuntimeAction::MoveAgent {
                        agent_id: "missing-agent".to_string(),
                        to: GeoPos::new(0.0, 0.0, 0.0),
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
                        to: GeoPos::new(0.0, 0.0, 0.0),
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
                    to: GeoPos::new(0.0, 0.0, 0.0),
                });
            }
        }
    }
}

pub(super) struct RuntimeLiveSession {
    pub(super) subscribed: HashSet<ViewerStream>,
    pub(super) event_filters: Option<HashSet<ViewerEventKind>>,
    pub(super) playing: bool,
    pub(super) next_play_step_at: Option<Instant>,
    pub(super) metrics: RunnerMetrics,
}

impl RuntimeLiveSession {
    pub(super) fn new() -> Self {
        Self {
            subscribed: HashSet::new(),
            event_filters: None,
            playing: false,
            next_play_step_at: None,
            metrics: RunnerMetrics::default(),
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

    pub(super) fn should_advance_play_step(&mut self, interval: Duration) -> bool {
        if !self.playing {
            self.next_play_step_at = None;
            return false;
        }
        let now = Instant::now();
        if let Some(next_step_at) = self.next_play_step_at {
            if now < next_step_at {
                return false;
            }
        }
        self.next_play_step_at = Some(now + interval);
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
                    "runtime live bootstrap set material balance failed material={} err={err:?}",
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
        seed_agents.push((
            "runtime-agent-0".to_string(),
            GeoPos::new(0.0, 0.0, 0.0),
            32,
            8,
        ));
        seed_agents.push((
            "runtime-agent-1".to_string(),
            GeoPos::new(0.0, 0.0, 0.0),
            32,
            8,
        ));
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
            .map_err(|err| format!("runtime live bootstrap register step failed: {err:?}"))?;
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
                    "runtime live bootstrap set electricity failed agent={} err={err:?}",
                    agent_id
                )
            })?;
        world
            .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, data)
            .map_err(|err| {
                format!(
                    "runtime live bootstrap set data failed agent={} err={err:?}",
                    agent_id
                )
            })?;
    }

    Ok((world, config))
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
        runtime_perf: Default::default(),
    }
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
    fn bootstrap_runtime_world_defaults_to_production_release_policy() {
        let (world, _) =
            bootstrap_runtime_world(WorldScenario::Minimal).expect("bootstrap runtime live world");
        assert!(world.release_security_policy().is_production_hardened());
    }
}
