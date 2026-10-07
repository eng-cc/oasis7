//! Real-driver TCP conformance, without passing node paths to the application client.
use super::*;
mod application_harness;
mod application_provider;
mod application_wake;
mod http_fixture;
use http_fixture::read_request;
mod process_restart;
mod production_pressure;
mod publication_pin;
mod viewer_process;
use oasis7::runtime::{Action, WorldState};
use oasis7::world_service::client::{
    RemoteWorldServiceClient, WorldServiceClientConfig, WorldServicePort,
};
use oasis7::world_service::*;
use oasis7_node::{
    NodeConfig, NodeConsensusAction, NodeExecutionCommitResult, NodeExecutionHook, NodeRuntime,
};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

struct SharedDriver(
    Arc<Mutex<NodeRuntimeExecutionDriver>>,
    Arc<AtomicBool>,
    Arc<AtomicBool>,
);
impl NodeExecutionHook for SharedDriver {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        while !self.1.load(Ordering::SeqCst) && !self.2.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(2));
        }
        if self.2.load(Ordering::SeqCst) {
            return Err("QA controlled automatic worker aborted at teardown".into());
        }
        self.0.lock().unwrap().on_commit(context)
    }
}

struct Fixture {
    driver: Arc<Mutex<NodeRuntimeExecutionDriver>>,
    automatic_commit_gate: Arc<AtomicBool>,
    abort_automatic_commit: Arc<AtomicBool>,
    preserve_root: bool,
    writer_lock: oasis7::viewer::ExclusiveDirectoryProcessLock,
    root: std::path::PathBuf,
    node: Arc<Mutex<NodeRuntime>>,
    halt: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    client: RemoteWorldServiceClient,
    owner: String,
    lose_next_submit: Arc<AtomicBool>,
    controlled_submit_commit: Arc<AtomicBool>,
    lookup_digests: Arc<Mutex<Vec<String>>>,
    outage: Arc<http_fixture::Outage>,
    tamper_next_describe: Arc<AtomicBool>,
}
impl Fixture {
    fn new() -> Self {
        Self::with_controlled_commits(false)
    }
    fn with_controlled_commits(controlled: bool) -> Self {
        Self::with_options(controlled, false)
    }
    fn with_options(controlled: bool, scheduler: bool) -> Self {
        let root = temp_dir("qa-world-service-tcp");
        let writer_lock =
            crate::world_writer_lock::acquire_live_world_writer_lock(&root.join("world")).unwrap();
        let owner = hex::encode([7u8; 32]);
        let public = sign_read_request("owner", (), &owner)
            .unwrap()
            .subject_public_key;
        let identity = WorldIdentity {
            world_id: "w1".into(),
            genesis_digest: "fixture-genesis-v1".into(),
        };
        let mut world = RuntimeWorld::new_with_state(WorldState::default());
        if scheduler {
            use oasis7::runtime::SchedulerPolicyV1;
            world = world
                .try_with_cognition_scheduler(
                    SchedulerPolicyV1 {
                        schema_version: SchedulerPolicyV1::SCHEMA_VERSION.into(),
                        max_total_wakes_per_tick: 8,
                        max_wakes_per_agent_per_tick: 1,
                        aging_after_ticks: 2,
                        max_starvation_ticks: 4,
                        initial_priority: 0,
                        comparator: SchedulerPolicyV1::COMPARATOR.into(),
                        service_order: SchedulerPolicyV1::SERVICE_ORDER.into(),
                    },
                    1,
                )
                .unwrap();
        }
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-a".into(),
            pos: oasis7::GeoPos::new(0, 0, 0),
        });
        world.step().unwrap();
        world.submit_action(Action::ClaimStarterOc {
            agent_id: "agent-a".into(),
            player_id: "owner-a".into(),
            public_key: Some(public),
        });
        world.step().unwrap();
        world
            .set_agent_resource_balance(
                "agent-a",
                oasis7::simulator::ResourceKind::Electricity,
                100_000,
            )
            .unwrap();
        world
            .install_capability_agent_identity("agent-a", "owner-a", 1)
            .unwrap();
        world
            .bind_cognition_runtime(
                "w1",
                "main",
                0,
                Some(format!(
                    "blake3:{}",
                    blake3::hash(b"qa-signed-finalized-provider-fixture:w1:main:0")
                )),
                "pending",
                0,
            )
            .unwrap();
        world
            .install_test_provider_capability_fixture_without_cognition_balance("agent-a")
            .unwrap();
        assert!(
            world.promote_cognition_runtime_finality().unwrap(),
            "signed finalized fixture authority must promote exact runtime binding"
        );
        assert_eq!(
            world.capability_revocation_state().agent_identities["agent-a"].owner_binding,
            "owner-a"
        );
        world
            .provision_cognition_for_agent(
                "agent-a",
                "qa-canonical-provision",
                "qa-runtime-authority",
                128,
            )
            .unwrap();
        let context_hash = super::super::execution_hash::execution_resource_context_hash("w1");
        world
            .save_to_dir_with_chain_resource_context(
                &root.join("world"),
                oasis7::runtime::ChainResourceDerivationContext {
                    world_id: "w1",
                    chain_id: "w1",
                    genesis_ref: Some(identity.genesis_digest.as_str()),
                    created_at_height: world.state().time,
                    manifest_height: world.state().time,
                    commit_block_hash: None,
                    tick: world.state().time,
                },
                context_hash.clone(),
                context_hash,
            )
            .unwrap();
        fs::write(
            root.join("world/world-service-identity.json"),
            serde_json::to_vec(&identity).unwrap(),
        )
        .unwrap();
        let baseline = super::super::local_bootstrap::derive_local_execution_bootstrap(
            &root.join("world"),
            &root.join("records"),
            "w1",
            2,
            "qa-local-setup-boundary-h2",
            &oasis7::runtime::ReleaseSecurityPolicy::default(),
        )
        .unwrap();
        let driver = NodeRuntimeExecutionDriver::new_with_local_bootstrap(
            root.join("state.json"),
            root.join("world"),
            root.join("records"),
            root.join("store"),
            &oasis7_proto::storage_profile::StorageProfileConfig::default(),
            baseline.clone(),
        )
        .unwrap();
        let driver = Arc::new(Mutex::new(driver));
        let node_baseline = if controlled {
            commit_request(&mut driver.lock().unwrap(), 1, None);
            super::super::local_bootstrap::derive_service_execution_bootstrap(
                &root.join("world"),
                &root.join("records"),
                &root.join("store"),
                "w1",
            )
            .unwrap()
        } else {
            baseline
        };
        let automatic_commit_gate = Arc::new(AtomicBool::new(!controlled));
        let abort_automatic_commit = Arc::new(AtomicBool::new(false));
        let config = NodeConfig::new("node-a", "w1", NodeRole::Sequencer)
            .unwrap()
            .with_tick_interval(if controlled {
                Duration::from_secs(60)
            } else {
                Duration::from_millis(40)
            })
            .unwrap();
        let mut node = NodeRuntime::new(config)
            .with_local_execution_bootstrap(node_baseline)
            .with_execution_hook(SharedDriver(
                driver.clone(),
                automatic_commit_gate.clone(),
                abort_automatic_commit.clone(),
            ));
        node.start().unwrap();
        let node = Arc::new(Mutex::new(node));
        let deadline = Instant::now() + Duration::from_secs(10);
        while super::super::world_service_read::pin(
            &root.join("records"),
            &root.join("store"),
            &identity,
            None,
        )
        .is_err()
        {
            assert!(
                Instant::now() < deadline,
                "real driver did not publish bootstrap record"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let key = hex::encode([9u8; 32]);
        let signer = crate::feedback_submit_api::FeedbackSubmitSigner {
            public_key_hex: sign_read_request("service", (), &key)
                .unwrap()
                .subject_public_key,
            private_key_hex: key,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let halt = Arc::new(AtomicBool::new(false));
        let lose_next_submit = Arc::new(AtomicBool::new(false));
        let worker_lose = lose_next_submit.clone();
        let tamper_next_describe = Arc::new(AtomicBool::new(false));
        let worker_tamper = tamper_next_describe.clone();
        let controlled_submit_commit = Arc::new(AtomicBool::new(false));
        let worker_controlled = controlled_submit_commit.clone();
        let lookup_digests = Arc::new(Mutex::new(Vec::new()));
        let worker_lookups = lookup_digests.clone();
        let outage = Arc::new(http_fixture::Outage::default());
        let worker_outage = outage.clone();
        let worker_driver = driver.clone();
        let worker_halt = halt.clone();
        let worker_node = node.clone();
        let worker_root = root.clone();
        let trusted_service_public_key = signer.public_key_hex.clone();
        let worker = thread::spawn(move || {
            while !worker_halt.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        http_fixture::configure_accepted_stream(&stream);
                        let bytes = read_request(&mut stream);
                        if bytes.is_empty() {
                            continue;
                        }
                        let first = std::str::from_utf8(&bytes).unwrap().lines().next().unwrap();
                        let mut parts = first.split_whitespace();
                        let method = parts.next().unwrap();
                        let path = parts.next().unwrap();
                        if worker_outage.reject(&stream) {
                            continue;
                        }
                        if path == LOOKUP_PATH {
                            http_fixture::record_lookup_digest(&bytes, &worker_lookups);
                        }
                        let lose = path == SUBMIT_PATH && worker_lose.swap(false, Ordering::SeqCst);
                        if lose {
                            stream.shutdown(std::net::Shutdown::Write).unwrap();
                        }
                        let tamper =
                            path == DESCRIBE_PATH && worker_tamper.swap(false, Ordering::SeqCst);
                        let mut capture_peer = None;
                        let mut output_stream = if tamper {
                            let capture = TcpListener::bind("127.0.0.1:0").unwrap();
                            capture_peer =
                                Some(TcpStream::connect(capture.local_addr().unwrap()).unwrap());
                            capture.accept().unwrap().0
                        } else {
                            stream.try_clone().unwrap()
                        };
                        let result = crate::world_service_api::maybe_handle(
                            &mut output_stream,
                            &bytes,
                            &worker_node,
                            method,
                            path,
                            "w1",
                            &worker_root.join("world"),
                            &worker_root.join("records"),
                            &worker_root.join("store"),
                            &signer,
                        );
                        eprintln!("fixture_dispatch method={method} path={path} result={result:?}");
                        if !lose {
                            assert!(
                                result.unwrap(),
                                "actual dispatcher did not handle requested route"
                            );
                        }
                        drop(output_stream);
                        if let Some(mut peer) = capture_peer {
                            let mut response = Vec::new();
                            peer.read_to_end(&mut response).unwrap();
                            let offset = response
                                .windows(4)
                                .position(|window| window == b"\r\n\r\n")
                                .unwrap()
                                + 4;
                            let mut body: serde_json::Value =
                                serde_json::from_slice(&response[offset..]).unwrap();
                            body["signature_hex"] = serde_json::json!("00".repeat(64));
                            crate::write_json_response(
                                &mut stream,
                                200,
                                &serde_json::to_vec(&body).unwrap(),
                                false,
                            )
                            .unwrap();
                        }
                        if path == SUBMIT_PATH && worker_controlled.load(Ordering::SeqCst) {
                            let body =
                                crate::feedback_submit_api::extract_http_json_body(&bytes).unwrap();
                            let request = serde_json::from_slice(body).unwrap();
                            let mut driver = worker_driver.lock().unwrap();
                            let height = driver.state.last_applied_committed_height + 1;
                            commit_request(&mut driver, height, Some(request));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            }
        });
        let client = RemoteWorldServiceClient::new(WorldServiceClientConfig {
            endpoint,
            trusted_service_public_key,
            expected_world: identity,
            scope_id: "public".into(),
            read_private_key_hex: owner.clone(),
            timeout: Duration::from_secs(2),
            max_response_bytes: 1_048_576,
        })
        .unwrap();
        http_fixture::wait_ready(&client, &worker);
        Self {
            driver,
            automatic_commit_gate,
            abort_automatic_commit,
            preserve_root: false,
            writer_lock,
            root,
            node,
            halt,
            worker: Some(worker),
            client,
            owner,
            lose_next_submit,
            controlled_submit_commit,
            lookup_digests,
            outage,
            tamper_next_describe,
        }
    }
    fn request(
        &self,
        payload: WorldServicePayloadV1,
    ) -> SubmitIntentRequest<WorldServicePayloadV1> {
        SubmitIntentRequest {
            contract_version: 1,
            correlation: derive_correlation(self.client.config().expected_world.clone(), &payload)
                .unwrap(),
            deadline_unix_ms: None,
            signed_payload: payload,
        }
    }
    fn delegation(&self) -> SubmitIntentRequest<WorldServicePayloadV1> {
        self.request(WorldServicePayloadV1::Delegation(
            sign_read_request(
                "delegation",
                AgentSignerDelegationChangeV1 {
                    world_id: "w1".into(),
                    branch_id: "main".into(),
                    agent_id: "agent-a".into(),
                    owner_binding: "owner-a".into(),
                    agent_identity_generation: 1,
                    generation: 1,
                    delegate_public_key: sign_read_request("delegate", (), &hex::encode([8u8; 32]))
                        .unwrap()
                        .subject_public_key,
                    revoked: false,
                    nonce: 1,
                },
                &self.owner,
            )
            .unwrap(),
        ))
    }
    fn committed(&self, request: &SubmitIntentRequest<WorldServicePayloadV1>) -> CommitRef {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let result = self
                .client
                .lookup(
                    LookupIntentRequest {
                        contract_version: 1,
                        key: request.correlation.key.clone(),
                    },
                    request.signed_payload.clone(),
                )
                .unwrap();
            match result.outcome {
                IntentOutcome::Committed { commit, .. } => return commit,
                IntentOutcome::Unknown
                | IntentOutcome::Received { .. }
                | IntentOutcome::Pending => {}
                other => {
                    let driver = self.driver.lock().unwrap();
                    let key = correlation::key_digest(&request.correlation.key).unwrap();
                    let reason = driver
                        .execution_world
                        .capability_revocation_state()
                        .world_service_results
                        .get(&key)
                        .and_then(|value| value.get("rejected"))
                        .cloned();
                    panic!(
                        "expected canonical commit, got {other:?}; canonical_rejected_reason={reason:?}"
                    );
                }
            }
            assert!(Instant::now() < deadline, "canonical receipt not published");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn view(&self, min_commit: Option<CommitRef>) -> ReadWorldViewRequest {
        ReadWorldViewRequest {
            contract_version: 1,
            world: self.client.config().expected_world.clone(),
            scope_id: "public".into(),
            min_commit,
            fixed_commit: None,
            deadline_unix_ms: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.halt.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.abort_automatic_commit.store(true, Ordering::SeqCst);
        self.automatic_commit_gate.store(true, Ordering::SeqCst);
        let _ = self.node.lock().unwrap().stop();
        if !self.preserve_root {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
#[test]
fn real_tcp_five_operations_delegation_and_minimum_commit() {
    let fixture = Fixture::new();
    fixture.client.describe().unwrap();
    let original = fixture.delegation();
    fixture.client.submit(original.clone()).unwrap();
    let commit = fixture.committed(&original);
    let view = fixture
        .client
        .read_view(fixture.view(Some(commit.clone())))
        .unwrap();
    assert!(view.version().commit.satisfies_minimum(&commit).unwrap());
    let changes = fixture
        .client
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: view.continuation().clone(),
            max_items: 32,
            max_bytes: 65_536,
        })
        .unwrap();
    assert_eq!(changes.next_cursor.scope_id, "public");
    let replay = fixture.client.submit(original.clone()).unwrap();
    assert!(matches!(
        replay,
        SubmitObservation::Response(IntentResponse {
            outcome: IntentOutcome::Committed { .. },
            ..
        })
    ));
    let mut future = commit;
    future.position = u64::MAX;
    assert!(
        fixture
            .client
            .read_view(fixture.view(Some(future)))
            .is_err()
    );
    let mut generation = view.version().commit.clone();
    generation.binding.permission_generation += 1;
    assert!(
        fixture
            .client
            .read_view(fixture.view(Some(generation)))
            .is_err()
    );
}
#[test]
fn real_tcp_rejects_forged_delegation_and_cursor() {
    let fixture = Fixture::new();
    let mut forged = fixture.delegation();
    if let WorldServicePayloadV1::Delegation(signed) = &mut forged.signed_payload {
        signed.signature_hex = "00".repeat(64);
    }
    assert!(fixture.client.submit(forged).is_err());
    let view = fixture.client.read_view(fixture.view(None)).unwrap();
    let mut cursor = view.continuation().clone();
    cursor.stream_id = "unrelated-stream".into();
    assert!(
        fixture
            .client
            .read_changes(ReadWorldChangesRequest {
                contract_version: 1,
                cursor,
                max_items: 32,
                max_bytes: 65_536
            })
            .is_err()
    );
}

#[test]
fn real_tcp_lost_response_signed_gameplay_and_driver_restart() {
    use oasis7::viewer::{CollectDataCommand, CollectDataRequest, sign_collect_data_auth_proof};
    let fixture = Fixture::new();
    let public = sign_read_request("owner", (), &fixture.owner)
        .unwrap()
        .subject_public_key;
    let mut command = CollectDataCommand::Submit {
        request: CollectDataRequest {
            electricity_cost: 7,
            data_amount: 11,
            player_id: "owner-a".into(),
            public_key: Some(public.clone()),
            auth: None,
        },
    };
    let proof = sign_collect_data_auth_proof(&command, 9, &public, &fixture.owner).unwrap();
    let CollectDataCommand::Submit { request } = &mut command else {
        unreachable!()
    };
    request.auth = Some(proof);
    let original = fixture.request(WorldServicePayloadV1::GameplayJson(
        serde_json::to_vec(&command).unwrap(),
    ));
    fixture.lose_next_submit.store(true, Ordering::SeqCst);
    assert!(matches!(
        fixture.client.submit(original.clone()).unwrap(),
        SubmitObservation::OutcomeUnknown(_)
    ));
    let commit = fixture.committed(&original);
    fixture.node.lock().unwrap().stop().unwrap();
    let restarted = NodeRuntimeExecutionDriver::new(
        fixture.root.join("state.json"),
        fixture.root.join("world"),
        fixture.root.join("records"),
        fixture.root.join("store"),
    )
    .unwrap();
    assert_eq!(
        restarted
            .execution_world
            .state()
            .authenticated_collect_data_last_nonces["owner-a"][&public],
        9
    );
    let outcome = fixture
        .client
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: original.correlation.key.clone(),
            },
            original.signed_payload.clone(),
        )
        .unwrap();
    assert!(
        matches!(outcome.outcome,IntentOutcome::Committed { commit: recovered,.. } if recovered==commit)
    );
    // The restarted driver and immutable persisted service read agree on the exact receipt.
    let key = correlation::key_digest(&original.correlation.key).unwrap();
    assert!(
        restarted
            .execution_world
            .capability_revocation_state()
            .world_service_results
            .contains_key(&key)
    );
}

#[test]
fn real_tcp_application_artifact_with_os_denied_node_directory() {
    run_isolated_application(false, false);
}

#[test]
fn real_tcp_application_genuine_wait_wake_no_node_directory() {
    run_isolated_application(true, false);
}

fn run_isolated_application(wake: bool, drift: bool) {
    let fixture = Fixture::with_options(true, wake);
    let app_dir = temp_dir("qa-world-service-application");
    fs::create_dir_all(&app_dir).unwrap();
    let registration = fixture.delegation();
    fixture.client.submit(registration.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        2,
        Some(registration.clone()),
    );
    fixture.committed(&registration);
    let _clock = application_wake::start_parent_clock(&fixture, wake && !drift);
    let original = cognition_request(&fixture);
    fixture
        .controlled_submit_commit
        .store(true, Ordering::SeqCst);
    let executable = std::env::current_exe().unwrap();
    // The application receives connection configuration and signed request only.
    // The forbidden path is supplied solely to prove OS denial, never as client configuration.
    let denied = fs::canonicalize(&fixture.root).unwrap();
    let profile = format!(
        "(version 1)(allow default)(deny file-read* file-write* (subpath \"{}\"))",
        denied.display()
    );
    let config = fixture.client.config();
    println!(
        "application_artifact_blake3={} mechanism=sandbox-exec",
        blake3::hash(&fs::read(&executable).unwrap())
    );
    println!(
        "application_config_identity={} sandbox_profile_blake3={} isolated_cwd=true",
        serde_json::json!({"endpoint":config.endpoint,"trusted_service_public_key":config.trusted_service_public_key,"world":config.expected_world,"scope":config.scope_id,"agent_scope":"agent:agent-a","owner_public_key":sign_read_request("owner",(),&config.read_private_key_hex).unwrap().subject_public_key,"agent_delegate_public_key":sign_read_request("delegate",(),&hex::encode([8u8;32])).unwrap().subject_public_key,"delegation_generation":1,"decision_source":"provider_backed","provider_backend":"provider_local_mock","execution_lane":"headless_agent"}),
        blake3::hash(profile.as_bytes())
    );
    let output = std::process::Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &profile])
        .arg(&executable)
        .args([
            "--ignored",
            "--exact",
            "execution_bridge_real_tests::real_execution_bridge::tests::qa_conformance::application_process_probe",
            "--nocapture",
        ])
        .current_dir(&app_dir)
        .env("OASIS7_AGENT_DECISION_SOURCE", "provider_backed")
        .env("OASIS7_AGENT_PROVIDER_BACKEND", "provider_local_mock")
        .env("OASIS7_AGENT_PROVIDER_CONTRACT", "worldsim_provider_v1")
        .env("OASIS7_AGENT_PROVIDER_TRANSPORT", "loopback_http")
        .env("OASIS7_AGENT_PROVIDER_PROFILE", "oasis7_p0_low_freq_npc")
        .env("OASIS7_AGENT_EXECUTION_LANE", "headless_agent")
        .env("OASIS7_AGENT_PROVIDER_URL", &config.endpoint)
        .env_remove("OASIS7_AGENT_PROVIDER_AUTH_TOKEN")
        .env("PRE2_APP_WAKE", if wake { "1" } else { "0" })
        .env("PRE2_APP_WAKE_DRIFT", if drift { "1" } else { "0" })
        .env("PRE2_APP_ENDPOINT", &config.endpoint)
        .env("PRE2_APP_TRUST", &config.trusted_service_public_key)
        .env(
            "PRE2_APP_WORLD",
            serde_json::to_string(&config.expected_world).unwrap(),
        )
        .env("PRE2_APP_OWNER", &config.read_private_key_hex)
        .env(
            "PRE2_APP_REQUEST",
            serde_json::to_string(&original).unwrap(),
        )
        .env("PRE2_DENIED_NODE_ROOT", &denied)
        .output()
        .unwrap();
    application_harness::validate_output(&fixture, &output, wake, drift);
    fs::remove_dir_all(app_dir).unwrap();
}

