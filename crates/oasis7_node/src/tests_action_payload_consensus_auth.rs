use super::*;

#[test]
fn submit_consensus_action_payload_rejects_unsigned_main_token_transfer_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-transfer",
            "world-token-transfer",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let (public_key_hex, _) = token_auth_test_signer(0x21);
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "TransferMainToken",
        "data": {
            "from_account_id": main_token_account_id_from_public_key(public_key_hex.as_str()),
            "to_account_id": "protocol:receiver",
            "amount": 7,
            "nonce": 1
        }
    }));
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("unsigned transfer payload must fail");
    assert!(err.to_string().contains("missing_main_token_auth"));
}

#[test]
fn submit_consensus_action_payload_accepts_signed_main_token_transfer_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-transfer-ok",
            "world-token-transfer-ok",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let (public_key_hex, _) = token_auth_test_signer(0x22);
    let account_id = main_token_account_id_from_public_key(public_key_hex.as_str());
    let payload = encode_signed_main_token_runtime_payload(
        json!({
            "type": "TransferMainToken",
            "data": {
                "from_account_id": account_id,
                "to_account_id": "protocol:receiver",
                "amount": 7,
                "nonce": 1
            }
        }),
        account_id.as_str(),
        0x22,
    );
    runtime
        .submit_consensus_action_payload(1, payload)
        .expect("signed transfer payload should pass");
}

#[test]
fn submit_consensus_action_payload_rejects_unsigned_main_token_claim_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new("node-token-claim", "world-token-claim", NodeRole::Observer)
            .expect("config"),
    );
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "ClaimMainTokenVesting",
        "data": {
            "bucket_id": "team_long_term_vesting",
            "beneficiary": "protocol:team-core-vesting",
            "nonce": 1
        }
    }));
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("unsigned claim payload must fail");
    assert!(err.to_string().contains("missing_main_token_auth"));
}

#[test]
fn submit_consensus_action_payload_accepts_signed_main_token_claim_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-claim-ok",
            "world-token-claim-ok",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_signed_main_token_runtime_payload(
        json!({
            "type": "ClaimMainTokenVesting",
            "data": {
                "bucket_id": "team_long_term_vesting",
                "beneficiary": "protocol:team-core-vesting",
                "nonce": 1
            }
        }),
        "protocol:team-core-vesting",
        0x23,
    );
    runtime
        .submit_consensus_action_payload(1, payload)
        .expect("signed claim payload should pass");
}

#[test]
fn submit_consensus_action_payload_rejects_unsigned_main_token_genesis_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis",
            "world-token-genesis",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "InitializeMainTokenGenesis",
        "data": {
            "allocations": [{
                "bucket_id": "team_long_term_vesting",
                "ratio_bps": 2000,
                "recipient": "protocol:team-core-vesting",
                "cliff_epochs": 365,
                "linear_unlock_epochs": 1095,
                "start_epoch": 0
            }]
        }
    }));
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("unsigned genesis payload must fail");
    assert!(err.to_string().contains("missing_main_token_auth"));
}

#[test]
fn submit_consensus_action_payload_accepts_signed_main_token_genesis_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis-ok",
            "world-token-genesis-ok",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "InitializeMainTokenGenesis",
            "data": {
                "allocations": [{
                    "bucket_id": "team_long_term_vesting",
                    "ratio_bps": 2000,
                    "recipient": "protocol:team-core-vesting",
                    "cliff_epochs": 365,
                    "linear_unlock_epochs": 1095,
                    "start_epoch": 0
                }]
            }
        }),
        DEFAULT_GENESIS_CONTROLLER_SLOT,
        2,
        &[0x24, 0x28],
    );
    runtime
        .submit_consensus_action_payload(1, payload)
        .expect("signed genesis payload should pass");
}

#[test]
fn submit_consensus_action_payload_rejects_signed_main_token_genesis_action_with_wrong_controller_slot(
) {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis-wrong-slot",
            "world-token-genesis-wrong-slot",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "InitializeMainTokenGenesis",
            "data": {
                "allocations": [{
                    "bucket_id": "team_long_term_vesting",
                    "ratio_bps": 2000,
                    "recipient": "protocol:team-core-vesting",
                    "cliff_epochs": 365,
                    "linear_unlock_epochs": 1095,
                    "start_epoch": 0
                }]
            }
        }),
        "msig.foundation_ops.v1",
        2,
        &[0x24, 0x28],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("wrong genesis controller slot must fail");
    assert!(err.to_string().contains("genesis controller slot"));
}

