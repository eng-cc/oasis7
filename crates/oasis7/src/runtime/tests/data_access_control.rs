use super::pos;
use crate::runtime::{
    Action, DomainEvent, EconomicContractStatus, RejectReason, World, WorldEventBody,
};
use crate::simulator::ResourceKind;

fn register_agent(world: &mut World, agent_id: &str) {
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");
}

fn assert_latest_rule_denied_contains(world: &World, needle: &str) {
    let event = world.journal().events.last().expect("latest event");
    match &event.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected {
            reason: RejectReason::RuleDenied { notes },
            ..
        }) => assert!(
            notes.iter().any(|value| value.contains(needle)),
            "expected RuleDenied containing '{needle}', got {notes:?}"
        ),
        other => panic!("expected ActionRejected RuleDenied, got {other:?}"),
    }
}

#[test]
fn collect_data_consumes_electricity_and_adds_data() {
    let mut world = World::new();
    register_agent(&mut world, "collector");
    world
        .set_agent_resource_balance("collector", ResourceKind::Electricity, 20)
        .expect("seed electricity");

    world.submit_action(Action::CollectData {
        collector_agent_id: "collector".to_string(),
        electricity_cost: 7,
        data_amount: 11,
    });
    world.step().expect("collect data");

    assert_eq!(
        world
            .agent_resource_balance("collector", ResourceKind::Electricity)
            .expect("collector electricity"),
        13
    );
    assert_eq!(
        world
            .agent_resource_balance("collector", ResourceKind::Data)
            .expect("collector data"),
        11
    );

    match &world.journal().events.last().expect("collect event").body {
        WorldEventBody::Domain(DomainEvent::DataCollected {
            collector_agent_id,
            electricity_cost,
            data_amount,
        }) => {
            assert_eq!(collector_agent_id, "collector");
            assert_eq!(*electricity_cost, 7);
            assert_eq!(*data_amount, 11);
        }
        other => panic!("expected DataCollected, got {other:?}"),
    }
}

#[test]
fn collect_data_rejects_when_electricity_is_insufficient() {
    let mut world = World::new();
    register_agent(&mut world, "collector");
    world
        .set_agent_resource_balance("collector", ResourceKind::Electricity, 3)
        .expect("seed electricity");

    world.submit_action(Action::CollectData {
        collector_agent_id: "collector".to_string(),
        electricity_cost: 5,
        data_amount: 8,
    });
    world.step().expect("collect should be rejected");

    match &world.journal().events.last().expect("reject event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected {
            reason:
                RejectReason::InsufficientResource {
                    agent_id,
                    kind,
                    requested,
                    available,
                },
            ..
        }) => {
            assert_eq!(agent_id, "collector");
            assert_eq!(*kind, ResourceKind::Electricity);
            assert_eq!(*requested, 5);
            assert_eq!(*available, 3);
        }
        other => panic!("expected InsufficientResource rejection, got {other:?}"),
    }

    assert_eq!(
        world
            .agent_resource_balance("collector", ResourceKind::Electricity)
            .expect("collector electricity"),
        3
    );
    assert_eq!(
        world
            .agent_resource_balance("collector", ResourceKind::Data)
            .expect("collector data"),
        0
    );
}

