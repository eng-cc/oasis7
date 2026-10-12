use oasis7_agent_api::{Digest32, game::*};
use serde_json::{Value, json};
fn proposal() -> GameGoalPrepareV1 {
    let content = GoalContentV1 {
        goal_text: "Complete private goal 正文".into(),
        soft_constraints: vec!["first".into(), "second".into()],
    };
    GameGoalPrepareV1 {
        api_rev: API_REVISION,
        world_id: "world".into(),
        genesis_digest: "genesis".into(),
        task_id: "task".into(),
        agent_id: "agent".into(),
        owner_public_key: "ab".repeat(32),
        owner_binding: "owner".into(),
        owner_generation: DecimalU64(1),
        branch_id: "branch".into(),
        reorg_epoch: DecimalU64(u64::MAX),
        expected_task_revision: DecimalU64(u64::MAX),
        expected_latest_goal_revision: DecimalU64(0),
        goal_revision: DecimalU64(1),
        client_operation_id: "operation".into(),
        client_nonce: "cd".repeat(32),
        goal_digest: game_goal_version_digest("task", DecimalU64(1), &content).unwrap(),
        content,
    }
}
#[test]
fn private_prepare_complete_codec_strict_decimals_and_unknown_fields() {
    let request = proposal();
    request.validate().unwrap();
    let bytes = serde_json::to_vec(&request).unwrap();
    let decoded: GameGoalPrepareV1 = parse_request(&bytes).unwrap();
    assert!(decoded == request);
    let valid = serde_json::to_value(request).unwrap();
    for field in [
        "owner_generation",
        "reorg_epoch",
        "expected_task_revision",
        "expected_latest_goal_revision",
        "goal_revision",
    ] {
        for invalid in [
            json!(1),
            json!("01"),
            json!("-1"),
            json!("18446744073709551616"),
        ] {
            let mut value = valid.clone();
            value[field] = invalid;
            assert!(
                parse_request::<GameGoalPrepareV1>(&serde_json::to_vec(&value).unwrap()).is_err()
            );
        }
    }
    for field in ["extra", "content.extra"] {
        let mut value = valid.clone();
        if field == "extra" {
            value[field] = json!(true)
        } else {
            value["content"]["extra"] = json!(true)
        };
        assert!(parse_request::<GameGoalPrepareV1>(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let duplicated = String::from_utf8(bytes)
        .unwrap()
        .replacen("{", "{\"api_rev\":1,", 1);
    assert!(parse_request::<GameGoalPrepareV1>(duplicated.as_bytes()).is_err());
}
#[test]
fn private_prepare_identity_revision_digest_nonce_and_content_bounds() {
    let valid = serde_json::to_value(proposal()).unwrap();
    for field in [
        "world_id",
        "genesis_digest",
        "task_id",
        "agent_id",
        "owner_binding",
        "branch_id",
        "client_operation_id",
    ] {
        for bad in [" ".to_owned(), "x".repeat(257)] {
            let mut value = valid.clone();
            value[field] = json!(bad);
            let request: GameGoalPrepareV1 = serde_json::from_value(value).unwrap();
            assert!(request.validate().is_err());
        }
    }
    for field in ["owner_public_key", "client_nonce"] {
        for bad in [
            "".to_owned(),
            "AB".repeat(32),
            "0x".to_owned() + &"ab".repeat(32),
            "gg".repeat(32),
        ] {
            let mut value = valid.clone();
            value[field] = json!(bad);
            let request: GameGoalPrepareV1 = serde_json::from_value(value).unwrap();
            assert!(request.validate().is_err());
        }
    }
    for (field, value) in [
        ("api_rev", json!(2)),
        ("owner_generation", json!("0")),
        ("expected_task_revision", json!("0")),
        ("goal_revision", json!("0")),
        ("expected_latest_goal_revision", json!(u64::MAX.to_string())),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        let request: GameGoalPrepareV1 = serde_json::from_value(bad).unwrap();
        assert!(request.validate().is_err());
    }
    let mut request = proposal();
    request.content.goal_text = "x".repeat(MAX_GOAL_CONTENT_BYTES);
    assert!(request.validate().is_err());
    request = proposal();
    request.content.soft_constraints.reverse();
    assert!(request.validate().is_err());
    request = proposal();
    request.goal_digest = Digest32("blake3:".to_owned() + &"00".repeat(32));
    assert!(request.validate().is_err());
    let mut request = proposal();
    request.client_operation_id = "x".repeat(256);
    request.validate().unwrap();
    let _: Value = serde_json::to_value(request).unwrap();
}

#[test]
fn private_prepare_latest_revision_overflow_and_private_debug_guard() {
    let mut request = proposal();
    request.expected_latest_goal_revision = DecimalU64(u64::MAX - 1);
    request.goal_revision = DecimalU64(u64::MAX);
    request.goal_digest =
        game_goal_version_digest(&request.task_id, request.goal_revision, &request.content)
            .unwrap();
    request.validate().unwrap();
    request.expected_latest_goal_revision = DecimalU64(u64::MAX);
    assert!(request.validate().is_err());
    // Ambiguous inference becomes a compile error if this private DTO gains Debug.
    trait AmbiguousIfDebug<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousIfDebug<()> for T {}
    impl<T: ?Sized + std::fmt::Debug> AmbiguousIfDebug<u8> for T {}
    let _ = <GameGoalPrepareV1 as AmbiguousIfDebug<_>>::marker;
}
