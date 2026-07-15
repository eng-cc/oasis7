use super::kernel_wasm_sandbox_bridge::{
    CapturingSandbox, empty_micro_depot_input, micro_depot_output_for,
};
use super::micro_depot_measured_supply::{
    installed_micro_depot_with_measured_supply, submit_measured_service,
};
use super::*;
use oasis7_wasm_abi::ModuleLimits;
use std::sync::{Arc, Mutex};

fn proposal(input: &MicroDepotEvalInput) -> MicroDepotProposal {
    MicroDepotProposal {
        action_id: input.action.action_id,
        facility_id: input.depot.facility_id.clone(),
        decision: MicroDepotDecision::Applicable,
        cost_delta_class: MicroDepotDeltaClass::MinorDecrease,
        risk_delta_class: MicroDepotDeltaClass::None,
        wait_delta_class: MicroDepotDeltaClass::None,
        blocker_change: None,
        consumed_resource_classes: Vec::new(),
        resource_debits: Vec::new(),
        evaluated_inventory_revision: input.depot.inventory_revision,
        evaluated_epoch: input.depot.current_epoch,
        explanation_code: "schema_compatibility".to_string(),
        proposal_hash: String::new(),
    }
}

fn evaluate_error(input: &MicroDepotEvalInput, mut proposal: MicroDepotProposal) -> String {
    proposal.proposal_hash = compute_micro_depot_proposal_hash(input, &proposal).unwrap();
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(
        micro_depot_output_for(proposal),
    ))));
    evaluate_micro_depot_quote_with_module(
        input,
        "regional.micro_depot",
        "hash-micro-depot",
        "evaluate_quote",
        vec![1],
        ModuleLimits::unbounded(),
        sandbox,
    )
    .expect_err("schema-mixed proposal must fail")
}

#[test]
fn micro_depot_v1_rejects_exact_resource_debits() {
    let input = empty_micro_depot_input(8);
    let mut proposal = proposal(&input);
    proposal.resource_debits = vec![MicroDepotProposalResourceDebit {
        resource_kind: ResourceKind::Data,
        units: 1,
    }];
    assert!(
        evaluate_error(&input, proposal)
            .contains("micro_depot.eval.v1 requires class-only resource consumption")
    );
}

#[test]
fn micro_depot_v2_rejects_legacy_consumed_resource_classes() {
    let mut input = empty_micro_depot_input(9);
    input.schema_version = "micro_depot.eval.v2".to_string();
    let mut proposal = proposal(&input);
    proposal.consumed_resource_classes = vec![MicroDepotConsumedResourceClass {
        resource_kind: "data".to_string(),
        amount_class: MicroDepotPressureClass::Low,
    }];
    proposal.resource_debits = vec![MicroDepotProposalResourceDebit {
        resource_kind: ResourceKind::Data,
        units: 1,
    }];
    assert!(
        evaluate_error(&input, proposal)
            .contains("micro_depot.eval.v2 requires exact-only resource debits")
    );
}

#[test]
fn micro_depot_historical_kind_amount_debit_decodes() {
    let decoded: MicroDepotResourceDebit =
        serde_json::from_value(serde_json::json!({ "kind": "data", "amount": 2 })).unwrap();
    assert_eq!(
        decoded,
        MicroDepotResourceDebit {
            kind: ResourceKind::Data,
            amount: 2
        }
    );
}

