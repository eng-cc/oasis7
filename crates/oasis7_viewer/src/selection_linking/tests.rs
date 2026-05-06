use super::*;
use oasis7::simulator::{Agent, Location, PowerPlant, WorldModel, WorldSnapshot};

#[test]
fn nearest_event_uses_smallest_tick_distance() {
    let events = vec![
        WorldEvent {
            id: 1,
            time: 3,
            kind: WorldEventKind::AgentMoved {
                agent_id: "a1".to_string(),
                from: "l1".to_string(),
                to: "l2".to_string(),
                distance_cm: 1,
                electricity_cost: 1,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 9,
            kind: WorldEventKind::AgentMoved {
                agent_id: "a1".to_string(),
                from: "l2".to_string(),
                to: "l1".to_string(),
                distance_cm: 1,
                electricity_cost: 1,
            },
            runtime_event: None,
        },
    ];

    let nearest = nearest_event_to_tick(&events, 8).expect("nearest");
    assert_eq!(nearest.id, 2);
}

#[test]
fn reject_reason_facility_maps_to_plant_target() {
    let mut model = WorldModel::default();
    model.power_plants.insert(
        "pp-1".to_string(),
        PowerPlant::new(
            "pp-1".to_string(),
            "loc-1".to_string(),
            ResourceOwner::Location {
                location_id: "loc-1".to_string(),
            },
            100,
        ),
    );
    let snapshot = WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 1,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let event = WorldEvent {
        id: 9,
        time: 2,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::FacilityNotFound {
                facility_id: "pp-1".to_string(),
            },
        },
        runtime_event: None,
    };

    let target = event_primary_target(&event, Some(&snapshot)).expect("target");
    assert_eq!(target.kind, SelectionKind::PowerPlant);
    assert_eq!(target.id, "pp-1");
}

#[test]
fn selection_related_ticks_match_agent_events() {
    let mut model = WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        Location::new(
            "loc-1",
            "L1",
            oasis7::geometry::GeoPos {
                x_cm: 0,
                y_cm: 0,
                z_cm: 0,
            },
        ),
    );
    model.agents.insert(
        "agent-1".to_string(),
        Agent::new(
            "agent-1",
            "loc-1",
            oasis7::geometry::GeoPos {
                x_cm: 0,
                y_cm: 0,
                z_cm: 0,
            },
        ),
    );
    let snapshot = WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 1,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let events = vec![
        WorldEvent {
            id: 1,
            time: 5,
            kind: WorldEventKind::AgentMoved {
                agent_id: "agent-1".to_string(),
                from: "loc-1".to_string(),
                to: "loc-1".to_string(),
                distance_cm: 1,
                electricity_cost: 1,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 7,
            kind: WorldEventKind::Power(PowerEvent::PowerConsumed {
                agent_id: "agent-1".to_string(),
                amount: 3,
                reason: oasis7::simulator::ConsumeReason::Decision,
                remaining: 9,
            }),
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 11,
            kind: WorldEventKind::LocationRegistered {
                location_id: "loc-2".to_string(),
                name: "L2".to_string(),
                pos: oasis7::geometry::GeoPos {
                    x_cm: 1,
                    y_cm: 1,
                    z_cm: 1,
                },
                profile: oasis7::simulator::LocationProfile::default(),
            },
            runtime_event: None,
        },
    ];

    let selection = SelectionInfo {
        entity: Entity::from_bits(1),
        kind: SelectionKind::Agent,
        id: "agent-1".to_string(),
        name: None,
    };

    let ticks = selection_related_ticks(&selection, &events, Some(&snapshot));
    assert_eq!(ticks, vec![5, 7]);
}

