use super::*;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

const SNAPSHOT_PLAYER_ID: &str = "player-snapshot";

fn read_probe_request(stream: &mut TcpStream) -> Vec<u8> {
    let started_at = Instant::now();
    let mut request = vec![0_u8; 1024];
    loop {
        match stream.read(&mut request) {
            Ok(bytes) => {
                request.truncate(bytes);
                return request;
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock
                    && started_at.elapsed() < Duration::from_millis(500) =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(err) => panic!("read request: {err}"),
        }
    }
}

fn spawn_runtime_provider_probe_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    listener
        .set_nonblocking(true)
        .expect("set test listener nonblocking");
    let bind = listener.local_addr().expect("listener addr");
    let serve = thread::spawn(move || {
        let started_at = Instant::now();
        let mut last_served_at = started_at;
        let mut served = 0_usize;
        while started_at.elapsed() < Duration::from_secs(2)
            && (served < 2 || last_served_at.elapsed() < Duration::from_millis(100))
        {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_probe_request(&mut stream);
                    let request_text = String::from_utf8_lossy(&request);
                    let body = if request_text.contains("GET /v1/provider/info") {
                        r#"{"provider_id":"provider_local_bridge","name":"Provider Local Bridge","version":"0.1.0","protocol_version":"world-simulator-provider-loopback-http-v1","chain_resource_manifest_schema_version":"oasis7.world_resource_manifest.v1","chain_resource_delta_schema_version":"oasis7.world_resource_delta.v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","wait_ticks","move_agent","speak_to_nearby","inspect_target","simple_interact"]}"#
                    } else {
                        r#"{"ok":true,"status":"ready","uptime_ms":42,"last_error":null,"queue_depth":0}"#
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    served += 1;
                    last_served_at = Instant::now();
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("accept probe connection: {err}"),
            }
        }
    });
    (format!("http://{bind}"), serve)
}

fn bind_agent_for_snapshot(server: &mut ViewerRuntimeLiveServer, agent_id: &str) {
    server
        .llm_sidecar
        .bind_agent_player(
            agent_id,
            SNAPSHOT_PLAYER_ID,
            Some("snapshot-public-key"),
            false,
        )
        .expect("bind snapshot player to agent");
}

#[test]
fn runtime_provider_compat_snapshot_exposes_agent_execution_debug_contexts() {
    let _guard = runtime_provider_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_runtime_provider_env();
    let (base_url, serve) = spawn_runtime_provider_probe_server();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let snapshot = server.compat_snapshot(None);
    let context = snapshot
        .model
        .agent_execution_debug_contexts
        .get(agent_id.as_str())
        .expect("debug context in snapshot");
    assert_eq!(
        context.provider_mode.as_deref(),
        Some("provider_loopback_http")
    );
    assert_eq!(context.compatibility_status.as_deref(), Some("ready"));
    assert_eq!(
        context.provider_check_source.as_deref(),
        Some("runtime_live_probe")
    );
    assert_eq!(context.provider_check_status.as_deref(), Some("ready"));
    assert_eq!(context.execution_mode.as_deref(), Some("player_parity"));
    assert_eq!(
        context.observation_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION)
    );
    assert_eq!(
        context.action_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION)
    );
    assert_eq!(context.environment_class.as_deref(), Some("runtime_live"));
    assert_eq!(
        context.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(
        context.supported_action_sets,
        vec![
            "wait".to_string(),
            "wait_ticks".to_string(),
            "move_agent".to_string(),
            "speak_to_nearby".to_string(),
            "inspect_target".to_string(),
            "simple_interact".to_string(),
        ]
    );
    assert_eq!(
        context.provider_reported_capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(context.provider_reported_supported_action_sets.len(), 6);
    assert_eq!(context.fallback_reason, None);
    assert_eq!(context.provider_check_fallback_reason, None);
    assert_eq!(context.provider_check_error, None);
    assert_eq!(
        context.agent_profile.as_deref(),
        Some("oasis7_p0_low_freq_npc")
    );
    clear_runtime_provider_env();
    serve.join().expect("server thread should finish");
}

#[test]
fn runtime_provider_compat_snapshot_tracks_alias_fallback_reason() {
    let _guard = runtime_provider_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_runtime_provider_env();
    let (base_url, serve) = spawn_runtime_provider_probe_server();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "agent_direct_connect");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let context = snapshot
        .model
        .agent_execution_debug_contexts
        .get(agent_id.as_str())
        .expect("debug context in snapshot");
    assert_eq!(context.compatibility_status.as_deref(), Some("degraded"));
    assert_eq!(
        context.fallback_reason.as_deref(),
        Some("provider_mode_alias:agent_direct_connect")
    );
    assert_eq!(context.provider_check_status.as_deref(), Some("ready"));
    assert_eq!(
        context.provider_check_source.as_deref(),
        Some("runtime_live_probe")
    );
    assert_eq!(context.provider_check_error, None);
    clear_runtime_provider_env();
    serve.join().expect("server thread should finish");
}

