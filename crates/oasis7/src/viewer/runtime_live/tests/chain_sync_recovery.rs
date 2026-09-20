use super::*;

#[test]
fn chain_linked_runtime_empty_poll_does_not_advance_world() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_idle");
    let execution_world = crate::runtime::World::new_production_hardened();
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist empty execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(0, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "chain_sync".to_string(),
        stage: "blocked".to_string(),
        effect: "committed runtime sync failed before the viewer could observe new world state"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("simulated missing persistence".to_string()),
        hint: Some("wait for execution world persistence".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(!progressed, "idle chain poll should not report progress");
    assert_eq!(server.world.state().time, initial_time);
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(
        server.latest_player_gameplay_feedback.is_none(),
        "successful zero-delta chain sync should clear stale chain_sync feedback"
    );
}

#[test]
fn chain_linked_runtime_zero_delta_does_not_accept_committed_height() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_zero_delta_height");
    let execution_world = crate::runtime::World::new_production_hardened();
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist empty execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(
        !progressed,
        "zero-delta chain poll should not report progress"
    );
    assert_eq!(server.world.state().time, initial_time);
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
}

#[test]
fn chain_linked_runtime_committed_height_zero_consumes_persisted_execution_world() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_zero_committed_height");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start_with_release_security_policy(
        execution_world_dir,
        ReleaseSecurityPolicy::default(),
    );
    chain_status.committed_height.store(0, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "chain_sync".to_string(),
        stage: "blocked".to_string(),
        effect: "stale bootstrap execution world should be ignored before the first commit"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("bootstrap-only".to_string()),
        hint: Some("wait for first committed height".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should consume persisted zero-height execution world");

    assert!(progressed);
    assert_eq!(server.world.state().time, execution_world.state().time);
    assert_ne!(server.world.state().time, initial_time);
    assert_eq!(
        server.last_chain_committed_height,
        execution_world.state().time.max(1)
    );
    assert!(server.latest_player_gameplay_feedback.is_none());
    let line = read_response_line(&peer, Duration::from_millis(200))
        .expect("expected zero-height execution-world sync response");
    assert!(!line.trim().is_empty());
}

#[test]
fn chain_linked_runtime_recipe_completion_is_delivered_once_across_replay() {
    let _guard = lock_test_llm_env();
    let mut source = super::super::setup_industrial_gameplay_with_completed_jobs(82, 1);
    let execution_world_dir = runtime_live_temp_dir("chain_sync_recipe_completion");
    source
        .world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist completed recipe execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
    chain_status.committed_height.store(1, Ordering::SeqCst);
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone()),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    let (mut writer, peer) = test_writer_pair();

    assert!(
        server
            .sync_chain_linked_runtime(&mut session, &mut writer)
            .expect("initial recipe completion chain sync")
    );
    let first_responses = read_available_runtime_live_responses(&peer, Duration::from_millis(200));
    assert!(first_responses.iter().any(|response| matches!(
        response,
        ViewerResponse::Event { event }
            if matches!(
                &event.kind,
                WorldEventKind::RuntimeEvent { kind, .. }
                    if kind == "runtime.economy.recipe_completed"
            )
    )));

    source.world.submit_action(RuntimeAction::MoveAgent {
        agent_id: "starter-agent-0".to_string(),
        to: crate::geometry::GeoPos::new(2, 1, 0),
    });
    source
        .world
        .step()
        .expect("append replay-following runtime event");
    source
        .world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist replay-following execution world");
    chain_status.committed_height.store(2, Ordering::SeqCst);

    let (mut replay_writer, replay_peer) = test_writer_pair();
    assert!(
        server
            .sync_chain_linked_runtime(&mut session, &mut replay_writer)
            .expect("replay-following recipe completion chain sync")
    );
    let replay_responses =
        read_available_runtime_live_responses(&replay_peer, Duration::from_millis(200));
    assert!(!replay_responses.iter().any(|response| matches!(
        response,
        ViewerResponse::Event { event }
            if matches!(
                &event.kind,
                WorldEventKind::RuntimeEvent { kind, .. }
                    if kind == "runtime.economy.recipe_completed"
            )
    )));
}

