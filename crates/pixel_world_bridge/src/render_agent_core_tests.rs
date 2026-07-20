use super::*;

#[test]
fn bevy_ecs_reconciles_agent_cores_without_semantic_or_hit_region_changes() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let first = visual_probe_summary(&mut app);

    assert_eq!(first.agents.len(), 2);
    assert_eq!(first.agent_cores.len(), 2);
    assert_eq!(first.agent_core_entity_count, 2);
    assert_eq!(first.hit_regions, 4, "cores must not add hit regions");
    let render_state = sample_render_state_with_beacon_candidates("agent", "agent-0");
    for (base, core) in first.agents.iter().zip(&first.agent_cores) {
        assert_eq!(core.id, base.id);
        let agent = render_state
            .agents
            .iter()
            .find(|agent| agent.id == core.id)
            .expect("core agent source");
        assert_eq!(
            core.size_px,
            agent_core_size_px(agent, core.id == "agent-0")
        );
        assert_eq!(core.z, base.z + AGENT_CORE_LAYER_Z_OFFSET);
        assert_eq!(core.x, base.x);
        assert_eq!(core.y, base.y);
    }
    let world = app.world_mut();
    let mut core_query = world.query::<(&PixelWorldAgentCoreVisual, &Sprite)>();
    for (_, sprite) in core_query.iter(world) {
        assert_eq!(sprite.color, AGENT_CORE_COLOR);
    }

    app.update();
    assert_eq!(
        visual_probe_summary(&mut app).agent_core_entity_count,
        2,
        "a consecutive visible reconcile must reuse each agent core"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut removed = sample_render_state(12_000.0);
        removed.agents.clear();
        runtime.render_state = Some(removed);
        runtime.render_version += 1;
        runtime.hit_regions_dirty = true;
    }
    app.update();
    let removed = visual_probe_summary(&mut app);
    assert!(removed.agent_cores.is_empty());
    assert_eq!(removed.agent_core_entity_count, 0);
    assert_eq!(
        removed.hit_regions, 1,
        "only the location hit region remains"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    let no_state = visual_probe_summary(&mut app);
    assert!(no_state.agent_cores.is_empty());
    assert_eq!(no_state.agent_core_entity_count, 0);
}

#[test]
fn agent_cores_keep_unanimated_sizes_while_base_markers_pulse() {
    let agent = sample_render_state(12_000.0).agents.remove(0);
    let base_at_start = agent_visual_style(&agent, true, 0.0, 0).size_px;
    let base_later = agent_visual_style(&agent, true, 120.0, 0).size_px;
    let core_at_start = agent_core_size_px(&agent, true);
    let core_later = agent_core_size_px(&agent, true);

    assert_ne!(
        base_at_start, base_later,
        "base marker must retain its pulse"
    );
    assert_eq!(
        core_at_start, core_later,
        "core size must remain invariant across animation times"
    );
    assert_eq!(core_at_start, 6.912);
}
