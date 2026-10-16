//! Deterministic conformance: a scripted, unpaid ModelProvider drives the real
//! SDK/Harness through HTTP and NodeRuntime. This is not autonomous model play.
use super::*;
use oasis7::runtime::{
    Action, MaterialLedgerId, STARTER_SMELTER_FACTORY_ID, STARTER_SMELTER_RECIPE_ID,
};
use oasis7_agent_harness::{
    ModelProvider, ModelProviderError, ModelProviderRequest, ModelProviderResponse,
    ModelProviderTrace, ModelProviderTurn, ModelProviderUsage, NativeTaskRunConfig,
    NativeTaskRunner, NativeTaskStopReason, SdkNativeTaskHost,
};
use oasis7_game_client::{device_client::Client, operation_store::ClientStateKind};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::atomic::AtomicUsize};
#[path = "world_service_task_native_runner_proxy.rs"]
mod delivery_race;

const SITE: &str = "starter-site";
const FACTORY: &str = STARTER_SMELTER_FACTORY_ID;

struct ScriptedFactoryChain {
    calls: Arc<AtomicUsize>,
}
impl ModelProvider for ScriptedFactoryChain {
    fn provider_id(&self) -> &str {
        "deterministic-native-factory-chain-conformance"
    }
    fn complete(
        &mut self,
        request: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            call < 10,
            "a resumed settled run must not ask the model again"
        );
        assert_eq!(request.budget.max_model_calls, 1);
        let prompt: Value = serde_json::from_str(&request.user_prompt).unwrap();
        let context = &prompt["authorized_context"];
        assert_eq!(
            context["goal"]["content"],
            serde_json::to_value(starter_chain_playtest::goal()).unwrap()
        );
        assert!(!context["actor_observation"].is_null());
        assert!(!context["capability_catalog"].is_null());
        // A rejected immutable slot is not a successful business stage. Only
        // the runner's authenticated, original-matched feedback permits this
        // new invocation to choose the same business operation in a NEW slot.
        let slot: usize = prompt["operation"]
            .as_str()
            .unwrap()
            .rsplit_once(':')
            .unwrap()
            .1
            .parse()
            .unwrap();
        let prior = prompt["prior_authorized_rejections"].as_array().unwrap();
        for rejection in prior {
            assert_eq!(
                rejection["authorized_feedback"]["feedback"]["reject_reason"],
                "stale_base"
            );
            assert_ne!(rejection["original_operation"], prompt["operation"]);
        }
        let stage = slot.checked_sub(prior.len()).unwrap();
        let reads = prompt["tool_results"].as_array().unwrap().len();
        let (name, arguments) = match stage {
            0 => (
                "build",
                json!({"site":SITE,"factory":FACTORY,"kind":"factory.smelter.mk1"}),
            ),
            1 | 2 => (
                "transfer",
                json!({"from":"agent:agent-a","to":"site:starter-site",
                "material":if stage == 1 { "iron_ore" } else { "carbon_fuel" },
                "amount":if stage == 1 { 8 } else { 2 }, "distance_km":1,"priority":"standard","routes":[]}),
            ),
            3 if reads == 0 => ("industry", json!({})),
            3 if reads == 1 => ("wait", json!({"ticks": 1})),
            3 => (
                "schedule",
                json!({"factory":FACTORY,"recipe":STARTER_SMELTER_RECIPE_ID,"batches":1}),
            ),
            _ => panic!("script must stop after four successful business effects"),
        };
        assert!(request.tools.iter().any(|tool| tool.name == name));
        Ok(ModelProviderResponse {
            request_digest: request.request_digest.clone(),
            turns: vec![ModelProviderTurn::ToolCall {
                name: name.into(),
                arguments,
            }],
            output: String::new(),
            model: Some("unpaid-scripted-conformance".into()),
            usage: ModelProviderUsage::default(),
            trace: ModelProviderTrace::default(),
        })
    }
}
struct StopFixture(PathBuf);
impl Drop for StopFixture {
    fn drop(&mut self) {
        let _ = fs::write(self.0.join("service-stop"), b"stop");
    }
}
fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn run_config() -> NativeTaskRunConfig {
    NativeTaskRunConfig {
        run_id: "native-factory-chain-conformance".into(),
        model: "unpaid-scripted-conformance".into(),
        system_prompt:
            "Deterministic conformance fixture; execute the prepared native factory chain.".into(),
        max_turns: 4,
        max_model_calls: 10,
        max_tool_calls_per_call: 1,
        model_timeout_ms: 10_000,
        control_poll_ms: 100,
    }
}
fn conformance_client(config_path: &Path) -> Client {
    let mut client = Client::open(config_path).unwrap();
    let mut transport = client.remote.config().clone();
    // This test deliberately holds one already-signed request across a normal
    // five-second node tick. Keep that in-flight request bounded independently
    // of the unchanged context TTL, cadence and scenario deadline.
    transport.timeout = Duration::from_secs(15);
    client.remote = oasis7_game_client::WorldServiceTaskClient::new(transport).unwrap();
    client
}
fn operation_slots() -> u32 {
    (run_config().max_turns + run_config().max_model_calls).min(128)
}
fn cognition_results(world: &RuntimeWorld) -> Vec<CanonicalIntentResultV1> {
    let mut results = world
        .capability_revocation_state()
        .world_service_results
        .values()
        .filter_map(|v| serde_json::from_value::<CanonicalIntentResultV1>(v.clone()).ok())
        .filter(|r| {
            matches!(
                &r.request.signed_payload,
                WorldServicePayloadV1::Cognition(_)
            )
        })
        .collect::<Vec<_>>();
    results.sort_by_key(|result| result.action_id);
    results
}

