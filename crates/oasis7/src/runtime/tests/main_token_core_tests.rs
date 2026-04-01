#[test]
fn main_token_queries_return_defaults_when_uninitialized() {
    let world = World::new();

    let config = world.main_token_config();
    assert_eq!(config.symbol, "AWT");
    assert_eq!(config.decimals, 9);
    assert_eq!(config.initial_supply, 0);
    assert_eq!(world.main_token_liquid_balance("missing-account"), 0);
    assert_eq!(world.main_token_treasury_balance("missing-bucket"), 0);
    assert!(world.main_token_genesis_bucket("missing-bucket").is_none());
    assert!(world.main_token_epoch_issuance_record(1).is_none());
    assert!(world
        .main_token_treasury_distribution_record("missing-distribution")
        .is_none());
}

#[test]
fn main_token_snapshot_roundtrip_persists_state() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        symbol: "AWT".to_string(),
        decimals: 9,
        initial_supply: 1_000_000_000,
        max_supply: Some(5_000_000_000),
        inflation_policy: MainTokenInflationPolicy {
            base_rate_bps: 410,
            ..MainTokenInflationPolicy::default()
        },
        issuance_split: MainTokenIssuanceSplitPolicy {
            node_service_reward_bps: 2_200,
            ..MainTokenIssuanceSplitPolicy::default()
        },
        burn_policy: MainTokenBurnPolicy {
            gas_base_fee_burn_bps: 3_200,
            ..MainTokenBurnPolicy::default()
        },
    });
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_050_000_000,
        circulating_supply: 820_000_000,
        total_issued: 100_000_000,
        total_burned: 50_000_000,
    });
    world
        .set_main_token_account_balance("protocol:treasury", 320_000_000, 0)
        .expect("set treasury account balance");
    world
        .set_main_token_account_balance("player:alice", 1_250_000, 350_000)
        .expect("set alice account balance");
    world
        .set_main_token_treasury_balance("ecosystem_pool", 120_000_000)
        .expect("set ecosystem pool");
    world
        .set_main_token_genesis_bucket(MainTokenGenesisAllocationBucketState {
            bucket_id: "ecosystem_growth_pool".to_string(),
            ratio_bps: 2_500,
            recipient: "protocol:treasury".to_string(),
            cliff_epochs: 30,
            linear_unlock_epochs: 360,
            start_epoch: 1,
            allocated_amount: 250_000_000,
            claimed_amount: 20_000_000,
        })
        .expect("set genesis bucket");
    world
        .record_main_token_epoch_issuance(MainTokenEpochIssuanceRecord {
            epoch_index: 12,
            inflation_rate_bps: 405,
            issued_amount: 1_337_000,
            staking_reward_amount: 802_200,
            node_service_reward_amount: 267_400,
            ecosystem_pool_amount: 200_550,
            security_reserve_amount: 66_850,
        })
        .expect("record issuance");

    let snapshot = world.snapshot();
    let restored = World::from_snapshot(snapshot, world.journal().clone()).expect("restore");

    assert_eq!(restored.main_token_config().initial_supply, 1_000_000_000);
    assert_eq!(restored.main_token_supply().total_supply, 1_050_000_000);
    assert_eq!(
        restored.main_token_liquid_balance("protocol:treasury"),
        320_000_000
    );
    assert_eq!(
        restored.main_token_liquid_balance("player:alice"),
        1_250_000
    );
    assert_eq!(
        restored.main_token_account_balance("player:alice"),
        Some(&MainTokenAccountBalance {
            account_id: "player:alice".to_string(),
            liquid_balance: 1_250_000,
            vested_balance: 350_000,
            restricted_starter_claim_balance: 0,
        })
    );
    assert_eq!(
        restored.main_token_treasury_balance("ecosystem_pool"),
        120_000_000
    );
    assert_eq!(
        restored
            .main_token_genesis_bucket("ecosystem_growth_pool")
            .expect("genesis bucket"),
        &MainTokenGenesisAllocationBucketState {
            bucket_id: "ecosystem_growth_pool".to_string(),
            ratio_bps: 2_500,
            recipient: "protocol:treasury".to_string(),
            cliff_epochs: 30,
            linear_unlock_epochs: 360,
            start_epoch: 1,
            allocated_amount: 250_000_000,
            claimed_amount: 20_000_000,
        }
    );
    assert_eq!(
        restored.main_token_epoch_issuance_record(12),
        Some(&MainTokenEpochIssuanceRecord {
            epoch_index: 12,
            inflation_rate_bps: 405,
            issued_amount: 1_337_000,
            staking_reward_amount: 802_200,
            node_service_reward_amount: 267_400,
            ecosystem_pool_amount: 200_550,
            security_reserve_amount: 66_850,
        })
    );
}

