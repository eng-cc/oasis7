use super::*;

#[test]
fn durable_record_result_identifies_exact_replay() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    let context = AgentIntentAuthorityContext {
        intent_tick: Some(7),
        world_id: Some("runtime-world".to_string()),
        reorg_epoch: Some(2),
        authority_scope: Some("player_agent_chat".to_string()),
        replaces_intent_id: None,
    };
    let first = world
        .record_agent_chat_intent_with_authority(
            "player-replay",
            AGENT_ID,
            1,
            "Start recipe",
            context.clone(),
        )
        .expect("first durable intent");
    assert!(matches!(first, AgentIntentRecordOutcome::Accepted { .. }));

    let replay = world
        .record_agent_chat_intent_with_authority(
            "player-replay",
            AGENT_ID,
            1,
            "Start recipe",
            context,
        )
        .expect("durable replay");
    assert!(matches!(replay, AgentIntentRecordOutcome::Replayed { .. }));
    assert_eq!(first.event_seq(), replay.event_seq());
    assert_eq!(world.journal().events.len(), 3);
    let intent = world.state().agents[AGENT_ID]
        .intent
        .as_ref()
        .expect("accepted runtime intent");
    assert_eq!(
        intent.summary,
        "Agent guidance accepted; the Agent will evaluate its next world action."
    );
    assert!(!intent.summary.contains("Start recipe"));
}

#[test]
fn player_chat_intent_journals_proposed_submitted_accepted_and_replays() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    let snapshot = world.snapshot();

    let outcome = world
        .record_agent_chat_intent("player-lifecycle", AGENT_ID, 1, "Start recipe")
        .expect("record lifecycle intent");
    assert_eq!(outcome, 3);
    assert_eq!(world.journal().events.len(), 3);
    assert_eq!(
        world
            .journal()
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    let statuses = world
        .journal()
        .events
        .iter()
        .map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::AgentIntentProposed { intent })
            | WorldEventBody::Domain(DomainEvent::AgentIntentSubmitted { intent })
            | WorldEventBody::Domain(DomainEvent::AgentIntentAccepted { intent }) => {
                intent.status.as_str()
            }
            other => panic!("unexpected lifecycle event: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(statuses, vec!["proposed", "submitted", "accepted"]);

    let restored =
        World::from_snapshot(snapshot, world.journal().clone()).expect("replay lifecycle journal");
    let intent = restored.state().agents[AGENT_ID]
        .intent
        .as_ref()
        .expect("accepted lifecycle intent");
    assert_eq!(intent.status, "accepted");
    assert_eq!(intent.event_seq, 3);
    assert_eq!(restored.state().agent_intent_ledger.len(), 1);
}

#[test]
fn durable_digest_binds_intent_tick_world_reorg_and_authority_scope() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    let original_context = AgentIntentAuthorityContext {
        intent_tick: Some(7),
        world_id: Some("runtime-world".to_string()),
        reorg_epoch: Some(2),
        authority_scope: Some("player_agent_chat".to_string()),
        replaces_intent_id: None,
    };
    world
        .record_agent_chat_intent_with_authority(
            "player-digest",
            AGENT_ID,
            1,
            "Start recipe",
            original_context,
        )
        .expect("first durable intent");

    for changed_context in [
        AgentIntentAuthorityContext {
            intent_tick: Some(8),
            world_id: Some("runtime-world".to_string()),
            reorg_epoch: Some(2),
            authority_scope: Some("player_agent_chat".to_string()),
            replaces_intent_id: None,
        },
        AgentIntentAuthorityContext {
            intent_tick: Some(7),
            world_id: Some("other-world".to_string()),
            reorg_epoch: Some(2),
            authority_scope: Some("player_agent_chat".to_string()),
            replaces_intent_id: None,
        },
        AgentIntentAuthorityContext {
            intent_tick: Some(7),
            world_id: Some("runtime-world".to_string()),
            reorg_epoch: Some(3),
            authority_scope: Some("player_agent_chat".to_string()),
            replaces_intent_id: None,
        },
        AgentIntentAuthorityContext {
            intent_tick: Some(7),
            world_id: Some("runtime-world".to_string()),
            reorg_epoch: Some(2),
            authority_scope: Some("other_scope".to_string()),
            replaces_intent_id: None,
        },
    ] {
        assert!(
            world
                .record_agent_chat_intent_with_authority(
                    "player-digest",
                    AGENT_ID,
                    1,
                    "Start recipe",
                    changed_context,
                )
                .is_err(),
            "same intent identity with changed authority binding must conflict"
        );
    }
}

