use std::collections::BTreeMap;

use crate::geometry::space_distance_cm;
use crate::runtime::{
    DomainEvent as RuntimeDomainEvent, MaterialStack as RuntimeMaterialStack,
    RejectReason as RuntimeRejectReason, WorldEvent as RuntimeWorldEvent,
    WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{
    provider_phase1_required_actions, provider_phase1_required_capabilities, Agent,
    AgentExecutionDebugContext, Location, RejectReason as SimulatorRejectReason, ResourceOwner,
    WorldConfig, WorldEvent, WorldEventKind, WorldModel, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};

use super::control_plane::{runtime_provider_settings_from_env, RuntimeLlmSidecar};
use super::location_id_for_pos;
use super::ViewerLiveDecisionMode;

pub(super) fn runtime_state_to_simulator_model(
    state: &crate::runtime::WorldState,
    sidecar: &RuntimeLlmSidecar,
) -> WorldModel {
    let mut model = WorldModel::default();

    for (agent_id, cell) in &state.agents {
        let location_id = location_id_for_pos(cell.state.pos);
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

        let mut agent = Agent::new(agent_id.clone(), location_id, cell.state.pos);
        agent.body = cell.state.body.clone();
        agent.resources = cell.state.resources.clone();
        model.agents.insert(agent_id.clone(), agent);
    }

    model.agent_prompt_profiles = sidecar.prompt_profiles.clone();
    model.agent_player_bindings = sidecar.agent_player_bindings.clone();
    model.agent_player_public_key_bindings = sidecar.agent_public_key_bindings.clone();
    model.agent_execution_debug_contexts = collect_agent_execution_debug_contexts(state, sidecar);
    model.player_auth_last_nonce = sidecar.player_auth_last_nonce.clone();
    model
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
            (
                agent_id.clone(),
                AgentExecutionDebugContext {
                    provider_mode: Some("provider_loopback_http".to_string()),
                    compatibility_status: Some(if fallback_reason.is_some() {
                        "degraded".to_string()
                    } else {
                        "ready".to_string()
                    }),
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
                    fallback_reason,
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
) -> WorldEvent {
    let kind = match &runtime_event.body {
        RuntimeWorldEventBody::Domain(domain) => map_runtime_domain_event(domain, config)
            .unwrap_or_else(|| runtime_fallback_event_kind(runtime_event)),
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
) -> Option<WorldEventKind> {
    match event {
        RuntimeDomainEvent::AgentRegistered { agent_id, pos } => {
            Some(WorldEventKind::AgentRegistered {
                agent_id: agent_id.clone(),
                location_id: location_id_for_pos(*pos),
                pos: *pos,
            })
        }
        RuntimeDomainEvent::AgentMoved { agent_id, from, to } => {
            let distance_cm = space_distance_cm(*from, *to);
            Some(WorldEventKind::AgentMoved {
                agent_id: agent_id.clone(),
                from: location_id_for_pos(*from),
                to: location_id_for_pos(*to),
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
            ..
        } => Some(runtime_structured_event(
            "runtime.action_accepted",
            format!(
                "action_id={action_id} action_kind={} actor_id={} eta_ticks={eta_ticks}",
                fallback_non_empty(action_kind, "unknown_action"),
                fallback_non_empty(actor_id, "system"),
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
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
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
    use crate::runtime::{FactoryModuleSpec, MaterialLedgerId, MaterialStack, SnapshotMeta};
    use crate::simulator::WorldScenario;
    use crate::viewer::runtime_live::{ViewerRuntimeLiveServer, ViewerRuntimeLiveServerConfig};

    #[test]
    fn map_runtime_domain_event_agent_registered_uses_runtime_location_id() {
        let event = RuntimeDomainEvent::AgentRegistered {
            agent_id: "a1".to_string(),
            pos: GeoPos::new(12.0, 34.0, 56.0),
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
        match mapped {
            WorldEventKind::AgentRegistered {
                agent_id,
                location_id,
                pos,
            } => {
                assert_eq!(agent_id, "a1");
                assert_eq!(location_id, "runtime:12:34:56");
                assert_eq!(pos, GeoPos::new(12.0, 34.0, 56.0));
            }
            other => panic!("unexpected mapped event: {other:?}"),
        }
    }

    #[test]
    fn map_runtime_domain_event_agent_moved_sets_distance_and_cost() {
        let config = WorldConfig::default();
        let event = RuntimeDomainEvent::AgentMoved {
            agent_id: "a1".to_string(),
            from: GeoPos::new(0.0, 0.0, 0.0),
            to: GeoPos::new(100_000.0, 0.0, 0.0),
        };
        let mapped = map_runtime_domain_event(&event, &config).expect("mapped event");
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
    fn map_runtime_domain_event_action_accepted_emits_structured_runtime_event() {
        let event = RuntimeDomainEvent::ActionAccepted {
            action_id: 7,
            action_kind: "".to_string(),
            actor_id: "".to_string(),
            eta_ticks: 3,
            notes: vec!["accepted".to_string()],
        };
        let mapped =
            map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
        match mapped {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                assert_eq!(kind, "runtime.action_accepted");
                let summary = domain_kind.expect("domain summary");
                assert!(summary.contains("action_id=7"));
                assert!(summary.contains("action_kind=unknown_action"));
                assert!(summary.contains("actor_id=system"));
                assert!(summary.contains("eta_ticks=3"));
            }
            other => panic!("unexpected mapped event: {other:?}"),
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
            map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
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
        };

        for (event, expected_kind) in [
            (started, "runtime.economy.recipe_started"),
            (completed, "runtime.economy.recipe_completed"),
        ] {
            let mapped =
                map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
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
            let mapped =
                map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
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
            map_runtime_domain_event(&event, &WorldConfig::default()).expect("mapped event");
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
        let mapped = map_runtime_event(&event, &WorldConfig::default());
        assert!(matches!(mapped.kind, WorldEventKind::RuntimeEvent { .. }));
        assert!(mapped.runtime_event.is_some());
        assert_eq!(mapped.id, 9);
        assert_eq!(mapped.time, 42);
    }

    #[test]
    fn runtime_live_snapshot_includes_runtime_snapshot_payload() {
        let mut server = ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(
            WorldScenario::Minimal,
        ))
        .expect("runtime server");
        let snapshot = server.compat_snapshot();
        assert!(snapshot.runtime_snapshot.is_some());
        assert_eq!(
            snapshot.runtime_snapshot.as_ref().unwrap().journal_len,
            server.world.snapshot().journal_len
        );
    }
}
