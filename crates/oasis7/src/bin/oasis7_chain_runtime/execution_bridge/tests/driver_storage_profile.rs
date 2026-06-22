use super::super::driver::{NodeRuntimeExecutionDriver, persist_execution_world};
use super::temp_dir;
use oasis7::runtime::{
    ReleaseSecurityPolicy, World as RuntimeWorld, production_hardened_main_token_config,
};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use std::fs;

#[test]
fn production_release_policy_release_default_applies_hardened_policy() {
    let dir = temp_dir("execution-driver-release-policy-release-default");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::ReleaseDefault);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::production_hardened()
    );
    assert!(
        driver
            .execution_world
            .release_security_policy()
            .is_production_hardened()
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dev_local_storage_profile_keeps_generic_supply_for_missing_execution_world() {
    let dir = temp_dir("execution-driver-release-policy-dev-local");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(driver.execution_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn new_driver_bootstraps_fresh_execution_world_persistence_files() {
    let dir = temp_dir("execution-driver-bootstrap-persistence");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);

    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir.clone(),
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert!(world_dir.join("snapshot.json").exists());
    assert!(world_dir.join("journal.json").exists());
    assert!(driver.simulator_world_dir.join("snapshot.json").exists());
    assert!(driver.simulator_world_dir.join("journal.json").exists());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn new_driver_repairs_partial_execution_world_persistence_files() {
    let dir = temp_dir("execution-driver-repair-partial-persistence");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);

    fs::create_dir_all(world_dir.as_path()).expect("create world dir");
    fs::write(world_dir.join("snapshot.json"), "{}").expect("write partial snapshot");

    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir.clone(),
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert!(world_dir.join("snapshot.json").exists());
    assert!(world_dir.join("journal.json").exists());
    assert!(driver.simulator_world_dir.join("snapshot.json").exists());
    assert!(driver.simulator_world_dir.join("journal.json").exists());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dev_local_storage_profile_clears_pristine_frozen_supply_from_existing_execution_world() {
    let dir = temp_dir("execution-driver-release-policy-dev-local-existing-world");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut world = RuntimeWorld::new();
    world.set_main_token_config(production_hardened_main_token_config());
    persist_execution_world(world_dir.as_path(), &world).expect("persist release-like world");

    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(driver.execution_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}
