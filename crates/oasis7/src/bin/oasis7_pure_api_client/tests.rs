use super::*;

fn fixed_private_key_hex(seed: u8) -> String {
    hex::encode([seed; 32])
}

#[test]
fn derive_public_key_hex_matches_signing_key() {
    let private_key_hex = fixed_private_key_hex(7);
    let derived = derive_public_key_hex(private_key_hex.as_str()).expect("derive public key");
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    assert_eq!(derived, hex::encode(signing_key.verifying_key().to_bytes()));
}

#[test]
fn build_signed_agent_chat_request_attaches_auth_and_intent_seq() {
    let private_key_hex = fixed_private_key_hex(8);
    let request = build_signed_agent_chat_request(
        "agent-0",
        "player-1",
        "hello",
        private_key_hex.as_str(),
        None,
        Some(42),
        Some(9),
    )
    .expect("signed chat request");
    assert_eq!(request.player_id.as_deref(), Some("player-1"));
    assert_eq!(request.intent_tick, Some(42));
    assert_eq!(request.intent_seq, Some(9));
    assert!(request.auth.is_some());
    assert!(request.public_key.is_some());
}

#[test]
fn build_signed_prompt_apply_request_supports_clear_and_set() {
    let private_key_hex = fixed_private_key_hex(9);
    let request = build_signed_prompt_apply_request(
        "agent-0",
        "player-1",
        private_key_hex.as_str(),
        None,
        Some(3),
        Some("tester".to_string()),
        Some(Some("system".to_string())),
        Some(None),
        None,
        false,
    )
    .expect("signed prompt apply request");
    assert_eq!(request.expected_version, Some(3));
    assert_eq!(request.updated_by.as_deref(), Some("tester"));
    assert_eq!(
        request.system_prompt_override,
        Some(Some("system".to_string()))
    );
    assert_eq!(request.short_term_goal_override, Some(None));
    assert!(request.auth.is_some());
}

#[test]
fn build_signed_gameplay_action_request_attaches_auth() {
    let private_key_hex = fixed_private_key_hex(10);
    let request = build_signed_gameplay_action_request(
        "build_factory_smelter_mk1",
        "runtime-agent-0",
        None,
        "player-1",
        private_key_hex.as_str(),
        None,
    )
    .expect("signed gameplay action request");
    assert_eq!(request.action_id, "build_factory_smelter_mk1");
    assert_eq!(request.target_agent_id, "runtime-agent-0");
    assert_eq!(request.player_id, "player-1");
    assert!(request.public_key.is_some());
    assert!(request.auth.is_some());
}
