use super::*;

#[test]
fn collect_chain_required_config_issues_accepts_valid_required_fields() {
    let chain_runtime_bin = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .to_string();
    let config = LaunchConfig {
        chain_enabled: true,
        chain_runtime_bin,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_node_tick_ms: "200".to_string(),
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    };

    let issues = collect_chain_required_config_issues(&config);
    assert!(issues.is_empty());
}

#[test]
fn collect_chain_required_config_issues_requires_public_entry_confirmation() {
    let issues = collect_chain_required_config_issues(&LaunchConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_enabled: true,
        chain_runtime_bin: std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: false,
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    });
    assert!(issues.contains(&ConfigIssue::ChainPublicEntryConfirmationRequired));
}

#[test]
fn build_chain_runtime_args_requires_public_entry_confirmation() {
    let err = build_chain_runtime_args(&LaunchConfig {
        deployment_mode: "trusted_local_only".to_string(),
        chain_enabled: true,
        chain_runtime_bin: "/tmp/oasis7_chain_runtime".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: false,
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    })
    .expect_err("public entry should require explicit confirmation");
    assert!(err.contains("explicit confirmation"));
}

#[test]
fn issue_field_ids_maps_phase_out_of_range_to_related_fields() {
    let ids = issue_field_ids(ConfigIssue::ChainPosProposalTickPhaseOutOfRange);
    assert_eq!(
        ids,
        &["chain_pos_ticks_per_slot", "chain_pos_proposal_tick_phase"]
    );
}