#[test]
fn factory_and_recipe_events_map_to_owner_target() {
    let factory_built = WorldEvent {
        id: 21,
        time: 4,
        kind: WorldEventKind::FactoryBuilt {
            owner: ResourceOwner::Agent {
                agent_id: "agent-7".to_string(),
            },
            location_id: "loc-1".to_string(),
            factory_id: "factory-1".to_string(),
            factory_kind: "miner".to_string(),
            electricity_cost: 12,
            hardware_cost: 3,
        },
        runtime_event: None,
    };
    let recipe_scheduled = WorldEvent {
        id: 22,
        time: 5,
        kind: WorldEventKind::RecipeScheduled {
            owner: ResourceOwner::Location {
                location_id: "loc-2".to_string(),
            },
            factory_id: "factory-1".to_string(),
            recipe_id: "recipe-1".to_string(),
            batches: 1,
            electricity_cost: 6,
            hardware_cost: 2,
            data_output: 8,
            finished_product_id: "drone".to_string(),
            finished_product_units: 1,
        },
        runtime_event: None,
    };

    let factory_target = event_primary_target(&factory_built, None).expect("factory target");
    assert_eq!(factory_target.kind, SelectionKind::Agent);
    assert_eq!(factory_target.id, "agent-7");

    let recipe_target = event_primary_target(&recipe_scheduled, None).expect("recipe target");
    assert_eq!(recipe_target.kind, SelectionKind::Location);
    assert_eq!(recipe_target.id, "loc-2");
}

#[test]
fn selection_related_ticks_include_factory_and_recipe_events() {
    let events = vec![
        WorldEvent {
            id: 30,
            time: 4,
            kind: WorldEventKind::FactoryBuilt {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-7".to_string(),
                },
                location_id: "loc-1".to_string(),
                factory_id: "factory-1".to_string(),
                factory_kind: "smelter".to_string(),
                electricity_cost: 18,
                hardware_cost: 7,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 31,
            time: 6,
            kind: WorldEventKind::RecipeScheduled {
                owner: ResourceOwner::Location {
                    location_id: "loc-1".to_string(),
                },
                factory_id: "factory-1".to_string(),
                recipe_id: "iron_ingot".to_string(),
                batches: 1,
                electricity_cost: 3,
                hardware_cost: 1,
                data_output: 5,
                finished_product_id: "iron_ingot".to_string(),
                finished_product_units: 1,
            },
            runtime_event: None,
        },
    ];

    let agent_selection = SelectionInfo {
        entity: Entity::from_bits(7),
        kind: SelectionKind::Agent,
        id: "agent-7".to_string(),
        name: None,
    };
    let location_selection = SelectionInfo {
        entity: Entity::from_bits(8),
        kind: SelectionKind::Location,
        id: "loc-1".to_string(),
        name: Some("L1".to_string()),
    };

    assert_eq!(
        selection_related_ticks(&agent_selection, &events, None),
        vec![4]
    );
    assert_eq!(
        selection_related_ticks(&location_selection, &events, None),
        vec![4, 6]
    );
}

#[test]
fn locate_focus_event_button_selects_target_and_updates_timeline() {
    let mut app = App::new();
    app.add_systems(Update, handle_locate_focus_event_button);

    let selected_entity = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();

    let mut scene = Viewer3dScene::default();
    scene
        .agent_entities
        .insert("agent-1".to_string(), selected_entity);

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: None,
        events: vec![WorldEvent {
            id: 1,
            time: 5,
            kind: WorldEventKind::AgentMoved {
                agent_id: "agent-1".to_string(),
                from: "loc-a".to_string(),
                to: "loc-b".to_string(),
                distance_cm: 100,
                electricity_cost: 1,
            },
            runtime_event: None,
        }],
        decision_traces: Vec::new(),
        metrics: None,
    };

    app.world_mut().insert_resource(state);
    app.world_mut().insert_resource(scene);
    app.world_mut().insert_resource(Viewer3dConfig::default());
    app.world_mut().insert_resource(ViewerSelection::default());
    app.world_mut()
        .insert_resource(EventObjectLinkState::default());
    app.world_mut().insert_resource(TimelineUiState::default());

    app.world_mut()
        .spawn((Button, Interaction::Pressed, LocateFocusEventButton));

    app.update();

    let selection = app.world().resource::<ViewerSelection>();
    let current = selection.current.as_ref().expect("selection");
    assert_eq!(current.kind, SelectionKind::Agent);
    assert_eq!(current.id, "agent-1");

    let timeline = app.world().resource::<TimelineUiState>();
    assert_eq!(timeline.target_tick, 5);
    assert!(timeline.manual_override);

    let link = app.world().resource::<EventObjectLinkState>();
    assert!(link.message.contains("event #1"));
}

