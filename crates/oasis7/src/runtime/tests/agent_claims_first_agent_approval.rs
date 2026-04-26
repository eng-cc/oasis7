use super::super::*;
use super::pos;
use std::collections::{BTreeMap, BTreeSet};

fn register_agent(world: &mut World, agent_id: &str) {
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: pos(0.0, 0.0),
    });
    world.step().expect("register agent");
}

fn setup_claim_world_with_treasury(
    treasury_balance: u64,
    alice_liquid_balance: u64,
    reputation_score: i64,
) -> World {
    let mut world = World::new();
    world
        .set_governance_execution_policy(GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..GovernanceExecutionPolicy::default()
        })
        .expect("set governance epoch length");
    register_agent(&mut world, "alice");
    register_agent(&mut world, "bob");
    register_agent(&mut world, "carol");
    world
        .set_agent_reputation_score("alice", reputation_score)
        .expect("set alice reputation");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: treasury_balance.saturating_add(alice_liquid_balance),
        circulating_supply: alice_liquid_balance,
        ..MainTokenSupplyState::default()
    });
    world
        .set_main_token_treasury_balance(
            MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
            treasury_balance,
        )
        .expect("seed restricted liveops treasury");
    if alice_liquid_balance > 0 {
        world
            .set_main_token_account_balance("alice", alice_liquid_balance, 0)
            .expect("seed alice liquid balance");
    }
    world
}

fn allowlist_restricted_grant_admins(world: &mut World, admin_account_ids: &[&str]) {
    configure_restricted_grant_registry(
        world,
        &["msig.ecosystem_governance.v1"],
        admin_account_ids,
    );
}

fn configure_restricted_grant_registry(
    world: &mut World,
    policy_account_ids: &[&str],
    admin_account_ids: &[&str],
) {
    let mut controller_signer_policies = BTreeMap::from([(
        "msig.genesis.v1".to_string(),
        GovernanceThresholdSignerPolicy {
            threshold: 1,
            allowed_public_keys: BTreeSet::from([
                "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30".to_string(),
            ]),
        },
    )]);
    let mut all_policy_account_ids = policy_account_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    for account_id in admin_account_ids {
        let account_id = account_id.trim();
        if !account_id.is_empty() {
            all_policy_account_ids.insert(account_id.to_string());
        }
    }
    for (index, account_id) in all_policy_account_ids.into_iter().enumerate() {
        controller_signer_policies.insert(
            account_id,
            GovernanceThresholdSignerPolicy {
                threshold: 1,
                allowed_public_keys: BTreeSet::from([format!("{:064x}", index + 3)]),
            },
        );
    }
    world
        .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
            genesis_controller_account_id: "msig.genesis.v1".to_string(),
            treasury_bucket_controller_slots: BTreeMap::from([(
                MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
                policy_account_ids
                    .first()
                    .copied()
                    .unwrap_or("msig.ecosystem_governance.v1")
                    .to_string(),
            )]),
            restricted_starter_claim_admin_account_ids: admin_account_ids
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            controller_signer_policies,
        })
        .expect("set restricted grant admin registry");
}

#[test]
fn first_agent_claim_approval_request_persists_pending_state() {
    let mut world = setup_claim_world_with_treasury(1_000, 0, 0);
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit first claim approval request");

    let request = world
        .state()
        .first_agent_claim_approval_requests
        .get(&1)
        .expect("approval request persisted");
    assert_eq!(request.claimer_agent_id, "alice");
    assert_eq!(request.requested_slot_index, 1);
    assert_eq!(
        request.status,
        FirstAgentClaimApprovalRequestStatus::Pending
    );
    assert!(matches!(
        &world.journal().events.last().expect("event").body,
        WorldEventBody::Domain(DomainEvent::FirstAgentClaimApprovalRequested { request_id, .. })
            if *request_id == 1
    ));
    assert_eq!(
        world
            .state()
            .latest_first_agent_claim_approval_request_ids_by_claimer
            .get("alice"),
        Some(&1)
    );
}

