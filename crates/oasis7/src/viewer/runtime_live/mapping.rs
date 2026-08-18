use super::ViewerLiveDecisionMode;
use super::control_plane::{RuntimeLlmSidecar, runtime_provider_settings_from_env};
use super::location_id_for_pos;
use super::power_projection::runtime_storage_power_statuses;
use super::support::{
    FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID, formal_release_default_seed_location_for_pos,
};
use crate::geometry::space_distance_cm;
use crate::runtime::{
    DomainEvent as RuntimeDomainEvent, MaterialStack as RuntimeMaterialStack,
    RejectReason as RuntimeRejectReason, WorldEvent as RuntimeWorldEvent,
    WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{
    Agent, AgentExecutionDebugContext, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION, Factory, Location,
    RejectReason as SimulatorRejectReason, ResourceOwner, WorldConfig, WorldEvent, WorldEventKind,
    WorldModel, provider_phase1_required_actions, provider_phase1_required_capabilities,
};
use std::collections::BTreeMap;

pub(super) fn runtime_state_to_simulator_model(
    state: &crate::runtime::WorldState,
    sidecar: &RuntimeLlmSidecar,
    seed_model: Option<&WorldModel>,
) -> WorldModel {
    let mut model = seed_model.cloned().unwrap_or_default();
    model.agents.clear();
    let runtime_power_statuses = runtime_storage_power_statuses(state);

    for (agent_id, cell) in &state.agents {
        let seeded_agent = seed_model.and_then(|seed| seed.agents.get(agent_id));
        let seeded_location = seeded_agent
            .and_then(|agent| model.locations.get(&agent.location_id))
            .filter(|location| location.pos == cell.state.pos)
            .cloned()
            .or_else(|| seed_location_for_runtime_agent(sidecar, agent_id, cell.state.pos));
        let location_id = seeded_location
            .as_ref()
            .map(|location| location.id.clone())
            .unwrap_or_else(|| location_id_for_pos(cell.state.pos));
        if let Some(location) = seeded_location {
            model
                .locations
                .entry(location_id.clone())
                .or_insert(location);
        } else {
            model
                .locations
                .entry(location_id.clone())
                .or_insert_with(|| {
                    Location::new(
                        location_id.clone(),
                        format!("runtime-{location_id}"),
                        cell.state.pos,
                    )
                });
        }

        let mut agent = Agent::new(agent_id.clone(), location_id, cell.state.pos);
        agent.body = cell.state.body.clone();
        agent.resources = cell.state.resources.clone();
        if let Some(power) = runtime_power_statuses.get(agent_id) {
            agent.power = power.clone();
        }
        model.agents.insert(agent_id.clone(), agent);
    }

    // Runtime factories supersede seed factories at their builder's canonical location.
    model.factories.clear();
    for factory in state.factories.values() {
        let location_id = model
            .agents
            .get(&factory.builder_agent_id)
            .map(|agent| agent.location_id.clone())
            .unwrap_or_else(|| factory.site_id.clone());
        model.factories.insert(
            factory.factory_id.clone(),
            Factory {
                id: factory.factory_id.clone(),
                owner: ResourceOwner::Agent {
                    agent_id: factory.builder_agent_id.clone(),
                },
                location_id,
                kind: factory.spec.factory_id.clone(),
            },
        );
    }

    model.agent_prompt_profiles = sidecar.prompt_profiles.clone();
    model.agent_player_bindings = sidecar.agent_player_bindings.clone();
    model.agent_player_public_key_bindings = sidecar.agent_public_key_bindings.clone();
    model.agent_execution_debug_contexts = collect_agent_execution_debug_contexts(state, sidecar);
    model.player_auth_last_nonce = sidecar.player_auth_last_nonce.clone();
    model
}

fn seed_location_for_runtime_agent(
    sidecar: &RuntimeLlmSidecar,
    agent_id: &str,
    pos: crate::geometry::GeoPos,
) -> Option<Location> {
    if let Some(location) = sidecar.seed_location_for_pos(pos) {
        return Some(location);
    }
    if agent_id != FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID {
        return None;
    }
    formal_release_default_seed_location_for_pos(pos)
}

fn collect_agent_execution_debug_contexts(
    state: &crate::runtime::WorldState,
    sidecar: &RuntimeLlmSidecar,
) -> BTreeMap<String, AgentExecutionDebugContext> {
    if !matches!(sidecar.decision_mode, ViewerLiveDecisionMode::Llm) {
        return BTreeMap::new();
    }

    let Ok(Some(settings)) = runtime_provider_settings_from_env() else {
        return BTreeMap::new();
    };

    state
        .agents
        .keys()
        .map(|agent_id| {
            let fallback_reason = settings.fallback_reason.clone();
            let provider_check_snapshot = sidecar.provider_check_snapshot().cloned();
            (
                agent_id.clone(),
                AgentExecutionDebugContext {
                    provider_mode: Some("provider_loopback_http".to_string()),
                    compatibility_status: Some(if fallback_reason.is_some() {
                        "degraded".to_string()
                    } else {
                        "ready".to_string()
                    }),
                    provider_check_source: provider_check_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.source.clone()),
                    provider_check_status: provider_check_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.status.clone()),
                    execution_mode: Some(settings.execution_mode.as_str().to_string()),
                    observation_schema_version: Some(
                        DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
                    ),
                    action_schema_version: Some(DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string()),
                    environment_class: Some("runtime_live".to_string()),
                    capabilities: provider_phase1_required_capabilities()
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect(),
                    supported_action_sets: provider_phase1_required_actions()
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect(),
                    provider_reported_capabilities: provider_check_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.capabilities.clone())
                        .unwrap_or_default(),
                    provider_reported_supported_action_sets: provider_check_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.supported_action_sets.clone())
                        .unwrap_or_default(),
                    fallback_reason,
                    provider_check_fallback_reason: provider_check_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.fallback_reason.clone()),
                    provider_check_error: provider_check_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.error.clone()),
                    provider_config_ref: Some(format!(
                        "provider://loopback-http/runtime-live/{}",
                        agent_id
                    )),
                    agent_profile: Some(settings.agent_profile.clone()),
                },
            )
        })
        .collect()
}

