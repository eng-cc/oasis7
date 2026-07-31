use super::pos;
use crate::runtime::{
    Action, DomainEvent, MaterialDefaultPriority, MaterialLedgerId, MaterialProfileV1,
    MaterialTransitPriority, MaterialTransportLossClass, World, WorldEventBody,
};

#[test]
fn logistics_transfer_quote_is_read_only_deterministic_and_matches_transit_authority() {
    let mut world = World::new();
    let source = MaterialLedgerId::site("quote-source");
    let destination = MaterialLedgerId::site("quote-destination");
    world.submit_action(Action::RegisterAgent {
        agent_id: "quote-operator".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register quote operator");
    world
        .upsert_material_profile(MaterialProfileV1 {
            kind: "copper_wire".to_string(),
            tier: 2,
            category: "intermediate".to_string(),
            stack_limit: 500,
            transport_loss_class: MaterialTransportLossClass::High,
            decay_bps_per_tick: 0,
            default_priority: MaterialDefaultPriority::Standard,
        })
        .expect("insert high-loss profile");
    world
        .set_ledger_material_balance(source.clone(), "copper_wire", 100)
        .expect("seed source");

    let state_before = world.snapshot();
    let journal_before = world.journal().clone();
    let quote = world
        .logistics_transfer_quote(
            "quote-operator",
            &source,
            &destination,
            "copper_wire",
            50,
            100,
            None,
        )
        .expect("default-priority logistics quote");
    let repeat = world
        .logistics_transfer_quote(
            "quote-operator",
            &source,
            &destination,
            "copper_wire",
            50,
            100,
            None,
        )
        .expect("repeat logistics quote");

    assert_eq!(quote, repeat);
    assert_eq!(world.snapshot(), state_before);
    assert_eq!(world.journal(), &journal_before);
    assert!(quote.conditional);
    assert!(quote.submission_feasible);
    assert_eq!(quote.max_transferable_amount, 100);
    assert_eq!(quote.sent_amount, 50);
    assert_eq!(quote.loss_bps, 20);
    assert_eq!(quote.expected_loss_amount, 10);
    assert_eq!(quote.expected_received_amount, 40);
    assert_eq!(quote.source_amount_before, 100);
    assert_eq!(quote.source_amount_after, 50);
    assert_eq!(quote.destination_amount_before, 0);
    assert_eq!(quote.destination_expected_amount_after, 40);
    assert_eq!(quote.ticks_until_arrival, 1);
    // `submit_action` is handled on the next `world.step()`, which advances
    // time before calculating the transit ready tick.
    assert_eq!(quote.ready_at, world.state().time.saturating_add(2));
    assert_eq!(quote.effective_priority, MaterialTransitPriority::Standard);
    assert!(!quote.priority_reason.is_empty());
    assert_eq!(quote.inflight_before, 0);
    assert!(quote.inflight_capacity >= 1);
    assert!(!quote.recommendation.is_empty());

    let zero_distance = world
        .logistics_transfer_quote(
            "quote-operator",
            &source,
            &destination,
            "copper_wire",
            10,
            0,
            Some(MaterialTransitPriority::Urgent),
        )
        .expect("zero-distance logistics quote");
    assert_eq!(zero_distance.loss_bps, 0);
    assert_eq!(zero_distance.expected_loss_amount, 0);
    assert_eq!(zero_distance.expected_received_amount, 10);
    assert_eq!(zero_distance.ticks_until_arrival, 0);
    assert_eq!(zero_distance.ready_at, world.state().time.saturating_add(1));
    assert_eq!(
        zero_distance.effective_priority,
        MaterialTransitPriority::Urgent
    );
    assert_ne!(zero_distance.priority_reason, quote.priority_reason);

    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "quote-operator".to_string(),
        from_ledger: source.clone(),
        to_ledger: destination.clone(),
        kind: "copper_wire".to_string(),
        amount: 50,
        distance_km: 100,
        priority: None,
    });
    world.step().expect("start quoted transit");
    let (started_loss_bps, started_ready_at, started_priority) = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted {
                loss_bps,
                ready_at,
                priority,
                ..
            }) => Some((*loss_bps, *ready_at, *priority)),
            _ => None,
        })
        .expect("quoted transit started");
    assert_eq!(started_loss_bps, quote.loss_bps);
    assert_eq!(started_ready_at, quote.ready_at);
    assert_eq!(started_priority, quote.effective_priority);

    world.step().expect("complete quoted transit");
    let (received_amount, loss_amount) = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted {
                received_amount,
                loss_amount,
                ..
            }) => Some((*received_amount, *loss_amount)),
            _ => None,
        })
        .expect("quoted transit completed");
    assert_eq!(loss_amount, quote.expected_loss_amount);
    assert_eq!(received_amount, quote.expected_received_amount);
}

#[test]
fn logistics_transfer_quote_makes_capacity_and_amount_recovery_recommendations_conditional() {
    let mut world = World::new();
    let source = MaterialLedgerId::site("quote-source");
    let destination = MaterialLedgerId::site("quote-destination");
    world.submit_action(Action::RegisterAgent {
        agent_id: "quote-operator".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register quote operator");
    world
        .set_ledger_material_balance(source.clone(), "iron_ingot", 100)
        .expect("seed source");

    for index in 0..2 {
        world.submit_action(Action::TransferMaterial {
            requester_agent_id: "quote-operator".to_string(),
            from_ledger: source.clone(),
            to_ledger: MaterialLedgerId::site(format!("quote-destination-{index}")),
            kind: "iron_ingot".to_string(),
            amount: 20,
            distance_km: 200,
            priority: None,
        });
    }
    world.step().expect("start capacity-filling transits");
    assert_eq!(world.pending_material_transits_len(), 2);

    let state_before = world.snapshot();
    let journal_before = world.journal().clone();
    let capacity_quote = world
        .logistics_transfer_quote(
            "quote-operator",
            &source,
            &destination,
            "iron_ingot",
            20,
            200,
            None,
        )
        .expect("capacity quote");
    let amount_quote = world
        .logistics_transfer_quote(
            "quote-operator",
            &source,
            &destination,
            "iron_ingot",
            61,
            200,
            None,
        )
        .expect("over-available amount quote");

    assert_eq!(world.snapshot(), state_before);
    assert_eq!(world.journal(), &journal_before);
    assert!(capacity_quote.conditional);
    assert_eq!(
        capacity_quote.inflight_before,
        capacity_quote.inflight_capacity
    );
    assert!(!capacity_quote.recommendation.is_empty());
    assert!(amount_quote.conditional);
    assert!(!amount_quote.submission_feasible);
    assert_eq!(amount_quote.max_transferable_amount, 60);
    assert_eq!(amount_quote.sent_amount, 0);
    assert_eq!(amount_quote.expected_loss_amount, 0);
    assert_eq!(amount_quote.expected_received_amount, 0);
    assert_eq!(
        amount_quote.source_amount_after,
        amount_quote.source_amount_before
    );
    assert_eq!(
        amount_quote.destination_expected_amount_after,
        amount_quote.destination_amount_before
    );
    assert_eq!(
        amount_quote.recommendation,
        "reduce_amount_or_source_materials"
    );
}
