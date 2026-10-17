use super::*;

#[test]
fn parse_options_rejects_invalid_port() {
    let err = parse_options(["--viewer-port", "70000"].into_iter()).expect_err("should fail");
    assert!(err.contains("integer"));
}

#[test]
fn parse_options_rejects_invalid_bind_format() {
    let err = parse_options(["--live-bind", "127.0.0.1"].into_iter()).expect_err("should fail");
    assert!(err.contains("<host:port>"));
}

#[test]
fn parse_host_port_parses_valid_value() {
    let (host, port) = parse_host_port("127.0.0.1:5011", "--web-bind").expect("ok");
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 5011);
}

#[test]
fn parse_host_port_accepts_bracketed_ipv6() {
    let (host, port) = parse_host_port("[::1]:5011", "--web-bind").expect("ok");
    assert_eq!(host, "::1");
    assert_eq!(port, 5011);
}

#[test]
fn parse_host_port_rejects_unbracketed_ipv6() {
    let err = parse_host_port("::1:5011", "--web-bind").expect_err("should fail");
    assert!(err.contains("wrapped in []"));
}

#[test]
fn parse_host_port_rejects_zero_port() {
    let err = parse_host_port("127.0.0.1:0", "--web-bind").expect_err("should fail");
    assert!(err.contains("1..=65535"));
}

#[test]
fn build_game_url_rewrites_zero_bind_host_to_loopback() {
    let options = CliOptions {
        viewer_host: "0.0.0.0".to_string(),
        deployment_mode: "hosted_public_join".to_string(),
        viewer_port: 4173,
        web_bind: "0.0.0.0:5011".to_string(),
        ..CliOptions::default()
    };
    let url = build_game_url(&options);
    assert!(url.starts_with(
        "http://127.0.0.1:4173/?render_mode=viewer&ws=ws%3A%2F%2F127.0.0.1%3A5011&hosted_access="
    ));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
    assert!(url.contains("%22local_chain_runtime%22%3A%22blocked_for_public_player_plane%22"));
    assert!(url.contains("%22node_admission%22%3A%22operator_managed_node_onboarding_only%22"));
}

#[test]
fn build_game_url_brackets_ipv6_hosts() {
    let options = CliOptions {
        viewer_host: "::1".to_string(),
        viewer_port: 4173,
        web_bind: "[::1]:5011".to_string(),
        ..CliOptions::default()
    };
    let url = build_game_url(&options);
    assert!(url.starts_with(
        "http://[::1]:4173/?render_mode=viewer&ws=ws%3A%2F%2F%5B%3A%3A1%5D%3A5011&hosted_access="
    ));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
}

#[test]
fn build_oasis7_chain_runtime_args_includes_storage_profile() {
    let options = CliOptions {
        scenario: "sandbox".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_storage_profile: StorageProfile::ReleaseDefault,
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: true,
        chain_replication_bootstrap_peers: vec![
            "/ip4/127.0.0.1/tcp/4100".to_string(),
            "/dns4/bootstrap.example/tcp/4101".to_string(),
        ],
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--storage-profile".to_string()));
    assert!(args.contains(&"release_default".to_string()));
    assert!(args.contains(&"--world-id".to_string()));
    assert!(args.contains(&"live-sandbox".to_string()));
    assert!(args.contains(&"--execution-world-dir".to_string()));
    assert!(args.contains(&"--p2p-user-mode".to_string()));
    assert!(args.contains(&"public_entry".to_string()));
    assert!(args.contains(&"--p2p-accept-public-entry".to_string()));
    assert_eq!(
        args.iter()
            .filter(|value| value.as_str() == "--replication-network-peer")
            .count(),
        2
    );
    assert!(args.contains(&"/ip4/127.0.0.1/tcp/4100".to_string()));
    assert!(args.contains(&"/dns4/bootstrap.example/tcp/4101".to_string()));
    assert!(
        args.contains(&"output/chain-runtime/chain-a/reward-runtime-execution-world".to_string())
    );
}

#[test]
fn build_oasis7_chain_runtime_args_derives_world_id_from_explicit_scenario() {
    let options = CliOptions {
        scenario: "sandbox".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_world_id: None,
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--world-id".to_string()));
    assert!(args.contains(&"live-sandbox".to_string()));
}

#[test]
fn parse_options_local_standalone_test_builds_self_validating_private_chain() {
    let options = parse_options(
        [
            "--chain-node-id",
            "local-node-a",
            "--deployment-mode",
            "trusted_local_only",
            "--allow-trusted-local-playtest",
            "--chain-local-standalone-test",
            "--no-open-browser",
        ]
        .into_iter(),
    )
    .expect("local standalone profile should parse");

    assert!(options.chain_local_standalone_test);
    assert_eq!(options.chain_p2p_user_mode, "private_safe");
    assert!(!options.chain_p2p_accept_public_entry);
    assert!(options.chain_replication_bootstrap_peers.is_empty());
    assert!(options.chain_node_auto_attest_all_validators);
    assert_eq!(options.chain_pos_slot_duration_ms, 1_000);
    assert_eq!(options.chain_pos_ticks_per_slot, 1);
    assert_eq!(options.chain_pos_proposal_tick_phase, 0);
    assert_eq!(
        options.chain_node_validators,
        vec!["local-node-a:100".to_string()]
    );

    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--node-auto-attest-all".to_string()));
    assert!(args.contains(&"--node-validator".to_string()));
    assert!(args.contains(&"local-node-a:100".to_string()));
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&"output/chain-runtime/local-node-a/config.toml".to_string()));
    assert!(!args.contains(&"--replication-network-peer".to_string()));
}

#[test]
fn build_oasis7_chain_runtime_args_supports_all_storage_profiles() {
    for (profile, expected) in [
        (StorageProfile::DevLocal, "dev_local"),
        (StorageProfile::ReleaseDefault, "release_default"),
        (StorageProfile::SoakForensics, "soak_forensics"),
    ] {
        let options = CliOptions {
            scenario: "sandbox".to_string(),
            chain_node_id: format!("chain-{expected}"),
            chain_status_bind: "127.0.0.1:6121".to_string(),
            chain_storage_profile: profile,
            ..CliOptions::default()
        };
        let args = build_oasis7_chain_runtime_args(&options);
        assert!(args.contains(&"--storage-profile".to_string()));
        assert!(args.contains(&expected.to_string()));
    }
}

#[test]
fn build_oasis7_chain_runtime_args_prefers_network_tier_manifest_when_present() {
    let options = CliOptions {
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_network_tier_manifest: "/tmp/public-testnet.json".to_string(),
        chain_storage_profile: StorageProfile::ReleaseDefault,
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--network-tier-manifest".to_string()));
    assert!(args.contains(&"/tmp/public-testnet.json".to_string()));
    assert!(!args.contains(&"--storage-profile".to_string()));
    assert!(args.contains(&"--node-role".to_string()));
    assert!(args.contains(&"sequencer".to_string()));
}

#[test]
fn parse_options_still_validates_chain_node_role_when_manifest_is_present() {
    let err = parse_options(
        [
            "--chain-network-tier-manifest",
            "/tmp/public-testnet.json",
            "--chain-node-role",
            "invalid",
        ]
        .into_iter(),
    )
    .expect_err("should fail");
    assert!(err.contains("--chain-node-role"));
}
