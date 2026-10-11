use serde_json::{Value, json};
fn schema(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../tests/fixtures/codex-0.137.0")
                .join(format!("{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn pinned_required_identities_and_reply_shapes_are_explicit() {
    for (name, fields) in [
        (
            "TurnSteerParams",
            vec!["expectedTurnId", "threadId", "input"],
        ),
        ("TurnInterruptParams", vec!["threadId", "turnId"]),
        (
            "ToolRequestUserInputParams",
            vec!["threadId", "turnId", "itemId", "questions"],
        ),
        (
            "ServerRequestResolvedNotification",
            vec!["threadId", "requestId"],
        ),
    ] {
        let s = schema(name);
        for field in fields {
            assert!(
                s["required"].as_array().unwrap().contains(&json!(field)),
                "{name}:{field}"
            );
        }
    }
    assert!(
        schema("TurnSteerParams")["properties"]
            .get("turnId")
            .is_none()
    );
    assert_eq!(
        schema("ToolRequestUserInputResponse")["properties"]["answers"]["type"],
        "object"
    );
}