#[test]
fn data_transfer_requires_access_grant() {
    let mut world = World::new();
    register_agent(&mut world, "owner");
    register_agent(&mut world, "buyer");
    world
        .set_agent_resource_balance("owner", ResourceKind::Data, 10)
        .expect("seed owner data");

    world.submit_action(Action::EmitResourceTransfer {
        from_agent_id: "owner".to_string(),
        to_agent_id: "buyer".to_string(),
        kind: ResourceKind::Data,
        amount: 4,
    });
    world.step().expect("reject transfer without access grant");
    assert_latest_rule_denied_contains(&world, "missing access grant");

    assert_eq!(
        world
            .agent_resource_balance("owner", ResourceKind::Data)
            .expect("owner data"),
        10
    );
    assert_eq!(
        world
            .agent_resource_balance("buyer", ResourceKind::Data)
            .expect("buyer data"),
        0
    );

    world.submit_action(Action::GrantDataAccess {
        owner_agent_id: "owner".to_string(),
        grantee_agent_id: "buyer".to_string(),
    });
    world.step().expect("grant data access");

    world.submit_action(Action::EmitResourceTransfer {
        from_agent_id: "owner".to_string(),
        to_agent_id: "buyer".to_string(),
        kind: ResourceKind::Data,
        amount: 4,
    });
    world.step().expect("transfer with access grant");

    assert_eq!(
        world
            .agent_resource_balance("owner", ResourceKind::Data)
            .expect("owner data"),
        6
    );
    assert_eq!(
        world
            .agent_resource_balance("buyer", ResourceKind::Data)
            .expect("buyer data"),
        4
    );

    world.submit_action(Action::RevokeDataAccess {
        owner_agent_id: "owner".to_string(),
        grantee_agent_id: "buyer".to_string(),
    });
    world.step().expect("revoke data access");

    world.submit_action(Action::EmitResourceTransfer {
        from_agent_id: "owner".to_string(),
        to_agent_id: "buyer".to_string(),
        kind: ResourceKind::Data,
        amount: 1,
    });
    world.step().expect("reject transfer after revoke");
    assert_latest_rule_denied_contains(&world, "missing access grant");
}

#[test]
fn economic_contract_data_settlement_requires_access_grant() {
    let mut world = World::new();
    register_agent(&mut world, "owner");
    register_agent(&mut world, "buyer");
    world
        .set_agent_resource_balance("owner", ResourceKind::Data, 50)
        .expect("seed owner data");

    let expires_at = world.state().time.saturating_add(10);
    world.submit_action(Action::OpenEconomicContract {
        creator_agent_id: "owner".to_string(),
        contract_id: "contract.data.access".to_string(),
        counterparty_agent_id: "buyer".to_string(),
        settlement_kind: ResourceKind::Data,
        settlement_amount: 20,
        reputation_stake: 4,
        expires_at,
        description: "data delivery".to_string(),
    });
    world.step().expect("open economic contract");
    world.submit_action(Action::AcceptEconomicContract {
        accepter_agent_id: "buyer".to_string(),
        contract_id: "contract.data.access".to_string(),
    });
    world.step().expect("accept economic contract");

    world.submit_action(Action::SettleEconomicContract {
        operator_agent_id: "owner".to_string(),
        contract_id: "contract.data.access".to_string(),
        success: true,
        notes: "attempt settle without grant".to_string(),
    });
    world
        .step()
        .expect("reject settlement without access grant");
    assert_latest_rule_denied_contains(&world, "missing access grant");

    let contract = world
        .state()
        .economic_contracts
        .get("contract.data.access")
        .expect("contract exists");
    assert_eq!(contract.status, EconomicContractStatus::Accepted);
    assert_eq!(contract.settled_at, None);
    assert_eq!(contract.settlement_success, None);

    world.submit_action(Action::GrantDataAccess {
        owner_agent_id: "owner".to_string(),
        grantee_agent_id: "buyer".to_string(),
    });
    world.step().expect("grant data access");

    world.submit_action(Action::SettleEconomicContract {
        operator_agent_id: "owner".to_string(),
        contract_id: "contract.data.access".to_string(),
        success: true,
        notes: "settle with access grant".to_string(),
    });
    world.step().expect("settle economic contract");

    let contract = world
        .state()
        .economic_contracts
        .get("contract.data.access")
        .expect("settled contract exists");
    assert_eq!(contract.status, EconomicContractStatus::Settled);
    assert_eq!(contract.transfer_amount, 20);
    assert_eq!(contract.settlement_success, Some(true));
    assert_eq!(
        world
            .agent_resource_balance("owner", ResourceKind::Data)
            .expect("owner data"),
        30
    );
    assert_eq!(
        world
            .agent_resource_balance("buyer", ResourceKind::Data)
            .expect("buyer data"),
        20
    );
}
