use super::kernel_wasm_sandbox_bridge::DynamicMicroDepotSandbox;
use super::*;
use oasis7_wasm_abi::ModuleLimits;
use std::sync::{Arc, Mutex};

fn install_fixture(data_units: i64) -> (WorldKernel, WorldSnapshot) {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-install".to_string(),
        name: "Install Hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().expect("register install location");
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-owner".to_string(),
        location_id: "loc-install".to_string(),
    });
    kernel.step().expect("register install owner");
    seed_owner_resource(
        &mut kernel,
        ResourceOwner::Agent {
            agent_id: "agent-owner".to_string(),
        },
        ResourceKind::Data,
        data_units,
    );
    let base = kernel.snapshot();
    (kernel, base)
}

fn install_action(facility_id: &str, supported_resource_kinds: Vec<&str>) -> Action {
    Action::InstallMicroDepot {
        installer_agent_id: "agent-owner".to_string(),
        facility_id: facility_id.to_string(),
        location_id: "loc-install".to_string(),
        owner_claim_id: "claim-install".to_string(),
        regional_blocker_receipt_id: "repair-logistics:loc-install:1".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.2.0".to_string(),
        wasm_hash: "hash-install".to_string(),
        entrypoint: "evaluate_quote".to_string(),
        service_radius_cm: 250_000,
        supported_resource_kinds: supported_resource_kinds
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

fn owner_data(kernel: &WorldKernel) -> i64 {
    kernel
        .model()
        .agents
        .get("agent-owner")
        .expect("install owner")
        .resources
        .get(ResourceKind::Data)
}

fn install(kernel: &mut WorldKernel, facility_id: &str) -> WorldEventKind {
    kernel.submit_action(install_action(facility_id, vec!["data"]));
    kernel.step().expect("resolve install").kind
}

fn reclaim(kernel: &mut WorldKernel, facility_id: &str) -> WorldEventKind {
    kernel.submit_action(Action::ReclaimMicroDepot {
        agent_id: "agent-owner".to_string(),
        facility_id: facility_id.to_string(),
    });
    kernel.step().expect("resolve reclaim").kind
}

fn configure_two_unit_service(kernel: &mut WorldKernel) {
    let sandbox = Arc::new(Mutex::new(DynamicMicroDepotSandbox {
        resource_debits: Some(vec![MicroDepotProposalResourceDebit {
            resource_kind: ResourceKind::Data,
            units: 2,
        }]),
        ..DynamicMicroDepotSandbox::default()
    }));
    kernel.set_micro_depot_wasm_module_evaluator(
        "regional.micro_depot",
        "hash-install",
        "evaluate_quote",
        vec![0x00, 0x61, 0x73, 0x6d],
        ModuleLimits::unbounded(),
        sandbox,
    );
}

fn service_repair(kernel: &mut WorldKernel, facility_id: &str) -> WorldEventKind {
    kernel.submit_action(Action::ServiceMicroDepotRepair {
        agent_id: "agent-owner".to_string(),
        facility_id: facility_id.to_string(),
        target_id: "loc-install".to_string(),
        base_cost_class: MicroDepotPressureClass::Medium,
        base_risk_class: MicroDepotPressureClass::Low,
        blocker_type: Some("supply_missing".to_string()),
    });
    kernel.step().expect("resolve repair service").kind
}

#[test]
fn micro_depot_install_requires_ten_data_and_commissions_eight_inventory_sixteen_throughput() {
    let (mut insufficient, _) = install_fixture(9);
    let before = insufficient.model().clone();
    assert!(matches!(
        install(&mut insufficient, "depot-nine"),
        WorldEventKind::ActionRejected { .. }
    ));
    assert_eq!(
        insufficient.model(),
        &before,
        "failed install must be atomic"
    );

    let (mut kernel, base) = install_fixture(10);
    let event = install(&mut kernel, "depot-ten");
    match event {
        WorldEventKind::MicroDepotInstalled {
            install_cost_resources,
            measured_supply_schema_version,
            supported_resource_kinds,
            ..
        } => {
            assert_eq!(
                install_cost_resources,
                vec![MicroDepotResourceDebit {
                    kind: ResourceKind::Data,
                    amount: 10,
                }]
            );
            assert_eq!(measured_supply_schema_version, 2);
            assert_eq!(supported_resource_kinds, vec!["data"]);
        }
        other => panic!("unexpected install event: {other:?}"),
    }
    assert_eq!(owner_data(&kernel), 0);
    let depot = kernel
        .model()
        .regional_infrastructure
        .get("depot-ten")
        .unwrap();
    assert_eq!(depot.available_units_by_kind.get("data"), Some(&8));
    assert_eq!(depot.throughput_limit_units_per_epoch, 16);
    assert_eq!(depot.throughput_remaining_units, 16);

    let replayed = WorldKernel::replay_from_snapshot(base, kernel.journal_snapshot())
        .expect("replay commissioned install");
    assert_eq!(replayed.model(), kernel.model());
}

#[test]
fn micro_depot_reclaim_destroys_supply_and_reinstall_is_fresh_non_subsidized_cycle() {
    let (mut short_owner, _) = install_fixture(19);
    assert!(matches!(
        install(&mut short_owner, "depot-short"),
        WorldEventKind::MicroDepotInstalled { .. }
    ));
    assert!(matches!(
        reclaim(&mut short_owner, "depot-short"),
        WorldEventKind::MicroDepotReclaimed { .. }
    ));
    assert_eq!(owner_data(&short_owner), 9);
    let before_failed_reinstall = short_owner.model().clone();
    assert!(matches!(
        install(&mut short_owner, "depot-short-reinstall"),
        WorldEventKind::ActionRejected { .. }
    ));
    assert_eq!(short_owner.model(), &before_failed_reinstall);

    let (mut kernel, base) = install_fixture(30);

    for cycle in 0..3 {
        let facility_id = format!("depot-cycle-{cycle}");
        assert!(matches!(
            install(&mut kernel, &facility_id),
            WorldEventKind::MicroDepotInstalled { .. }
        ));
        let facility = kernel
            .model()
            .regional_infrastructure
            .get(&facility_id)
            .unwrap();
        assert_eq!(facility.available_units_by_kind.get("data"), Some(&8));
        assert_eq!(facility.throughput_remaining_units, 16);
        assert!(matches!(
            reclaim(&mut kernel, &facility_id),
            WorldEventKind::MicroDepotReclaimed { .. }
        ));
        assert!(
            !kernel
                .model()
                .regional_infrastructure
                .contains_key(&facility_id)
        );
        assert_eq!(owner_data(&kernel), 30 - (cycle + 1) * 10);
    }

    assert_eq!(owner_data(&kernel), 0, "reclaim must refund zero");
    assert!(matches!(
        install(&mut kernel, "depot-no-subsidy"),
        WorldEventKind::ActionRejected { .. }
    ));
    assert!(kernel.model().regional_infrastructure.is_empty());

    let replayed = WorldKernel::replay_from_snapshot(base, kernel.journal_snapshot())
        .expect("replay repeated install/reclaim cycles");
    assert_eq!(replayed.model(), kernel.model());
}

#[test]
fn micro_depot_reachable_exhaustion_rejects_upkeep_then_reclaims_and_reinstalls_fresh() {
    let (mut kernel, _) = install_fixture(20);
    assert!(matches!(
        install(&mut kernel, "depot-lifecycle"),
        WorldEventKind::MicroDepotInstalled { .. }
    ));
    assert_eq!(
        owner_data(&kernel),
        10,
        "owner retains enough funds for upkeep"
    );
    let initial_snapshot = kernel
        .micro_depot_player_facility_snapshots()
        .pop()
        .expect("commissioned depot snapshot");
    assert_eq!(
        initial_snapshot.available_units_by_kind.get("data"),
        Some(&8)
    );
    assert_eq!(initial_snapshot.throughput_remaining_units, 16);
    configure_two_unit_service(&mut kernel);

    for expected in [(6, 14), (4, 12), (2, 10), (0, 8)] {
        assert!(matches!(
            service_repair(&mut kernel, "depot-lifecycle"),
            WorldEventKind::MicroDepotServiceApplied { .. }
        ));
        let post_service_snapshot = kernel
            .micro_depot_player_facility_snapshots()
            .pop()
            .expect("post-service depot snapshot");
        assert_eq!(
            post_service_snapshot.available_units_by_kind.get("data"),
            Some(&expected.0)
        );
        assert_eq!(post_service_snapshot.throughput_remaining_units, expected.1);
    }
    let exhausted = kernel
        .model()
        .regional_infrastructure
        .get("depot-lifecycle")
        .expect("exhausted depot remains")
        .clone();
    assert_eq!(exhausted.available_units_by_kind.get("data"), Some(&0));
    assert_eq!(exhausted.throughput_remaining_units, 8);

    let depleted_snapshot = kernel
        .micro_depot_player_facility_snapshots()
        .pop()
        .expect("depleted depot snapshot");
    assert_eq!(
        depleted_snapshot.available_units_by_kind.get("data"),
        Some(&0)
    );
    assert_eq!(depleted_snapshot.throughput_remaining_units, 8);

    let owner_before_service_rejection = kernel
        .model()
        .agents
        .get("agent-owner")
        .expect("owner before rejected fifth service")
        .clone();
    let journal_before_service_rejection = kernel.journal_snapshot();
    let rejected_service = service_repair(&mut kernel, "depot-lifecycle");
    let rejection_json =
        serde_json::to_string(&rejected_service).expect("serialize fifth service rejection");
    assert!(matches!(
        rejected_service,
        WorldEventKind::ActionRejected { .. }
    ));
    assert!(rejection_json.contains("DEPOT_INVENTORY_INSUFFICIENT"));
    assert!(rejection_json.contains("available_units=0"));
    assert!(!rejection_json.contains("DEPOT_THROUGHPUT_EXHAUSTED"));
    assert_eq!(
        kernel
            .model()
            .agents
            .get("agent-owner")
            .expect("owner after rejected fifth service"),
        &owner_before_service_rejection,
        "rejected fifth service must preserve owner business state"
    );
    assert_eq!(
        kernel
            .model()
            .regional_infrastructure
            .get("depot-lifecycle")
            .expect("depot after rejected fifth service"),
        &exhausted,
        "rejected fifth service must preserve facility business state"
    );
    let journal_after_service_rejection = kernel.journal_snapshot();
    assert_eq!(
        &journal_after_service_rejection.events[..journal_before_service_rejection.events.len()],
        journal_before_service_rejection.events.as_slice(),
        "rejected fifth service must not rewrite canonical history"
    );
    assert_eq!(
        journal_after_service_rejection.events.len(),
        journal_before_service_rejection.events.len() + 1
    );
    assert!(matches!(
        journal_after_service_rejection
            .events
            .last()
            .map(|event| &event.kind),
        Some(WorldEventKind::ActionRejected { .. })
    ));

    let owner_before_upkeep = owner_data(&kernel);
    let journal_before_upkeep = kernel.journal_snapshot();
    kernel.submit_action(Action::PayMicroDepotUpkeep {
        agent_id: "agent-owner".to_string(),
        facility_id: "depot-lifecycle".to_string(),
    });
    let upkeep = kernel.step().expect("resolve exhausted upkeep");
    assert!(matches!(upkeep.kind, WorldEventKind::ActionRejected { .. }));
    assert_eq!(owner_data(&kernel), owner_before_upkeep);
    assert_eq!(
        kernel
            .model()
            .regional_infrastructure
            .get("depot-lifecycle")
            .expect("rejected upkeep preserves depot"),
        &exhausted
    );
    let journal_after_upkeep = kernel.journal_snapshot();
    assert_eq!(
        &journal_after_upkeep.events[..journal_before_upkeep.events.len()],
        journal_before_upkeep.events.as_slice(),
        "rejected upkeep must not rewrite canonical history"
    );
    assert_eq!(
        journal_after_upkeep.events.len(),
        journal_before_upkeep.events.len() + 1
    );
    assert!(matches!(
        journal_after_upkeep.events.last().map(|event| &event.kind),
        Some(WorldEventKind::ActionRejected { .. })
    ));

    let snapshot = kernel
        .micro_depot_player_facility_snapshots()
        .pop()
        .expect("exhausted depot snapshot");
    assert!(
        !snapshot
            .available_actions
            .contains(&"pay_micro_depot_upkeep".to_string())
    );
    assert_eq!(
        snapshot.available_actions,
        vec!["reclaim_micro_depot".to_string()]
    );

    let owner_before_reclaim = owner_data(&kernel);
    assert!(matches!(
        reclaim(&mut kernel, "depot-lifecycle"),
        WorldEventKind::MicroDepotReclaimed { .. }
    ));
    assert_eq!(
        owner_data(&kernel),
        owner_before_reclaim,
        "reclaim refunds zero"
    );

    assert!(matches!(
        install(&mut kernel, "depot-lifecycle"),
        WorldEventKind::MicroDepotInstalled { .. }
    ));
    assert_eq!(owner_data(&kernel), 0, "reinstall charges a fresh ten data");
    let reinstalled = kernel
        .model()
        .regional_infrastructure
        .get("depot-lifecycle")
        .expect("reinstalled depot");
    assert_eq!(reinstalled.available_units_by_kind.get("data"), Some(&8));
    assert_eq!(reinstalled.throughput_remaining_units, 16);
}

#[test]
fn micro_depot_v2_install_accepts_exactly_canonical_data_kind_without_mutating_rejections() {
    let cases = [
        (vec!["data"], true),
        (vec![], false),
        (vec!["hardware"], false),
        (vec!["data", "hardware"], false),
        (vec!["data", "data"], false),
        (vec!["Data"], false),
    ];

    for (index, (kinds, accepted)) in cases.into_iter().enumerate() {
        let (mut kernel, _) = install_fixture(10);
        let before = kernel.model().clone();
        kernel.submit_action(install_action(
            &format!("depot-kind-{index}"),
            kinds.clone(),
        ));
        let event = kernel.step().expect("resolve supported-kind case").kind;
        if accepted {
            assert!(
                matches!(event, WorldEventKind::MicroDepotInstalled { .. }),
                "{kinds:?}"
            );
        } else {
            assert!(
                matches!(event, WorldEventKind::ActionRejected { .. }),
                "{kinds:?}"
            );
            assert_eq!(kernel.model(), &before, "rejected {kinds:?} mutated state");
        }
    }
}