#[test]
fn durable_replay_returns_latest_terminal_disposition_event_seq() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    let context = AgentIntentAuthorityContext {
        intent_tick: Some(7),
        world_id: Some("runtime-world".to_string()),
        reorg_epoch: Some(2),
        authority_scope: Some("player_agent_chat".to_string()),
        replaces_intent_id: None,
    };
    let accepted = world
        .record_agent_chat_intent_with_authority(
            "player-terminal-replay",
            AGENT_ID,
            1,
            "Start recipe",
            context.clone(),
        )
        .expect("accepted intent");
    let accepted_intent = world.state().agents[AGENT_ID]
        .intent
        .clone()
        .expect("accepted runtime intent");
    let mut rejected = accepted_intent.clone();
    rejected.status = "rejected".to_string();
    rejected.summary = lifecycle_summary("rejected").to_string();
    rejected.event_seq = accepted.event_seq().saturating_add(1);
    rejected.updated_at = 8;
    rejected.reason_code = Some("policy_denied".to_string());
    rejected.reason_summary = Some("This instruction is not permitted".to_string());

    let mut terminal_state = world.state().clone();
    terminal_state
        .apply_domain_event(
            &DomainEvent::AgentIntentTransitioned {
                intent: rejected.clone(),
            },
            8,
        )
        .expect("record terminal disposition");
    let mut world = World::new_with_state(terminal_state);
    let replay = world
        .record_agent_chat_intent_with_authority(
            "player-terminal-replay",
            AGENT_ID,
            1,
            "Start recipe",
            context,
        )
        .expect("durable terminal replay");

    assert!(matches!(
        replay,
        AgentIntentRecordOutcome::Replayed { event_seq } if event_seq == rejected.event_seq
    ));
}

#[test]
fn provider_failure_disposition_is_exact_idempotent_and_replayable() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    let context = AgentIntentAuthorityContext {
        intent_tick: Some(7),
        world_id: Some("runtime-world".to_string()),
        reorg_epoch: Some(2),
        authority_scope: Some("player_agent_chat".to_string()),
        replaces_intent_id: None,
    };
    world
        .record_agent_chat_intent_with_authority(
            "player-provider-failure",
            AGENT_ID,
            1,
            "Start recipe",
            context.clone(),
        )
        .expect("accepted intent");
    let accepted = world.state().agents[AGENT_ID]
        .intent
        .clone()
        .expect("accepted runtime intent");
    let blocked = world
        .transition_agent_chat_provider_failure_exact(
            AGENT_ID,
            accepted.intent_id.as_str(),
            accepted.request_digest.as_str(),
            AgentIntentProviderFailureDisposition::Blocked,
        )
        .expect("persist provider failure disposition");
    assert_eq!(blocked.intent_id, accepted.intent_id);
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.reason_code.as_deref(), Some("provider_unavailable"));
    assert_eq!(
        blocked.reason_summary.as_deref(),
        Some("Agent service is temporarily unavailable")
    );
    assert_eq!(world.journal().events.len(), 4);

    let replay = world
        .transition_agent_chat_provider_failure_exact(
            AGENT_ID,
            accepted.intent_id.as_str(),
            accepted.request_digest.as_str(),
            AgentIntentProviderFailureDisposition::Blocked,
        )
        .expect("replayed provider failure disposition");
    assert_eq!(replay, blocked);
    assert_eq!(world.journal().events.len(), 4);

    let durable = world
        .agent_chat_intent_replay_disposition(
            "player-provider-failure",
            AGENT_ID,
            1,
            "Start recipe",
            context,
        )
        .expect("lookup durable provider disposition")
        .expect("durable disposition");
    assert_eq!(durable, blocked);
}

#[test]
fn provider_failure_identity_mismatch_has_no_state_effect() {
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let mut world = World::new_with_state(state);
    world
        .record_agent_chat_intent("player-provider-identity", AGENT_ID, 1, "Start recipe")
        .expect("accepted intent");
    let accepted = world.state().agents[AGENT_ID]
        .intent
        .clone()
        .expect("accepted runtime intent");
    let before = serde_json::to_vec(world.state()).expect("encode state before mismatch");
    assert!(
        world
            .transition_agent_chat_provider_failure_exact(
                "other-agent",
                accepted.intent_id.as_str(),
                accepted.request_digest.as_str(),
                AgentIntentProviderFailureDisposition::Rejected,
            )
            .is_err()
    );
    assert!(
        world
            .transition_agent_chat_provider_failure_exact(
                AGENT_ID,
                accepted.intent_id.as_str(),
                "wrong-request-digest",
                AgentIntentProviderFailureDisposition::Rejected,
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_vec(world.state()).expect("encode state after mismatch"),
        before
    );
    assert_eq!(world.journal().events.len(), 3);
}

#[test]
fn provider_advisory_intent_cannot_become_runtime_accepted() {
    let mut advisory = canonical_intent("intent-advisory-1", "accepted", "Start recipe", None);
    advisory["source"] = serde_json::json!("provider_advisory");
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    assert!(
        state
            .apply_domain_event(&intent_event("AgentIntentAccepted", advisory), 7)
            .is_err(),
        "provider advisory output must not enter the accepted runtime lifecycle"
    );
}
