use super::*;

#[test]
fn neutral_agent_silhouette_accents_are_id_stable_offset_and_behind_agent_bodies() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let first_offsets = separate_agent_silhouette_accent_offsets(&mut app);

    assert_ne!(
        first_offsets["agent-0"], first_offsets["agent-1"],
        "stable agent ids must choose visibly distinct neutral silhouette offsets"
    );

    let mut reordered = sample_render_state_with_beacon_candidates("agent", "agent-1");
    reordered.agents.reverse();
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(reordered);
        runtime.render_version += 1;
    }
    app.update();

    let second_offsets = separate_agent_silhouette_accent_offsets(&mut app);
    for (id, offset) in second_offsets {
        assert_eq!(
            offset, first_offsets[&id],
            "neutral silhouette accent for {id} must depend on stable id, not selection or input order",
        );
    }
}

#[test]
fn agent_silhouettes_reconcile_once_per_eligible_agent_and_clean_up_stale_state() {
    let mut state = sample_render_state_with_beacon_candidates("agent", "agent-0");
    state.agents.push(Agent {
        id: "agent-too-small".to_string(),
        label: "Too Small For Silhouette".to_string(),
        pos: Some(sample_position(1_660_000.0, 1_120_000.0)),
        location_id: Some("loc-1".to_string()),
        resource_summary: "-".to_string(),
        status_badges: vec![],
        size_hint_px: Some(5.0),
    });
    let mut app = render_test_app(state);
    let first = silhouette_entities(&mut app);

    assert_eq!(
        first.keys().map(String::as_str).collect::<Vec<_>>(),
        ["agent-0", "agent-1"],
        "only agents at or above the silhouette legibility threshold receive an accent"
    );

    app.update();
    assert_eq!(
        silhouette_entities(&mut app),
        first,
        "a repeated reconcile must retain exactly one silhouette entity per eligible agent"
    );

    let mut reordered = sample_render_state_with_beacon_candidates("agent", "agent-1");
    reordered.agents.reverse();
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(reordered);
        runtime.render_version += 1;
    }
    app.update();
    assert_eq!(
        silhouette_entities(&mut app),
        first,
        "reordering eligible agents must not duplicate or replace their silhouette entities"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut stale_removed = sample_render_state(12_000.0);
        stale_removed.selection = None;
        runtime.render_state = Some(stale_removed);
        runtime.render_version += 1;
    }
    app.update();
    assert_eq!(
        silhouette_entities(&mut app)
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["agent-0"],
        "removing an agent from render state must despawn its stale silhouette"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        silhouette_entities(&mut app).is_empty(),
        "removing render state must despawn every silhouette"
    );
}

fn silhouette_entities(app: &mut App) -> std::collections::BTreeMap<String, Entity> {
    let world = app.world_mut();
    let mut silhouettes = world.query::<(Entity, &PixelWorldAgentSilhouetteVisual)>();
    silhouettes
        .iter(world)
        .map(|(entity, silhouette)| (silhouette.id.clone(), entity))
        .collect()
}

fn separate_agent_silhouette_accent_offsets(
    app: &mut App,
) -> std::collections::BTreeMap<String, (f32, f32)> {
    let world = app.world_mut();
    let mut agent_query = world.query::<(Entity, &PixelWorldAgentVisual, &Sprite, &Transform)>();
    let agents = agent_query
        .iter(world)
        .map(|(entity, visual, sprite, transform)| {
            (
                entity,
                visual.id.clone(),
                row(&visual.id, sprite, transform),
            )
        })
        .collect::<Vec<_>>();
    let mut core_query = world.query::<(Entity, &PixelWorldAgentCoreVisual)>();
    let core_entities = core_query
        .iter(world)
        .map(|(entity, _)| entity)
        .collect::<std::collections::HashSet<_>>();
    let agent_entities = agents
        .iter()
        .map(|(entity, _, _)| *entity)
        .collect::<std::collections::HashSet<_>>();
    let mut sprite_query = world.query::<(Entity, &Sprite, &Transform)>();
    let sprites = sprite_query
        .iter(world)
        .filter(|(entity, _, _)| {
            !agent_entities.contains(entity) && !core_entities.contains(entity)
        })
        .map(|(entity, sprite, transform)| (entity, row("candidate", sprite, transform)))
        .collect::<Vec<_>>();

    agents
        .into_iter()
        .map(|(_, id, agent)| {
            let accent = sprites
                .iter()
                .filter_map(|(_, candidate)| {
                    let offset = (candidate.x - agent.x, candidate.y - agent.y);
                    let is_beside_body = offset.0.abs() >= 1.0 || offset.1.abs() >= 1.0;
                    let is_near_body =
                        offset.0.abs() <= agent.size_px && offset.1.abs() <= agent.size_px;
                    let is_small = candidate.size_px <= agent.size_px * 0.6;
                    let is_behind_body = candidate.z < agent.z && candidate.z >= agent.z - 0.5;
                    (is_beside_body && is_near_body && is_small && is_behind_body)
                        .then_some(offset)
                })
                .next()
                .expect("every visible agent must have a separate neutral silhouette accent behind and beside its body");
            (id, accent)
        })
        .collect()
}
