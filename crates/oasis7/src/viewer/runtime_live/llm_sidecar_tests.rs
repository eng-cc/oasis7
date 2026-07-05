use super::*;

#[test]
fn bind_agent_player_emits_unbind_before_rebind_for_same_agent() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar
        .agent_player_bindings
        .insert("agent-1".to_string(), "player-a".to_string());
    sidecar
        .player_agent_bindings
        .insert("player-a".to_string(), "agent-1".to_string());
    sidecar
        .agent_public_key_bindings
        .insert("agent-1".to_string(), "pubkey-a".to_string());

    let events = sidecar
        .bind_agent_player("agent-1", "player-b", Some("pubkey-b"), false)
        .expect("rebind should succeed");
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        WorldEventKind::AgentPlayerUnbound {
            agent_id,
            player_id,
            public_key
        } if agent_id == "agent-1"
            && player_id == "player-a"
            && public_key.as_deref() == Some("pubkey-a")
    ));
    assert!(matches!(
        &events[1],
        WorldEventKind::AgentPlayerBound {
            agent_id,
            player_id,
            public_key
        } if agent_id == "agent-1"
            && player_id == "player-b"
            && public_key.as_deref() == Some("pubkey-b")
    ));
    assert_eq!(
        sidecar
            .agent_player_bindings
            .get("agent-1")
            .map(String::as_str),
        Some("player-b")
    );
    assert_eq!(
        sidecar
            .player_agent_bindings
            .get("player-b")
            .map(String::as_str),
        Some("agent-1")
    );
    assert!(!sidecar.player_agent_bindings.contains_key("player-a"));
    assert_eq!(
        sidecar
            .agent_public_key_bindings
            .get("agent-1")
            .map(String::as_str),
        Some("pubkey-b")
    );
}

#[test]
fn sync_shadow_kernel_accepts_empty_synthetic_runtime_snapshot() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    let world = RuntimeWorld::default();

    sidecar
        .sync_shadow_kernel(&world, &WorldConfig::default())
        .expect("empty synthetic runtime snapshot should sync");

    let shadow = sidecar.shadow_kernel.as_ref().expect("shadow kernel");
    assert!(shadow.journal().is_empty());
}

#[test]
fn sync_shadow_kernel_preserves_generated_seed_locations() {
    let seed_pos = GeoPos::new(7, 8, 9);
    let mut seed_model = WorldModel::default();
    seed_model.locations.insert(
        "frag-shadow".to_string(),
        Location::new("frag-shadow", "shadow fragment", seed_pos),
    );
    let mut sidecar =
        RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm).with_runtime_seed_model(&seed_model);
    let world = RuntimeWorld::default();

    sidecar
        .sync_shadow_kernel(&world, &WorldConfig::default())
        .expect("runtime seed shadow sync");

    let shadow = sidecar.shadow_kernel.as_ref().expect("shadow kernel");
    assert!(
        shadow
            .snapshot()
            .model
            .locations
            .contains_key("frag-shadow")
    );
}