#[test]
fn submit_consensus_action_payload_rejects_genesis_when_controller_policy_missing() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis-policy-missing",
            "world-token-genesis-policy-missing",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "InitializeMainTokenGenesis",
            "data": {
                "allocations": [{
                    "bucket_id": "team_long_term_vesting",
                    "ratio_bps": 2000,
                    "recipient": "protocol:team-core-vesting",
                    "cliff_epochs": 365,
                    "linear_unlock_epochs": 1095,
                    "start_epoch": 0
                }]
            }
        }),
        DEFAULT_GENESIS_CONTROLLER_SLOT,
        1,
        &[0x24],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("missing controller policy must fail");
    assert!(err.to_string().contains("allowlist is empty"));
}

#[test]
fn submit_consensus_action_payload_rejects_genesis_when_threshold_not_met() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis-threshold-miss",
            "world-token-genesis-threshold-miss",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "InitializeMainTokenGenesis",
            "data": {
                "allocations": [{
                    "bucket_id": "team_long_term_vesting",
                    "ratio_bps": 2000,
                    "recipient": "protocol:team-core-vesting",
                    "cliff_epochs": 365,
                    "linear_unlock_epochs": 1095,
                    "start_epoch": 0
                }]
            }
        }),
        DEFAULT_GENESIS_CONTROLLER_SLOT,
        1,
        &[0x24],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("threshold mismatch must fail");
    assert!(err.to_string().contains("threshold mismatch"));
}

#[test]
fn submit_consensus_action_payload_rejects_genesis_when_signer_not_allowlisted() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-genesis-allowlist-miss",
            "world-token-genesis-allowlist-miss",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            1,
            &[0x24],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_signed_main_token_runtime_payload(
        json!({
            "type": "InitializeMainTokenGenesis",
            "data": {
                "allocations": [{
                    "bucket_id": "team_long_term_vesting",
                    "ratio_bps": 2000,
                    "recipient": "protocol:team-core-vesting",
                    "cliff_epochs": 365,
                    "linear_unlock_epochs": 1095,
                    "start_epoch": 0
                }]
            }
        }),
        DEFAULT_GENESIS_CONTROLLER_SLOT,
        0x26,
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("signer outside allowlist must fail");
    assert!(err.to_string().contains("not allowlisted"));
}

#[test]
fn submit_consensus_action_payload_rejects_unsigned_main_token_treasury_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury",
            "world-token-treasury",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "DistributeMainTokenTreasury",
        "data": {
            "proposal_id": 1,
            "distribution_id": "treasury-1",
            "bucket_id": "ecosystem_pool",
            "distributions": [{
                "account_id": "protocol:ecosystem-grant",
                "amount": 50
            }]
        }
    }));
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("unsigned treasury payload must fail");
    assert!(err.to_string().contains("missing_main_token_auth"));
}

#[test]
fn submit_consensus_action_payload_accepts_signed_main_token_treasury_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury-ok",
            "world-token-treasury-ok",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "DistributeMainTokenTreasury",
            "data": {
                "proposal_id": 1,
                "distribution_id": "treasury-1",
                "bucket_id": "ecosystem_pool",
                "distributions": [{
                    "account_id": "protocol:ecosystem-grant",
                    "amount": 50
                }]
            }
        }),
        DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
        2,
        &[0x25, 0x29],
    );
    runtime
        .submit_consensus_action_payload(1, payload)
        .expect("signed treasury payload should pass");
}

#[test]
fn submit_consensus_action_payload_rejects_signed_main_token_treasury_action_with_wrong_controller_slot(
) {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury-wrong-slot",
            "world-token-treasury-wrong-slot",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "DistributeMainTokenTreasury",
            "data": {
                "proposal_id": 1,
                "distribution_id": "treasury-1",
                "bucket_id": "ecosystem_pool",
                "distributions": [{
                    "account_id": "protocol:ecosystem-grant",
                    "amount": 50
                }]
            }
        }),
        "msig.treasury.v1",
        2,
        &[0x25, 0x29],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("wrong treasury controller slot must fail");
    assert!(err.to_string().contains("treasury controller slot"));
}