#[test]
fn chain_linked_runtime_event_suffix_delivers_new_era_recipe_completion() {
    let recipe_completion = crate::runtime::WorldEvent {
        id: u64::MAX,
        time: 83,
        caused_by: None,
        body: crate::runtime::WorldEventBody::Domain(
            crate::runtime::DomainEvent::RecipeCompleted {
                job_id: 7,
                requester_agent_id: "agent-0".to_string(),
                factory_id: "factory-smelter".to_string(),
                recipe_id: "recipe.iron-ingot".to_string(),
                accepted_batches: 1,
                produce: Vec::new(),
                byproducts: Vec::new(),
                output_ledger: crate::runtime::MaterialLedgerId::world(),
                bottleneck_tags: Vec::new(),
                logistics_route_ids: Vec::new(),
                logistics_path_ids: Vec::new(),
            },
        ),
    };
    let mut rollover_recipe_completion = recipe_completion.clone();
    rollover_recipe_completion.id = 1;
    let baseline_events = vec![recipe_completion.clone()];
    let prepared_events = vec![recipe_completion, rollover_recipe_completion];

    let selected = super::super::super::chain_link::runtime_events_after_baseline(
        baseline_events.as_slice(),
        prepared_events.as_slice(),
        0,
        1,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, 1);
    assert!(matches!(
        &selected[0].body,
        crate::runtime::WorldEventBody::Domain(crate::runtime::DomainEvent::RecipeCompleted { .. })
    ));

    let compacted_selected = super::super::super::chain_link::runtime_events_after_baseline(
        baseline_events.as_slice(),
        std::slice::from_ref(&selected[0]),
        0,
        1,
    );
    assert_eq!(compacted_selected.len(), 1);
    assert_eq!(compacted_selected[0].id, 1);
}

#[test]
fn chain_linked_runtime_revalidates_initial_snapshot_after_previous_session() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_second_session_revalidation");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world
        .step()
        .expect("advance initial execution world");
    execution_world
        .save_to_dir_with_chain_resource_context(
            execution_world_dir.as_path(),
            crate::runtime::ChainResourceDerivationContext {
                world_id: "testnet-world",
                chain_id: "testnet-chain",
                genesis_ref: Some("testnet-genesis"),
                created_at_height: 1,
                manifest_height: 1,
                commit_block_hash: Some("testnet-block-1"),
                tick: execution_world.state().time,
            },
            "testnet-world-config",
            "testnet-generation-algorithm",
        )
        .expect("persist initial execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut first_session = RuntimeLiveSession::new();
    let (mut first_writer, first_peer) = test_writer_pair();

    server
        .handle_request(
            ViewerRequest::RequestSnapshot,
            &mut first_session,
            &mut first_writer,
        )
        .expect("first session snapshot");
    first_writer.flush().expect("flush first session snapshot");
    let _ = read_available_runtime_live_responses(&first_peer, Duration::from_millis(200));
    assert_eq!(chain_status.status_requests(), 1);
    let first_snapshot_time = server.world.state().time;

    let mut updated_execution_world =
        crate::runtime::World::load_from_dir(execution_world_dir.as_path())
            .expect("reload initial execution world");
    updated_execution_world
        .step()
        .expect("advance execution world after first session");
    updated_execution_world
        .save_to_dir_with_chain_resource_context(
            execution_world_dir.as_path(),
            crate::runtime::ChainResourceDerivationContext {
                world_id: "testnet-world",
                chain_id: "testnet-chain",
                genesis_ref: Some("testnet-genesis"),
                created_at_height: 1,
                manifest_height: 2,
                commit_block_hash: Some("testnet-block-2"),
                tick: updated_execution_world.state().time,
            },
            "testnet-world-config",
            "testnet-generation-algorithm",
        )
        .expect("persist updated execution world");
    chain_status.committed_height.store(2, Ordering::SeqCst);

    let mut second_session = RuntimeLiveSession::new();
    let (mut second_writer, second_peer) = test_writer_pair();
    server
        .handle_request(
            ViewerRequest::RequestSnapshot,
            &mut second_session,
            &mut second_writer,
        )
        .expect("second session snapshot");
    second_writer
        .flush()
        .expect("flush second session snapshot");

    assert_eq!(
        chain_status.status_requests(),
        2,
        "a later session must revalidate the chain before its first snapshot"
    );
    assert!(
        server.world.state().time > first_snapshot_time,
        "second session must not receive the prior session's stale projection"
    );
    let responses = read_available_runtime_live_responses(&second_peer, Duration::from_millis(200));
    let snapshot = responses
        .iter()
        .find_map(|response| match response {
            ViewerResponse::Snapshot { snapshot } => Some(snapshot),
            _ => None,
        })
        .expect("second session snapshot response");
    assert_eq!(snapshot.time, server.world.state().time);
}
