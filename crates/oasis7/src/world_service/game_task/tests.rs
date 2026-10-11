use super::*;
use crate::runtime::{Action, WorldState};
fn fixture() -> (World, String) {
    let private = hex::encode([71u8; 32]);
    let public = sign_read_request("fixture", (), &private)
        .unwrap()
        .subject_public_key;
    let mut world = World::new_with_state(WorldState::default());
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent".into(),
        pos: crate::GeoPos::new(0, 0, 0),
    });
    world.step().unwrap();
    world.submit_action(Action::ClaimStarterOc {
        agent_id: "agent".into(),
        player_id: "owner".into(),
        public_key: Some(public),
    });
    world.step().unwrap();
    world
        .install_capability_agent_identity("agent", "owner", 1)
        .unwrap();
    world
        .bind_cognition_runtime("world", "main", 0, None, "pending", 0)
        .unwrap();
    (world, private)
}
fn request(private: &str, id: &str) -> SubmitIntentRequest<WorldServicePayloadV1> {
    let create = GameTaskCreateV1 {
        api_rev: 1,
        client_operation_id: id.into(),
        world_id: "world".into(),
        genesis_digest: "fixture".into(),
        branch_id: "main".into(),
        reorg_epoch: DecimalU64(0),
        agent_id: "agent".into(),
        owner_binding: "owner".into(),
        owner_generation: DecimalU64(1),
    };
    let payload = WorldServicePayloadV1::GameTaskCreate(
        sign_read_request(GAME_TASK_CREATE_DOMAIN, create, private).unwrap(),
    );
    SubmitIntentRequest {
        contract_version: 1,
        correlation: derive_correlation(
            WorldIdentity {
                world_id: "world".into(),
                genesis_digest: "fixture".into(),
            },
            &payload,
        )
        .unwrap(),
        deadline_unix_ms: None,
        signed_payload: payload,
    }
}
fn result(
    world: &World,
    request: SubmitIntentRequest<WorldServicePayloadV1>,
) -> CanonicalIntentResultV1 {
    CanonicalIntentResultV1 {
        receipt: prepare_create(world, &request, []).unwrap(),
        request,
        action_id: 1,
        committed_height: 1,
        rejected: None,
    }
}
#[test]
fn game_task_bootstrap_persists_suspended_and_rejects_conflicting_create() {
    let (mut world, private) = fixture();
    let before = serde_json::to_value(world.snapshot()).unwrap();
    let first = result(&world, request(&private, "first"));
    assert_eq!(serde_json::to_value(world.snapshot()).unwrap(), before);
    let second = request(&private, "second");
    assert!(
        prepare_create(&world, &second, [&first])
            .unwrap_err()
            .contains("within block")
    );
    world.record_world_service_result(first.clone()).unwrap();
    world.record_world_service_result(first).unwrap(); // immutable exact publication replay
    assert!(
        prepare_create(&world, &second, [])
            .unwrap_err()
            .contains("already exists")
    );
    let snapshot = world.snapshot_with_chain_resource_context(
        crate::runtime::ChainResourceDerivationContext {
            world_id: "world",
            chain_id: "fixture-chain",
            genesis_ref: None,
            created_at_height: world.state().time,
            manifest_height: world.state().time,
            commit_block_hash: None,
            tick: world.state().time,
        },
        "fixture",
        "fixture",
    );
    let key = correlation::key_digest(&request(&private, "first").correlation.key).unwrap();
    let mut corrupt_snapshot = serde_json::to_value(&snapshot).unwrap();
    fn corrupt(value: &mut serde_json::Value, key: &str) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(result) = map.get_mut(key) {
                    result["receipt"]["executor_epoch"] = "9".into();
                    return true;
                }
                map.values_mut().any(|child| corrupt(child, key))
            }
            serde_json::Value::Array(values) => values.iter_mut().any(|child| corrupt(child, key)),
            _ => false,
        }
    }
    assert!(corrupt(&mut corrupt_snapshot, &key));
    assert!(
        World::from_snapshot(
            serde_json::from_value(corrupt_snapshot).unwrap(),
            world.journal().clone()
        )
        .is_err()
    );
    let restored = World::from_snapshot(snapshot, world.journal().clone()).unwrap();
    let control = current_control(&restored, "agent").unwrap().unwrap();
    assert_eq!(control.task_status, GameTaskStatusV1::Suspended);
    assert_eq!(control.executor_epoch, DecimalU64(0));
    assert_eq!(control.active_goal_revision, DecimalU64(0));
    assert!(control.executor_id.is_none());
    assert_eq!(control.created_operation_id, "first");
}
#[test]
fn game_task_owner_history_and_receipt_integrity_fail_closed() {
    let (mut world, private) = fixture();
    let original = request(&private, "first");
    for variant in 0..6 {
        let mut invalid = original.clone();
        let WorldServicePayloadV1::GameTaskCreate(signed) = &mut invalid.signed_payload else {
            unreachable!()
        };
        match variant {
            0 => signed.request.world_id = "foreign".into(),
            1 => signed.request.branch_id = "fork".into(),
            2 => signed.request.reorg_epoch = DecimalU64(1),
            3 => signed.request.owner_binding = "other".into(),
            4 => signed.request.owner_generation = DecimalU64(2),
            _ => signed.subject_public_key = "00".repeat(32),
        }
        if variant < 5 {
            *signed = sign_read_request(GAME_TASK_CREATE_DOMAIN, signed.request.clone(), &private)
                .unwrap();
        }
        assert!(validate_owner(&world, signed).is_err());
    }
    let mut corrupt = result(&world, original.clone());
    corrupt.receipt["executor_epoch"] = "1".into();
    assert!(world.record_world_service_result(corrupt).is_err());
    let mut foreign = original.clone();
    let WorldServicePayloadV1::GameTaskCreate(signed) = &mut foreign.signed_payload else {
        unreachable!()
    };
    signed.request.genesis_digest = "foreign".into();
    *signed = sign_read_request(GAME_TASK_CREATE_DOMAIN, signed.request.clone(), &private).unwrap();
    assert!(
        derive_correlation(
            original.correlation.key.world.clone(),
            &foreign.signed_payload
        )
        .is_err()
    );
    let mut changed = original.clone();
    let WorldServicePayloadV1::GameTaskCreate(signed) = &mut changed.signed_payload else {
        unreachable!()
    };
    signed.request.branch_id = "other".into();
    *signed = sign_read_request(GAME_TASK_CREATE_DOMAIN, signed.request.clone(), &private).unwrap();
    let changed_correlation = derive_correlation(
        original.correlation.key.world.clone(),
        &changed.signed_payload,
    )
    .unwrap();
    assert_eq!(changed_correlation.key, original.correlation.key);
    assert_ne!(
        changed_correlation.payload_digest,
        original.correlation.payload_digest
    );
    let accepted = result(&world, original);
    world.record_world_service_result(accepted).unwrap();
    world.invalidate_cognition_for_reorg(1).unwrap();
    world
        .bind_cognition_runtime("world", "fork", 1, None, "pending", 1)
        .unwrap();
    assert!(current_control(&world, "agent").unwrap().is_none());
    let mut fresh = request(&private, "after-fork");
    let WorldServicePayloadV1::GameTaskCreate(signed) = &mut fresh.signed_payload else {
        unreachable!()
    };
    signed.request.branch_id = "fork".into();
    signed.request.reorg_epoch = DecimalU64(1);
    *signed = sign_read_request(GAME_TASK_CREATE_DOMAIN, signed.request.clone(), &private).unwrap();
    fresh.correlation =
        derive_correlation(fresh.correlation.key.world.clone(), &fresh.signed_payload).unwrap();
    let accepted = result(&world, fresh);
    world.record_world_service_result(accepted).unwrap();
    assert_eq!(
        current_control(&world, "agent")
            .unwrap()
            .unwrap()
            .created_operation_id,
        "after-fork"
    );
}
