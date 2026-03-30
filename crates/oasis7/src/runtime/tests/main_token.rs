use super::super::*;
use std::collections::{BTreeMap, BTreeSet};

fn set_main_token_controller_registry_for_tests(world: &mut World, ecosystem_controller: &str) {
    world
        .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
            genesis_controller_account_id: "msig.genesis.v1".to_string(),
            treasury_bucket_controller_slots: BTreeMap::from([(
                MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
                ecosystem_controller.to_string(),
            )]),
            restricted_starter_claim_admin_account_ids: BTreeSet::new(),
            controller_signer_policies: BTreeMap::from([
                (
                    "msig.genesis.v1".to_string(),
                    GovernanceThresholdSignerPolicy {
                        threshold: 1,
                        allowed_public_keys: BTreeSet::from([
                            "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                                .to_string(),
                        ]),
                    },
                ),
                (
                    ecosystem_controller.to_string(),
                    GovernanceThresholdSignerPolicy {
                        threshold: 2,
                        allowed_public_keys: BTreeSet::from([
                            "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                                .to_string(),
                            "10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"
                                .to_string(),
                        ]),
                    },
                ),
            ]),
        })
        .expect("set controller registry for tests");
}

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

#[test]
fn main_token_policy_update_is_delayed_and_audited_before_it_affects_issuance() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        inflation_policy: MainTokenInflationPolicy {
            base_rate_bps: 300,
            min_rate_bps: 300,
            max_rate_bps: 300,
            target_stake_ratio_bps: 6_000,
            stake_feedback_gain_bps: 0,
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

    let schedule_base_epoch = world.state().time;
    let mut scheduled_config = world.main_token_config().clone();
    scheduled_config.initial_supply = 1_000;
    scheduled_config.inflation_policy = MainTokenInflationPolicy {
        base_rate_bps: 800,
        min_rate_bps: 800,
        max_rate_bps: 800,
        target_stake_ratio_bps: 6_000,
        stake_feedback_gain_bps: 0,
        epochs_per_year: 10,
    };
    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow governance proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve governance proposal");
    world.submit_action(Action::UpdateMainTokenPolicy {
        proposal_id,
        next: scheduled_config,
    });
    world.step().expect("schedule main token policy update");

    let effective_epoch = match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::MainTokenPolicyUpdateScheduled {
            proposal_id: event_proposal_id,
            effective_epoch: event_effective_epoch,
            ..
        }) => {
            assert_eq!(*event_proposal_id, proposal_id);
            assert!(*event_effective_epoch > schedule_base_epoch);
            *event_effective_epoch
        }
        other => panic!("expected MainTokenPolicyUpdateScheduled, got {other:?}"),
    };
    assert!(world
        .main_token_scheduled_policy_update(effective_epoch)
        .is_some());

    world.submit_action(Action::ApplyMainTokenEpochIssuance {
        epoch_index: effective_epoch - 1,
        actual_stake_ratio_bps: 6_000,
    });
    world.step().expect("issue on old policy epoch");
    assert_eq!(
        world
            .main_token_epoch_issuance_record(effective_epoch - 1)
            .expect("old policy issuance")
            .inflation_rate_bps,
        300
    );

    world.submit_action(Action::ApplyMainTokenEpochIssuance {
        epoch_index: effective_epoch,
        actual_stake_ratio_bps: 6_000,
    });
    world.step().expect("issue on effective policy epoch");
    assert_eq!(
        world
            .main_token_epoch_issuance_record(effective_epoch)
            .expect("new policy issuance")
            .inflation_rate_bps,
        800
    );
}

#[test]
fn main_token_policy_update_requires_approved_or_applied_governance_proposal() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        ..MainTokenConfig::default()
    });

    let mut next_config = world.main_token_config().clone();
    next_config.inflation_policy.base_rate_bps = 500;

    world.submit_action(Action::UpdateMainTokenPolicy {
        proposal_id: 9_999,
        next: next_config.clone(),
    });
    world
        .step()
        .expect("missing governance proposal should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("governance proposal not found")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }

    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world.submit_action(Action::UpdateMainTokenPolicy {
        proposal_id,
        next: next_config.clone(),
    });
    world
        .step()
        .expect("unapproved governance proposal should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| {
                    note.contains("governance proposal must be approved or applied")
                }));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }

    world
        .shadow_proposal(proposal_id)
        .expect("shadow governance proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve governance proposal");
    world.submit_action(Action::UpdateMainTokenPolicy {
        proposal_id,
        next: next_config,
    });
    world
        .step()
        .expect("approved governance proposal should schedule policy update");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::MainTokenPolicyUpdateScheduled {
            proposal_id: event_proposal_id,
            ..
        }) => {
            assert_eq!(*event_proposal_id, proposal_id);
        }
        other => panic!("expected MainTokenPolicyUpdateScheduled, got {other:?}"),
    }
}

