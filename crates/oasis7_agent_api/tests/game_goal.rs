use oasis7_agent_api::game::*;
use serde_json::json;

fn content() -> GoalContentV1 {
    GoalContentV1 {
        goal_text: "  建造仓库\n保留原文  ".into(),
        soft_constraints: vec!["avoid the river".into(), "reserve 10 units".into()],
    }
}

fn projection() -> GoalVersionProjectionV1 {
    let content = content();
    GoalVersionProjectionV1 {
        schema_revision: 1,
        task_id: "task-1".into(),
        goal_revision: DecimalU64(u64::MAX),
        goal_digest: game_goal_version_digest("task-1", DecimalU64(u64::MAX), &content).unwrap(),
        activation_commit_ref: "commit-1".into(),
        content,
    }
}

#[test]
fn game_goal_version_codec_vector() {
    assert_eq!(
        projection().goal_digest.as_str(),
        "blake3:a63d96068f9e82600ad2fb6bdaa0e60b3f861e18ca811247f0b674969c7f7a78"
    );
}

#[test]
fn game_goal_full_body_roundtrip_preserves_unicode_whitespace_and_u64() {
    let expected = projection();
    let raw = serde_json::to_vec(&expected).unwrap();
    let actual: GoalVersionProjectionV1 = parse_request(&raw).unwrap();
    actual.validate().unwrap();
    assert_eq!(actual, expected);
    let value = serde_json::to_value(actual).unwrap();
    assert_eq!(value["goal_revision"], u64::MAX.to_string());
    assert_eq!(value["content"]["goal_text"], content().goal_text);
}

#[test]
fn game_goal_limit_counts_complete_encoded_body_without_truncation() {
    let mut value = GoalContentV1 {
        goal_text: "x".into(),
        soft_constraints: vec![],
    };
    let overhead = serde_json::to_vec(&value).unwrap().len() - 1;
    value.goal_text = "x".repeat(MAX_GOAL_CONTENT_BYTES - overhead);
    assert_eq!(
        serde_json::to_vec(&value).unwrap().len(),
        MAX_GOAL_CONTENT_BYTES
    );
    value.validate().unwrap();
    value.soft_constraints.push("x".into());
    assert!(value.validate().is_err());
    value.soft_constraints.clear();
    value.goal_text.push('界');
    assert!(value.validate().is_err());
    value.goal_text = "\"".repeat(MAX_GOAL_CONTENT_BYTES / 2);
    assert!(
        value.validate().is_err(),
        "JSON escaping also consumes the limit"
    );
    value.goal_text = " \n\t".into();
    assert!(value.validate().is_err());
}

#[test]
fn game_goal_digest_binds_identity_revision_and_every_content_field() {
    let original = projection();
    for field in ["task", "revision", "text", "constraints", "order", "schema"] {
        let mut altered = original.clone();
        match field {
            "task" => altered.task_id.push('x'),
            "revision" => altered.goal_revision = DecimalU64(1),
            "text" => altered.content.goal_text.push('x'),
            "constraints" => altered.content.soft_constraints[0].push('x'),
            "order" => altered.content.soft_constraints.reverse(),
            "schema" => altered.schema_revision = 2,
            _ => unreachable!(),
        }
        assert!(
            altered.validate().is_err(),
            "{field} must not preserve the digest"
        );
    }
    assert!(game_goal_version_digest("task-1", DecimalU64(0), &content()).is_err());
    assert!(game_goal_version_digest(" ", DecimalU64(1), &content()).is_err());
}

#[test]
fn game_goal_self_consistent_body_cannot_replace_canonical_active_version() {
    let original = projection();
    original
        .validate_against(
            &original.task_id,
            original.goal_revision,
            &original.goal_digest,
            &original.activation_commit_ref,
        )
        .unwrap();
    for field in ["task", "revision", "body", "activation"] {
        let mut altered = original.clone();
        match field {
            "task" => altered.task_id.push('x'),
            "revision" => altered.goal_revision = DecimalU64(1),
            "body" => altered.content.goal_text.push('x'),
            "activation" => altered.activation_commit_ref.push('x'),
            _ => unreachable!(),
        }
        altered.goal_digest =
            game_goal_version_digest(&altered.task_id, altered.goal_revision, &altered.content)
                .unwrap();
        altered.validate().unwrap();
        assert!(
            altered
                .validate_against(
                    &original.task_id,
                    original.goal_revision,
                    &original.goal_digest,
                    &original.activation_commit_ref
                )
                .is_err()
        );
    }
    assert!(
        original
            .validate_against(
                &original.task_id,
                DecimalU64(0),
                &original.goal_digest,
                &original.activation_commit_ref
            )
            .is_err()
    );
}

#[test]
fn game_goal_incomplete_unknown_duplicate_and_numeric_shapes_fail_closed() {
    let valid = serde_json::to_value(projection()).unwrap();
    for field in ["content", "goal_digest", "activation_commit_ref"] {
        let mut altered = valid.clone();
        altered.as_object_mut().unwrap().remove(field);
        assert!(
            parse_request::<GoalVersionProjectionV1>(&serde_json::to_vec(&altered).unwrap())
                .is_err()
        );
    }
    for (field, invalid) in [
        ("goal_revision", json!(1)),
        ("goal_revision", json!("01")),
        ("grant", json!("claimed")),
    ] {
        let mut altered = valid.clone();
        altered[field] = invalid;
        assert!(
            parse_request::<GoalVersionProjectionV1>(&serde_json::to_vec(&altered).unwrap())
                .is_err()
        );
    }
    let mut altered = valid;
    altered["content"]["authority"] = json!("owner");
    assert!(
        parse_request::<GoalVersionProjectionV1>(&serde_json::to_vec(&altered).unwrap()).is_err()
    );
    assert!(
        parse_request::<GoalContentV1>(
            br#"{"goal_text":"a","goal_text":"b","soft_constraints":[]}"#
        )
        .is_err()
    );
}