#[test]
fn micro_depot_eval_v1_matches_origin_main_golden_hash_and_action_payload() {
    let input = empty_micro_depot_input(42);
    let mut historical_proposal = proposal(&input);
    historical_proposal.consumed_resource_classes = vec![MicroDepotConsumedResourceClass {
        resource_kind: "data".to_string(),
        amount_class: MicroDepotPressureClass::Low,
    }];
    historical_proposal.explanation_code = "origin_main_golden".to_string();

    let actual_hash = compute_micro_depot_proposal_hash(&input, &historical_proposal).unwrap();
    assert_eq!(
        actual_hash,
        "sha256:bce86fce70e024232fc8bf2199e51fe23af9c9a34b54f33d1a9edc5aab83522a"
    );
    historical_proposal.proposal_hash = actual_hash;

    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(
        micro_depot_output_for(historical_proposal),
    ))));
    evaluate_micro_depot_quote_with_module(
        &input,
        "regional.micro_depot",
        "hash-micro-depot",
        "evaluate_quote",
        vec![1],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    )
    .expect("new runtime accepts the historical v1 hash and proposal shape");

    let request = &sandbox.lock().unwrap().requests[0];
    let call_input: oasis7_wasm_abi::ModuleCallInput =
        serde_cbor::from_slice(&request.input).expect("decode module call wrapper");
    let action = call_input.action.expect("historical action payload");
    assert_eq!(
        hex::encode(&action),
        "d9d9f7a7656465706f74a766737461747573666163746976656b666163696c6974795f69646b6465706f742d616c7068616b75706b6565705f70616964f56e63617061636974795f636c617373666d656469756d6e6f776e65725f636c61696d5f696467636c61696d2d3171736572766963655f7261646975735f636d1a0003d0907818737570706f727465645f7265736f757263655f6b696e647382686861726477617265646461746166616374696f6ea6646b696e646672657061697269616374696f6e5f6964182a697461726765745f696469666163746f72792d376c626c6f636b65725f747970656f6c6f63616c5f70617274735f6761706f626173655f636f73745f636c61737364686967686f626173655f7269736b5f636c617373666d656469756d66706c61796572a268636c61696d5f696467636c61696d2d316a6163636f756e745f696468706c617965722d3167636f6e74657874a4727265736f757263655f6761705f636c617373646869676874726f7574655f70726573737572655f636c617373666d656469756d7564697374616e63655f746f5f7461726765745f636d1a0001e84875726567696f6e5f70726573737572655f636c617373666d656469756d696d6f64756c655f696474726567696f6e616c2e6d6963726f5f6465706f746e6d6f64756c655f76657273696f6e65302e312e306e736368656d615f76657273696f6e736d6963726f5f6465706f742e6576616c2e7631"
    );
    let value: serde_cbor::Value = serde_cbor::from_slice(&action).unwrap();
    let debug = format!("{value:?}");
    for v2_only in [
        "inventory_revision",
        "current_epoch",
        "available_units_by_kind",
        "throughput_limit_units_per_epoch",
        "throughput_remaining_units",
    ] {
        assert!(!debug.contains(v2_only), "legacy action leaked {v2_only}");
    }
}

#[test]
fn micro_depot_proposal_hash_ignores_map_insertion_order() {
    let mut left = empty_micro_depot_input(10);
    left.schema_version = "micro_depot.eval.v2".to_string();
    left.depot.available_units_by_kind.clear();
    left.depot
        .available_units_by_kind
        .insert("data".to_string(), 3);
    left.depot
        .available_units_by_kind
        .insert("electricity".to_string(), 4);
    let mut right = left.clone();
    right.depot.available_units_by_kind.clear();
    right
        .depot
        .available_units_by_kind
        .insert("electricity".to_string(), 4);
    right
        .depot
        .available_units_by_kind
        .insert("data".to_string(), 3);
    let mut proposal = proposal(&left);
    proposal.resource_debits = vec![MicroDepotProposalResourceDebit {
        resource_kind: ResourceKind::Data,
        units: 1,
    }];
    assert_eq!(
        compute_micro_depot_proposal_hash(&left, &proposal).unwrap(),
        compute_micro_depot_proposal_hash(&right, &proposal).unwrap()
    );
}

#[test]
fn micro_depot_live_input_rejects_throughput_remaining_above_limit() {
    let mut input = empty_micro_depot_input(11);
    input.depot.throughput_limit_units_per_epoch = 5;
    input.depot.throughput_remaining_units = 6;
    let error = evaluate_error(&input, proposal(&input));
    assert!(error.contains("throughput remaining exceeds epoch limit"));
}

#[test]
fn micro_depot_v2_replay_without_exact_debits_fails_closed() {
    let mut kernel = installed_micro_depot_with_measured_supply(5, 4, 2);
    let base = kernel.snapshot();
    submit_measured_service(&mut kernel);
    let mut journal = kernel.journal_snapshot();
    let service = journal.events.last_mut().unwrap();
    match &mut service.kind {
        WorldEventKind::MicroDepotServiceApplied {
            resource_debits, ..
        } => {
            resource_debits.clear();
        }
        other => panic!("expected service event, got {other:?}"),
    }
    let err = WorldKernel::replay_from_snapshot(base, journal)
        .expect_err("v2 replay without exact debits must fail closed");
    assert!(format!("{err:?}").contains("v2 service requires exact resource debits"));
}

