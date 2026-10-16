use super::*;
use oasis7_agent_api::game::{DecimalU64, GameExecutorBindingV1, GameGoalReferenceV1};
fn fence() -> GameTaskFenceV1 {
    GameTaskFenceV1 {
        world_id: "world".into(),
        genesis_digest: "genesis".into(),
        branch_id: "main".into(),
        reorg_epoch: DecimalU64(0),
        agent_id: "agent".into(),
        owner_binding: "player".into(),
        owner_generation: DecimalU64(1),
        owner_public_key: "a".repeat(64),
        task_id: "task".into(),
        task_revision: DecimalU64(1),
        task_status: GameTaskStatusV1::Active,
        latest_goal_revision: DecimalU64(1),
        active_goal_revision: DecimalU64(1),
        active_goal_reference: Some(GameGoalReferenceV1 {
            goal_revision: DecimalU64(1),
            goal_ref: "c".repeat(64),
            prepare_operation_id: "prepare".into(),
            publish_operation_id: "publish".into(),
            activation_committed_height: DecimalU64(1),
        }),
        executor_epoch: DecimalU64(1),
        executor_binding: Some(GameExecutorBindingV1 {
            executor_id: "executor".into(),
            executor_public_key: "b".repeat(64),
            delegation_generation: DecimalU64(1),
            executor_epoch: DecimalU64(1),
        }),
    }
}
#[test]
fn configured_device_requires_active_goal_and_exact_runtime_executor_signer() {
    let f = fence();
    let key = "b".repeat(64);
    assert!(validate_device_fence(&f, "world", "genesis", "agent", &key).is_ok());
    for (world, genesis, agent, signer) in [
        ("other", "genesis", "agent", key.as_str()),
        ("world", "other", "agent", key.as_str()),
        ("world", "genesis", "other", key.as_str()),
        ("world", "genesis", "agent", "wrong-key"),
    ] {
        assert!(validate_device_fence(&f, world, genesis, agent, signer).is_err());
    }
    let mut no_goal = f.clone();
    no_goal.latest_goal_revision = DecimalU64(0);
    no_goal.active_goal_revision = DecimalU64(0);
    no_goal.active_goal_reference = None;
    assert!(validate_device_fence(&no_goal, "world", "genesis", "agent", &key).is_err());
    let mut stopped = f;
    stopped.task_status = GameTaskStatusV1::Suspended;
    stopped.executor_binding = None;
    assert!(validate_device_fence(&stopped, "world", "genesis", "agent", &key).is_err());
}
#[test]
fn run_namespace_is_stable_without_session_connection_or_generation_and_binds_full_fence() {
    let f = fence();
    let key = "b".repeat(64);
    let original = run_identity(&key, &f);
    assert_eq!(original, run_identity(&key, &f));
    assert!(original.len() <= 128);
    for field in ["branch", "owner", "goal", "executor", "task"] {
        let mut changed = f.clone();
        match field {
            "branch" => changed.branch_id = "fork".into(),
            "owner" => changed.owner_generation = DecimalU64(2),
            "goal" => changed.active_goal_reference.as_mut().unwrap().goal_ref = "d".repeat(64),
            "executor" => changed.executor_epoch = DecimalU64(2),
            "task" => changed.task_revision = DecimalU64(2),
            _ => unreachable!(),
        }
        assert_ne!(original, run_identity(&key, &changed), "{field}");
    }
}
#[test]
fn no_operator_devices_never_resolve_browser_supplied_authority() {
    let registry = Executors::load(&[], Arc::new(Mutex::new(None))).unwrap();
    let p = Principal {
        account_id: "a".into(),
        session_id: "s".into(),
        auth_ref: "not-authority".into(),
    };
    assert!(!registry.available(&p));
    assert!(registry.resolve(&p, "agent").is_none());
    assert!(!registry.authorize(&p, &target(&fence())));
}

#[test]
fn full_fence_digest_binds_authority_fields_omitted_from_partial_target() {
    let original = fence();
    let old = target(&original);
    assert!(!old.full_fence_digest.is_empty());
    for field in [
        "branch",
        "reorg",
        "genesis",
        "owner",
        "owner_generation",
        "owner_key",
        "task_revision",
        "latest_goal",
        "goal_reference",
        "delegation_generation",
    ] {
        let mut fresh = original.clone();
        match field {
            "branch" => fresh.branch_id = "fork".into(),
            "reorg" => fresh.reorg_epoch = DecimalU64(1),
            "genesis" => fresh.genesis_digest = "other-genesis".into(),
            "owner" => fresh.owner_binding = "other-player".into(),
            "owner_generation" => fresh.owner_generation = DecimalU64(2),
            "owner_key" => fresh.owner_public_key = "e".repeat(64),
            "task_revision" => fresh.task_revision = DecimalU64(2),
            "latest_goal" => fresh.latest_goal_revision = DecimalU64(2),
            "goal_reference" => {
                fresh.active_goal_reference.as_mut().unwrap().goal_ref = "d".repeat(64)
            }
            "delegation_generation" => {
                fresh
                    .executor_binding
                    .as_mut()
                    .unwrap()
                    .delegation_generation = DecimalU64(2)
            }
            _ => unreachable!(),
        }
        fresh.validate().unwrap();
        let mut next = target(&fresh);
        assert_ne!(next.full_fence_digest, old.full_fence_digest, "{field}");
        next.full_fence_digest.clone_from(&old.full_fence_digest);
        assert_eq!(next, old, "partial target omits {field}");
    }
}

#[cfg(unix)]
#[path = "executor_http_tests.rs"]
mod signed_http;
