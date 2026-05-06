use super::*;

fn agent_owner(agent_id: &str) -> ResourceOwner {
    ResourceOwner::Agent {
        agent_id: agent_id.to_string(),
    }
}

fn first_evidence_event_id(kernel: &WorldKernel) -> WorldEventId {
    kernel
        .journal()
        .first()
        .map(|event| event.id)
        .expect("seed event id")
}

fn electricity_of(kernel: &WorldKernel, agent_id: &str) -> i64 {
    kernel
        .model()
        .agents
        .get(agent_id)
        .expect("agent exists")
        .resources
        .get(ResourceKind::Electricity)
}

fn setup_social_kernel() -> WorldKernel {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-social".to_string(),
        name: "social-hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    for agent_id in ["agent-a", "agent-b", "agent-c"] {
        kernel.submit_action(Action::RegisterAgent {
            agent_id: agent_id.to_string(),
            location_id: "loc-social".to_string(),
        });
    }
    kernel.step_until_empty();

    for agent_id in ["agent-a", "agent-b", "agent-c"] {
        seed_owner_resource(
            &mut kernel,
            agent_owner(agent_id),
            ResourceKind::Electricity,
            1_000,
        );
    }
    kernel
}

#[test]
fn social_publish_rejects_missing_evidence_event() {
    let mut kernel = setup_social_kernel();
    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.reputation.v1".to_string(),
        subject: agent_owner("agent-b"),
        object: None,
        claim: "agent-b delivers contract obligations".to_string(),
        confidence_ppm: 900_000,
        evidence_event_ids: vec![999_999],
        ttl_ticks: None,
        stake: None,
    });

    let event = kernel.step().expect("publish event");
    match event.kind {
        WorldEventKind::ActionRejected {
            reason: RejectReason::RuleDenied { notes },
        } => {
            assert!(
                notes
                    .iter()
                    .any(|note| note.contains("evidence event missing")),
                "missing evidence rejection note: {notes:?}"
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(kernel.model().social_facts.is_empty());
}

#[test]
fn social_adjudication_confirm_slashes_challenge_stake_and_releases_publisher() {
    let mut kernel = setup_social_kernel();
    let evidence_event_id = first_evidence_event_id(&kernel);

    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.reputation.v1".to_string(),
        subject: agent_owner("agent-b"),
        object: None,
        claim: "agent-b delivered mission data".to_string(),
        confidence_ppm: 800_000,
        evidence_event_ids: vec![evidence_event_id],
        ttl_ticks: None,
        stake: Some(SocialStake {
            kind: ResourceKind::Electricity,
            amount: 30,
        }),
    });
    let publish = kernel.step().expect("publish");
    let fact_id = match publish.kind {
        WorldEventKind::SocialFactPublished { fact } => fact.fact_id,
        other => panic!("unexpected publish event: {other:?}"),
    };
    assert_eq!(electricity_of(&kernel, "agent-a"), 970);

    kernel.submit_action(Action::ChallengeSocialFact {
        challenger: agent_owner("agent-c"),
        fact_id,
        reason: "insufficient on-chain proof".to_string(),
        stake: Some(SocialStake {
            kind: ResourceKind::Electricity,
            amount: 20,
        }),
    });
    let challenged = kernel.step().expect("challenge");
    assert!(matches!(
        challenged.kind,
        WorldEventKind::SocialFactChallenged { .. }
    ));
    assert_eq!(electricity_of(&kernel, "agent-c"), 980);

    kernel.submit_action(Action::AdjudicateSocialFact {
        adjudicator: agent_owner("agent-b"),
        fact_id,
        decision: SocialAdjudicationDecision::Confirm,
        notes: "evidence satisfies schema thresholds".to_string(),
    });
    let adjudicated = kernel.step().expect("adjudicate");
    assert!(matches!(
        adjudicated.kind,
        WorldEventKind::SocialFactAdjudicated {
            decision: SocialAdjudicationDecision::Confirm,
            ..
        }
    ));

    let fact = kernel
        .model()
        .social_facts
        .get(&fact_id)
        .expect("fact exists");
    assert_eq!(fact.lifecycle, SocialFactLifecycleState::Confirmed);
    assert!(fact.stake.is_none());
    assert!(fact
        .challenge
        .as_ref()
        .expect("challenge exists")
        .stake
        .is_none());
    assert_eq!(electricity_of(&kernel, "agent-a"), 1_000);
    assert_eq!(electricity_of(&kernel, "agent-c"), 980);
    assert_eq!(
        kernel
            .model()
            .social_stake_pool
            .get(ResourceKind::Electricity),
        20
    );
}

