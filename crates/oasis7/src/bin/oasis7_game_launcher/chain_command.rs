use std::path::{Path, PathBuf};

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