#[test]
fn jump_selection_events_button_moves_timeline_target() {
    let mut app = App::new();
    app.add_systems(Update, handle_jump_selection_events_button);

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: None,
        events: vec![
            WorldEvent {
                id: 1,
                time: 3,
                kind: WorldEventKind::AgentMoved {
                    agent_id: "agent-1".to_string(),
                    from: "loc-a".to_string(),
                    to: "loc-b".to_string(),
                    distance_cm: 100,
                    electricity_cost: 1,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 9,
                kind: WorldEventKind::Power(PowerEvent::PowerConsumed {
                    agent_id: "agent-1".to_string(),
                    amount: 3,
                    reason: oasis7::simulator::ConsumeReason::Decision,
                    remaining: 5,
                }),
                runtime_event: None,
            },
        ],
        decision_traces: Vec::new(),
        metrics: None,
    };

    app.world_mut().insert_resource(state);
    app.world_mut().insert_resource(ViewerSelection {
        current: Some(SelectionInfo {
            entity: Entity::from_bits(1),
            kind: SelectionKind::Agent,
            id: "agent-1".to_string(),
            name: None,
        }),
    });
    app.world_mut()
        .insert_resource(EventObjectLinkState::default());
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 3,
        max_tick_seen: 12,
        manual_override: true,
        drag_active: false,
    });

    app.world_mut()
        .spawn((Button, Interaction::Pressed, JumpSelectionEventsButton));

    app.update();

    let timeline = app.world().resource::<TimelineUiState>();
    assert_eq!(timeline.target_tick, 9);
    assert!(timeline.manual_override);

    let link = app.world().resource::<EventObjectLinkState>();
    assert!(link.message.contains("-> t9"));
}

#[test]
fn event_object_link_controls_use_wrapping_layout() {
    let mut app = App::new();
    app.add_systems(Startup, |mut commands: Commands| {
        let root = commands.spawn(Node::default()).id();
        commands.entity(root).with_children(|parent| {
            spawn_event_object_link_controls(
                parent,
                Handle::<Font>::default(),
                crate::i18n::UiLocale::EnUs,
            );
        });
    });

    app.update();

    let world = app.world_mut();

    let mut wrapping_row_count = 0usize;
    let mut row_query = world.query::<&Node>();
    for node in row_query.iter(world) {
        if node.flex_wrap == FlexWrap::Wrap && node.min_height == Val::Px(24.0) {
            wrapping_row_count += 1;
        }
    }
    assert!(wrapping_row_count >= 1, "expected wrapping controls row");

    let mut locate_query = world.query::<(&Node, &LocateFocusEventButton)>();
    let (locate_button, _) = locate_query.single(world).expect("locate button");
    assert_eq!(locate_button.min_width, Val::Px(120.0));
    assert_eq!(locate_button.flex_grow, 1.0);

    let mut quick_query = world.query::<(&Node, &QuickLocateAgentButton)>();
    let (quick_button, _) = quick_query.single(world).expect("quick locate button");
    assert_eq!(quick_button.min_width, Val::Px(120.0));
    assert_eq!(quick_button.flex_grow, 1.0);

    let mut jump_query = world.query::<(&Node, &JumpSelectionEventsButton)>();
    let (jump_button, _) = jump_query.single(world).expect("jump button");
    assert_eq!(jump_button.min_width, Val::Px(120.0));
    assert_eq!(jump_button.flex_grow, 1.0);
}

