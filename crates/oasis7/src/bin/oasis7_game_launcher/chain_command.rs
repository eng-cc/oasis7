use std::path::{Component, Path, PathBuf};

use super::CliOptions;

pub(super) fn chain_world_id(options: &CliOptions) -> String {
    options
        .chain_world_id
        .clone()
        .unwrap_or_else(|| super::default_chain_world_id(options.scenario.as_str()))
}

pub(super) fn chain_execution_world_dir(node_id: &str) -> String {
    Path::new("output")
        .join("chain-runtime")
        .join(node_id)
        .join("reward-runtime-execution-world")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn chain_config_path(node_id: &str) -> String {
    Path::new("output")
        .join("chain-runtime")
        .join(node_id)
        .join("config.toml")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn missing_execution_world_persistence_files(world_dir: &Path) -> Vec<PathBuf> {
    ["snapshot.json", "journal.json"]
        .into_iter()
        .map(|name| world_dir.join(name))
        .filter(|path| !path.exists())
        .collect()
}

pub(super) fn build_oasis7_chain_runtime_args(options: &CliOptions) -> Vec<String> {
    let execution_world_dir = chain_execution_world_dir(options.chain_node_id.as_str());
    let mut args = vec![
        "--node-id".to_string(),
        options.chain_node_id.clone(),
        "--world-id".to_string(),
        chain_world_id(options),
        "--status-bind".to_string(),
        options.chain_status_bind.clone(),
        "--config".to_string(),
        chain_config_path(options.chain_node_id.as_str()),
        "--execution-world-dir".to_string(),
        execution_world_dir,
        "--node-role".to_string(),
        options.chain_node_role.clone(),
        "--p2p-user-mode".to_string(),
        options.chain_p2p_user_mode.clone(),
        "--node-tick-ms".to_string(),
        options.chain_node_tick_ms.to_string(),
        "--pos-slot-duration-ms".to_string(),
        options.chain_pos_slot_duration_ms.to_string(),
        "--pos-ticks-per-slot".to_string(),
        options.chain_pos_ticks_per_slot.to_string(),
        "--pos-proposal-tick-phase".to_string(),
        options.chain_pos_proposal_tick_phase.to_string(),
        if options.chain_pos_adaptive_tick_scheduler_enabled {
            "--pos-adaptive-tick-scheduler".to_string()
        } else {
            "--pos-no-adaptive-tick-scheduler".to_string()
        },
        "--pos-max-past-slot-lag".to_string(),
        options.chain_pos_max_past_slot_lag.to_string(),
    ];
    if options.chain_node_auto_attest_all_validators {
        args.push("--node-auto-attest-all".to_string());
    } else {
        args.push("--node-no-auto-attest-all".to_string());
    }
    if options.chain_network_tier_manifest.trim().is_empty() {
        args.push("--storage-profile".to_string());
        args.push(options.chain_storage_profile.as_str().to_string());
    } else {
        args.push("--network-tier-manifest".to_string());
        args.push(options.chain_network_tier_manifest.trim().to_string());
    }
    args.push(if options.chain_p2p_accept_public_entry {
        "--p2p-accept-public-entry".to_string()
    } else {
        "--p2p-reject-public-entry".to_string()
    });
    if let Some(genesis) = options.chain_pos_slot_clock_genesis_unix_ms {
        args.push("--pos-slot-clock-genesis-unix-ms".to_string());
        args.push(genesis.to_string());
    }
    for validator in &options.chain_node_validators {
        args.push("--node-validator".to_string());
        args.push(validator.clone());
    }
    for peer in &options.chain_replication_bootstrap_peers {
        args.push("--replication-network-peer".to_string());
        args.push(peer.clone());
    }
    if let Some(registry_path) = observer_registry_path_from_manifest(
        options.chain_network_tier_manifest.as_str(),
        options.chain_node_role.as_str(),
    ) {
        args.push("--genesis-validator-registry".to_string());
        args.push(registry_path);
    }
    if options.chain_enabled {
        for path in &options.provider_bootstrap_authority_paths {
            args.push("--provider-bootstrap-authority".to_string());
            args.push(path.clone());
        }
        if let Some(path) = options.local_test_provider_authority_path.as_ref() {
            args.push("--local-test-provider-authority".to_string());
            args.push(path.clone());
            args.push("--local-test-provider-wasm".to_string());
            args.push(
                options
                    .local_test_provider_wasm_path
                    .as_deref()
                    .expect("validated local provider WASM path")
                    .to_string(),
            );
            args.push("--local-test-provider-metadata".to_string());
            args.push(
                options
                    .local_test_provider_metadata_path
                    .as_deref()
                    .expect("validated local provider metadata path")
                    .to_string(),
            );
            args.push("--local-test-provider-agent-id".to_string());
            args.push(options.local_test_provider_agent_id.clone());
            args.push("--local-test-provider-owner-binding".to_string());
            args.push(options.local_test_provider_owner_binding.clone());
            args.push("--local-test-provider-finality-block-hash".to_string());
            args.push(
                options
                    .local_test_provider_finality_block_hash
                    .as_deref()
                    .expect("validated local provider finality marker")
                    .to_string(),
            );
            args.push("--local-test-provider-session-mode".to_string());
            args.push(options.local_test_provider_session_mode.clone());
        }
    }
    args
}

fn observer_registry_path_from_manifest(manifest_path: &str, chain_role: &str) -> Option<String> {
    if chain_role != "observer" {
        return None;
    }
    let manifest_path = Path::new(manifest_path.trim());
    let manifest_bytes = std::fs::read(manifest_path).ok()?;
    let manifest: serde_json::Value = serde_json::from_slice(manifest_bytes.as_slice()).ok()?;
    if manifest.get("tier").and_then(serde_json::Value::as_str) != Some("public_testnet")
        || manifest
            .get("validator_policy")
            .and_then(serde_json::Value::as_object)
            .and_then(|policy| policy.get("allow_observer_nodes"))
            != Some(&serde_json::Value::Bool(true))
    {
        return None;
    }
    let registry_ref = manifest
        .get("deployment_validator_registry")
        .and_then(serde_json::Value::as_object)
        .and_then(|binding| binding.get("ref"))
        .and_then(serde_json::Value::as_str)?;
    let registry_ref_path = Path::new(registry_ref);
    if registry_ref_path.is_absolute()
        || registry_ref_path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return None;
    }
    let registry_name = registry_ref_path.file_name()?.to_str()?;
    if registry_name.is_empty() {
        return None;
    }
    Some(
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(registry_name)
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(test)]
mod observer_registry_launcher_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture_dir() -> std::path::PathBuf {
        let base = std::env::temp_dir();
        for _ in 0..64 {
            let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "oasis7-game-launcher-observer-authority-{}-{sequence}",
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

    #[test]
    fn observer_manifest_forwards_bound_registry_to_runtime() {
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
        let options = CliOptions {
            chain_node_role: "observer".to_string(),
            chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
            ..CliOptions::default()
        };

        let args = build_oasis7_chain_runtime_args(&options);
        let registry_arg = args
            .windows(2)
            .find(|pair| pair[0] == "--genesis-validator-registry")
            .expect("bound registry argument");
        assert_eq!(registry_arg[1], registry.to_string_lossy());
    }

    #[test]
    fn observer_manifest_without_binding_does_not_synthesize_registry() {
        let root = fixture_dir();
        let manifest = root.join("manifest.json");
        std::fs::write(
            &manifest,
            br#"{
                "tier":"public_testnet",
                "validator_policy":{"allow_observer_nodes":true}
            }"#,
        )
        .expect("write manifest fixture");
        let options = CliOptions {
            chain_node_role: "observer".to_string(),
            chain_network_tier_manifest: manifest.to_string_lossy().into_owned(),
            ..CliOptions::default()
        };

        let args = build_oasis7_chain_runtime_args(&options);
        assert!(!args.iter().any(|arg| arg == "--genesis-validator-registry"));
    }
}