#[test]
fn social_adjudication_retract_slashes_publisher_and_refunds_challenger() {
    let mut kernel = setup_social_kernel();
    let evidence_event_id = first_evidence_event_id(&kernel);

    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.reputation.v1".to_string(),
        subject: agent_owner("agent-b"),
        object: None,
        claim: "agent-b fulfilled 100% SLA".to_string(),
        confidence_ppm: 700_000,
        evidence_event_ids: vec![evidence_event_id],
        ttl_ticks: None,
        stake: Some(SocialStake {
            kind: ResourceKind::Electricity,
            amount: 40,
        }),
    });
    let publish = kernel.step().expect("publish");
    let fact_id = match publish.kind {
        WorldEventKind::SocialFactPublished { fact } => fact.fact_id,
        other => panic!("unexpected publish event: {other:?}"),
    };

    kernel.submit_action(Action::ChallengeSocialFact {
        challenger: agent_owner("agent-c"),
        fact_id,
        reason: "proofs do not cover full SLA period".to_string(),
        stake: Some(SocialStake {
            kind: ResourceKind::Electricity,
            amount: 30,
        }),
    });
    let challenged = kernel.step().expect("challenge");
    assert!(matches!(
        challenged.kind,
        WorldEventKind::SocialFactChallenged { .. }
    ));

    kernel.submit_action(Action::AdjudicateSocialFact {
        adjudicator: agent_owner("agent-b"),
        fact_id,
        decision: SocialAdjudicationDecision::Retract,
        notes: "publisher evidence is incomplete".to_string(),
    });
    let adjudicated = kernel.step().expect("adjudicate");
    assert!(matches!(
        adjudicated.kind,
        WorldEventKind::SocialFactAdjudicated {
            decision: SocialAdjudicationDecision::Retract,
            ..
        }
    ));

    let fact = kernel
        .model()
        .social_facts
        .get(&fact_id)
        .expect("fact exists");
    assert_eq!(fact.lifecycle, SocialFactLifecycleState::Retracted);
    assert_eq!(electricity_of(&kernel, "agent-a"), 960);
    assert_eq!(electricity_of(&kernel, "agent-c"), 1_000);
    assert_eq!(
        kernel
            .model()
            .social_stake_pool
            .get(ResourceKind::Electricity),
        40
    );
}

#[test]
fn social_fact_expiry_triggers_backing_edge_expiry() {
    let mut kernel = setup_social_kernel();
    let evidence_event_id = first_evidence_event_id(&kernel);

    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.relation.v1".to_string(),
        subject: agent_owner("agent-a"),
        object: Some(agent_owner("agent-b")),
        claim: "agent-a and agent-b have cooperative history".to_string(),
        confidence_ppm: 750_000,
        evidence_event_ids: vec![evidence_event_id],
        ttl_ticks: Some(2),
        stake: None,
    });
    let published = kernel.step().expect("publish");
    let fact_id = match published.kind {
        WorldEventKind::SocialFactPublished { fact } => fact.fact_id,
        other => panic!("unexpected publish event: {other:?}"),
    };

    kernel.submit_action(Action::DeclareSocialEdge {
        declarer: agent_owner("agent-a"),
        schema_id: "social.relation.v1".to_string(),
        relation_kind: "trust".to_string(),
        from: agent_owner("agent-a"),
        to: agent_owner("agent-b"),
        weight_bps: 2_000,
        backing_fact_ids: vec![fact_id],
        ttl_ticks: None,
    });
    let declared = kernel.step().expect("declare edge");
    let edge_id = match declared.kind {
        WorldEventKind::SocialEdgeDeclared { edge } => edge.edge_id,
        other => panic!("unexpected edge event: {other:?}"),
    };

    kernel.submit_action(Action::DebugGrantResource {
        owner: agent_owner("agent-a"),
        kind: ResourceKind::Data,
        amount: 1,
    });
    let _ = kernel.step().expect("advance tick");

    let fact = kernel
        .model()
        .social_facts
        .get(&fact_id)
        .expect("fact still tracked");
    assert_eq!(fact.lifecycle, SocialFactLifecycleState::Expired);
    let edge = kernel
        .model()
        .social_edges
        .get(&edge_id)
        .expect("edge still tracked");
    assert_eq!(edge.lifecycle, SocialEdgeLifecycleState::Expired);

    let fact_expired_index = kernel
        .journal()
        .iter()
        .position(|event| {
            matches!(
                &event.kind,
                WorldEventKind::SocialFactExpired { fact_id: value, .. } if *value == fact_id
            )
        })
        .expect("fact expired event");
    let (edge_expired_index, reason) = kernel
        .journal()
        .iter()
        .enumerate()
        .find_map(|(index, event)| match &event.kind {
            WorldEventKind::SocialEdgeExpired {
                edge_id: value,
                reason,
                ..
            } if *value == edge_id => Some((index, reason.clone())),
            _ => None,
        })
        .expect("edge expired event");
    assert_eq!(reason, "backing_fact_inactive");
    assert!(fact_expired_index < edge_expired_index);
}
