use super::*;

pub(super) fn spawn_label_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-1",
        "Alpha",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        100,
        0,
        None,
    );
}

pub(super) fn spawn_location_scale_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-scale",
        "Scale",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        20_000,
        0,
        None,
    );
}

pub(super) fn spawn_location_detail_ring_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-detail-ring",
        "DetailRing",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        20_000,
        0,
        None,
    );
}

pub(super) fn spawn_location_detail_halo_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-detail-halo",
        "DetailHalo",
        GeoPos::new(0, 0, 0),
        MaterialKind::Metal,
        6_000,
        10_000,
        None,
    );
}

pub(super) fn spawn_location_damage_detail_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    let mut budget = oasis7::simulator::FragmentResourceBudget::default();
    budget
        .total_by_element_g
        .insert(oasis7::simulator::FragmentElementKind::Iron, 1_000);
    budget
        .remaining_by_element_g
        .insert(oasis7::simulator::FragmentElementKind::Iron, 100);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-damage",
        "Damage",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        8_000,
        1_000,
        Some(&budget),
    );
}

pub(super) fn spawn_location_carbon_material_detail_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-carbon",
        "Carbon",
        GeoPos::new(0, 0, 0),
        MaterialKind::Carbon,
        7_000,
        500,
        None,
    );
}

pub(super) fn spawn_location_composite_material_detail_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-composite",
        "Composite",
        GeoPos::new(0, 0, 0),
        MaterialKind::Composite,
        7_000,
        500,
        None,
    );
}

pub(super) fn spawn_power_facility_scale_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    let location_id = "loc-facility-scale";
    let location_pos = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        location_id,
        "FacilityScale",
        location_pos,
        MaterialKind::Metal,
        800,
        0,
        None,
    );

    spawn_power_plant_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "plant-scale",
        location_id,
        location_pos,
    );
}

pub(super) fn spawn_agent_scale_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-scale",
        None,
        GeoPos::new(0, 0, 0),
        200,
        5,
        None,
    );
}

pub(super) fn spawn_agent_surface_attachment_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-surface",
        "Surface",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        240,
        0,
        None,
    );
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-surface",
        Some("loc-surface"),
        GeoPos::new(0, 0, 0),
        100,
        6,
        None,
    );
}

pub(super) fn spawn_agent_surface_standoff_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_location_entity_with_radiation(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "loc-surface-standoff",
        "SurfaceStandoff",
        GeoPos::new(0, 0, 0),
        MaterialKind::Silicate,
        240,
        0,
        None,
    );
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-surface-standoff",
        Some("loc-surface-standoff"),
        GeoPos::new(0, 0, 5_240),
        100,
        6,
        None,
    );
}

pub(super) fn spawn_agent_module_marker_count_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-module-cap",
        None,
        GeoPos::new(0, 0, 0),
        180,
        24,
        None,
    );
}

pub(super) fn spawn_agent_robot_layout_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-robot-layout",
        None,
        GeoPos::new(0, 0, 0),
        180,
        8,
        None,
    );
}

pub(super) fn spawn_agent_motion_feedback_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let origin = GeoPos::new(0, 0, 0);
    let kinematics = oasis7::simulator::AgentKinematics {
        speed_cm_per_tick: 320_000,
        move_target_location_id: None,
        move_target: Some(GeoPos::new(10_000, 0, 0)),
        move_started_at_tick: Some(12),
        move_eta_tick: Some(13),
        move_remaining_cm: 10_000,
    };
    spawn_agent_entity(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        origin,
        "agent-motion",
        None,
        GeoPos::new(0, 0, 0),
        170,
        4,
        Some(&kinematics),
    );
}

pub(super) fn rebuild_scene_module_count_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let mut model = oasis7::simulator::WorldModel::default();
    model.agents.insert(
        "agent-modules".to_string(),
        oasis7::simulator::Agent::new("agent-modules", "loc-1", GeoPos::new(0, 0, 0)),
    );
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Loc", GeoPos::new(0, 0, 0)),
    );
    model.module_visual_entities.insert(
        "mv-1".to_string(),
        oasis7::simulator::ModuleVisualEntity {
            entity_id: "mv-1".to_string(),
            module_id: "m.power".to_string(),
            kind: "artifact".to_string(),
            label: None,
            anchor: oasis7::simulator::ModuleVisualAnchor::Agent {
                agent_id: "agent-modules".to_string(),
            },
        },
    );
    model.module_visual_entities.insert(
        "mv-2".to_string(),
        oasis7::simulator::ModuleVisualEntity {
            entity_id: "mv-2".to_string(),
            module_id: "m.sensor".to_string(),
            kind: "artifact".to_string(),
            label: None,
            anchor: oasis7::simulator::ModuleVisualAnchor::Agent {
                agent_id: "agent-modules".to_string(),
            },
        },
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
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

    rebuild_scene_from_snapshot(&mut commands, &config, &assets, &mut scene, &snapshot);
}

pub(super) fn rebuild_scene_default_module_count_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let mut model = oasis7::simulator::WorldModel::default();
    model.agents.insert(
        "agent-default-modules".to_string(),
        oasis7::simulator::Agent::new("agent-default-modules", "loc-1", GeoPos::new(0, 0, 0)),
    );
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Loc", GeoPos::new(0, 0, 0)),
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
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

    rebuild_scene_from_snapshot(&mut commands, &config, &assets, &mut scene, &snapshot);
}