// Failure diagnostics contain bounded categories only: never signed payloads,
// private goal text, signer material, endpoint headers or provider responses.
fn diagnose_original_queries(client: &Client) {
    use oasis7_game_client::WorldServiceClientError as E;
    for index in 0..operation_slots() {
        let operation = run_config().operation_id(index);
        for (label, kind) in [
            ("prefix", ClientStateKind::Prefix),
            ("action", ClientStateKind::Action),
            ("outcome", ClientStateKind::Outcome),
        ] {
            let original = match client.cached_intent(kind, &operation) {
                Ok(Some(original)) => original,
                Ok(None) => continue,
                Err(_) => {
                    eprintln!(
                        "NATIVE_CONFORMANCE_QUERY phase=followup_after_run_failure stage={index} kind={label} category=cache_invalid"
                    );
                    continue;
                }
            };
            let result = client.remote.lookup(
                oasis7_client_api::world_service::LookupIntentRequest {
                    contract_version: 1,
                    key: original.correlation.key.clone(),
                },
                original.signed_payload,
            );
            let category = match result {
                Ok(response) => match response.outcome {
                    IntentOutcome::Received { .. } => "received".into(),
                    IntentOutcome::Pending => "pending".into(),
                    IntentOutcome::Unknown => "unknown".into(),
                    IntentOutcome::Committed { .. } => "committed".into(),
                    IntentOutcome::Rejected { reason } => {
                        format!("canonical_rejected_{reason:?}")
                    }
                    IntentOutcome::Expired => "expired".into(),
                    IntentOutcome::HistoryUnavailable => "history_unavailable".into(),
                },
                Err(E::Configuration(_)) => "configuration".into(),
                Err(E::Transport(_)) => "transport".into(),
                Err(E::Assurance(_)) => "assurance".into(),
                Err(E::Rejected { status, .. }) => format!("http_{status}"),
                Err(E::Cooldown {
                    status,
                    retry_after,
                }) => format!("cooldown_{status}_remaining_ms_{}", retry_after.as_millis()),
            };
            eprintln!(
                "NATIVE_CONFORMANCE_QUERY phase=followup_after_run_failure stage={index} kind={label} category={category}"
            );
        }
    }
}