#[test]
fn compat_snapshot_exposes_player_gameplay_snapshot() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let mut server = server;
    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(gameplay.goal_id, "first_session_loop.configure_llm_access");
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Blocked
    );
    assert_eq!(gameplay.blocker_kind.as_deref(), Some("llm_required"));
    assert_eq!(
        gameplay.available_actions[0].protocol_action,
        "request_snapshot"
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "advance_step"
                && action.disabled_reason.as_deref().is_some())
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_smelter_mk1")
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "chat_first_agent")
    );
    assert!(gameplay.recent_feedback.is_none());
}

#[test]
fn compat_snapshot_exposes_micro_depot_facility_state_and_evidence() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let mut model = WorldModel::default();
    model.regional_infrastructure.insert(
        "depot-compat".to_string(),
        crate::simulator::RegionalInfrastructure {
            facility_id: "depot-compat".to_string(),
            kind: "micro_depot".to_string(),
            location_id: "loc-compat".to_string(),
            owner: crate::simulator::ResourceOwner::Agent {
                agent_id: "agent-0".to_string(),
            },
            owner_claim_id: "claim-compat".to_string(),
            regional_blocker_receipt_id: "blocker-compat".to_string(),
            status: "active".to_string(),
            module_id: "regional.micro_depot".to_string(),
            module_version: "0.2.0".to_string(),
            wasm_hash: "sha256:micro-depot-compat".to_string(),
            entrypoint: "evaluate_quote".to_string(),
            service_radius_cm: 250_000,
            supported_resource_kinds: vec!["data".to_string()],
            measured_supply_schema_version: 2,
            inventory_revision: 7,
            available_units_by_kind: [("data".to_string(), 5)].into_iter().collect(),
            throughput_limit_units_per_epoch: 16,
            throughput_epoch: 11,
            throughput_remaining_units: 13,
            upkeep_paid: true,
            last_proposal_hash: Some("sha256:proposal-compat".to_string()),
            last_receipt_id: Some("receipt-compat".to_string()),
            last_serviced_target_id: None,
        },
    );
    server.seed_model = Some(model);

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let facility = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot")
        .micro_depot_facilities
        .first()
        .expect("micro-depot facility in canonical gameplay snapshot");
    assert_eq!(facility.facility_id, "depot-compat");
    assert_eq!(facility.available_units_by_kind.get("data"), Some(&5));
    assert_eq!(facility.module_id, "regional.micro_depot");
    assert_eq!(facility.module_version, "0.2.0");
    assert_eq!(facility.wasm_hash, "sha256:micro-depot-compat");
    assert_eq!(facility.last_receipt_id.as_deref(), Some("receipt-compat"));
    assert_eq!(
        facility.last_proposal_hash.as_deref(),
        Some("sha256:proposal-compat")
    );
}

#[test]
fn compat_snapshot_requires_starter_oc_before_first_agent_chat() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");
    bind_agent_for_snapshot(&mut server, primary_agent_id.as_str());

    let snapshot = server.compat_snapshot(Some("player-snapshot"));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    let starter_oc_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == crate::viewer::ACTION_CLAIM_STARTER_OC)
        .expect("starter OC action");
    assert_eq!(starter_oc_action.protocol_action, "gameplay_action.submit");
    assert!(starter_oc_action.disabled_reason.is_none());
    let chat_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "chat_first_agent")
        .expect("first agent chat action");
    assert_eq!(
        chat_action.disabled_reason.as_deref(),
        Some("claim starter OC before using LLM/agent chat for this Agent")
    );
}

