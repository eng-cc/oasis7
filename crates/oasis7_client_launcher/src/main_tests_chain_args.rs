use super::*;

#[test]
fn build_chain_runtime_args_resolves_public_testnet_manifest_from_tier() {
    let config = LaunchConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_enabled: true,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_network_tier: "public_testnet".to_string(),
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: true,
        ..LaunchConfig::default()
    };

    let args = build_chain_runtime_args(&config).expect("args should build");

    assert!(args.contains(&"--network-tier-manifest".to_string()));
    assert!(args
        .contains(&"doc/testing/templates/network-tier-public-testnet.example.json".to_string()));
}
