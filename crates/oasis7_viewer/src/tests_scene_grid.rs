use super::*;
use crate::viewer_3d_config::Viewer3dConfig;

#[test]
fn spawn_world_background_adds_bounds_and_chunk_sized_grid() {
    let mut app = App::new();
    app.add_systems(Update, spawn_background_test_system);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(Viewer3dScene::default());
    app.insert_resource(sample_assets());

    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&Name>();
    let names: Vec<String> = query.iter(world).map(|name| name.to_string()).collect();
    assert!(names.iter().any(|name| name == "world:bounds"));
    assert!(names.iter().any(|name| name == "world:floor"));
    assert!(names.iter().any(|name| name.starts_with("world:grid:x:")));
    assert!(names.iter().any(|name| name.starts_with("world:grid:z:")));

    let world_grid_x = names
        .iter()
        .filter(|name| name.starts_with("world:grid:x:"))
        .count();
    let world_grid_z = names
        .iter()
        .filter(|name| name.starts_with("world:grid:z:"))
        .count();
    assert_eq!(world_grid_x, 6);
    assert_eq!(world_grid_z, 6);
}

#[test]
fn rebuild_scene_spawns_chunk_grid_lines_for_all_chunks() {
    let mut app = App::new();
    app.add_systems(Update, rebuild_scene_chunks_test_system);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(Viewer3dScene::default());
    app.insert_resource(sample_assets());

    app.update();

    let scene = app.world().resource::<Viewer3dScene>();
    assert_eq!(scene.chunk_entities.len(), 25);
    assert_eq!(scene.chunk_line_entities.len(), 25);
    let line_count: usize = scene
        .chunk_line_entities
        .values()
        .map(|items| items.len())
        .sum();
    assert_eq!(line_count, 100);
}

#[test]
fn rebuild_scene_spawns_fragments_by_default_without_location_mesh() {
    let mut app = App::new();
    app.add_systems(Update, rebuild_scene_fragments_default_test_system);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(Viewer3dScene::default());
    app.insert_resource(sample_assets());

    app.update();

    let world = app.world_mut();

    let mut fragment_query = world.query::<&Name>();
    let fragment_count = fragment_query
        .iter(world)
        .filter(|name| name.as_str().starts_with("location:fragment:block:loc-1:"))
        .count();
    assert!(fragment_count > 0);

    let mut location_mesh_query = world.query_filtered::<&LocationMarker, With<Mesh3d>>();
    assert_eq!(location_mesh_query.iter(world).count(), 0);
}

#[test]
fn rebuild_scene_fragment_entities_preserve_base_scale_for_selection_reset() {
    let mut app = App::new();
    app.add_systems(Update, rebuild_scene_fragments_default_test_system);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(Viewer3dScene::default());
    app.insert_resource(sample_assets());

    app.update();

    let world = app.world_mut();
    let mut fragment_query = world.query::<(&Name, &Transform, &BaseScale)>();
    let mut checked = 0usize;
    for (name, transform, base_scale) in fragment_query.iter(world) {
        if !name.as_str().starts_with("location:fragment:block:loc-1:") {
            continue;
        }
        checked += 1;
        assert!((transform.scale.x - base_scale.0.x).abs() < 1e-6);
        assert!((transform.scale.y - base_scale.0.y).abs() < 1e-6);
        assert!((transform.scale.z - base_scale.0.z).abs() < 1e-6);
    }

    assert!(checked > 0);
}

fn spawn_background_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    spawn_world_background(
        &mut commands,
        &config,
        &assets,
        &mut scene,
        &sample_snapshot(),
    );
}

fn rebuild_scene_chunks_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let snapshot = sample_snapshot();
    rebuild_scene_from_snapshot(&mut commands, &config, &assets, &mut scene, &snapshot);
}

fn rebuild_scene_fragments_default_test_system(
    mut commands: Commands,
    config: Res<Viewer3dConfig>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
) {
    let snapshot = sample_fragment_snapshot();
    rebuild_scene_from_snapshot(&mut commands, &config, &assets, &mut scene, &snapshot);
}

fn sample_snapshot() -> WorldSnapshot {
    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Alpha", oasis7::geometry::GeoPos::new(0, 0, 0)),
    );
    let mut config = oasis7::simulator::WorldConfig::default();
    config.space.width_cm = 10_000_000;
    config.space.depth_cm = 10_000_000;
    config.space.height_cm = 1_000_000;

    WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 1,
        config,
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    }
}

fn sample_fragment_snapshot() -> WorldSnapshot {
    let mut snapshot = sample_snapshot();
    let location = snapshot
        .model
        .locations
        .get_mut("loc-1")
        .expect("location exists");
    location.fragment_profile = Some(oasis7::simulator::synthesize_fragment_profile(
        7,
        600,
        oasis7::simulator::MaterialKind::Silicate,
    ));
    snapshot
}

fn sample_assets() -> Viewer3dAssets {
    Viewer3dAssets {
        agent_mesh: Handle::default(),
        agent_material: Handle::default(),
        agent_module_marker_mesh: Handle::default(),
        agent_module_marker_material: Handle::default(),
        location_mesh: Handle::default(),
        fragment_element_material_library: FragmentElementMaterialHandles::default(),
        asset_mesh: Handle::default(),
        asset_material: Handle::default(),
        power_plant_mesh: Handle::default(),
        power_plant_material: Handle::default(),
        location_core_silicate_material: Handle::default(),
        location_core_metal_material: Handle::default(),
        location_core_ice_material: Handle::default(),
        location_halo_material: Handle::default(),
        chunk_unexplored_material: Handle::default(),
        chunk_generated_material: Handle::default(),
        chunk_exhausted_material: Handle::default(),
        world_box_mesh: Handle::default(),
        world_floor_material: Handle::default(),
        world_bounds_material: Handle::default(),
        world_grid_material: Handle::default(),
        heat_low_material: Handle::default(),
        heat_mid_material: Handle::default(),
        heat_high_material: Handle::default(),
        flow_power_material: Handle::default(),
        flow_trade_material: Handle::default(),
        label_font: Handle::default(),
    }
}