#[test]
fn micro_depot_v2_replay_rejects_conflicting_legacy_consumed_resources() {
    let mut kernel = installed_micro_depot_with_measured_supply(5, 4, 2);
    let base = kernel.snapshot();
    submit_measured_service(&mut kernel);
    let mut journal = kernel.journal_snapshot();
    let service = journal.events.last_mut().unwrap();
    match &mut service.kind {
        WorldEventKind::MicroDepotServiceApplied {
            consumed_resources, ..
        } => {
            *consumed_resources = vec![MicroDepotResourceDebit {
                kind: ResourceKind::Electricity,
                amount: 2,
            }];
        }
        other => panic!("expected service event, got {other:?}"),
    }
    let err = WorldKernel::replay_from_snapshot(base, journal)
        .expect_err("conflicting v2 legacy debit mirror must fail closed");
    assert!(
        format!("{err:?}").contains("v2 consumed resources do not match exact resource debits")
    );
}

#[derive(Clone, Copy, Debug)]
enum InvalidV2Receipt {
    NegativeDebit,
    DuplicateKind,
    ReorderedKind,
    ExceedsThroughput,
    NegativeBefore,
    NegativeAfter,
    NegativeLimit,
}

fn invalid_v2_replay_error(case: InvalidV2Receipt) -> String {
    let mut kernel = installed_micro_depot_with_measured_supply(5, 4, 2);
    let mut base = kernel.snapshot();
    if matches!(case, InvalidV2Receipt::NegativeLimit) {
        let mut value = serde_json::to_value(&base).unwrap();
        value["model"]["regional_infrastructure"]["depot-measured"]["throughput_limit_units_per_epoch"] =
            serde_json::json!(-1);
        base = serde_json::from_value(value).unwrap();
    }
    submit_measured_service(&mut kernel);
    let mut journal = kernel.journal_snapshot();
    let service = journal.events.last_mut().unwrap();
    let WorldEventKind::MicroDepotServiceApplied {
        consumed_resources,
        resource_debits,
        throughput_units_before,
        throughput_units_after,
        ..
    } = &mut service.kind
    else {
        panic!("expected service event");
    };
    match case {
        InvalidV2Receipt::NegativeDebit => {
            consumed_resources[0].amount = -1;
            resource_debits[0].units = -1;
        }
        InvalidV2Receipt::DuplicateKind => {
            consumed_resources.push(MicroDepotResourceDebit {
                kind: ResourceKind::Data,
                amount: 1,
            });
            resource_debits.push(MicroDepotProposalResourceDebit {
                resource_kind: ResourceKind::Data,
                units: 1,
            });
        }
        InvalidV2Receipt::ReorderedKind => {
            *consumed_resources = vec![
                MicroDepotResourceDebit {
                    kind: ResourceKind::Electricity,
                    amount: 1,
                },
                MicroDepotResourceDebit {
                    kind: ResourceKind::Data,
                    amount: 1,
                },
            ];
            *resource_debits = vec![
                MicroDepotProposalResourceDebit {
                    resource_kind: ResourceKind::Electricity,
                    units: 1,
                },
                MicroDepotProposalResourceDebit {
                    resource_kind: ResourceKind::Data,
                    units: 1,
                },
            ];
        }
        InvalidV2Receipt::ExceedsThroughput => {
            consumed_resources[0].amount = 5;
            resource_debits[0].units = 5;
        }
        InvalidV2Receipt::NegativeBefore => *throughput_units_before = -1,
        InvalidV2Receipt::NegativeAfter => *throughput_units_after = -1,
        InvalidV2Receipt::NegativeLimit => {}
    }
    format!(
        "{:?}",
        WorldKernel::replay_from_snapshot(base, journal)
            .expect_err("invalid v2 receipt must fail closed")
    )
}

#[test]
fn micro_depot_v2_replay_rejects_invalid_measured_receipt_matrix() {
    let cases = [
        (InvalidV2Receipt::NegativeDebit, "units must be positive"),
        (InvalidV2Receipt::DuplicateKind, "unique kinds"),
        (InvalidV2Receipt::ReorderedKind, "strict canonical order"),
        (InvalidV2Receipt::ExceedsThroughput, "exceed throughput"),
        (InvalidV2Receipt::NegativeBefore, "negative measured supply"),
        (InvalidV2Receipt::NegativeAfter, "negative measured supply"),
        (
            InvalidV2Receipt::NegativeLimit,
            "throughput limit is negative",
        ),
    ];
    for (case, expected) in cases {
        let error = invalid_v2_replay_error(case);
        assert!(error.contains(expected), "{case:?}: {error}");
    }
}