pub(super) fn map_runtime_event(
    runtime_event: &RuntimeWorldEvent,
    config: &WorldConfig,
    seed_model: Option<&WorldModel>,
) -> WorldEvent {
    // Runtime-live compat keeps unmapped runtime events visible to the viewer
    // while preserving the canonical runtime payload for later inspection.
    let kind = match &runtime_event.body {
        RuntimeWorldEventBody::Domain(domain) => {
            map_runtime_domain_event(domain, config, seed_model)
                .unwrap_or_else(|| runtime_fallback_event_kind(runtime_event))
        }
        _ => runtime_fallback_event_kind(runtime_event),
    };

    WorldEvent {
        id: runtime_event.id,
        time: runtime_event.time,
        kind,
        runtime_event: Some(runtime_event.clone()),
    }
}

pub(super) fn map_runtime_domain_event(
    event: &RuntimeDomainEvent,
    config: &WorldConfig,
    seed_model: Option<&WorldModel>,
) -> Option<WorldEventKind> {
    match event {
        RuntimeDomainEvent::AgentRegistered { agent_id, pos } => {
            Some(WorldEventKind::AgentRegistered {
                agent_id: agent_id.clone(),
                location_id: seed_location_id_for_agent_or_pos(seed_model, agent_id, *pos)
                    .unwrap_or_else(|| location_id_for_pos(*pos)),
                pos: *pos,
            })
        }
        RuntimeDomainEvent::AgentMoved { agent_id, from, to } => {
            let distance_cm = space_distance_cm(*from, *to);
            Some(WorldEventKind::AgentMoved {
                agent_id: agent_id.clone(),
                from: seed_location_id_for_pos(seed_model, *from)
                    .unwrap_or_else(|| location_id_for_pos(*from)),
                to: seed_location_id_for_pos(seed_model, *to)
                    .unwrap_or_else(|| location_id_for_pos(*to)),
                distance_cm,
                electricity_cost: config.movement_cost(distance_cm),
            })
        }
        RuntimeDomainEvent::ResourceTransferred {
            from_agent_id,
            to_agent_id,
            kind,
            amount,
        } => Some(WorldEventKind::ResourceTransferred {
            from: ResourceOwner::Agent {
                agent_id: from_agent_id.clone(),
            },
            to: ResourceOwner::Agent {
                agent_id: to_agent_id.clone(),
            },
            kind: *kind,
            amount: *amount,
        }),
        RuntimeDomainEvent::ActionRejected { reason, .. } => Some(WorldEventKind::ActionRejected {
            reason: runtime_reject_reason_to_simulator(reason),
        }),
        RuntimeDomainEvent::ActionAccepted {
            action_id,
            action_kind,
            actor_id,
            eta_ticks,
            notes,
        } => Some(runtime_structured_event(
            "runtime.action_accepted",
            format!(
                "action_id={action_id} action_kind={} actor_id={} eta_ticks={eta_ticks} player_feedback={}",
                fallback_non_empty(action_kind, "unknown_action"),
                fallback_non_empty(actor_id, "system"),
                action_accepted_player_feedback(notes),
            ),
        )),
        RuntimeDomainEvent::WarDeclared {
            war_id,
            objective,
            intensity,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.war_declared",
            format!(
                "war_id={} objective={} intensity={intensity}",
                fallback_non_empty(war_id, "unknown_war"),
                fallback_non_empty(objective, "unknown_objective"),
            ),
        )),
        RuntimeDomainEvent::WarConcluded {
            war_id,
            winner_alliance_id,
            summary,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.war_concluded",
            format!(
                "war_id={} winner={} summary={}",
                fallback_non_empty(war_id, "unknown_war"),
                fallback_non_empty(winner_alliance_id, "unknown_winner"),
                fallback_non_empty(summary, "none"),
            ),
        )),
        RuntimeDomainEvent::GovernanceProposalOpened {
            proposal_key,
            title,
            closes_at,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.governance_proposal_opened",
            format!(
                "proposal_key={} title={} closes_at={closes_at}",
                fallback_non_empty(proposal_key, "unknown_proposal"),
                fallback_non_empty(title, "untitled"),
            ),
        )),
        RuntimeDomainEvent::GovernanceVoteCast {
            proposal_key,
            option,
            weight,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.governance_vote_cast",
            format!(
                "proposal_key={} option={} weight={weight}",
                fallback_non_empty(proposal_key, "unknown_proposal"),
                fallback_non_empty(option, "unknown_option"),
            ),
        )),
        RuntimeDomainEvent::GovernanceProposalFinalized {
            proposal_key,
            winning_option,
            passed,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.governance_proposal_finalized",
            format!(
                "proposal_key={} winning_option={} passed={passed}",
                fallback_non_empty(proposal_key, "unknown_proposal"),
                winning_option.as_deref().unwrap_or("none"),
            ),
        )),
        RuntimeDomainEvent::CrisisSpawned {
            crisis_id,
            kind,
            severity,
            expires_at,
        } => Some(runtime_structured_event(
            "runtime.gameplay.crisis_spawned",
            format!(
                "crisis_id={} kind={} severity={severity} expires_at={expires_at}",
                fallback_non_empty(crisis_id, "unknown_crisis"),
                fallback_non_empty(kind, "unknown_kind"),
            ),
        )),
        RuntimeDomainEvent::CrisisResolved {
            crisis_id,
            strategy,
            success,
            impact,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.crisis_resolved",
            format!(
                "crisis_id={} strategy={} success={success} impact={impact}",
                fallback_non_empty(crisis_id, "unknown_crisis"),
                fallback_non_empty(strategy, "unknown_strategy"),
            ),
        )),
        RuntimeDomainEvent::CrisisTimedOut {
            crisis_id,
            penalty_impact,
        } => Some(runtime_structured_event(
            "runtime.gameplay.crisis_timed_out",
            format!(
                "crisis_id={} penalty_impact={penalty_impact}",
                fallback_non_empty(crisis_id, "unknown_crisis"),
            ),
        )),
        RuntimeDomainEvent::EconomicContractOpened {
            contract_id,
            counterparty_agent_id,
            settlement_amount,
            expires_at,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.economic_contract_opened",
            format!(
                "contract_id={} counterparty={} settlement_amount={settlement_amount} expires_at={expires_at}",
                fallback_non_empty(contract_id, "unknown_contract"),
                fallback_non_empty(counterparty_agent_id, "unknown_counterparty"),
            ),
        )),
        RuntimeDomainEvent::EconomicContractAccepted {
            contract_id,
            accepter_agent_id,
        } => Some(runtime_structured_event(
            "runtime.gameplay.economic_contract_accepted",
            format!(
                "contract_id={} accepter={}",
                fallback_non_empty(contract_id, "unknown_contract"),
                fallback_non_empty(accepter_agent_id, "unknown_accepter"),
            ),
        )),
        RuntimeDomainEvent::EconomicContractSettled {
            contract_id,
            success,
            transfer_amount,
            tax_amount,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.economic_contract_settled",
            format!(
                "contract_id={} success={success} transfer_amount={transfer_amount} tax_amount={tax_amount}",
                fallback_non_empty(contract_id, "unknown_contract"),
            ),
        )),
        RuntimeDomainEvent::EconomicContractExpired {
            contract_id,
            creator_agent_id,
            counterparty_agent_id,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.economic_contract_expired",
            format!(
                "contract_id={} creator={} counterparty={}",
                fallback_non_empty(contract_id, "unknown_contract"),
                fallback_non_empty(creator_agent_id, "unknown_creator"),
                fallback_non_empty(counterparty_agent_id, "unknown_counterparty"),
            ),
        )),
        RuntimeDomainEvent::MetaProgressGranted {
            target_agent_id,
            track,
            points,
            achievement_id,
            ..
        } => Some(runtime_structured_event(
            "runtime.gameplay.meta_progress_granted",
            format!(
                "target={} track={} points={points} achievement_id={}",
                fallback_non_empty(target_agent_id, "unknown_target"),
                fallback_non_empty(track, "unknown_track"),
                achievement_id.as_deref().unwrap_or("none"),
            ),
        )),
        RuntimeDomainEvent::FactoryBuilt {
            builder_agent_id,
            site_id,
            spec,
            ..
        } => Some(runtime_structured_event(
            "runtime.economy.factory_built",
            format!(
                "factory={} builder={} site={}",
                fallback_non_empty(&spec.factory_id, "unknown_factory"),
                fallback_non_empty(builder_agent_id, "unknown_builder"),
                fallback_non_empty(site_id, "unknown_site"),
            ),
        )),
        RuntimeDomainEvent::RecipeStarted {
            requester_agent_id,
            factory_id,
            recipe_id,
            accepted_batches,
            produce,
            ..
        } => Some(runtime_structured_event(
            "runtime.economy.recipe_started",
            format!(
                "factory={} recipe={} requester={} batches={accepted_batches} outputs={}",
                fallback_non_empty(factory_id, "unknown_factory"),
                fallback_non_empty(recipe_id, "unknown_recipe"),
                fallback_non_empty(requester_agent_id, "unknown_requester"),
                material_stack_summary(produce),
            ),
        )),
        RuntimeDomainEvent::RecipeCompleted {
            requester_agent_id,
            factory_id,
            recipe_id,
            accepted_batches,
            produce,
            ..
        } => Some(runtime_structured_event(
            "runtime.economy.recipe_completed",
            format!(
                "factory={} recipe={} requester={} batches={accepted_batches} outputs={}",
                fallback_non_empty(factory_id, "unknown_factory"),
                fallback_non_empty(recipe_id, "unknown_recipe"),
                fallback_non_empty(requester_agent_id, "unknown_requester"),
                material_stack_summary(produce),
            ),
        )),
        RuntimeDomainEvent::FactoryProductionBlocked {
            requester_agent_id,
            factory_id,
            recipe_id,
            blocker_kind,
            blocker_detail,
            ..
        } => Some(runtime_structured_event(
            "runtime.economy.factory_production_blocked",
            format!(
                "factory={} recipe={} requester={} reason={} detail={}",
                fallback_non_empty(factory_id, "unknown_factory"),
                fallback_non_empty(recipe_id, "unknown_recipe"),
                fallback_non_empty(requester_agent_id, "unknown_requester"),
                fallback_non_empty(blocker_kind, "unknown_reason"),
                fallback_non_empty(blocker_detail, "none"),
            ),
        )),
        RuntimeDomainEvent::FactoryProductionResumed {
            requester_agent_id,
            factory_id,
            recipe_id,
            previous_blocker_kind,
            previous_blocker_detail,
            ..
        } => Some(runtime_structured_event(
            "runtime.economy.factory_production_resumed",
            format!(
                "factory={} recipe={} requester={} previous_reason={} previous_detail={}",
                fallback_non_empty(factory_id, "unknown_factory"),
                fallback_non_empty(recipe_id, "unknown_recipe"),
                fallback_non_empty(requester_agent_id, "unknown_requester"),
                previous_blocker_kind.as_deref().unwrap_or("none"),
                previous_blocker_detail.as_deref().unwrap_or("none"),
            ),
        )),
        _ => None,
    }
}