#[test]
fn main_token_policy_update_rejects_out_of_bounds_configuration() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
        ..MainTokenConfig::default()
    });

    let mut invalid_config = world.main_token_config().clone();
    invalid_config.initial_supply = 1_000;
    invalid_config.issuance_split = MainTokenIssuanceSplitPolicy {
        staking_reward_bps: 6_000,
        node_service_reward_bps: 2_000,
        ecosystem_pool_bps: 1_500,
        security_reserve_bps: 100,
    };
    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow governance proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve governance proposal");
    world.submit_action(Action::UpdateMainTokenPolicy {
        proposal_id,
        next: invalid_config,
    });
    world
        .step()
        .expect("invalid main token policy update should be rejected");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("update main token policy rejected")));
                assert!(notes
                    .iter()
                    .any(|note| note.contains("split sum must be 10000")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
    assert!(world.state().main_token_scheduled_policy_updates.is_empty());
}

#[test]
fn main_token_treasury_distribution_applies_closed_loop_and_records_audit() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_500,
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
        total_supply: 1_500,
        circulating_supply: 1_000,
        total_issued: 500,
        total_burned: 0,
    });
    world
        .set_main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD, 300)
        .expect("set staking treasury balance");

    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow governance proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve governance proposal");

    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id,
        distribution_id: "dist-1".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
        distributions: vec![
            MainTokenTreasuryDistribution {
                account_id: "node:alice".to_string(),
                amount: 120,
            },
            MainTokenTreasuryDistribution {
                account_id: "node:bob".to_string(),
                amount: 80,
            },
        ],
    });
    world.step().expect("distribute main token treasury");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::MainTokenTreasuryDistributed {
            proposal_id: event_proposal_id,
            distribution_id,
            bucket_id,
            total_amount,
            distributions,
        }) => {
            assert_eq!(*event_proposal_id, proposal_id);
            assert_eq!(distribution_id, "dist-1");
            assert_eq!(bucket_id, MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD);
            assert_eq!(*total_amount, 200);
            assert_eq!(distributions.len(), 2);
        }
        other => panic!("expected MainTokenTreasuryDistributed, got {other:?}"),
    }
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD),
        100
    );
    assert_eq!(world.main_token_liquid_balance("node:alice"), 120);
    assert_eq!(world.main_token_liquid_balance("node:bob"), 80);
    assert_eq!(world.main_token_supply().circulating_supply, 1_200);

    let record = world
        .main_token_treasury_distribution_record("dist-1")
        .expect("distribution record");
    assert_eq!(record.proposal_id, proposal_id);
    assert_eq!(record.bucket_id, MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD);
    assert_eq!(record.total_amount, 200);
    assert_eq!(record.distributions.len(), 2);
}