#[test]
#[ignore = "child application entrypoint; parent supplies explicit config and OS sandbox"]
fn application_process_probe() {
    let env = |name: &str| std::env::var(name).unwrap();
    let denied = std::path::PathBuf::from(env("PRE2_DENIED_NODE_ROOT"));
    let error = fs::read(denied.join("world/world-service-identity.json")).unwrap_err();
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied,
        "node file must exist and be denied by the OS"
    );
    println!("PRE2_APPLICATION_OS_DENIAL_PROBE_PASSED");
    let client = RemoteWorldServiceClient::new(WorldServiceClientConfig {
        endpoint: env("PRE2_APP_ENDPOINT"),
        trusted_service_public_key: env("PRE2_APP_TRUST"),
        expected_world: serde_json::from_str(&env("PRE2_APP_WORLD")).unwrap(),
        scope_id: "public".into(),
        read_private_key_hex: env("PRE2_APP_OWNER"),
        timeout: Duration::from_secs(2),
        max_response_bytes: 1_048_576,
    })
    .unwrap();
    if env("PRE2_APP_WAKE") == "1" {
        application_wake::verify_wait_wake(&client);
        if std::env::var("PRE2_APP_WAKE_DRIFT").unwrap() != "1" {
            println!("PRE2_APPLICATION_OS_DENIAL_AND_GENUINE_WAKE_PASSED");
        }
        return;
    }
    let original: SubmitIntentRequest<WorldServicePayloadV1> =
        serde_json::from_str(&env("PRE2_APP_REQUEST")).unwrap();
    client.describe().unwrap();
    client.submit(original.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let commit = loop {
        let response = client
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: original.correlation.key.clone(),
                },
                original.signed_payload.clone(),
            )
            .unwrap();
        if let IntentOutcome::Committed { commit, .. } = response.outcome {
            break commit;
        }
        assert!(
            Instant::now() < deadline,
            "no application canonical receipt"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let view = client
        .read_view(ReadWorldViewRequest {
            contract_version: 1,
            world: client.config().expected_world.clone(),
            scope_id: "public".into(),
            min_commit: Some(commit),
            fixed_commit: None,
            deadline_unix_ms: None,
        })
        .unwrap();
    client
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: view.continuation().clone(),
            max_items: 32,
            max_bytes: 65_536,
        })
        .unwrap();
    let mut protected_config = client.config().clone();
    protected_config.scope_id = "agent:agent-a".into();
    let protected = RemoteWorldServiceClient::new(protected_config).unwrap();
    let baseline = protected
        .read_view(ReadWorldViewRequest {
            contract_version: 1,
            world: protected.config().expected_world.clone(),
            scope_id: protected.config().scope_id.clone(),
            min_commit: None,
            fixed_commit: None,
            deadline_unix_ms: None,
        })
        .unwrap();
    // A real signed gameplay mutation crosses the same isolated application boundary.
    use oasis7::viewer::{CollectDataCommand, CollectDataRequest, sign_collect_data_auth_proof};
    let public = sign_read_request("owner", (), &env("PRE2_APP_OWNER"))
        .unwrap()
        .subject_public_key;
    let mut gameplay = CollectDataCommand::Submit {
        request: CollectDataRequest {
            electricity_cost: 7,
            data_amount: 11,
            player_id: "owner-a".into(),
            public_key: Some(public.clone()),
            auth: None,
        },
    };
    let proof =
        sign_collect_data_auth_proof(&gameplay, 10, &public, &env("PRE2_APP_OWNER")).unwrap();
    let CollectDataCommand::Submit { request } = &mut gameplay else {
        unreachable!()
    };
    request.auth = Some(proof);
    let payload = WorldServicePayloadV1::GameplayJson(serde_json::to_vec(&gameplay).unwrap());
    let gameplay = SubmitIntentRequest {
        contract_version: 1,
        correlation: derive_correlation(client.config().expected_world.clone(), &payload).unwrap(),
        deadline_unix_ms: None,
        signed_payload: payload,
    };
    client.submit(gameplay.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let gameplay_commit = loop {
        let response = client
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: gameplay.correlation.key.clone(),
                },
                gameplay.signed_payload.clone(),
            )
            .unwrap();
        match response.outcome {
            IntentOutcome::Committed { commit, receipt } => {
                assert!(
                    receipt["events"]
                        .as_array()
                        .is_some_and(|events| !events.is_empty())
                );
                break commit;
            }
            IntentOutcome::Unknown | IntentOutcome::Pending | IntentOutcome::Received { .. } => {}
            other => panic!("signed gameplay did not commit: {other:?}"),
        }
        assert!(Instant::now() < deadline, "signed gameplay receipt timeout");
        thread::sleep(Duration::from_millis(10));
    };
    let gameplay_view = client
        .read_view(ReadWorldViewRequest {
            contract_version: 1,
            world: client.config().expected_world.clone(),
            scope_id: "public".into(),
            min_commit: Some(gameplay_commit.clone()),
            fixed_commit: None,
            deadline_unix_ms: None,
        })
        .unwrap();
    client
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: gameplay_view.continuation().clone(),
            max_items: 32,
            max_bytes: 65_536,
        })
        .unwrap();
    let continuation_request = ReadWorldChangesRequest {
        contract_version: 1,
        cursor: baseline.continuation().clone(),
        max_items: 1,
        max_bytes: 65_536,
    };
    let first = protected
        .read_changes(continuation_request.clone())
        .unwrap();
    assert!(
        !first.changes.is_empty(),
        "snapshot-to-changes boundary lost the gameplay event"
    );
    assert!(first.changes.len() <= 1);
    let repeated = protected.read_changes(continuation_request).unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&repeated).unwrap(),
        "stable cursor replay changed delivery"
    );
    let mut cursor = baseline.continuation().clone();
    let mut delivered = Vec::new();
    loop {
        let changes = protected
            .read_changes(ReadWorldChangesRequest {
                contract_version: 1,
                cursor: cursor.clone(),
                max_items: 1,
                max_bytes: 65_536,
            })
            .unwrap();
        if changes.changes.is_empty() {
            break;
        }
        assert_ne!(
            cursor.sequence, changes.next_cursor.sequence,
            "cursor did not advance"
        );
        delivered.extend(changes.changes.into_iter().map(|change| change.change));
        cursor = changes.next_cursor;
        assert!(
            delivered.len() < 128,
            "bounded fixture change drain did not finish"
        );
    }
    assert!(delivered.iter().any(|change|matches!(serde_json::from_value::<oasis7::runtime::WorldEvent>(change.clone()).unwrap().body,oasis7::runtime::WorldEventBody::Domain(oasis7::runtime::DomainEvent::DataCollectedAuthenticated {collector_agent_id,electricity_cost:7,data_amount:11,player_id,nonce:10,..}) if collector_agent_id=="agent-a" && player_id=="owner-a")),"signed gameplay event missing from protected continuation");
    let mut wrong_era = cursor;
    wrong_era.era = wrong_era.era.saturating_add(1);
    assert!(
        protected
            .read_changes(ReadWorldChangesRequest {
                contract_version: 1,
                cursor: wrong_era,
                max_items: 1,
                max_bytes: 65_536
            })
            .is_err(),
        "cross-era cursor must demand resync"
    );
    protected
        .read_view(ReadWorldViewRequest {
            contract_version: 1,
            world: protected.config().expected_world.clone(),
            scope_id: protected.config().scope_id.clone(),
            min_commit: Some(gameplay_commit),
            fixed_commit: None,
            deadline_unix_ms: None,
        })
        .unwrap();
    application_provider::verify_provider_closure(&client);
    println!(
        "PRE2_APPLICATION_OS_DENIAL_AND_FIVE_OPS_PASSED signed_gameplay=true signed_cognition=true"
    );
}

