//! Actual local NodeRuntime engineering entry. Deployment remains deferred by
//! the user; this opt-in path supplies no formal readiness or network finality.
use super::{
    CliOptions, NodeConfig, NodeRole, NodeRuntime, build_feedback_submit_signer,
    controlled_live_config::GuardedAuthority,
    execution_bridge::controlled_live::GuardedExecutionDriver, node_keypair_config,
    status_server_support,
};
use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    time::Duration,
};

pub(super) fn run(mut options: CliOptions) -> Result<(), String> {
    // Reject every external transport/provider lane before acquiring endpoint
    // locks or creating any directory. This path never builds a libp2p network.
    let defaults = CliOptions::default();
    if options.status_bind != defaults.status_bind
        || options.storage_profile != defaults.storage_profile
        || options.traffic_profile != defaults.traffic_profile
        || options.node_tick_ms != defaults.node_tick_ms
        || options.pos_slot_duration_ms != defaults.pos_slot_duration_ms
        || options.pos_ticks_per_slot != defaults.pos_ticks_per_slot
        || options.pos_proposal_tick_phase != defaults.pos_proposal_tick_phase
        || options.pos_adaptive_tick_scheduler_enabled
            != defaults.pos_adaptive_tick_scheduler_enabled
        || options.pos_slot_clock_genesis_unix_ms != defaults.pos_slot_clock_genesis_unix_ms
        || options.pos_max_past_slot_lag != defaults.pos_max_past_slot_lag
        || options.chain_local_standalone_test
        || options.p2p_accept_public_entry
        || options.agent_decision_source.is_some()
        || options.agent_provider_backend.is_some()
        || options.node_gossip_bind.is_some()
        || !options.node_gossip_peers.is_empty()
        || !options.replication_network_listen_addrs.is_empty()
        || !options.replication_network_bootstrap_peers.is_empty()
        || !options.replication_remote_writer_public_keys.is_empty()
        || options.network_tier_manifest_path.is_some()
        || options.loaded_network_tier_manifest.is_some()
        || options.genesis_validator_registry_path.is_some()
        || options.deployment_inventory_path.is_some()
        || options.local_test_provider_authority_path.is_some()
        || !options.provider_backed_bootstrap_authority_paths.is_empty()
        || options.reward_runtime_enabled
        || options.capture_schedule_recipe_history
        || !options.node_validators.is_empty()
        || !options.node_validator_signer_public_keys.is_empty()
        || !matches!(options.node_role, NodeRole::Sequencer)
    {
        return Err("guarded engineering startup rejects external transports, providers and consensus overrides".into());
    }
    let external = options
        .guarded_initial_config
        .as_deref()
        .ok_or("guarded configuration missing")?;
    let authority = GuardedAuthority::load(external)?;
    super::controlled_live_config::require_private_owned(
        &authority.config.node_keypair_directory,
        true,
    )?;
    let (keypair, key_path) = node_keypair_config::read_existing_node_keypair_in_secure_config_dir(
        &authority.config.node_keypair_directory,
    )?;
    super::controlled_live_config::require_private_owned(&key_path, false)?;
    let signer = build_feedback_submit_signer(&authority.config.node_id, &keypair)?;
    let records = authority.config.records_directory.clone();
    let bind = authority.config.status_bind;
    options.node_id = authority.config.node_id.clone();
    options.world_id = authority.policy.trust.world_id.clone();
    let interval = authority.config.node_tick_ms;
    // Existing local single-validator decision machinery determines the
    // decision. Its continuation is engineering recovery material, not a
    // signature proof of network PoS approval.
    let config = NodeConfig::new(&options.node_id, &options.world_id, NodeRole::Sequencer)
        .and_then(|c| c.with_tick_interval(Duration::from_millis(interval)))
        .map_err(|e| format!("guarded local node configuration: {e:?}"))?
        .with_require_execution_on_commit(true)
        .with_require_peer_execution_hashes(false)
        .with_auto_attest_all_validators(true);
    let driver = GuardedExecutionDriver::new(authority, records.clone())?;
    let bootstrap = driver.bootstrap()?;
    options.guarded_read_authority = Some(driver.read_authority());
    let listener = TcpListener::bind(bind).map_err(|e| format!("guarded loopback ingress: {e}"))?;
    let mut node = NodeRuntime::new(config)
        .with_local_execution_bootstrap(bootstrap)
        .with_execution_hook(driver);
    node.start()
        .map_err(|e| format!("guarded NodeRuntime start: {e:?}"))?;
    let runtime = Arc::new(Mutex::new(node));
    tracing::info!(
        scope = super::controlled_live_config::ENGINEERING_SCOPE,
        "local engineering checkpoint prerequisite ingress started"
    );
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                runtime
                    .lock()
                    .map_err(|_| "guarded runtime lock unavailable")?
                    .stop()
                    .map_err(|e| format!("guarded stop: {e:?}"))?;
                return Err(format!("guarded loopback ingress failed: {error}"));
            }
        };
        if let Err(reason) = status_server_support::handle_guarded_connection(
            stream, &runtime, &options, &records, &signer,
        ) {
            tracing::warn!(error = %reason, "guarded service request rejected");
        }
    }
    Err("guarded loopback ingress ended".into())
}