#[test]
fn main_token_treasury_distribution_requires_approved_or_applied_governance_proposal() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
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
    world
        .set_main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD, 100)
        .expect("set staking treasury balance");

    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id: 9_999,
        distribution_id: "dist-missing".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
        distributions: vec![MainTokenTreasuryDistribution {
            account_id: "node:alice".to_string(),
            amount: 50,
        }],
    });
    world
        .step()
        .expect("missing governance proposal should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("governance proposal not found")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }

    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id,
        distribution_id: "dist-unapproved".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
        distributions: vec![MainTokenTreasuryDistribution {
            account_id: "node:alice".to_string(),
            amount: 50,
        }],
    });
    world
        .step()
        .expect("unapproved governance proposal should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| {
                    note.contains("governance proposal must be approved or applied")
                }));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn main_token_treasury_distribution_rejects_unsupported_bucket_and_duplicate_distribution_id() {
    let mut world = World::new();
    world.set_main_token_config(MainTokenConfig {
        initial_supply: 1_000,
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
        circulating_supply: 600,
        total_issued: 0,
        total_burned: 0,
    });
    world
        .set_main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD, 100)
        .expect("set staking treasury balance");

    let proposal_id = world
        .propose_manifest_update(world.manifest().clone(), "alice")
        .expect("create governance proposal");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow governance proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve governance proposal");

    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id,
        distribution_id: "dist-invalid-bucket".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD.to_string(),
        distributions: vec![MainTokenTreasuryDistribution {
            account_id: "node:alice".to_string(),
            amount: 30,
        }],
    });
    world.step().expect("unsupported bucket should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes.iter().any(|note| note.contains("unsupported bucket")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }

    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id,
        distribution_id: "dist-dup".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
        distributions: vec![MainTokenTreasuryDistribution {
            account_id: "node:alice".to_string(),
            amount: 40,
        }],
    });
    world
        .step()
        .expect("first treasury distribution should pass");
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD),
        60
    );

    world.submit_action(Action::DistributeMainTokenTreasury {
        proposal_id,
        distribution_id: "dist-dup".to_string(),
        bucket_id: MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
        distributions: vec![MainTokenTreasuryDistribution {
            account_id: "node:bob".to_string(),
            amount: 20,
        }],
    });
    world
        .step()
        .expect("duplicate distribution_id should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("distribution_id already exists")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn restricted_claim_liveops_pool_top_up_moves_balance_and_records_event() {
    let mut world = World::new();
    set_main_token_controller_registry_for_tests(&mut world, "msig.ecosystem_governance.v1");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_000,
        circulating_supply: 0,
        total_issued: 1_000,
        total_burned: 0,
    });
    world
        .set_main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, 900)
        .expect("seed ecosystem treasury");

    world.submit_action(Action::TopUpRestrictedStarterClaimLiveopsPool {
        controller_account_id: "msig.ecosystem_governance.v1".to_string(),
        top_up_id: "liveops-topup-1".to_string(),
        amount: 300,
    });
    world.step().expect("top up restricted claim liveops pool");

    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::RestrictedStarterClaimLiveopsPoolToppedUp {
            controller_account_id,
            top_up_id,
            source_treasury_bucket_id,
            target_treasury_bucket_id,
            amount,
            ..
        }) => {
            assert_eq!(controller_account_id, "msig.ecosystem_governance.v1");
            assert_eq!(top_up_id, "liveops-topup-1");
            assert_eq!(
                source_treasury_bucket_id,
                MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL
            );
            assert_eq!(
                target_treasury_bucket_id,
                MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL
            );
            assert_eq!(*amount, 300);
        }
        other => panic!(
            "expected RestrictedStarterClaimLiveopsPoolToppedUp, got {other:?}"
        ),
    }
    assert_eq!(
        world.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL),
        600
    );
    assert_eq!(
        world.main_token_treasury_balance(
            MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL
        ),
        300
    );
    let record = world
        .restricted_starter_claim_liveops_pool_top_up_record("liveops-topup-1")
        .expect("top-up record");
    assert_eq!(record.controller_account_id, "msig.ecosystem_governance.v1");
    assert_eq!(record.amount, 300);
}

#[test]
fn restricted_claim_liveops_pool_top_up_rejects_wrong_controller_slot_and_duplicate_top_up_id() {
    let mut world = World::new();
    set_main_token_controller_registry_for_tests(&mut world, "msig.ecosystem_governance.v1");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 1_000,
        circulating_supply: 0,
        total_issued: 1_000,
        total_burned: 0,
    });
    world
        .set_main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, 900)
        .expect("seed ecosystem treasury");

    world.submit_action(Action::TopUpRestrictedStarterClaimLiveopsPool {
        controller_account_id: "msig.wrong_controller.v1".to_string(),
        top_up_id: "liveops-topup-reject".to_string(),
        amount: 300,
    });
    world.step().expect("reject wrong controller slot");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("ecosystem treasury controller slot")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }

    world.submit_action(Action::TopUpRestrictedStarterClaimLiveopsPool {
        controller_account_id: "msig.ecosystem_governance.v1".to_string(),
        top_up_id: "liveops-topup-dup".to_string(),
        amount: 300,
    });
    world.step().expect("first top-up should pass");

    world.submit_action(Action::TopUpRestrictedStarterClaimLiveopsPool {
        controller_account_id: "msig.ecosystem_governance.v1".to_string(),
        top_up_id: "liveops-topup-dup".to_string(),
        amount: 50,
    });
    world.step().expect("duplicate top_up_id should reject");
    match &world.journal().events.last().expect("event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("top_up_id already exists")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}