#[test]
fn compat_snapshot_does_not_publish_player_bound_actions_without_bound_agent() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let snapshot = server.compat_snapshot(Some("player-snapshot"));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert!(gameplay.agent_claim.is_none());
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == crate::viewer::ACTION_CLAIM_STARTER_OC)
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "chat_first_agent")
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_smelter_mk1")
    );
}

#[test]
fn compat_snapshot_with_unbound_starter_only_publishes_first_agent_claim_for_unbound_player() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    if !server
        .world
        .state()
        .agents
        .contains_key(crate::viewer::FIRST_AGENT_CLAIM_TARGET_AGENT_ID)
    {
        server
            .world
            .submit_action(crate::runtime::Action::RegisterAgent {
                agent_id: crate::viewer::FIRST_AGENT_CLAIM_TARGET_AGENT_ID.to_string(),
                pos: crate::viewer::gameplay_actions::formal_release_default_first_agent_spawn_pos(
                )
                .expect("formal release starter spawn"),
            });
        server.world.step().expect("register unbound starter agent");
    }

    let snapshot = server.compat_snapshot(Some("unbound-player"));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == crate::viewer::ACTION_CLAIM_FIRST_AGENT)
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == crate::viewer::ACTION_CLAIM_STARTER_OC)
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "chat_first_agent")
    );
    assert!(
        !gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_smelter_mk1")
    );
}

#[test]
fn compat_snapshot_surfaces_agent_override_causality_from_runtime_events() {
    let _guard = runtime_provider_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_runtime_provider_env();
    let (base_url, serve) = spawn_runtime_provider_probe_server();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let original_action = crate::runtime::Action::MoveAgent {
        agent_id: "agent-primary".to_string(),
        to: crate::geometry::GeoPos::new(100, 0, 0),
    };
    let override_action = crate::runtime::Action::MoveAgent {
        agent_id: "agent-primary".to_string(),
        to: crate::geometry::GeoPos::new(250, 0, 0),
    };
    let causality =
        super::super::gameplay_snapshot::player_gameplay_causality_from_runtime_events(&[
            crate::runtime::WorldEvent {
                id: 1,
                time: 1,
                caused_by: None,
                body: crate::runtime::WorldEventBody::RuleDecisionRecorded(
                    crate::runtime::RuleDecisionRecord {
                        action_id: 7,
                        module_id: "test.rule".to_string(),
                        stage: crate::runtime::ModuleSubscriptionStage::PreAction,
                        verdict: crate::runtime::RuleVerdict::Modify,
                        override_action: Some(override_action.clone()),
                        cost: crate::runtime::ResourceDelta::default(),
                        notes: vec!["reroute to safer waypoint".to_string()],
                    },
                ),
            },
            crate::runtime::WorldEvent {
                id: 2,
                time: 1,
                caused_by: None,
                body: crate::runtime::WorldEventBody::ActionOverridden(
                    crate::runtime::ActionOverrideRecord {
                        action_id: 7,
                        original_action,
                        override_action,
                    },
                ),
            },
        ])
        .expect("agent override causality");
    assert_eq!(
        causality.kind,
        crate::simulator::PlayerGameplayCausalityKind::AgentOverride
    );
    assert!(causality.detail.contains("test.rule"));
    assert!(causality.detail.contains("reroute to safer waypoint"));

    server.set_latest_player_gameplay_feedback_with_causality(
        crate::simulator::PlayerGameplayRecentFeedback {
            action: "move_agent".to_string(),
            stage: "completed_advanced".to_string(),
            effect: "world advanced: logicalTime +1, eventSeq +2".to_string(),
            intent_summary: None,
            target_agent_id: None,
            reason: None,
            hint: None,
            delta_logical_time: 1,
            delta_event_seq: 2,
        },
        Some(causality),
    );

    let snapshot = server.compat_snapshot(Some("player-snapshot"));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.execution_state,
        crate::simulator::PlayerGameplayExecutionState::Completed
    );
    assert_eq!(
        gameplay.causality_kind,
        Some(crate::simulator::PlayerGameplayCausalityKind::AgentOverride)
    );
    assert!(
        gameplay
            .causality_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("redirected the accepted action"))
    );
    assert!(
        gameplay
            .causality_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("reroute to safer waypoint"))
    );
    assert_eq!(gameplay.accepted_intent_id.as_deref(), Some("move_agent"));
    assert_eq!(
        gameplay.status_reason.as_deref(),
        gameplay.causality_detail.as_deref()
    );
    assert_eq!(
        gameplay.last_world_change.as_deref(),
        Some("world advanced: logicalTime +1, eventSeq +2")
    );
    assert_eq!(
        gameplay.resume_anchor.as_deref(),
        Some(
            "Establish your first sustainable capability (post_onboarding.establish_first_capability)"
        )
    );
    assert_eq!(
        gameplay.resume_next_step.as_deref(),
        Some(
            "Advance 2-3 more times and prioritize the first output, the first stable line, or one clear recovery signal."
        )
    );

    clear_runtime_provider_env();
    serve.join().expect("server thread should finish");
}