#[test]
fn event_object_link_button_labels_follow_locale_without_query_conflict() {
    let mut app = App::new();
    app.add_systems(Update, update_event_object_link_button_labels);
    app.world_mut().insert_resource(crate::i18n::UiI18n {
        locale: crate::i18n::UiLocale::ZhCn,
    });

    app.world_mut()
        .spawn((Text::new("定位 Agent"), QuickLocateAgentButtonLabel));
    app.world_mut()
        .spawn((Text::new("定位焦点事件"), LocateFocusEventButtonLabel));
    app.world_mut().spawn((
        Text::new("跳转选中对象事件"),
        JumpSelectionEventsButtonLabel,
    ));

    app.update();

    {
        let mut query = app
            .world_mut()
            .query::<(&Text, &QuickLocateAgentButtonLabel)>();
        let (text, _) = query.single(app.world()).expect("quick locate label");
        assert_eq!(text.0, "定位 Agent");
    }

    {
        let mut query = app
            .world_mut()
            .query::<(&Text, &LocateFocusEventButtonLabel)>();
        let (text, _) = query.single(app.world()).expect("locate label");
        assert_eq!(text.0, "定位焦点事件");
    }

    app.world_mut().insert_resource(crate::i18n::UiI18n {
        locale: crate::i18n::UiLocale::EnUs,
    });
    app.update();

    {
        let mut query = app
            .world_mut()
            .query::<(&Text, &QuickLocateAgentButtonLabel)>();
        let (text, _) = query.single(app.world()).expect("quick locate label");
        assert_eq!(text.0, "Locate Agent");
    }
    {
        let mut query = app
            .world_mut()
            .query::<(&Text, &LocateFocusEventButtonLabel)>();
        let (text, _) = query.single(app.world()).expect("locate label");
        assert_eq!(text.0, "Locate Focus");
    }
    {
        let mut query = app
            .world_mut()
            .query::<(&Text, &JumpSelectionEventsButtonLabel)>();
        let (text, _) = query.single(app.world()).expect("jump label");
        assert_eq!(text.0, "Jump Selection");
    }
}

#[test]
fn quick_locate_agent_button_prefers_current_agent_selection() {
    let mut app = App::new();
    app.add_systems(Update, handle_quick_locate_agent_button);

    let selected_entity = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();
    let fallback_entity = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();

    let mut scene = Viewer3dScene::default();
    scene
        .agent_entities
        .insert("agent-9".to_string(), selected_entity);
    scene
        .agent_entities
        .insert("agent-1".to_string(), fallback_entity);

    app.world_mut().insert_resource(scene);
    app.world_mut().insert_resource(Viewer3dConfig::default());
    app.world_mut().insert_resource(ViewerSelection {
        current: Some(SelectionInfo {
            entity: selected_entity,
            kind: SelectionKind::Agent,
            id: "agent-9".to_string(),
            name: None,
        }),
    });
    app.world_mut()
        .insert_resource(EventObjectLinkState::default());

    app.world_mut()
        .spawn((Button, Interaction::Pressed, QuickLocateAgentButton));
    app.update();

    let selection = app.world().resource::<ViewerSelection>();
    let current = selection.current.as_ref().expect("selection");
    assert_eq!(current.kind, SelectionKind::Agent);
    assert_eq!(current.id, "agent-9");

    let link = app.world().resource::<EventObjectLinkState>();
    assert_eq!(link.message, "Link: located agent agent-9");
}

#[test]
fn quick_locate_agent_button_falls_back_to_sorted_first_agent() {
    let mut app = App::new();
    app.add_systems(Update, handle_quick_locate_agent_button);

    let location_entity = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();
    let agent_a = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();
    let agent_b = app
        .world_mut()
        .spawn((Transform::default(), BaseScale(Vec3::ONE)))
        .id();

    let mut scene = Viewer3dScene::default();
    scene.agent_entities.insert("agent-9".to_string(), agent_b);
    scene.agent_entities.insert("agent-1".to_string(), agent_a);

    app.world_mut().insert_resource(scene);
    app.world_mut().insert_resource(Viewer3dConfig::default());
    app.world_mut().insert_resource(ViewerSelection {
        current: Some(SelectionInfo {
            entity: location_entity,
            kind: SelectionKind::Location,
            id: "loc-1".to_string(),
            name: Some("L1".to_string()),
        }),
    });
    app.world_mut()
        .insert_resource(EventObjectLinkState::default());

    app.world_mut()
        .spawn((Button, Interaction::Pressed, QuickLocateAgentButton));
    app.update();

    let selection = app.world().resource::<ViewerSelection>();
    let current = selection.current.as_ref().expect("selection");
    assert_eq!(current.kind, SelectionKind::Agent);
    assert_eq!(current.id, "agent-1");

    let link = app.world().resource::<EventObjectLinkState>();
    assert_eq!(link.message, "Link: located agent agent-1");
}