fn seed_location_id_for_agent_or_pos(
    seed_model: Option<&WorldModel>,
    agent_id: &str,
    pos: crate::geometry::GeoPos,
) -> Option<String> {
    let seed = seed_model?;
    seed.agents
        .get(agent_id)
        .and_then(|agent| seed.locations.get(&agent.location_id))
        .filter(|location| location.pos == pos)
        .map(|location| location.id.clone())
        .or_else(|| seed_location_id_for_pos(Some(seed), pos))
}

fn seed_location_id_for_pos(
    seed_model: Option<&WorldModel>,
    pos: crate::geometry::GeoPos,
) -> Option<String> {
    seed_model?
        .locations
        .values()
        .find(|location| location.pos == pos)
        .map(|location| location.id.clone())
}

pub(super) fn runtime_reject_reason_to_simulator(
    reason: &RuntimeRejectReason,
) -> SimulatorRejectReason {
    match reason {
        RuntimeRejectReason::AgentAlreadyExists { agent_id } => {
            SimulatorRejectReason::AgentAlreadyExists {
                agent_id: agent_id.clone(),
            }
        }
        RuntimeRejectReason::AgentNotFound { agent_id } => SimulatorRejectReason::AgentNotFound {
            agent_id: agent_id.clone(),
        },
        RuntimeRejectReason::AgentsNotCoLocated {
            agent_id,
            other_agent_id,
        } => SimulatorRejectReason::AgentsNotCoLocated {
            agent_id: agent_id.clone(),
            other_agent_id: other_agent_id.clone(),
        },
        RuntimeRejectReason::InvalidAmount { amount } => {
            SimulatorRejectReason::InvalidAmount { amount: *amount }
        }
        RuntimeRejectReason::InsufficientResource {
            agent_id,
            kind,
            requested,
            available,
        } => SimulatorRejectReason::InsufficientResource {
            owner: ResourceOwner::Agent {
                agent_id: agent_id.clone(),
            },
            kind: *kind,
            requested: *requested,
            available: *available,
        },
        RuntimeRejectReason::FactoryNotFound { factory_id } => {
            SimulatorRejectReason::FacilityNotFound {
                facility_id: factory_id.clone(),
            }
        }
        RuntimeRejectReason::RuleDenied { notes } => SimulatorRejectReason::RuleDenied {
            notes: notes.clone(),
        },
        other => SimulatorRejectReason::RuleDenied {
            notes: vec![format!("runtime reject: {other:?}")],
        },
    }
}