#[test]
fn native_task_runner_http_scripted_factory_chain_and_restart_conformance() {
    let _probe_slot = prepared_world_probe_slot();
    let ready = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "oasis7-native-runner-conformance-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let deadline = Instant::now() + Duration::from_secs(300);
    std::thread::scope(|scope| {
        let service = scope.spawn(|| {
            run_prepared_world_service_kind(
                ready.clone(),
                Duration::from_secs(5),
                false,
                None,
                false,
                Duration::from_secs(30),
                PreparedWorldKind::StarterChain,
                Duration::ZERO,
            )
        });
        while !ready.exists() {
            assert!(
                Instant::now() < deadline,
                "native runner service readiness deadline"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let metadata = read_json(&ready);
        let root = PathBuf::from(metadata["root"].as_str().unwrap());
        let stop = StopFixture(root.clone());
        let config_path = PathBuf::from(metadata["config"].as_str().unwrap());
        let device = read_json(&config_path);
        let world = WorldIdentity {
            world_id: device["world_id"].as_str().unwrap().into(),
            genesis_digest: device["genesis_digest"].as_str().unwrap().into(),
        };
        let pin = || {
            super::super::super::super::world_service_read::pin(
                &root.join("records"),
                &root.join("store"),
                &world,
                None,
            )
            .unwrap()
        };
        let before = pin();
        assert!(before.world.state().factories.is_empty());
        assert!(cognition_results(&before.world).is_empty());
        // The proxy delays only the first original Transfer submission until
        // a real node commit advances. It never edits signed request bytes.
        let proxy = delivery_race::OriginalDeliveryRace::start(
            device["endpoint"].as_str().unwrap(),
            root.clone(),
            world.clone(),
            deadline,
        );
        let mut proxy_device = device.clone();
        proxy_device["endpoint"] = json!(proxy.endpoint());
        let proxy_config = root.join("native-runner-proxy-config.json");
        write_private(&proxy_config, &serde_json::to_vec(&proxy_device).unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let client = Arc::new(conformance_client(&proxy_config));
        let run_result = NativeTaskRunner::new(
            Arc::new(SdkNativeTaskHost::new(client.clone())),
            Box::new(ScriptedFactoryChain {
                calls: calls.clone(),
            }),
            run_config(),
        )
        .unwrap()
        .run();
        if !matches!(&run_result, Ok(report) if report.stop_reason == NativeTaskStopReason::Completed)
        {
            diagnose_original_queries(&client);
            let canonical = pin();
            for index in 0..operation_slots() {
                let operation = run_config().operation_id(index);
                let Some(original) = client
                    .cached_intent(ClientStateKind::Action, &operation)
                    .unwrap()
                else {
                    continue;
                };
                for result in cognition_results(&canonical.world) {
                    if result.request.correlation != original.correlation {
                        continue;
                    }
                    if let Some(reason) = result.rejected {
                        let category = if reason
                            == format!(
                                "{:?}",
                                oasis7::runtime::WorldError::DistributedValidationFailed {
                                    reason: "stale_base".into()
                                }
                            ) {
                            "stale_base"
                        } else {
                            "other_canonical_rejection"
                        };
                        eprintln!("NATIVE_CONFORMANCE_CANONICAL stage={index} category={category}");
                    }
                }
            }
        }
        let report = run_result.unwrap();
        assert_eq!(
            report.stop_reason,
            NativeTaskStopReason::Completed,
            "{report:?}"
        );
        assert_eq!(report.turns_completed, 4);
        assert!(proxy.delayed_original());
        assert_eq!(
            report.model_calls_spent as usize,
            calls.load(Ordering::SeqCst)
        );
        assert!((7..=run_config().max_model_calls).contains(&report.model_calls_spent));
        assert!(
            client
                .outcome(&run_config().operation_id(0), "executor_aborted")
                .unwrap_err()
                .contains("original action")
        );

        // The external node owns production time. A committed schedule is not
        // completion: use an immutable canonical pin as the outcome oracle.
        let delivered = loop {
            let current = pin();
            if current
                .world
                .ledger_material_balance(&MaterialLedgerId::site(SITE), "iron_ingot")
                == 3
                && current.world.state().pending_recipe_jobs.is_empty()
                && current.world.state().pending_material_transits.is_empty()
                && current.world.state().pending_factory_builds.is_empty()
            {
                break current;
            }
            assert!(
                Instant::now() < deadline,
                "native chain did not deliver finished products"
            );
            std::thread::sleep(Duration::from_millis(100));
        };
        assert!(delivered.world.state().factories.contains_key(FACTORY));
        for material in [
            "structural_frame",
            "heat_coil",
            "refractory_brick",
            "iron_ore",
            "carbon_fuel",
        ] {
            assert_eq!(
                delivered
                    .world
                    .ledger_material_balance(&MaterialLedgerId::agent("agent-a"), material),
                0,
                "{material}"
            );
        }
        for (material, amount) in [
            ("iron_ore", 4),
            ("carbon_fuel", 1),
            ("iron_ingot", 3),
            ("slag", 1),
        ] {
            assert_eq!(
                delivered
                    .world
                    .ledger_material_balance(&MaterialLedgerId::site(SITE), material),
                amount,
                "{material}"
            );
        }
        assert!(
            delivered
                .world
                .state()
                .industry_progress
                .starter_industrial_milestone
                .is_some()
        );
        let receipts = cognition_results(&delivered.world);
        let accepted = receipts
            .iter()
            .filter(|r| r.rejected.is_none())
            .collect::<Vec<_>>();
        let rejected = receipts
            .iter()
            .filter(|r| r.rejected.is_some())
            .collect::<Vec<_>>();
        assert_eq!(accepted.len(), 4);
        assert!(
            !rejected.is_empty(),
            "controlled delivery must produce canonical rejection"
        );
        for receipt in &rejected {
            assert_eq!(
                receipt.rejected.as_deref(),
                Some(
                    format!(
                        "{:?}",
                        oasis7::runtime::WorldError::DistributedValidationFailed {
                            reason: "stale_base".into()
                        }
                    )
                    .as_str()
                )
            );
            let operation = (0..operation_slots())
                .map(|i| run_config().operation_id(i))
                .find(|op| {
                    client
                        .cached_intent(ClientStateKind::Action, op)
                        .unwrap()
                        .is_some_and(|original| original.correlation == receipt.request.correlation)
                })
                .unwrap();
            // Canonical rejection belongs to the exact immutable signed
            // original, not merely the same action kind or business stage.
            assert!(
                client
                    .cached_intent(ClientStateKind::Action, &operation)
                    .unwrap()
                    .unwrap()
                    == receipt.request
            );
            let feedback = client.known_stale_rejection(&operation).unwrap().unwrap();
            assert_eq!(
                feedback.feedback.reject_reason.as_deref(),
                Some("stale_base")
            );
        }
        let mut action_kinds = [0; 3];
        for receipt in &accepted {
            let WorldServicePayloadV1::Cognition(signed) = &receipt.request.signed_payload else {
                unreachable!()
            };
            match &signed.request.action {
                Action::BuildFactory { .. } => action_kinds[0] += 1,
                Action::TransferMaterial { .. } => action_kinds[1] += 1,
                Action::ScheduleRecipe { .. } => action_kinds[2] += 1,
                other => panic!("unexpected native action: {other:?}"),
            }
        }
        assert_eq!(action_kinds, [1, 2, 1]);
        let applied = delivered
            .world
            .agent_causal_receipts("agent-a")
            .unwrap()
            .into_iter()
            .filter(|r| {
                r.disposition == "applied"
                    && ["BuildFactory", "TransferMaterial", "ScheduleRecipe"]
                        .contains(&r.action_kind.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(applied.len(), 4);
        let total = applied
            .iter()
            .map(|r| r.authorization.as_ref().unwrap().cost_units)
            .sum::<u64>();
        assert_eq!(
            delivered.world.state().agents["agent-a"]
                .state
                .resources
                .get(oasis7::simulator::ResourceKind::Electricity),
            100 - i64::try_from(total).unwrap()
        );
        for (kind, grant, cost) in [
            ("BuildFactory", "starter-build", 6),
            ("ScheduleRecipe", "starter-recipe", 8),
        ] {
            let receipt = applied.iter().find(|r| r.action_kind == kind).unwrap();
            assert_eq!(
                receipt.authorization.as_ref().unwrap().grant.grant_id,
                grant
            );
            assert_eq!(receipt.authorization.as_ref().unwrap().cost_units, cost);
        }
        let transfer_grants = applied
            .iter()
            .filter(|r| r.action_kind == "TransferMaterial")
            .map(|r| r.authorization.as_ref().unwrap().grant.grant_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            transfer_grants,
            ["starter-ore", "starter-fuel"].into_iter().collect()
        );

        // Close and reopen the SDK device to exercise the encrypted on-disk
        // originals/budget. No provider request, prefix or action may be added.
        let mut originals = Vec::new();
        for index in 0..operation_slots() {
            let operation = run_config().operation_id(index);
            for kind in [ClientStateKind::Prefix, ClientStateKind::Action] {
                let Some(original) = client.cached_intent(kind, &operation).unwrap() else {
                    continue;
                };
                assert!(matches!(
                    client.lookup(&original).unwrap().outcome,
                    IntentOutcome::Committed { .. } | IntentOutcome::Rejected { .. }
                ));
                originals.push((kind, operation.clone(), original));
            }
        }
        drop(client);
        let reopened = Arc::new(conformance_client(&proxy_config));
        let resumed = NativeTaskRunner::new(
            Arc::new(SdkNativeTaskHost::new(reopened.clone())),
            Box::new(ScriptedFactoryChain {
                calls: calls.clone(),
            }),
            run_config(),
        )
        .unwrap()
        .run()
        .unwrap();
        assert_eq!(resumed.stop_reason, NativeTaskStopReason::Completed);
        assert_eq!(resumed.turns_completed, 4);
        assert_eq!(resumed.model_calls_spent, report.model_calls_spent);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            report.model_calls_spent as usize
        );
        for (kind, operation, original) in originals {
            assert_eq!(
                reopened.cached_intent(kind, &operation).unwrap().unwrap(),
                original
            );
        }
        let resumed_pin = pin();
        assert_eq!(cognition_results(&resumed_pin.world), receipts);
        assert_eq!(
            resumed_pin
                .world
                .capability_revocation_state()
                .world_service_results,
            delivered
                .world
                .capability_revocation_state()
                .world_service_results,
            "restart must not append a replacement prefix, action or outcome"
        );
        assert_eq!(
            resumed_pin
                .world
                .agent_causal_receipts("agent-a")
                .unwrap()
                .len(),
            delivered
                .world
                .agent_causal_receipts("agent-a")
                .unwrap()
                .len()
        );
        assert_eq!(
            resumed_pin
                .world
                .ledger_material_balance(&MaterialLedgerId::site(SITE), "iron_ingot"),
            3
        );
        drop(reopened);
        drop(proxy);
        drop(stop);
        assert_eq!(service.join().unwrap(), root);
        fs::remove_file(&ready).unwrap();
        fs::remove_dir_all(root).unwrap();
    });
}
