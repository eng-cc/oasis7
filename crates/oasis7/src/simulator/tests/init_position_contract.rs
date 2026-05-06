use super::*;

#[test]
fn init_rounds_fractional_location_seed_positions_to_centimeters() {
    let config = WorldConfig::default();
    let mut init = WorldInitConfig::default();
    init.origin.enabled = false;
    init.asteroid_fragment.enabled = false;
    init.agents.count = 0;

    let mut location_seed = LocationSeedConfig::default();
    location_seed.location_id = "fractional".to_string();
    location_seed.name = "Fractional".to_string();
    location_seed.pos = Some(GeoPos {
        x_cm: 10.4,
        y_cm: 19.6,
        z_cm: 30.5,
    });
    init.locations.push(location_seed);

    let (model, _) = build_world_model(&config, &init).expect("init should succeed");
    let location = model.locations.get("fractional").expect("location exists");

    assert_eq!(location.pos.x_cm, 10.0);
    assert_eq!(location.pos.y_cm, 20.0);
    assert_eq!(location.pos.z_cm, 31.0);
}

#[test]
fn init_generated_fragments_snap_positions_to_centimeter_grid() {
    let mut config = WorldConfig::default();
    config.space = SpaceConfig {
        width_cm: 200_000,
        depth_cm: 200_000,
        height_cm: 200_000,
    };
    config.asteroid_fragment.layer_scale_height_km = 0.0;
    config.asteroid_fragment.radius_min_cm = 10;
    config.asteroid_fragment.radius_max_cm = 10;
    config.asteroid_fragment.min_fragment_spacing_cm = 0;

    let mut init = WorldInitConfig::default();
    init.origin.enabled = false;
    init.agents.count = 0;
    init.asteroid_fragment
        .bootstrap_chunks
        .push(ChunkCoord { x: 0, y: 0, z: 0 });

    let (model, _) = build_world_model(&config, &init).expect("init should succeed");
    let fragment = model
        .locations
        .values()
        .find(|location| location.id.starts_with("frag-"))
        .expect("fragment exists");

    assert_eq!(fragment.pos.x_cm.fract(), 0.0);
    assert_eq!(fragment.pos.y_cm.fract(), 0.0);
    assert_eq!(fragment.pos.z_cm.fract(), 0.0);
}