#[test]
fn main_token_initialize_genesis_action_populates_buckets_and_vested_balances() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        ..MainTokenConfig::default()
    });

    world.submit_action(Action::InitializeMainTokenGenesis {
        allocations: vec![
            MainTokenGenesisAllocationPlan {
                bucket_id: "consensus_bootstrap_pool".to_string(),
                ratio_bps: 6_000,
                recipient: "node:validator".to_string(),
                cliff_epochs: 0,
                linear_unlock_epochs: 10,
                start_epoch: 0,
            },
            MainTokenGenesisAllocationPlan {
                bucket_id: "ecosystem_growth_pool".to_string(),
                ratio_bps: 4_000,
                recipient: "protocol:treasury".to_string(),
                cliff_epochs: 0,
                linear_unlock_epochs: 20,
                start_epoch: 0,
            },
        ],
    });
    world.step().expect("initialize main token genesis");

    assert_eq!(world.main_token_supply().total_supply, 1_000);
    assert_eq!(world.main_token_supply().circulating_supply, 0);
    assert_eq!(world.main_token_liquid_balance("node:validator"), 0);
    assert_eq!(world.main_token_liquid_balance("protocol:treasury"), 0);
    assert_eq!(
        world
            .main_token_account_balance("node:validator")
            .expect("validator account")
            .vested_balance,
        600
    );
    assert_eq!(
        world
            .main_token_account_balance("protocol:treasury")
            .expect("treasury account")
            .vested_balance,
        400
    );
    let total_allocated = world
        .state()
        .main_token_genesis_buckets
        .values()
        .map(|bucket| bucket.allocated_amount)
        .sum::<u64>();
    assert_eq!(total_allocated, 1_000);
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::MainTokenGenesisInitialized {
            total_supply,
            allocations,
        }) => {
            assert_eq!(*total_supply, 1_000);
            assert_eq!(allocations.len(), 2);
        }
        other => panic!("expected MainTokenGenesisInitialized, got {other:?}"),
    }
}

#[test]
fn main_token_claim_vesting_action_releases_unlocked_balance_and_rejects_nonce_replay() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        ..MainTokenConfig::default()
    });
    world.submit_action(Action::InitializeMainTokenGenesis {
        allocations: vec![MainTokenGenesisAllocationPlan {
            bucket_id: "core_contributor_vesting".to_string(),
            ratio_bps: 10_000,
            recipient: "player:alice".to_string(),
            cliff_epochs: 0,
            linear_unlock_epochs: 10,
            start_epoch: 0,
        }],
    });
    world.step().expect("initialize main token genesis");

    world.submit_action(Action::ClaimMainTokenVesting {
        bucket_id: "core_contributor_vesting".to_string(),
        beneficiary: "player:alice".to_string(),
        nonce: 1,
    });
    world.step().expect("claim vesting");

    let alice = world
        .main_token_account_balance("player:alice")
        .expect("alice account");
    assert_eq!(alice.liquid_balance, 200);
    assert_eq!(alice.vested_balance, 800);
    assert_eq!(world.main_token_supply().circulating_supply, 200);
    assert_eq!(world.main_token_last_claim_nonce("player:alice"), Some(1));
    assert_eq!(
        world
            .main_token_genesis_bucket("core_contributor_vesting")
            .expect("bucket")
            .claimed_amount,
        200
    );

    world.submit_action(Action::ClaimMainTokenVesting {
        bucket_id: "core_contributor_vesting".to_string(),
        beneficiary: "player:alice".to_string(),
        nonce: 1,
    });
    world.step().expect("nonce replay should be rejected");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("nonce replay")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn main_token_transfer_action_moves_liquid_balance_and_updates_nonce() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        ..MainTokenConfig::default()
    });
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_000,
        circulating_supply: 1_000,
        total_issued: 0,
        total_burned: 0,
    });
    world
        .set_main_token_account_balance("player:alice", 900, 0)
        .expect("seed alice");
    world
        .set_main_token_account_balance("player:bob", 100, 0)
        .expect("seed bob");

    world.submit_action(Action::TransferMainToken {
        from_account_id: "player:alice".to_string(),
        to_account_id: "player:bob".to_string(),
        amount: 250,
        nonce: 1,
    });
    world.step().expect("transfer main token");

    assert_eq!(world.main_token_liquid_balance("player:alice"), 650);
    assert_eq!(world.main_token_liquid_balance("player:bob"), 350);
    assert_eq!(
        world.main_token_last_transfer_nonce("player:alice"),
        Some(1)
    );
    assert_eq!(world.main_token_supply().total_supply, 1_000);
    assert_eq!(world.main_token_supply().circulating_supply, 1_000);

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::MainTokenTransferred {
            from_account_id,
            to_account_id,
            amount,
            nonce,
        }) => {
            assert_eq!(from_account_id, "player:alice");
            assert_eq!(to_account_id, "player:bob");
            assert_eq!(*amount, 250);
            assert_eq!(*nonce, 1);
        }
        other => panic!("expected MainTokenTransferred, got {other:?}"),
    }
}