#[test]
fn compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "gameplay_action:build_factory_smelter_mk1".to_string(),
        stage: "accepted".to_string(),
        effect: "queued gameplay action build_factory_smelter_mk1 for agent-0 as runtime action 7"
            .to_string(),
        intent_summary: Some(
            "queue gameplay action build_factory_smelter_mk1 for agent-0".to_string(),
        ),
        target_agent_id: Some("agent-0".to_string()),
        reason: None,
        hint: Some("advance 1-2 steps to apply the queued gameplay action".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(Some("player-snapshot"));
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.accepted_intent_id.as_deref(),
        Some("gameplay_action:build_factory_smelter_mk1")
    );
    assert_eq!(
        gameplay.intent_summary.as_deref(),
        Some("queue gameplay action build_factory_smelter_mk1 for agent-0")
    );
    assert_eq!(gameplay.intent_scope.as_deref(), Some("gameplay_action"));
    assert_eq!(gameplay.intent_target.as_deref(), Some("agent-0"));
    assert_eq!(
        gameplay.status_reason.as_deref(),
        Some("advance 1-2 steps to apply the queued gameplay action")
    );
    assert_eq!(gameplay.last_world_change, None);
    assert_eq!(
        gameplay.resume_anchor.as_deref(),
        Some(
            "Create the first visible world feedback (first_session_loop.create_first_world_feedback)"
        )
    );
    assert_eq!(
        gameplay.resume_next_step.as_deref(),
        Some("Request a snapshot, advance 1 step, then inspect the new delta and events.")
    );
    assert_eq!(
        gameplay.response_window_class.as_deref(),
        Some("waiting_for_committed_progress")
    );
    assert_eq!(gameplay.stalled_reason, None);
    assert_eq!(
        gameplay.escalation_hint.as_deref(),
        Some("advance 1-2 steps to apply the queued gameplay action")
    );
    assert_eq!(gameplay.fallback_action_id.as_deref(), Some("advance_step"));
    assert_eq!(
        gameplay.fallback_action_label.as_deref(),
        Some("Advance 1 step to create the first world feedback")
    );
}