#[test]
fn quick_locate_agent_button_reports_when_no_agent_exists() {
    let mut app = App::new();
    app.add_systems(Update, handle_quick_locate_agent_button);

    app.world_mut().insert_resource(Viewer3dScene::default());
    app.world_mut().insert_resource(Viewer3dConfig::default());
    app.world_mut().insert_resource(ViewerSelection::default());
    app.world_mut()
        .insert_resource(EventObjectLinkState::default());

    app.world_mut()
        .spawn((Button, Interaction::Pressed, QuickLocateAgentButton));
    app.update();

    let selection = app.world().resource::<ViewerSelection>();
    assert!(selection.current.is_none());

    let link = app.world().resource::<EventObjectLinkState>();
    assert_eq!(link.message, "Link: no agents available");
}

#[test]
fn apply_selection_does_not_scale_fragment_entity() {
    let mut app = App::new();
    app.add_systems(Update, apply_fragment_selection_test_system);
    app.world_mut().insert_resource(ViewerSelection::default());
    app.world_mut().insert_resource(Viewer3dConfig::default());

    let base = Vec3::new(0.2, 0.3, 0.4);
    app.world_mut().spawn((
        FragmentSelectionTestMarker,
        Transform::from_scale(base),
        BaseScale(base),
    ));

    app.update();

    let mut query = app
        .world_mut()
        .query::<(&Transform, &BaseScale, &FragmentSelectionTestMarker)>();
    let (transform, base_scale, _) = query
        .single(app.world())
        .expect("fragment test entity should exist");
    assert!((transform.scale.x - base_scale.0.x).abs() < 1e-6);
    assert!((transform.scale.y - base_scale.0.y).abs() < 1e-6);
    assert!((transform.scale.z - base_scale.0.z).abs() < 1e-6);
}

#[derive(Component)]
struct FragmentSelectionTestMarker;

fn apply_fragment_selection_test_system(
    config: Res<Viewer3dConfig>,
    mut selection: ResMut<ViewerSelection>,
    markers: Query<Entity, With<FragmentSelectionTestMarker>>,
    mut transforms: Query<(&mut Transform, Option<&BaseScale>)>,
) {
    let entity = markers
        .iter()
        .next()
        .expect("fragment test entity should exist");
    apply_selection(
        &mut selection,
        &mut transforms,
        &config,
        entity,
        SelectionKind::Fragment,
        "frag-1#0".to_string(),
        Some("frag-1".to_string()),
    );
}

#[test]
fn ray_chunk_grid_hit_distance_matches_interior_hit() {
    let marker = ChunkMarker {
        id: "0,0,0".to_string(),
        state: "generated".to_string(),
        min_x: -1.0,
        max_x: 1.0,
        min_z: -1.5,
        max_z: 1.5,
        pick_y: 0.0,
    };
    let ray = Ray3d {
        origin: Vec3::new(0.2, 4.0, -0.8),
        direction: Dir3::new(Vec3::new(0.0, -1.0, 0.0)).expect("dir"),
    };

    let distance = ray_chunk_grid_hit_distance(ray, &marker).expect("hit");
    assert!((distance - 4.0).abs() < 1e-6);
}

#[test]
fn ray_chunk_grid_hit_distance_rejects_outside_bounds() {
    let marker = ChunkMarker {
        id: "0,0,0".to_string(),
        state: "generated".to_string(),
        min_x: -1.0,
        max_x: 1.0,
        min_z: -1.0,
        max_z: 1.0,
        pick_y: 0.0,
    };
    let ray = Ray3d {
        origin: Vec3::new(2.1, 3.0, 0.0),
        direction: Dir3::new(Vec3::new(0.0, -1.0, 0.0)).expect("dir"),
    };

    assert!(ray_chunk_grid_hit_distance(ray, &marker).is_none());
}
