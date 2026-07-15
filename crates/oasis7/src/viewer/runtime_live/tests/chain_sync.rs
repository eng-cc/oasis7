use super::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub(crate) struct TestChainStatusServer {
    pub(crate) addr: String,
    pub(crate) committed_height: Arc<AtomicU64>,
    submitted_gameplay_requests: Arc<Mutex<Vec<crate::viewer::GameplayActionRequest>>>,
    stop: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl TestChainStatusServer {
    pub(crate) fn start(execution_world_dir: std::path::PathBuf) -> Self {
        Self::start_with_release_security_policy(
            execution_world_dir,
            ReleaseSecurityPolicy::production_hardened(),
        )
    }

    pub(crate) fn start_with_release_security_policy(
        execution_world_dir: std::path::PathBuf,
        release_security_policy: ReleaseSecurityPolicy,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind chain status server");
        listener
            .set_nonblocking(true)
            .expect("set chain status listener nonblocking");
        let addr = listener.local_addr().expect("chain status local addr");
        let committed_height = Arc::new(AtomicU64::new(0));
        let submitted_gameplay_requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let committed_height_for_thread = Arc::clone(&committed_height);
        let submitted_requests_for_thread = Arc::clone(&submitted_gameplay_requests);
        let next_gameplay_action_id_for_thread = Arc::new(AtomicU64::new(1));
        let stop_for_thread = Arc::clone(&stop);
        let execution_world_dir_for_thread = execution_world_dir.clone();
        let release_security_policy_for_thread = release_security_policy.clone();
        let join_handle = thread::spawn(move || {
            loop {
                if stop_for_thread.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if stop_for_thread.load(Ordering::SeqCst) {
                            break;
                        }
                        let request = read_test_http_request(&mut stream);
                        let request_bytes = request.as_slice();
                        let request_text = String::from_utf8_lossy(request_bytes);
                        let mut parts = request_text
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .split_whitespace();
                        let method = parts.next().unwrap_or_default();
                        let path = parts
                            .next()
                            .unwrap_or_default()
                            .split('?')
                            .next()
                            .unwrap_or_default();

                        match (method, path) {
                            ("GET", "/v1/chain/status") => {
                                let body = serde_json::json!({
                                    "consensus": {
                                        "committed_height": committed_height_for_thread.load(Ordering::SeqCst),
                                    },
                                    "execution_world_dir": execution_world_dir_for_thread,
                                    "release_security_policy": release_security_policy_for_thread,
                                });
                                let body =
                                    serde_json::to_vec(&body).expect("encode chain status body");
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                stream
                                    .write_all(response.as_bytes())
                                    .expect("write chain status header");
                                stream
                                    .write_all(body.as_slice())
                                    .expect("write chain status body");
                                stream.flush().expect("flush chain status response");
                            }
                            ("POST", "/v1/chain/gameplay/submit") => {
                                let boundary = request_bytes
                                    .windows(4)
                                    .position(|window| window == b"\r\n\r\n")
                                    .expect("gameplay submit body boundary");
                                let body = &request_bytes[(boundary + 4)..];
                                let gameplay_request = serde_json::from_slice::<
                                    crate::viewer::GameplayActionRequest,
                                >(body)
                                .expect("decode gameplay submit request");
                                submitted_requests_for_thread
                                    .lock()
                                    .expect("lock submitted requests")
                                    .push(gameplay_request);
                                let action_id = next_gameplay_action_id_for_thread
                                    .fetch_add(1, Ordering::SeqCst);
                                let body = serde_json::json!({
                                    "ok": true,
                                    "action_id": action_id,
                                    "submitted_at_unix_ms": test_now_unix_ms(),
                                });
                                let body =
                                    serde_json::to_vec(&body).expect("encode gameplay submit body");
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                stream
                                    .write_all(response.as_bytes())
                                    .expect("write gameplay submit header");
                                stream
                                    .write_all(body.as_slice())
                                    .expect("write gameplay submit body");
                                stream.flush().expect("flush gameplay submit response");
                            }
                            _ => {
                                stream
                                .write_all(
                                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 21\r\nConnection: close\r\n\r\n{\"error\":\"not found\"}",
                                )
                                .expect("write 404 response");
                                stream.flush().expect("flush 404 response");
                            }
                        }
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        panic!("accept chain status connection failed: {err}");
                    }
                }
            }
        });
        Self {
            addr: addr.to_string(),
            committed_height,
            submitted_gameplay_requests,
            stop,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn submitted_gameplay_requests(&self) -> Vec<crate::viewer::GameplayActionRequest> {
        self.submitted_gameplay_requests
            .lock()
            .expect("lock submitted requests")
            .clone()
    }
}

impl Drop for TestChainStatusServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr.as_str());
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().expect("join chain status server");
        }
    }
}