#[test]
fn compat_snapshot_exposes_player_agent_claim_overview() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");
    bind_agent_for_snapshot(&mut server, primary_agent_id.as_str());

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_agent_reputation_score(primary_agent_id.as_str(), 0)
        .expect("set reputation");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000,
            circulating_supply: 1_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance(primary_agent_id.as_str(), 1_000, 0)
        .expect("seed main token balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server.world.step().expect("register claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ClaimAgent {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ReleaseAgentClaim {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("request release");

    let anonymous_snapshot = server.compat_snapshot(None);
    let anonymous_gameplay = anonymous_snapshot
        .player_gameplay
        .as_ref()
        .expect("anonymous player gameplay snapshot");
    assert!(anonymous_gameplay.agent_claim.is_none());
    assert!(anonymous_gameplay.available_actions.iter().all(|action| {
        action.action_id != crate::viewer::ACTION_CLAIM_STARTER_OC
            && action.action_id != "chat_first_agent"
            && !action
                .target_agent_id
                .as_deref()
                .is_some_and(|target| target == primary_agent_id.as_str())
    }));

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    assert_eq!(claim.claimer_agent_id, primary_agent_id);
    assert_eq!(claim.claim_cap, 1);
    assert_eq!(claim.owned_claim_count, 1);
    assert_eq!(claim.current_epoch, snapshot.time);
    assert_eq!(claim.restricted_starter_claim_balance, 0);
    assert_eq!(claim.slot_1_eligible_claim_balance, 650);

    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.transferable_liquid_balance, 650);
    assert_eq!(quote.restricted_starter_claim_balance, 0);
    assert_eq!(
        quote.blocked_reason.as_deref(),
        Some("agent claim cap exceeded: owned=1 cap=1")
    );

    let owned = claim.owned_claims.first().expect("owned claim entry");
    assert_eq!(owned.target_agent_id, "agent-claim-target");
    assert_eq!(owned.status, "release_cooldown");
    assert_eq!(owned.claim_bond_locked_restricted_amount, 0);
    assert_eq!(owned.claim_bond_locked_liquid_amount, 200);
    assert!(owned.release_ready_in_epochs.is_some());
    assert!(owned.forced_reclaim_in_epochs.is_some());
}

#[test]
fn compat_snapshot_flags_restricted_balance_as_ineligible_for_slot_2() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");
    bind_agent_for_snapshot(&mut server, primary_agent_id.as_str());

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_agent_reputation_score(primary_agent_id.as_str(), 10)
        .expect("set reputation");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 650,
            circulating_supply: 650,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance_with_restricted(primary_agent_id.as_str(), 0, 0, 650)
        .expect("seed restricted-only claim balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-slot2-target-a".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-slot2-target-b".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server.world.step().expect("register claim targets");
    server
        .world
        .submit_action(crate::runtime::Action::ClaimAgent {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-slot2-target-a".to_string(),
        });
    server
        .world
        .step()
        .expect("claim slot 1 using restricted balance");

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.slot_index, 2);
    assert_eq!(quote.transferable_liquid_balance, 0);
    assert_eq!(quote.restricted_starter_claim_balance, 325);
    assert_eq!(quote.eligible_claim_balance, 0);
    assert_eq!(
        quote.blocked_reason.as_deref(),
        Some(
            "restricted_balance_not_eligible_for_slot slot=2 liquid=0 restricted=325 required=488"
        )
    );
}

#[test]
fn compat_snapshot_exposes_slot_1_auto_funding_from_dedicated_pool() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");
    bind_agent_for_snapshot(&mut server, primary_agent_id.as_str());

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 325,
            circulating_supply: 0,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_treasury_balance(
            crate::runtime::MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
            325,
        )
        .expect("seed dedicated pool");

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    assert_eq!(claim.claimer_agent_id, primary_agent_id);
    assert_eq!(claim.restricted_starter_claim_balance, 0);
    assert_eq!(claim.slot_1_auto_restricted_starter_claim_amount, 325);
    assert_eq!(claim.slot_1_eligible_claim_balance, 325);

    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.slot_index, 1);
    assert_eq!(quote.auto_restricted_starter_claim_amount, 325);
    assert_eq!(quote.eligible_claim_balance, 325);
    assert_eq!(quote.eligible_balance_after, 0);
    assert_eq!(quote.upkeep_runway_epochs, 0);
    assert_eq!(quote.next_upkeep_due_epoch, Some(claim.current_epoch + 1));
    assert_eq!(
        quote.projected_grace_entry_epoch,
        Some(claim.current_epoch + 1)
    );
    assert!(quote.low_runway_warning);
    assert_eq!(
        quote.recommended_claim_action.as_deref(),
        Some("wait_or_fund_first")
    );
    assert_eq!(quote.blocked_reason, None);
}

#[test]
fn compat_snapshot_promotes_to_post_onboarding_after_control_feedback() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "step".to_string(),
        stage: "completed_advanced".to_string(),
        effect: "world advanced: logicalTime +1, eventSeq +1".to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: None,
        hint: None,
        delta_logical_time: 1,
        delta_event_seq: 1,
    });
    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::PostOnboarding
    );
    assert!(gameplay.goal_id.starts_with("post_onboarding."));
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "completed_advanced"
    );
}