// Explicit controlled canonical execution is a deterministic service fixture;
// it does not claim automatic consensus-worker scheduling coverage.
fn commit_request(
    driver: &mut NodeRuntimeExecutionDriver,
    _logical_fixture_step: u64,
    request: Option<SubmitIntentRequest<WorldServicePayloadV1>>,
) {
    let height = driver.state.last_applied_committed_height + 1;
    let actions = request
        .into_iter()
        .map(|request| {
            let digest = correlation::key_digest(&request.correlation.key).unwrap();
            let hash = blake3::hash(digest.as_bytes());
            let action_id = u64::from_be_bytes(hash.as_bytes()[..8].try_into().unwrap()).max(1);
            let bytes = correlation::encode_consensus_intent(&request).unwrap();
            NodeConsensusAction::from_payload(action_id, "node-a", bytes).unwrap()
        })
        .collect::<Vec<_>>();
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".into(),
            node_id: "node-a".into(),
            proposer_id: "node-a".into(),
            height,
            slot: height,
            epoch: 0,
            node_block_hash: format!("controlled-node-h{height}"),
            action_root: compute_consensus_action_root(&actions).unwrap(),
            committed_actions: actions,
            committed_at_unix_ms: height as i64 * 1000,
        })
        .unwrap();
}
#[test]
fn real_tcp_controlled_agent_cognition_receipt_and_read() {
    let fixture = Fixture::with_controlled_commits(true);
    fixture.client.describe().unwrap();
    let registration = fixture.delegation();
    fixture.client.submit(registration.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        2,
        Some(registration.clone()),
    );
    fixture.committed(&registration);
    let original = cognition_request(&fixture);
    fixture.client.submit(original.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        3,
        Some(original.clone()),
    );
    let commit = fixture.committed(&original);
    let outcome = fixture
        .client
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: original.correlation.key.clone(),
            },
            original.signed_payload.clone(),
        )
        .unwrap();
    let IntentOutcome::Committed { receipt, .. } = outcome.outcome else {
        panic!("missing cognition receipt")
    };
    assert!(receipt.get("commit_record").is_some());
    assert!(receipt.get("lineage").is_some());
    assert!(receipt.get("feedback").is_some());
    let view = fixture
        .client
        .read_view(fixture.view(Some(commit)))
        .unwrap();
    fixture
        .client
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: view.continuation().clone(),
            max_items: 32,
            max_bytes: 65_536,
        })
        .unwrap();
}