#[test]
fn submit_consensus_action_payload_rejects_treasury_when_allowlist_is_empty() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury-policy-missing",
            "world-token-treasury-policy-missing",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "DistributeMainTokenTreasury",
            "data": {
                "proposal_id": 1,
                "distribution_id": "treasury-1",
                "bucket_id": "ecosystem_pool",
                "distributions": [{
                    "account_id": "protocol:ecosystem-grant",
                    "amount": 50
                }]
            }
        }),
        DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
        1,
        &[0x25],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("empty treasury allowlist must fail");
    assert!(err.to_string().contains("allowlist is empty"));
}

#[test]
fn submit_consensus_action_payload_rejects_treasury_when_threshold_not_met() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury-threshold-miss",
            "world-token-treasury-threshold-miss",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "DistributeMainTokenTreasury",
            "data": {
                "proposal_id": 1,
                "distribution_id": "treasury-1",
                "bucket_id": "ecosystem_pool",
                "distributions": [{
                    "account_id": "protocol:ecosystem-grant",
                    "amount": 50
                }]
            }
        }),
        DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
        1,
        &[0x25],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("treasury threshold mismatch must fail");
    assert!(err.to_string().contains("threshold mismatch"));
}

#[test]
fn submit_consensus_action_payload_rejects_treasury_when_signer_not_allowlisted() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-treasury-allowlist-miss",
            "world-token-treasury-allowlist-miss",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            1,
            &[0x25],
        ))
        .expect("controller binding"),
    );
    let payload = encode_signed_main_token_runtime_payload(
        json!({
            "type": "DistributeMainTokenTreasury",
            "data": {
                "proposal_id": 1,
                "distribution_id": "treasury-1",
                "bucket_id": "ecosystem_pool",
                "distributions": [{
                    "account_id": "protocol:ecosystem-grant",
                    "amount": 50
                }]
            }
        }),
        DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
        0x27,
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("treasury signer outside allowlist must fail");
    assert!(err.to_string().contains("not allowlisted"));
}

#[test]
fn submit_consensus_action_payload_rejects_unsigned_restricted_grant_admin_registry_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-restricted-admin",
            "world-token-restricted-admin",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "UpdateRestrictedStarterClaimAdminRegistry",
        "data": {
            "controller_account_id": DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
            "next_admin_account_ids": ["liveops"]
        }
    }));
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("unsigned restricted admin registry payload must fail");
    assert!(err.to_string().contains("missing_main_token_auth"));
}

#[test]
fn submit_consensus_action_payload_accepts_signed_restricted_grant_admin_registry_action() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-restricted-admin-ok",
            "world-token-restricted-admin-ok",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "UpdateRestrictedStarterClaimAdminRegistry",
            "data": {
                "controller_account_id": DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
                "next_admin_account_ids": ["liveops", "ops_backup"]
            }
        }),
        DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
        2,
        &[0x25, 0x29],
    );
    runtime
        .submit_consensus_action_payload(1, payload)
        .expect("signed restricted admin registry payload should pass");
}

#[test]
fn submit_consensus_action_payload_rejects_restricted_grant_admin_registry_action_with_wrong_controller_slot(
) {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-token-restricted-admin-wrong-slot",
            "world-token-restricted-admin-wrong-slot",
            NodeRole::Observer,
        )
        .expect("config")
        .with_main_token_controller_binding(configured_controller_binding(
            2,
            &[0x24, 0x28],
            2,
            &[0x25, 0x29],
        ))
        .expect("controller binding"),
    );
    let payload = encode_threshold_signed_main_token_runtime_payload(
        json!({
            "type": "UpdateRestrictedStarterClaimAdminRegistry",
            "data": {
                "controller_account_id": "msig.foundation_ops.v1",
                "next_admin_account_ids": ["liveops"]
            }
        }),
        "msig.foundation_ops.v1",
        2,
        &[0x25, 0x29],
    );
    let err = runtime
        .submit_consensus_action_payload(1, payload)
        .expect_err("wrong restricted admin registry controller slot must fail");
    assert!(err.to_string().contains("restricted claim admin registry controller slot"));
}
