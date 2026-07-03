use super::*;
use std::path::PathBuf;

fn unique_runtime_live_test_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "oasis7-runtime-live-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write_generated_world_fixture(label: &str) -> PathBuf {
    let generated_world_dir = unique_runtime_live_test_dir(label);
    let sidecar_dir = generated_world_dir.join("generated-scenario-world");
    std::fs::create_dir_all(&sidecar_dir).expect("create generated sidecar dir");
    let config = WorldConfig::default();
    let init = WorldInitConfig::from_scenario(WorldScenario::AsteroidFragmentBootstrap, &config);
    let (kernel, _) =
        oasis7::simulator::initialize_kernel(config, init).expect("initialize kernel");
    kernel
        .save_to_dir(&sidecar_dir)
        .expect("save generated sidecar world");
    std::fs::write(
        generated_world_dir.join("world-generation-provenance.json"),
        r#"{"scenario_id":"asteroid_fragment_bootstrap","seed":42}"#,
    )
    .expect("write generated world provenance");
    generated_world_dir
}

#[test]
fn generated_world_sidecar_initializes_runtime_live_snapshot_map() {
    let generated_world_dir = write_generated_world_fixture("generated-sidecar-snapshot-map");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::formal_release_default()
            .with_generated_world_dir(generated_world_dir.clone()),
    )
    .expect("runtime live from generated sidecar");

    let snapshot = server.compat_snapshot(None);
    assert!(snapshot.model.locations.len() > 1);
    assert!(
        snapshot
            .model
            .locations
            .values()
            .any(|location| location.fragment_profile.is_some()),
        "generated asteroid locations should survive viewer runtime-live mapping"
    );
    let first_agent = snapshot
        .model
        .agents
        .values()
        .next()
        .expect("generated sidecar seeds at least one agent");
    assert!(
        snapshot
            .model
            .locations
            .contains_key(&first_agent.location_id),
        "agent should keep a generated sidecar location ref"
    );
    assert!(
        !first_agent.location_id.starts_with("runtime-"),
        "viewer should not replace sidecar location ids with runtime fallback ids"
    );

    std::fs::remove_dir_all(generated_world_dir).expect("cleanup generated sidecar fixture");
}