#[test]
fn corrupted_snapshot_and_matching_v2_receipt_above_limit_fail_closed() {
    let mut kernel = installed_micro_depot_with_measured_supply(5, 4, 2);
    let base = kernel.snapshot();
    let mut value = serde_json::to_value(base).unwrap();
    let depot = &mut value["model"]["regional_infrastructure"]["depot-measured"];
    depot["throughput_limit_units_per_epoch"] = serde_json::json!(16);
    depot["throughput_remaining_units"] = serde_json::json!(100);
    let base = serde_json::from_value(value).unwrap();

    submit_measured_service(&mut kernel);
    let mut journal = kernel.journal_snapshot();
    let WorldEventKind::MicroDepotServiceApplied {
        throughput_units_before,
        throughput_units_after,
        ..
    } = &mut journal.events.last_mut().unwrap().kind
    else {
        panic!("expected service event");
    };
    *throughput_units_before = 100;
    *throughput_units_after = 98;

    let err = WorldKernel::replay_from_snapshot(base, journal)
        .expect_err("matching corrupted throughput above limit must fail closed");
    assert!(format!("{err:?}").contains("throughput balance exceeds epoch limit"));
}

#[test]
fn legacy_snapshot_and_v1_install_journal_converge_to_zero_measured_supply() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-legacy".to_string(),
        name: "Legacy Hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().unwrap();
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-legacy".to_string(),
        location_id: "loc-legacy".to_string(),
    });
    kernel.step().unwrap();
    seed_owner_resource(
        &mut kernel,
        ResourceOwner::Agent {
            agent_id: "agent-legacy".to_string(),
        },
        ResourceKind::Data,
        10,
    );
    let base = kernel.snapshot();
    let legacy_base = serde_json::from_value(serde_json::to_value(&base).unwrap()).unwrap();
    kernel.submit_action(Action::InstallMicroDepot {
        installer_agent_id: "agent-legacy".to_string(),
        facility_id: "depot-legacy".to_string(),
        location_id: "loc-legacy".to_string(),
        owner_claim_id: "claim-legacy".to_string(),
        regional_blocker_receipt_id: "legacy-receipt".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.1.0".to_string(),
        wasm_hash: "hash-legacy".to_string(),
        entrypoint: "evaluate_quote".to_string(),
        service_radius_cm: 10,
        supported_resource_kinds: vec!["data".to_string()],
    });
    kernel.step().unwrap();
    let mut journal = kernel.journal_snapshot();
    let install = journal.events.last_mut().unwrap();
    match &mut install.kind {
        WorldEventKind::MicroDepotInstalled {
            measured_supply_schema_version: version,
            ..
        } => {
            *version = 0;
        }
        other => panic!("expected install event, got {other:?}"),
    }
    let current = WorldKernel::replay_from_snapshot(base, journal.clone()).unwrap();
    let legacy = WorldKernel::replay_from_snapshot(legacy_base, journal).unwrap();
    assert_eq!(legacy.model(), current.model());
    let depot = legacy
        .model()
        .regional_infrastructure
        .get("depot-legacy")
        .unwrap();
    assert!(depot.available_units_by_kind.is_empty());
    assert_eq!(depot.throughput_limit_units_per_epoch, 0);
    assert_eq!(depot.throughput_remaining_units, 0);
}

#[test]
fn unknown_install_measured_supply_schema_fails_closed() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-unknown".to_string(),
        name: "Unknown".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().unwrap();
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-unknown".to_string(),
        location_id: "loc-unknown".to_string(),
    });
    kernel.step().unwrap();
    seed_owner_resource(
        &mut kernel,
        ResourceOwner::Agent {
            agent_id: "agent-unknown".to_string(),
        },
        ResourceKind::Data,
        10,
    );
    let base = kernel.snapshot();
    kernel.submit_action(Action::InstallMicroDepot {
        installer_agent_id: "agent-unknown".to_string(),
        facility_id: "depot-unknown".to_string(),
        location_id: "loc-unknown".to_string(),
        owner_claim_id: "claim".to_string(),
        regional_blocker_receipt_id: "receipt".to_string(),
        module_id: "module".to_string(),
        module_version: "1".to_string(),
        wasm_hash: "hash".to_string(),
        entrypoint: "eval".to_string(),
        service_radius_cm: 1,
        supported_resource_kinds: vec!["data".to_string()],
    });
    kernel.step().unwrap();
    let mut journal = kernel.journal_snapshot();
    let WorldEventKind::MicroDepotInstalled {
        measured_supply_schema_version,
        ..
    } = &mut journal.events.last_mut().unwrap().kind
    else {
        panic!("expected install")
    };
    *measured_supply_schema_version = 3;
    let err = WorldKernel::replay_from_snapshot(base, journal)
        .expect_err("unknown measured supply schema must fail closed");
    assert!(format!("{err:?}").contains("unsupported micro_depot measured supply schema"));
}