#[test]
fn main_token_transfer_action_rejects_insufficient_balance_without_mutation() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 100,
        ..MainTokenConfig::default()
    });
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 100,
        circulating_supply: 100,
        total_issued: 0,
        total_burned: 0,
    });
    world
        .set_main_token_account_balance("player:alice", 10, 0)
        .expect("seed alice");
    world
        .set_main_token_account_balance("player:bob", 0, 0)
        .expect("seed bob");

    world.submit_action(Action::TransferMainToken {
        from_account_id: "player:alice".to_string(),
        to_account_id: "player:bob".to_string(),
        amount: 11,
        nonce: 1,
    });
    world
        .step()
        .expect("insufficient transfer should be rejected");

    assert_eq!(world.main_token_liquid_balance("player:alice"), 10);
    assert_eq!(world.main_token_liquid_balance("player:bob"), 0);
    assert_eq!(world.main_token_last_transfer_nonce("player:alice"), None);

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("insufficient")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn main_token_transfer_action_rejects_nonce_replay() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 100,
        ..MainTokenConfig::default()
    });
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 100,
        circulating_supply: 100,
        total_issued: 0,
        total_burned: 0,
    });
    world
        .set_main_token_account_balance("player:alice", 100, 0)
        .expect("seed alice");
    world
        .set_main_token_account_balance("player:bob", 0, 0)
        .expect("seed bob");

    world.submit_action(Action::TransferMainToken {
        from_account_id: "player:alice".to_string(),
        to_account_id: "player:bob".to_string(),
        amount: 30,
        nonce: 1,
    });
    world.step().expect("first transfer");
    assert_eq!(
        world.main_token_last_transfer_nonce("player:alice"),
        Some(1)
    );

    world.submit_action(Action::TransferMainToken {
        from_account_id: "player:alice".to_string(),
        to_account_id: "player:bob".to_string(),
        amount: 10,
        nonce: 1,
    });
    world.step().expect("replay transfer should be rejected");

    assert_eq!(world.main_token_liquid_balance("player:alice"), 70);
    assert_eq!(world.main_token_liquid_balance("player:bob"), 30);
    assert_eq!(
        world.main_token_last_transfer_nonce("player:alice"),
        Some(1)
    );

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("nonce replay")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn main_token_epoch_issuance_applies_formula_clamp_split_and_rejects_duplicate_epoch() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        max_supply: Some(1_005),
        inflation_policy: MainTokenInflationPolicy {
            base_rate_bps: 300,
            min_rate_bps: 200,
            max_rate_bps: 800,
            target_stake_ratio_bps: 6_000,
            stake_feedback_gain_bps: 1_000,
            epochs_per_year: 10,
        },
        issuance_split: MainTokenIssuanceSplitPolicy::default(),
        ..MainTokenConfig::default()
    });
    world.submit_action(Action::InitializeMainTokenGenesis {
        allocations: vec![MainTokenGenesisAllocationPlan {
            bucket_id: "genesis_pool".to_string(),
            ratio_bps: 10_000,
            recipient: "protocol:treasury".to_string(),
            cliff_epochs: 0,
            linear_unlock_epochs: 0,
            start_epoch: 0,
        }],
    });
    world.step().expect("initialize main token genesis");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_000,
        circulating_supply: 1_000,
        total_issued: 0,
        total_burned: 0,
    });

    world.submit_action(Action::ApplyMainTokenEpochIssuance {
        epoch_index: 1,
        actual_stake_ratio_bps: 6_000,
    });
    world.step().expect("apply epoch issuance #1");

    assert_eq!(world.main_token_supply().total_issued, 3);
    assert_eq!(world.main_token_supply().total_supply, 1_003);
    assert_eq!(
        world.main_token_epoch_issuance_record(1),
        Some(&MainTokenEpochIssuanceRecord {
            epoch_index: 1,
            inflation_rate_bps: 300,
            issued_amount: 3,
            staking_reward_amount: 1,
            node_service_reward_amount: 0,
            ecosystem_pool_amount: 0,
            security_reserve_amount: 2,
        })
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD),
        1
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD),
        0
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL),
        0
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE),
        2
    );

    world.submit_action(Action::ApplyMainTokenEpochIssuance {
        epoch_index: 2,
        actual_stake_ratio_bps: 0,
    });
    world.step().expect("apply epoch issuance #2");

    assert_eq!(world.main_token_supply().total_issued, 5);
    assert_eq!(world.main_token_supply().total_supply, 1_005);
    assert_eq!(
        world.main_token_epoch_issuance_record(2),
        Some(&MainTokenEpochIssuanceRecord {
            epoch_index: 2,
            inflation_rate_bps: 800,
            issued_amount: 2,
            staking_reward_amount: 1,
            node_service_reward_amount: 0,
            ecosystem_pool_amount: 0,
            security_reserve_amount: 1,
        })
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD),
        2
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE),
        3
    );

    world.submit_action(Action::ApplyMainTokenEpochIssuance {
        epoch_index: 2,
        actual_stake_ratio_bps: 6_000,
    });
    world
        .step()
        .expect("duplicate epoch issuance should be rejected");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("already exists")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn main_token_fee_settlement_burns_supply_and_tracks_treasury_buckets() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        burn_policy: MainTokenBurnPolicy {
            gas_base_fee_burn_bps: 3_000,
            slash_burn_bps: 5_000,
            module_fee_burn_bps: 2_000,
        },
        ..MainTokenConfig::default()
    });
    world.submit_action(Action::InitializeMainTokenGenesis {
        allocations: vec![MainTokenGenesisAllocationPlan {
            bucket_id: "genesis_pool".to_string(),
            ratio_bps: 10_000,
            recipient: "protocol:treasury".to_string(),
            cliff_epochs: 0,
            linear_unlock_epochs: 0,
            start_epoch: 0,
        }],
    });
    world.step().expect("initialize main token genesis");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_000,
        circulating_supply: 500,
        total_issued: 0,
        total_burned: 0,
    });

    world.submit_action(Action::SettleMainTokenFee {
        fee_kind: MainTokenFeeKind::GasBaseFee,
        amount: 100,
    });
    world.step().expect("settle gas fee");

    world.submit_action(Action::SettleMainTokenFee {
        fee_kind: MainTokenFeeKind::SlashPenalty,
        amount: 100,
    });
    world.step().expect("settle slash fee");

    world.submit_action(Action::SettleMainTokenFee {
        fee_kind: MainTokenFeeKind::ModuleFee,
        amount: 100,
    });
    world.step().expect("settle module fee");

    assert_eq!(world.main_token_supply().total_supply, 900);
    assert_eq!(world.main_token_supply().circulating_supply, 200);
    assert_eq!(world.main_token_supply().total_burned, 100);
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_GAS_FEE),
        70
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_SLASH),
        50
    );
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_MODULE_FEE),
        80
    );

    world.submit_action(Action::SettleMainTokenFee {
        fee_kind: MainTokenFeeKind::GasBaseFee,
        amount: 250,
    });
    world
        .step()
        .expect("insufficient circulating should reject");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("circulating")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}