#[test]
fn compat_snapshot_ignores_zero_delta_completed_advanced_for_last_world_change() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "prompt_control.apply".to_string(),
        stage: "completed_advanced".to_string(),
        effect: "applied prompt-control override".to_string(),
        intent_summary: Some("apply prompt-control override".to_string()),
        target_agent_id: None,
        reason: None,
        hint: None,
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_ne!(
        gameplay.last_world_change.as_deref(),
        Some("applied prompt-control override")
    );
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "completed_advanced"
    );
}

#[test]
fn compat_snapshot_keeps_first_session_loop_for_fresh_llm_session() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Active
    );
    assert_eq!(
        gameplay.goal_id,
        "first_session_loop.create_first_world_feedback"
    );
    assert_eq!(gameplay.blocker_kind, None);
}

#[test]
fn compat_snapshot_keeps_first_session_loop_after_bootstrap_tick_blocked_feedback() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.world.step().expect("advance world once");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "play".to_string(),
        stage: "blocked".to_string(),
        effect: "gameplay blocked before requested advance completed: logicalTime +0, eventSeq +0"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("simulated retention blocker".to_string()),
        hint: Some("repair the blocker before retrying play".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Active
    );
    assert_eq!(
        gameplay.goal_id,
        "first_session_loop.create_first_world_feedback"
    );
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "blocked"
    );
    assert_eq!(gameplay.blocker_kind, None);
}

#[test]
fn compat_snapshot_blocks_first_session_when_chain_sync_is_unavailable() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "chain_sync".to_string(),
        stage: "blocked".to_string(),
        effect: "committed runtime sync failed before the viewer could observe new world state"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some(
            "execution world is not ready; missing persistence file(s): snapshot.json, journal.json"
                .to_string(),
        ),
        hint: Some(
            "wait for the execution world persistence files to appear, or restart/repair the chain runtime bootstrap before refreshing gameplay"
                .to_string(),
        ),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Blocked
    );
    assert_eq!(gameplay.goal_id, "first_session_loop.recover_runtime_sync");
    assert_eq!(
        gameplay.blocker_kind.as_deref(),
        Some("execution_world_not_ready")
    );
    assert!(
        gameplay
            .blocker_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("execution world is not ready"))
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .filter(|action| action.protocol_action != "request_snapshot")
            .all(|action| action.disabled_reason.is_some())
    );
}

#[test]
fn compat_snapshot_keeps_post_onboarding_blocked_after_confirmed_progress() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.confirmed_player_gameplay_progress_time = Some(server.world.state().time);
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "play".to_string(),
        stage: "blocked".to_string(),
        effect: "gameplay blocked before requested advance completed: logicalTime +0, eventSeq +0"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("simulated retention blocker".to_string()),
        hint: Some("repair the blocker before retrying play".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::PostOnboarding
    );
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Blocked
    );
    assert_eq!(gameplay.goal_id, "post_onboarding.recover_capability");
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "blocked"
    );
    assert_eq!(gameplay.blocker_kind.as_deref(), Some("no_progress"));
}

#[test]
fn compat_snapshot_keeps_post_onboarding_no_progress_after_confirmed_progress() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.confirmed_player_gameplay_progress_time = Some(server.world.state().time);
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "play".to_string(),
        stage: "completed_no_progress".to_string(),
        effect: "no visible world delta: logicalTime +0, eventSeq +0".to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("latest command did not create forward progress".to_string()),
        hint: Some("inspect blockers before retrying play".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::PostOnboarding
    );
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Blocked
    );
    assert_eq!(gameplay.goal_id, "post_onboarding.recover_capability");
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "completed_no_progress"
    );
    assert_eq!(gameplay.blocker_kind.as_deref(), Some("no_progress"));
    assert_eq!(
        gameplay.response_window_class.as_deref(),
        Some("stalled_needs_escalation")
    );
    assert_eq!(
        gameplay.stalled_reason.as_deref(),
        Some("latest command did not create forward progress")
    );
    assert_eq!(
        gameplay.escalation_hint.as_deref(),
        Some("inspect blockers before retrying play")
    );
    assert_eq!(
        gameplay.fallback_action_id.as_deref(),
        Some("request_snapshot")
    );
    assert_eq!(
        gameplay.fallback_action_label.as_deref(),
        Some("Refresh gameplay snapshot")
    );
}