fn cognition_request(fixture: &Fixture) -> SubmitIntentRequest<WorldServicePayloadV1> {
    use oasis7::runtime::{RuntimeCognitionCommitRequestV1, RuntimeCognitionResponseArtifactV1};
    let world = fixture.driver.lock().unwrap().execution_world.clone();
    let digest =
        |label: &str| oasis7::simulator::h_v1("oasis7.test.qa-conformance.v1", &label).to_string();
    let request = RuntimeCognitionCommitRequestV1 {
        agent_id: "agent-a".into(),
        agent_session_id: "qa-session".into(),
        agent_turn_id: "qa-turn".into(),
        decision_request_id: "qa-cognition".into(),
        retry_seq: 1,
        transport_attempt: 1,
        request_digest: digest("request"),
        observation_digest: digest("observation"),
        context_digest: digest("context"),
        capability_snapshot_hash: oasis7::simulator::h_v1(
            "oasis7.runtime.manifest.v1",
            &world.capability_authorization_root(),
        )
        .to_string(),
        authority_context_hash: oasis7::simulator::h_v1(
            "oasis7.runtime.authority-context.v1",
            &world.capability_authorization_root(),
        )
        .to_string(),
        captured_base_binding: world.current_cognition_base_binding().unwrap(),
    };
    let mut response_artifact = RuntimeCognitionResponseArtifactV1 {
        schema_version: 1,
        context_discriminator: RuntimeCognitionResponseArtifactV1::CONTEXT_DISCRIMINATOR.into(),
        context_version: RuntimeCognitionResponseArtifactV1::CONTEXT_VERSION,
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        retry_seq: request.retry_seq,
        transport_attempt: request.transport_attempt,
        request_digest: request.request_digest.clone(),
        response_digest: digest("response"),
        artifact_digest: String::new(),
    };
    response_artifact.refresh_artifact_digest();
    let payload = WorldServicePayloadV1::Cognition(
        sign_read_request(
            "cognition",
            CognitionIntentV1 {
                request,
                action: Action::MoveAgent {
                    agent_id: "agent-a".into(),
                    to: oasis7::GeoPos::new(1, 1, 0),
                },
                response_artifact,
                delegation_generation: 1,
            },
            &hex::encode([8u8; 32]),
        )
        .unwrap(),
    );
    fixture.request(payload)
}

