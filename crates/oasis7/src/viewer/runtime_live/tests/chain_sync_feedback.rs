use super::*;
use crate::runtime::{MainTokenConfig, ReleaseSecurityPolicy};
use crate::viewer::runtime_live::chain_link::load_chain_execution_world;

#[test]
fn chain_linked_runtime_missing_persistence_keeps_world_and_height() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_missing_persistence");
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

    let err = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect_err("chain sync should retry when persistence files are missing");

    match err {
        ViewerRuntimeLiveServerError::Serde(message) => {
            assert!(message.contains("execution world is not ready"));
            assert!(message.contains("snapshot.json"));
            assert!(message.contains("journal.json"));
        }
        other => panic!("unexpected chain sync error: {other:?}"),
    }
    assert_eq!(server.world.state().time, initial_time);
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("chain sync failure should be reflected in gameplay feedback");
    assert_eq!(feedback.action, "chain_sync");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("execution world is not ready")));
}

#[test]
fn chain_linked_runtime_missing_persistence_without_subscription_does_not_poison_feedback() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_missing_persistence_unsubscribed");
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
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let err = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect_err("chain sync should still fail when persistence files are missing");

    match err {
        ViewerRuntimeLiveServerError::Serde(message) => {
            assert!(message.contains("execution world is not ready"));
        }
        other => panic!("unexpected chain sync error: {other:?}"),
    }
    assert_eq!(server.world.state().time, initial_time);
    assert!(server.latest_player_gameplay_feedback.is_none());
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
}

#[test]
fn chain_linked_runtime_dev_local_policy_normalizes_main_token_config_before_verifying_tick_consensus(
) {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_dev_local_main_token_normalize");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let world = load_chain_execution_world(
        execution_world_dir.as_path(),
        ReleaseSecurityPolicy::default(),
    )
    .expect("dev-local viewer load should normalize persisted execution world");

    assert_eq!(world.main_token_config(), &MainTokenConfig::default());
    world
        .verify_tick_consensus_chain()
        .expect("viewer world should retain a valid tick consensus chain");
}