#[test]
fn first_agent_claim_approval_index_migrates_legacy_requests() {
    let mut state = WorldState::default();
    state.first_agent_claim_approval_requests.insert(
        3,
        FirstAgentClaimApprovalRequestState {
            request_id: 3,
            claimer_agent_id: "alice".to_string(),
            requested_slot_index: 1,
            requested_reputation_tier: 0,
            requested_total_upfront_amount: 325,
            requested_at_epoch: 3,
            status: FirstAgentClaimApprovalRequestStatus::Rejected,
            updated_at_epoch: 4,
            operator_account_id: Some("liveops".to_string()),
            approved_amount: None,
            expires_at_epoch: None,
            rejection_reason: Some("legacy".to_string()),
        },
    );
    state.first_agent_claim_approval_requests.insert(
        5,
        FirstAgentClaimApprovalRequestState {
            request_id: 5,
            claimer_agent_id: "alice".to_string(),
            requested_slot_index: 1,
            requested_reputation_tier: 1,
            requested_total_upfront_amount: 488,
            requested_at_epoch: 5,
            status: FirstAgentClaimApprovalRequestStatus::Pending,
            updated_at_epoch: 5,
            operator_account_id: None,
            approved_amount: None,
            expires_at_epoch: None,
            rejection_reason: None,
        },
    );
    state.first_agent_claim_approval_requests.insert(
        4,
        FirstAgentClaimApprovalRequestState {
            request_id: 4,
            claimer_agent_id: "bob".to_string(),
            requested_slot_index: 1,
            requested_reputation_tier: 0,
            requested_total_upfront_amount: 325,
            requested_at_epoch: 4,
            status: FirstAgentClaimApprovalRequestStatus::Approved,
            updated_at_epoch: 4,
            operator_account_id: Some("liveops".to_string()),
            approved_amount: Some(325),
            expires_at_epoch: Some(10),
            rejection_reason: None,
        },
    );

    state.migrate_compat_first_agent_claim_approval_request_index();

    assert_eq!(
        state
            .latest_first_agent_claim_approval_request_ids_by_claimer
            .get("alice"),
        Some(&5)
    );
    assert_eq!(
        state
            .latest_first_agent_claim_approval_request_ids_by_claimer
            .get("bob"),
        Some(&4)
    );
}

#[test]
fn first_agent_claim_approval_issues_restricted_grant_and_enables_slot_1_claim() {
    let mut world = setup_claim_world_with_treasury(1_000, 0, 0);
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit first claim approval request");
    let requested_total_upfront_amount = world
        .state()
        .first_agent_claim_approval_requests
        .get(&1)
        .expect("approval request")
        .requested_total_upfront_amount;

    world.submit_action(Action::ApproveFirstAgentClaimApprovalRequest {
        operator_account_id: "liveops".to_string(),
        request_id: 1,
        expires_at_epoch: 10,
    });
    world.step().expect("approve first claim approval request");

    let request = world
        .state()
        .first_agent_claim_approval_requests
        .get(&1)
        .expect("approval request persisted");
    assert_eq!(
        request.status,
        FirstAgentClaimApprovalRequestStatus::Approved
    );
    assert_eq!(
        request.approved_amount,
        Some(requested_total_upfront_amount)
    );
    assert_eq!(
        world.main_token_restricted_starter_claim_balance("alice"),
        requested_total_upfront_amount
    );

    world.submit_action(Action::ClaimAgent {
        claimer_agent_id: "alice".to_string(),
        target_agent_id: "bob".to_string(),
    });
    world.step().expect("claim slot 1 after approval");

    let claim = world.agent_claim("bob").expect("claim persisted");
    assert_eq!(claim.claim_owner_id, "alice");
    assert_eq!(claim.slot_index, 1);
    assert_eq!(
        world.main_token_restricted_starter_claim_balance("alice"),
        0
    );
}

#[test]
fn first_agent_claim_approval_rejection_persists_reason() {
    let mut world = setup_claim_world_with_treasury(1_000, 0, 0);
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit first claim approval request");
    world.submit_action(Action::RejectFirstAgentClaimApprovalRequest {
        operator_account_id: "liveops".to_string(),
        request_id: 1,
        reason: "manual_review_failed".to_string(),
    });
    world.step().expect("reject first claim approval request");

    let request = world
        .state()
        .first_agent_claim_approval_requests
        .get(&1)
        .expect("approval request persisted");
    assert_eq!(
        request.status,
        FirstAgentClaimApprovalRequestStatus::Rejected
    );
    assert_eq!(
        request.rejection_reason.as_deref(),
        Some("manual_review_failed")
    );
}

#[test]
fn first_agent_claim_approval_submit_rejects_duplicate_pending_request() {
    let mut world = setup_claim_world_with_treasury(1_000, 0, 0);
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit first claim approval request");
    let journal_len_before = world.journal().events.len();

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("reject duplicate pending request");

    let rejection = world.journal().events[journal_len_before..]
        .iter()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => Some(reason),
            _ => None,
        })
        .expect("duplicate pending rejection");
    match rejection {
        RejectReason::RuleDenied { notes } => {
            assert!(notes.iter().any(|note| note.contains("already pending")));
        }
        other => panic!("expected rule denied, got {other:?}"),
    }
}

#[test]
fn first_agent_claim_approval_approve_rejects_non_admin_operator() {
    let mut world = setup_claim_world_with_treasury(1_000, 0, 0);
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);

    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit first claim approval request");
    let journal_len_before = world.journal().events.len();

    world.submit_action(Action::ApproveFirstAgentClaimApprovalRequest {
        operator_account_id: "qa".to_string(),
        request_id: 1,
        expires_at_epoch: 10,
    });
    world.step().expect("reject non-admin approval");

    let rejection = world.journal().events[journal_len_before..]
        .iter()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => Some(reason),
            _ => None,
        })
        .expect("non-admin approval rejection");
    match rejection {
        RejectReason::RuleDenied { notes } => {
            assert!(notes
                .iter()
                .any(|note| note.contains("not allowlisted admin")));
        }
        other => panic!("expected rule denied, got {other:?}"),
    }
}