#[test]
fn real_tcp_conflicting_key_and_revoked_delegate_leave_canonical_result_unchanged() {
    let fixture = Fixture::with_controlled_commits(true);
    let registration = fixture.delegation();
    fixture.client.submit(registration.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        2,
        Some(registration.clone()),
    );
    let original_commit = fixture.committed(&registration);
    let cognition = cognition_request(&fixture);
    let WorldServicePayloadV1::Delegation(signed) = &registration.signed_payload else {
        unreachable!()
    };
    let mut conflicting = signed.request.clone();
    conflicting.revoked = true;
    let conflicting = fixture.request(WorldServicePayloadV1::Delegation(
        sign_read_request("delegation", conflicting, &fixture.owner).unwrap(),
    ));
    assert_eq!(conflicting.correlation.key, registration.correlation.key);
    assert_ne!(
        conflicting.correlation.payload_digest,
        registration.correlation.payload_digest
    );
    assert!(fixture.client.submit(conflicting).is_err());
    assert_eq!(fixture.committed(&registration), original_commit);
    let mut revocation = signed.request.clone();
    revocation.revoked = true;
    revocation.generation = 2;
    revocation.nonce = 2;
    let revocation = fixture.request(WorldServicePayloadV1::Delegation(
        sign_read_request("delegation", revocation, &fixture.owner).unwrap(),
    ));
    fixture.client.submit(revocation.clone()).unwrap();
    commit_request(
        &mut fixture.driver.lock().unwrap(),
        3,
        Some(revocation.clone()),
    );
    fixture.committed(&revocation);
    assert!(
        fixture.client.submit(cognition).is_err(),
        "revoked provider cannot admit cognition"
    );
    let mut config = fixture.client.config().clone();
    config.scope_id = "agent:agent-a".into();
    config.read_private_key_hex = hex::encode([11u8; 32]);
    let client = RemoteWorldServiceClient::new(config).unwrap();
    let mut read = fixture.view(None);
    read.scope_id = "agent:agent-a".into();
    assert!(
        client.read_view(read).is_err(),
        "unrelated key cannot read protected Agent scope"
    );
}

#[test]
fn real_tcp_tampered_service_response_is_not_trusted() {
    let fixture = Fixture::new();
    fixture.client.describe().unwrap();
    fixture.tamper_next_describe.store(true, Ordering::SeqCst);
    assert!(
        matches!(
            fixture.client.describe(),
            Err(oasis7::world_service::client::WorldServiceClientError::Assurance(_))
        ),
        "tampered actual service response must fail configured trust"
    );
    fixture.client.describe().unwrap();
}

#[test]
fn real_tcp_live_writer_lock_excludes_second_writer() {
    let fixture = Fixture::new();
    let error =
        crate::world_writer_lock::acquire_live_world_writer_lock(&fixture.root.join("world"))
            .err()
            .expect("second writer must be excluded");
    assert!(
        error.contains("held") && error.contains("lock"),
        "unexpected lock failure: {error}"
    );
    fixture.client.describe().unwrap();
    let _held = &fixture.writer_lock;
}