#[test]
fn chain_linked_formal_default_starts_without_local_fallback_agent() {
    let server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::formal_release_default()
            .with_chain_status_bind("127.0.0.1:1")
            .with_chain_link_policy(ChainLinkPolicy::Enforcing),
    )
    .expect("runtime server");

    assert!(
        server.world.state().agents.is_empty(),
        "chain-linked default entry should wait for committed runtime or player claim"
    );
}

#[test]
fn chain_linked_runtime_sync_advances_without_play() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_progress");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world.submit_action(RuntimeAction::MoveAgent {
        agent_id: "chain-agent".to_string(),
        to: crate::geometry::GeoPos::new(5, 2, 0),
    });
    execution_world
        .step()
        .expect("advance execution world again");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
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
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(progressed, "chain-linked sync should report progress");
    assert_eq!(server.world.state().time, execution_world.state().time);
    let line =
        read_response_line(&peer, Duration::from_millis(200)).expect("expected sync response");
    assert!(!line.trim().is_empty());
}

#[test]
fn chain_linked_runtime_primes_initial_snapshot() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_initial_snapshot");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
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
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(ViewerRequest::RequestSnapshot, &mut session, &mut writer)
        .expect("request snapshot");
    writer.flush().expect("flush snapshot");

    assert_eq!(server.world.state().time, execution_world.state().time);
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(200));
    let snapshot = responses
        .iter()
        .find_map(|response| match response {
            ViewerResponse::Snapshot { snapshot } => Some(snapshot),
            _ => None,
        })
        .expect("initial snapshot response");
    assert_eq!(snapshot.time, execution_world.state().time);
    assert_eq!(snapshot.chain_resource_manifest.world_id, "testnet-world");
    assert_eq!(snapshot.chain_resource_manifest.chain_id, "testnet-chain");
    assert_eq!(
        snapshot.latest_chain_resource_delta.world_id,
        "testnet-world"
    );
    assert_eq!(
        snapshot.latest_chain_resource_delta.chain_id,
        "testnet-chain"
    );
    let runtime_snapshot = snapshot
        .runtime_snapshot
        .as_ref()
        .expect("runtime snapshot should be embedded");
    assert_eq!(
        runtime_snapshot.chain_resource_manifest.world_id,
        snapshot.chain_resource_manifest.world_id
    );
    assert_eq!(
        runtime_snapshot.chain_resource_manifest.chain_id,
        snapshot.chain_resource_manifest.chain_id
    );
    assert!(
        snapshot
            .model
            .agents
            .iter()
            .any(|(_, agent)| agent.id == "chain-agent")
    );
}

#[test]
fn chain_linked_runtime_enforcing_rejects_initial_snapshot_when_prime_fails() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind("127.0.0.1:1")
            .with_chain_link_policy(ChainLinkPolicy::Enforcing)
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(ViewerRequest::RequestSnapshot, &mut session, &mut writer)
        .expect_err("enforcing initial snapshot should reject chain prime failure");
    writer.flush().expect("flush writer");

    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(50));
    assert!(
        responses.is_empty(),
        "enforcing chain prime failure must not emit a fallback snapshot"
    );
    assert!(
        !session.initial_snapshot_sent,
        "failed enforcing prime must not mark initial snapshot sent"
    );
}

