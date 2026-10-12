use oasis7_agent_api::game::*;
use serde_json::json;
fn request() -> GameGoalPublishV1 {
    GameGoalPublishV1 {
        api_rev: 1,
        world_id: "world".into(),
        genesis_digest: "genesis".into(),
        task_id: "task".into(),
        agent_id: "agent".into(),
        owner_public_key: "ab".repeat(32),
        owner_binding: "owner".into(),
        owner_generation: DecimalU64(1),
        branch_id: "main".into(),
        reorg_epoch: DecimalU64(0),
        expected_task_revision: DecimalU64(1),
        expected_latest_goal_revision: DecimalU64(0),
        goal_revision: DecimalU64(1),
        client_operation_id: "publish".into(),
        prepare_operation_id: "prepare".into(),
        goal_ref: "cd".repeat(32),
    }
}
#[test]
fn goal_publish_metadata_has_no_private_body_and_strict_codec() {
    let r = request();
    r.validate().unwrap();
    let bytes = serde_json::to_vec(&r).unwrap();
    assert_eq!(parse_request::<GameGoalPublishV1>(&bytes).unwrap(), r);
    let valid = serde_json::to_value(r).unwrap();
    for field in ["content", "goal_digest", "signature_hex", "salt"] {
        let mut value = valid.clone();
        value[field] = json!("private");
        assert!(parse_request::<GameGoalPublishV1>(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    for field in [
        "goal_revision",
        "expected_task_revision",
        "expected_latest_goal_revision",
        "owner_generation",
        "reorg_epoch",
    ] {
        for bad in [json!(1), json!("01"), json!("-1")] {
            let mut v = valid.clone();
            v[field] = bad;
            assert!(parse_request::<GameGoalPublishV1>(&serde_json::to_vec(&v).unwrap()).is_err());
        }
    }
    let mut r = request();
    r.expected_latest_goal_revision = DecimalU64(u64::MAX);
    r.goal_revision = DecimalU64(u64::MAX);
    assert!(r.validate().is_err());
    r = request();
    r.expected_task_revision = DecimalU64(u64::MAX);
    assert!(r.validate().is_err());
    for field in ["goal_ref", "owner_public_key"] {
        for invalid in ["AA".repeat(32), "gg".repeat(32), "".into()] {
            let mut v = valid.clone();
            v[field] = json!(invalid);
            assert!(
                serde_json::from_value::<GameGoalPublishV1>(v)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
    for field in [
        "prepare_operation_id",
        "client_operation_id",
        "task_id",
        "world_id",
        "genesis_digest",
        "agent_id",
        "owner_binding",
        "branch_id",
    ] {
        for invalid in [" ".to_owned(), "x".repeat(257)] {
            let mut v = valid.clone();
            v[field] = json!(invalid);
            assert!(
                serde_json::from_value::<GameGoalPublishV1>(v)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
}
#[test]
fn goal_reference_requires_actual_height_and_public_identifiers() {
    let mut reference = GameGoalReferenceV1 {
        goal_revision: DecimalU64(1),
        goal_ref: "ab".repeat(32),
        prepare_operation_id: "prepare".into(),
        publish_operation_id: "publish".into(),
        activation_committed_height: DecimalU64(7),
    };
    reference.validate().unwrap();
    reference.activation_committed_height = DecimalU64(0);
    assert!(reference.validate().is_err());
}