fn runtime_fallback_event_kind(runtime_event: &RuntimeWorldEvent) -> WorldEventKind {
    let (kind, domain_kind) = runtime_event_kind_label(&runtime_event.body);
    WorldEventKind::RuntimeEvent { kind, domain_kind }
}

fn runtime_structured_event(kind: &str, domain_kind: String) -> WorldEventKind {
    WorldEventKind::RuntimeEvent {
        kind: kind.to_string(),
        domain_kind: Some(domain_kind),
    }
}

fn material_stack_summary(stacks: &[RuntimeMaterialStack]) -> String {
    if stacks.is_empty() {
        return "none".to_string();
    }

    stacks
        .iter()
        .map(|stack| {
            format!(
                "{}x{}",
                fallback_non_empty(&stack.kind, "unknown_material"),
                stack.amount
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn fallback_non_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    // Summary fallbacks are display/observability labels only; the preserved
    // runtime payload remains the source of truth for empty or unknown fields.
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

fn action_accepted_player_feedback(notes: &[String]) -> &'static str {
    if notes.iter().any(|note| !note.trim().is_empty()) {
        "Action accepted — processing queued."
    } else {
        "Action accepted."
    }
}

fn runtime_event_kind_label(body: &RuntimeWorldEventBody) -> (String, Option<String>) {
    let label = match body {
        RuntimeWorldEventBody::Domain(_) => "domain",
        RuntimeWorldEventBody::EffectQueued(_) => "effect_queued",
        RuntimeWorldEventBody::ReceiptAppended(_) => "receipt_appended",
        RuntimeWorldEventBody::PolicyDecisionRecorded(_) => "policy_decision_recorded",
        RuntimeWorldEventBody::RuleDecisionRecorded(_) => "rule_decision_recorded",
        RuntimeWorldEventBody::ActionOverridden(_) => "action_overridden",
        RuntimeWorldEventBody::Governance(_) => "governance",
        RuntimeWorldEventBody::ModuleEvent(_) => "module_event",
        RuntimeWorldEventBody::ModuleCallFailed(_) => "module_call_failed",
        RuntimeWorldEventBody::ModuleEmitted(_) => "module_emitted",
        RuntimeWorldEventBody::ModuleStateUpdated(_) => "module_state_updated",
        RuntimeWorldEventBody::ModuleRuntimeCharged(_) => "module_runtime_charged",
        RuntimeWorldEventBody::SnapshotCreated(_) => "snapshot_created",
        RuntimeWorldEventBody::ManifestUpdated(_) => "manifest_updated",
        RuntimeWorldEventBody::RollbackApplied(_) => "rollback_applied",
    };
    (label.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::GeoPos;
    use crate::runtime::{
        Action, FactoryModuleSpec, FactoryProductionState, FactoryState, MaterialLedgerId,
        MaterialStack, SnapshotMeta,
    };
    use crate::simulator::{ResourceKind, WorldKernel, WorldScenario};
    use crate::viewer::runtime_live::support::FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID;
    use crate::viewer::runtime_live::{ViewerRuntimeLiveServer, ViewerRuntimeLiveServerConfig};
    #[test]
    fn runtime_state_to_simulator_model_preserves_formal_release_seed_fragment_location() {
        let (world, _) = super::super::support::bootstrap_formal_release_runtime_world()
            .expect("formal release runtime world");
        let sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Script);
        let model = runtime_state_to_simulator_model(world.state(), &sidecar, None);
        let agent = model
            .agents
            .get(FORMAL_RELEASE_DEFAULT_BOOTSTRAP_AGENT_ID)
            .expect("formal release starter agent should be mapped");
        let location = model
            .locations
            .get(&agent.location_id)
            .expect("formal release starter location should be mapped");

        assert!(agent.location_id.starts_with("frag-"));
        assert!(!agent.location_id.starts_with("runtime:"));
        assert_eq!(agent.location_id, location.id);
        assert!(location.fragment_budget.is_some());
    }
    #[test]
    fn runtime_state_to_simulator_model_recomputes_seeded_location_after_agent_moves() {
        let mut world = crate::runtime::World::default();
        let from_pos = GeoPos::new(10, 0, 0);
        let to_pos = GeoPos::new(20, 0, 0);
        world.submit_action(Action::RegisterAgent {
            agent_id: "a1".to_string(),
            pos: from_pos,
        });
        world.step().expect("register runtime agent");
        world.submit_action(Action::MoveAgent {
            agent_id: "a1".to_string(),
            to: to_pos,
        });
        world.step().expect("move runtime agent");

        let mut seed_model = WorldModel::default();
        seed_model.locations.insert(
            "frag-from".to_string(),
            Location::new("frag-from", "from fragment", from_pos),
        );
        seed_model.locations.insert(
            "frag-to".to_string(),
            Location::new("frag-to", "to fragment", to_pos),
        );
        seed_model.agents.insert(
            "a1".to_string(),
            Agent::new("a1", "frag-from".to_string(), from_pos),
        );
        let sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Script)
            .with_runtime_seed_model(&seed_model);

        let model = runtime_state_to_simulator_model(world.state(), &sidecar, Some(&seed_model));
        let agent = model.agents.get("a1").expect("mapped agent");
        assert_eq!(agent.location_id, "frag-to");
        assert_eq!(agent.pos, to_pos);
    }
    #[test]
    fn runtime_state_to_simulator_model_projects_runtime_factory_for_canonical_schedule_quote() {
        let mut world = crate::runtime::World::default();
        world.submit_action(Action::RegisterAgent {
            agent_id: "builder-a".to_string(),
            pos: GeoPos::new(10, 20, 0),
        });
        world.step().expect("register runtime builder");
        world
            .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 100)
            .expect("seed electricity");
        world
            .set_agent_resource_balance("builder-a", ResourceKind::Data, 100)
            .expect("seed hardware");
        let mut state = world.state().clone();
        state.factories.insert(
            "factory.smelter.alpha".to_string(),
            FactoryState {
                factory_id: "factory.smelter.alpha".to_string(),
                site_id: "site-smelter".to_string(),
                builder_agent_id: "builder-a".to_string(),
                spec: FactoryModuleSpec {
                    factory_id: "factory.smelter.mk1".to_string(),
                    display_name: "Smelter MK1".to_string(),
                    tier: 2,
                    tags: vec!["smelter".to_string()],
                    build_cost: vec![],
                    build_time_ticks: 1,
                    base_power_draw: 20,
                    recipe_slots: 2,
                    throughput_bps: 10_000,
                    maintenance_per_tick: 1,
                },
                input_ledger: MaterialLedgerId::site("site-smelter"),
                output_ledger: MaterialLedgerId::site("site-smelter"),
                durability_ppm: 1_000_000,
                production: FactoryProductionState::default(),
                built_at: 1,
            },
        );
        let world = crate::runtime::World::new_with_state(state);
        let sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Script);
        let model = runtime_state_to_simulator_model(world.state(), &sidecar, None);
        let factory = model
            .factories
            .get("factory.smelter.alpha")
            .expect("runtime factory is projected");
        let builder = model.agents.get("builder-a").expect("mapped builder");
        assert_eq!(
            factory.owner,
            ResourceOwner::Agent {
                agent_id: "builder-a".to_string()
            }
        );
        assert_eq!(factory.location_id, builder.location_id);
        assert_eq!(factory.kind, "factory.smelter.mk1");
        WorldKernel::with_model(WorldConfig::default(), model)
            .quote_schedule_recipe(
                &ResourceOwner::Agent {
                    agent_id: "builder-a".to_string(),
                },
                "factory.smelter.alpha",
                "recipe.smelter.iron_ingot",
                1,
            )
            .expect("projected runtime factory supports the canonical quote");
    }
    #[test]
    fn map_runtime_domain_event_agent_registered_uses_runtime_location_id() {
        let event = RuntimeDomainEvent::AgentRegistered {
            agent_id: "a1".to_string(),
            pos: GeoPos::new(12, 34, 56),
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default(), None).expect("mapped event");
        match mapped {
            WorldEventKind::AgentRegistered {
                agent_id,
                location_id,
                pos,
            } => {
                assert_eq!(agent_id, "a1");
                assert_eq!(location_id, "runtime:12:34:56");
                assert_eq!(pos, GeoPos::new(12, 34, 56));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }
    #[test]
    fn map_runtime_domain_event_agent_moved_sets_distance_and_cost() {
        let config = WorldConfig::default();
        let event = RuntimeDomainEvent::AgentMoved {
            agent_id: "a1".to_string(),
            from: GeoPos::new(0, 0, 0),
            to: GeoPos::new(100_000, 0, 0),
        };
        let mapped = map_runtime_domain_event(&event, &config, None).expect("mapped event");
        match mapped {
            WorldEventKind::AgentMoved {
                distance_cm,
                electricity_cost,
                ..
            } => {
                assert_eq!(distance_cm, 100_000);
                assert_eq!(electricity_cost, config.movement_cost(distance_cm));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_domain_event_uses_generated_seed_location_ids() {
        let mut seed_model = WorldModel::default();
        let from_pos = GeoPos::new(10, 0, 0);
        let to_pos = GeoPos::new(20, 0, 0);
        seed_model.locations.insert(
            "frag-from".to_string(),
            Location::new("frag-from", "from fragment", from_pos),
        );
        seed_model.locations.insert(
            "frag-to".to_string(),
            Location::new("frag-to", "to fragment", to_pos),
        );
        seed_model.agents.insert(
            "a1".to_string(),
            Agent::new("a1", "frag-from".to_string(), from_pos),
        );

        let registered = map_runtime_domain_event(
            &RuntimeDomainEvent::AgentRegistered {
                agent_id: "a1".to_string(),
                pos: from_pos,
            },
            &WorldConfig::default(),
            Some(&seed_model),
        )
        .expect("registered event mapped");
        match registered {
            WorldEventKind::AgentRegistered { location_id, .. } => {
                assert_eq!(location_id, "frag-from");
            }
            other => panic!("unexpected registered event: {other:?}"),
        }

        let moved = map_runtime_domain_event(
            &RuntimeDomainEvent::AgentMoved {
                agent_id: "a1".to_string(),
                from: from_pos,
                to: to_pos,
            },
            &WorldConfig::default(),
            Some(&seed_model),
        )
        .expect("moved event mapped");
        match moved {
            WorldEventKind::AgentMoved { from, to, .. } => {
                assert_eq!(from, "frag-from");
                assert_eq!(to, "frag-to");
            }
            other => panic!("unexpected moved event: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_domain_event_action_accepted_emits_structured_runtime_event() {
        let event = RuntimeDomainEvent::ActionAccepted {
            action_id: 7,
            action_kind: "".to_string(),
            actor_id: "".to_string(),
            eta_ticks: 3,
            notes: vec!["raw note must not be exposed".to_string()],
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default(), None).expect("mapped event");
        match mapped {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                assert_eq!(kind, "runtime.action_accepted");
                let summary = domain_kind.expect("domain summary");
                assert!(summary.contains("action_id=7"));
                assert!(summary.contains("action_kind=unknown_action"));
                assert!(summary.contains("actor_id=system"));
                assert!(summary.contains("eta_ticks=3"));
                assert!(summary.contains("player_feedback=Action accepted — processing queued."));
                assert!(!summary.contains("raw note must not be exposed"));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_domain_event_action_accepted_empty_or_whitespace_notes_use_plain_feedback() {
        for notes in [vec![], vec![" \t\n ".to_string()]] {
            let event = RuntimeDomainEvent::ActionAccepted {
                action_id: 7,
                action_kind: "act".to_string(),
                actor_id: "actor".to_string(),
                eta_ticks: 3,
                notes,
            };
            let mapped = map_runtime_domain_event(&event, &WorldConfig::default(), None)
                .expect("mapped event");
            match mapped {
                WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                    assert_eq!(kind, "runtime.action_accepted");
                    assert_eq!(
                        domain_kind.as_deref(),
                        Some(
                            "action_id=7 action_kind=act actor_id=actor eta_ticks=3 \
                             player_feedback=Action accepted."
                        ),
                    );
                }
                other => panic!("unexpected mapped event: {other:?}"),
            }
        }
    }

    #[test]
    fn map_runtime_domain_event_factory_built_emits_structured_runtime_event() {
        let event = RuntimeDomainEvent::FactoryBuilt {
            job_id: 11,
            builder_agent_id: "builder.alpha".to_string(),
            site_id: "site.alpha".to_string(),
            spec: FactoryModuleSpec {
                factory_id: "factory.alpha".to_string(),
                display_name: "Alpha Plant".to_string(),
                tier: 1,
                tags: vec!["assembly".to_string()],
                build_cost: vec![MaterialStack::new("steel_plate", 10)],
                build_time_ticks: 4,
                base_power_draw: 8,
                recipe_slots: 1,
                throughput_bps: 10_000,
                maintenance_per_tick: 1,
            },
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default(), None).expect("mapped event");
        match mapped {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                assert_eq!(kind, "runtime.economy.factory_built");
                let summary = domain_kind.expect("domain summary");
                assert!(summary.contains("factory=factory.alpha"));
                assert!(summary.contains("builder=builder.alpha"));
                assert!(summary.contains("site=site.alpha"));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_domain_event_recipe_started_and_completed_emit_structured_runtime_events() {
        let started = RuntimeDomainEvent::RecipeStarted {
            job_id: 21,
            requester_agent_id: "agent.alpha".to_string(),
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.motor".to_string(),
            accepted_batches: 2,
            consume: vec![MaterialStack::new("iron_ingot", 4)],
            produce: vec![MaterialStack::new("motor_mk1", 2)],
            byproducts: Vec::new(),
            power_required: 12,
            duration_ticks: 3,
            consume_ledger: MaterialLedgerId::world(),
            output_ledger: MaterialLedgerId::world(),
            bottleneck_tags: Vec::new(),
            market_quotes: Vec::new(),
            logistics_route_ids: Vec::new(),
            logistics_path_ids: Vec::new(),
            ready_at: 99,
        };
        let completed = RuntimeDomainEvent::RecipeCompleted {
            job_id: 21,
            requester_agent_id: "agent.alpha".to_string(),
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.motor".to_string(),
            accepted_batches: 2,
            produce: vec![MaterialStack::new("motor_mk1", 2)],
            byproducts: Vec::new(),
            output_ledger: MaterialLedgerId::world(),
            bottleneck_tags: Vec::new(),
            logistics_route_ids: Vec::new(),
            logistics_path_ids: Vec::new(),
        };

        for (event, expected_kind) in [
            (started, "runtime.economy.recipe_started"),
            (completed, "runtime.economy.recipe_completed"),
        ] {
            let mapped = map_runtime_domain_event(&event, &WorldConfig::default(), None)
                .expect("mapped event");
            match mapped {
                WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                    assert_eq!(kind, expected_kind);
                    let summary = domain_kind.expect("domain summary");
                    assert!(summary.contains("factory=factory.alpha"));
                    assert!(summary.contains("recipe=recipe.motor"));
                    assert!(summary.contains("requester=agent.alpha"));
                    assert!(summary.contains("outputs=motor_mk1x2"));
                }
                other => panic!("unexpected mapped event: {other:?}"),
            }
        }
    }

    #[test]
    fn map_runtime_domain_event_factory_blocked_and_resumed_emit_structured_runtime_events() {
        let blocked = RuntimeDomainEvent::FactoryProductionBlocked {
            action_id: 31,
            requester_agent_id: "agent.alpha".to_string(),
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.motor".to_string(),
            blocker_kind: "material_shortage".to_string(),
            blocker_detail: "material_shortage:iron_ingot".to_string(),
        };
        let resumed = RuntimeDomainEvent::FactoryProductionResumed {
            job_id: 32,
            requester_agent_id: "agent.alpha".to_string(),
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.motor".to_string(),
            previous_blocked_at: Some(88),
            previous_blocker_kind: Some("material_shortage".to_string()),
            previous_blocker_detail: Some("material_shortage:iron_ingot".to_string()),
        };

        for (event, expected_kind, expected_fragment) in [
            (
                blocked,
                "runtime.economy.factory_production_blocked",
                "reason=material_shortage",
            ),
            (
                resumed,
                "runtime.economy.factory_production_resumed",
                "previous_reason=material_shortage",
            ),
        ] {
            let mapped = map_runtime_domain_event(&event, &WorldConfig::default(), None)
                .expect("mapped event");
            match mapped {
                WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                    assert_eq!(kind, expected_kind);
                    let summary = domain_kind.expect("domain summary");
                    assert!(summary.contains("factory=factory.alpha"));
                    assert!(summary.contains("recipe=recipe.motor"));
                    assert!(summary.contains(expected_fragment));
                }
                other => panic!("unexpected mapped event: {other:?}"),
            }
        }
    }

    #[test]
    fn map_runtime_domain_event_governance_finalize_keeps_compat_fallbacks() {
        let event = RuntimeDomainEvent::GovernanceProposalFinalized {
            proposal_key: "proposal.alpha".to_string(),
            winning_option: None,
            winning_weight: 0,
            total_weight: 0,
            passed: false,
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default(), None).expect("mapped event");
        match mapped {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                assert_eq!(kind, "runtime.gameplay.governance_proposal_finalized");
                let summary = domain_kind.expect("domain summary");
                assert!(summary.contains("proposal_key=proposal.alpha"));
                assert!(summary.contains("winning_option=none"));
                assert!(summary.contains("passed=false"));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }

    #[test]
    fn runtime_reject_reason_maps_agent_not_found() {
        let reason = RuntimeRejectReason::AgentNotFound {
            agent_id: "ghost".to_string(),
        };
        let mapped = runtime_reject_reason_to_simulator(&reason);
        match mapped {
            SimulatorRejectReason::AgentNotFound { agent_id } => {
                assert_eq!(agent_id, "ghost");
            }
            other => panic!("unexpected reject mapping: {other:?}"),
        }
    }

    #[test]
    fn runtime_reject_reason_unmapped_falls_back_to_rule_denied() {
        let reason = RuntimeRejectReason::InsufficientMaterial {
            material_kind: "iron".to_string(),
            requested: 10,
            available: 0,
        };
        let mapped = runtime_reject_reason_to_simulator(&reason);
        match mapped {
            SimulatorRejectReason::RuleDenied { notes } => {
                assert_eq!(notes.len(), 1);
                assert!(notes[0].contains("runtime reject"));
            }
            other => panic!("unexpected reject mapping: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_event_fallback_includes_runtime_payload() {
        let event = RuntimeWorldEvent {
            id: 9,
            time: 42,
            caused_by: None,
            body: RuntimeWorldEventBody::SnapshotCreated(SnapshotMeta { journal_len: 1 }),
        };
        let mapped = map_runtime_event(&event, &WorldConfig::default(), None);
        assert!(matches!(mapped.kind, WorldEventKind::RuntimeEvent { .. }));
        assert!(mapped.runtime_event.is_some());
        assert_eq!(mapped.id, 9);
        assert_eq!(mapped.time, 42);
    }

    #[test]
    fn map_runtime_event_unmapped_domain_event_keeps_payload_with_domain_label() {
        let event = RuntimeWorldEvent {
            id: 10,
            time: 43,
            caused_by: None,
            body: RuntimeWorldEventBody::Domain(RuntimeDomainEvent::DataCollected {
                collector_agent_id: "agent.alpha".to_string(),
                electricity_cost: 2,
                data_amount: 5,
            }),
        };
        let mapped = map_runtime_event(&event, &WorldConfig::default(), None);
        match mapped.kind {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                assert_eq!(kind, "domain");
                assert_eq!(domain_kind, None);
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
        assert_eq!(mapped.runtime_event, Some(event));
    }

    #[test]
    fn fallback_non_empty_trims_values_and_labels_empty_summaries() {
        assert_eq!(
            fallback_non_empty("  agent.alpha  ", "system"),
            "agent.alpha"
        );
        assert_eq!(fallback_non_empty("  ", "unknown_action"), "unknown_action");
    }

    #[test]
    fn runtime_live_snapshot_includes_runtime_snapshot_payload() {
        let mut server = ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(
            WorldScenario::Minimal,
        ))
        .expect("runtime server");
        let snapshot = server.compat_snapshot(None);
        assert!(snapshot.runtime_snapshot.is_some());
        assert_eq!(
            snapshot.runtime_snapshot.as_ref().unwrap().journal_len,
            server.world.snapshot().journal_len
        );
    }
}
