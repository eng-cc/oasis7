use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn fixture_dir() -> std::path::PathBuf {
    let base = std::env::temp_dir();
    for _ in 0..64 {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!(
            "oasis7-web-launcher-observer-authority-{}-{sequence}",
            std::process::id()
        ));
        match std::fs::create_dir(path.as_path()) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create fixture directory failed: {error}"),
        }
    }
    panic!("create fixture directory exhausted retries");
}

fn observer_fixture() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = fixture_dir();
    let manifest = root.join("manifest.json");
    let registry = root.join("observer-registry.json");
    std::fs::write(&registry, b"{}\n").expect("write registry fixture");
    std::fs::write(
        &manifest,
        br#"{
            "tier":"public_testnet",
            "validator_policy":{"allow_observer_nodes":true},
            "deployment_validator_registry":{"ref":"config/observer-registry.json"}
        }"#,
    )
    .expect("write manifest fixture");
    (manifest, registry)
}

#[test]
fn observer_manifest_forwards_bound_registry_to_runtime() {
    let (manifest, registry) = observer_fixture();
    let config = LauncherConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_node_role: "observer".to_string(),
        chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
        ..LauncherConfig::default()
    };
    let args = build_chain_runtime_args(&config).expect("observer args");
    let registry_arg = args
        .windows(2)
        .find(|pair| pair[0] == "--genesis-validator-registry")
        .expect("bound registry argument");
    assert_eq!(registry_arg[1], registry.to_string_lossy());
}

#[test]
fn public_testnet_default_lane_uses_non_managed_observer_authority() {
    let (manifest, registry) = observer_fixture();
    let config = LauncherConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_network_tier: "public_testnet".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
        ..LauncherConfig::default()
    };

    let args = build_chain_runtime_args(&config).expect("public testnet args");
    let role_arg = args
        .windows(2)
        .find(|pair| pair[0] == "--node-role")
        .expect("node role argument");
    assert_eq!(role_arg[1], "observer");
    let registry_arg = args
        .windows(2)
        .find(|pair| pair[0] == "--genesis-validator-registry")
        .expect("bound registry argument");
    assert_eq!(registry_arg[1], registry.to_string_lossy());
}

#[test]
fn shipped_public_testnet_template_binds_adjacent_triad_registry() {
    let manifest = repo_root_dir().join(PUBLIC_TESTNET_NETWORK_TIER_MANIFEST);
    let registry = manifest
        .parent()
        .expect("template parent")
        .join("public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json");
    let loaded = LoadedNetworkTierManifest::load(manifest.as_path())
        .expect("shipped public testnet template should load");
    assert_eq!(loaded.manifest.validator_policy.target_validator_count, 3);

    let config = LauncherConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_network_tier: "public_testnet".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
        ..LauncherConfig::default()
    };
    let args = build_chain_runtime_args(&config).expect("shipped template args");
    assert!(
        args.windows(2)
            .any(|pair| { pair[0] == "--node-role" && pair[1] == "observer" })
    );
    assert!(args.windows(2).any(|pair| {
        pair[0] == "--genesis-validator-registry" && pair[1] == registry.to_string_lossy()
    }));
}

#[test]
fn missing_public_testnet_registry_is_not_synthesized() {
    let root = fixture_dir();
    let manifest = root.join("manifest.json");
    std::fs::write(
        &manifest,
        br#"{
            "tier":"public_testnet",
            "validator_policy":{"allow_observer_nodes":true},
            "deployment_validator_registry":{
                "ref":"missing-registry.json",
                "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "semantic_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            }
        }"#,
    )
    .expect("write manifest fixture");
    let config = LauncherConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_network_tier: "public_testnet".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
        ..LauncherConfig::default()
    };

    let args = build_chain_runtime_args(&config).expect("public testnet args");
    assert!(
        args.windows(2)
            .any(|pair| { pair[0] == "--node-role" && pair[1] == "observer" })
    );
    assert!(!args.iter().any(|arg| arg == "--genesis-validator-registry"));
}