#[test]
fn chain_linked_runtime_sync_accepts_same_watermark_snapshot_rebuild() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_same_watermark_rebuild");
    let mut first_world = crate::runtime::World::new_production_hardened();
    first_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "first-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    first_world.step().expect("advance first execution world");
    first_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist first execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
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
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("first chain sync should succeed");
    assert!(progressed);
    assert!(server.world.state().agents.contains_key("first-agent"));
    assert!(read_response_line(&peer, Duration::from_millis(200)).is_some());

    let mut rebuilt_world = crate::runtime::World::new_production_hardened();
    rebuilt_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "rebuilt-agent".to_string(),
        pos: crate::geometry::GeoPos::new(9, 2, 0),
    });
    rebuilt_world
        .step()
        .expect("advance rebuilt execution world");
    assert_eq!(rebuilt_world.state().time, first_world.state().time);
    assert_eq!(
        latest_runtime_event_seq(&rebuilt_world),
        latest_runtime_event_seq(&first_world)
    );
    rebuilt_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("replace execution world with same-watermark rebuilt world");

    let (mut writer, peer) = test_writer_pair();
    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("same-watermark rebuilt chain sync should succeed");

    assert!(
        progressed,
        "materially different generated-map rebuild should advance despite the same sync watermark"
    );
    assert!(!server.world.state().agents.contains_key("first-agent"));
    assert!(server.world.state().agents.contains_key("rebuilt-agent"));
    let line = read_response_line(&peer, Duration::from_millis(200))
        .expect("expected rebuilt execution-world sync response");
    assert!(!line.trim().is_empty());
}

#[test]
fn chain_linked_runtime_sync_clears_stale_local_test_sidecar_binding() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_stale_local_test_binding");
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
    server.llm_sidecar.agent_player_bindings.insert(
        "starter-agent-0".to_string(),
        "local-test-player-old".to_string(),
    );
    server.llm_sidecar.player_agent_bindings.insert(
        "local-test-player-old".to_string(),
        "starter-agent-0".to_string(),
    );
    server
        .llm_sidecar
        .agent_public_key_bindings
        .insert("starter-agent-0".to_string(), "old-key".to_string());
    server
        .llm_sidecar
        .agent_player_bindings
        .insert("agent-real".to_string(), "player-real".to_string());
    server
        .llm_sidecar
        .player_agent_bindings
        .insert("player-real".to_string(), "agent-real".to_string());

    let mut session = RuntimeLiveSession::new();
    let (mut writer, _peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(
        !progressed,
        "empty chain world may not advance viewer state, but stale local binding should be pruned"
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get("starter-agent-0"),
        None
    );
    assert_eq!(
        server
            .llm_sidecar
            .player_agent_bindings
            .get("local-test-player-old"),
        None
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_public_key_bindings
            .get("starter-agent-0"),
        None
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get("agent-real")
            .map(String::as_str),
        Some("player-real")
    );
}

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
fn chain_linked_runtime_material_progress_resolves_submitted_gameplay_feedback() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_resolves_submitted_feedback");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world.submit_action(RuntimeAction::MoveAgent {
        agent_id: "chain-agent".to_string(),
        to: crate::geometry::GeoPos::new(2, 2, 0),
    });
    execution_world
        .step()
        .expect("advance execution world beyond viewer baseline");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(7, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "gameplay_action:mine".to_string(),
        stage: "submitted".to_string(),
        effect: "submitted gameplay action to the chain runtime".to_string(),
        intent_summary: Some("mine committed resources".to_string()),
        target_agent_id: Some("chain-agent".to_string()),
        reason: None,
        hint: Some("wait for committed world sync".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });
    let baseline_time = server.world.state().time;
    let baseline_event_seq = latest_runtime_event_seq(&server.world);
    let mut session = RuntimeLiveSession::new();
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let (mut writer, _peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("materially newer committed world should sync");

    let delta_logical_time = server.world.state().time.saturating_sub(baseline_time);
    let delta_event_seq =
        latest_runtime_event_seq(&server.world).saturating_sub(baseline_event_seq);
    assert!(progressed, "material world/event delta should report progress");
    assert!(
        delta_logical_time > 0 || delta_event_seq > 0,
        "committed height alone must not resolve submitted feedback"
    );
    let snapshot = server.compat_snapshot(None);
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(gameplay.response_window_class.as_deref(), Some("resolved"));
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("resolved gameplay feedback should remain observable");
    assert_eq!(feedback.stage, "completed_advanced");
    assert_eq!(feedback.delta_logical_time, delta_logical_time);
    assert_eq!(feedback.delta_event_seq, delta_event_seq);
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
