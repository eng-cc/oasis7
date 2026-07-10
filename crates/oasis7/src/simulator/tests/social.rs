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
fn social_fact_impact_quote_previews_publish_without_mutating_journal() {
    let kernel = setup_social_kernel();
    let evidence_event_id = first_evidence_event_id(&kernel);
    let journal_len_before_quote = kernel.journal().len();
    let social_facts_before_quote = kernel.model().social_facts.clone();
    let actor_power_before_quote = electricity_of(&kernel, "agent-a");

    let quote = kernel
        .quote_publish_social_fact(
            &agent_owner("agent-a"),
            "social.reputation.v1",
            &agent_owner("agent-b"),
            Some(&agent_owner("agent-c")),
            "agent-b fulfilled delivery contract for agent-c",
            875_000,
            &[evidence_event_id],
            Some(12),
            Some(&SocialStake {
                kind: ResourceKind::Electricity,
                amount: 30,
            }),
        )
        .expect("publish quote");

    assert_eq!(kernel.journal().len(), journal_len_before_quote);
    assert_eq!(kernel.model().social_facts, social_facts_before_quote);
    assert_eq!(electricity_of(&kernel, "agent-a"), actor_power_before_quote);
    assert_eq!(quote.actor_id, "agent-a");
    assert_eq!(quote.action_kind, "publish_social_fact");
    assert_eq!(quote.schema_id, "social.reputation.v1");
    assert_eq!(quote.subject_id.as_deref(), Some("agent-b"));
    assert_eq!(quote.object_id.as_deref(), Some("agent-c"));
    assert!(quote.claim_summary.contains("fulfilled delivery"));
    assert_eq!(quote.confidence_ppm, Some(875_000));
    assert_eq!(quote.stake_at_risk, 30);
    assert_eq!(quote.ttl_ticks, Some(12));
    assert!(
        quote
            .affected_social_surfaces
            .contains(&"reputation".to_string())
    );
    assert_eq!(quote.cooperation_opportunity_delta, "positive");
    assert_eq!(quote.blacklist_or_dispute_risk, "stake_at_risk");
    assert_eq!(quote.governance_or_claim_relevance, "evidence_backed_claim");
    assert_eq!(quote.recommended_social_action, "publish_fact");
    assert!(quote.why_this_action_matters.contains("agent-b"));
}

#[test]
fn social_fact_impact_quote_previews_challenge_without_mutating_journal() {
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
        stake: None,
    });
    let publish = kernel.step().expect("publish");
    let fact_id = match publish.kind {
        WorldEventKind::SocialFactPublished { fact } => fact.fact_id,
        other => panic!("unexpected publish event: {other:?}"),
    };

    let journal_len_before_quote = kernel.journal().len();
    let fact_before_quote = kernel
        .model()
        .social_facts
        .get(&fact_id)
        .expect("fact before quote")
        .clone();
    let challenger_power_before_quote = electricity_of(&kernel, "agent-c");
    let quote = kernel
        .quote_challenge_social_fact(
            &agent_owner("agent-c"),
            fact_id,
            "evidence does not prove delivery",
            Some(&SocialStake {
                kind: ResourceKind::Electricity,
                amount: 20,
            }),
        )
        .expect("challenge quote");

    assert_eq!(kernel.journal().len(), journal_len_before_quote);
    assert_eq!(
        kernel.model().social_facts.get(&fact_id),
        Some(&fact_before_quote)
    );
    assert_eq!(
        electricity_of(&kernel, "agent-c"),
        challenger_power_before_quote
    );
    assert_eq!(quote.actor_id, "agent-c");
    assert_eq!(quote.action_kind, "challenge_social_fact");
    assert_eq!(quote.schema_id, "social.reputation.v1");
    assert_eq!(quote.subject_id.as_deref(), Some("agent-b"));
    assert_eq!(quote.claim_summary, "evidence does not prove delivery");
    assert_eq!(quote.stake_at_risk, 20);
    assert_eq!(quote.blacklist_or_dispute_risk, "opens_dispute");
    assert_eq!(quote.recommended_social_action, "challenge_fact");
    assert!(quote.why_this_action_matters.contains("dispute"));
}

#[test]
fn social_fact_impact_quote_rejects_location_electricity_stake_like_execution() {
    let mut kernel = setup_social_kernel();
    let actor = ResourceOwner::Location {
        location_id: "loc-social".to_string(),
    };
    seed_owner_resource(&mut kernel, actor.clone(), ResourceKind::Electricity, 100);
    let evidence_event_id = first_evidence_event_id(&kernel);
    let journal_len_before_quote = kernel.journal().len();

    let reason = kernel
        .quote_publish_social_fact(
            &actor,
            "social.reputation.v1",
            &agent_owner("agent-b"),
            None,
            "location sponsored reputation claim",
            750_000,
            &[evidence_event_id],
            None,
            Some(&SocialStake {
                kind: ResourceKind::Electricity,
                amount: 10,
            }),
        )
        .expect_err("location electricity stake should reject");

    assert_eq!(kernel.journal().len(), journal_len_before_quote);
    match reason {
        RejectReason::RuleDenied { notes } => {
            assert!(
                notes
                    .iter()
                    .any(|note| note.contains("location electricity pool removed"))
            );
        }
        other => panic!("unexpected reject reason: {other:?}"),
    }
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
fn social_publish_validates_evidence_ids_without_journal_scan_semantic_drift() {
    let mut kernel = setup_social_kernel();
    let existing_event_ids = kernel
        .journal()
        .iter()
        .take(2)
        .map(|event| event.id)
        .collect::<Vec<_>>();
    assert_eq!(existing_event_ids.len(), 2);

    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.reputation.v1".to_string(),
        subject: agent_owner("agent-b"),
        object: None,
        claim: "agent-b has two corroborating setup events".to_string(),
        confidence_ppm: 880_000,
        evidence_event_ids: existing_event_ids.clone(),
        ttl_ticks: None,
        stake: None,
    });
    let published = kernel.step().expect("publish event");
    match published.kind {
        WorldEventKind::SocialFactPublished { fact } => {
            assert_eq!(fact.evidence_event_ids, existing_event_ids);
        }
        other => panic!("unexpected publish event: {other:?}"),
    }

    let next_event_id = kernel.journal().last().expect("last event").id + 1;
    kernel.submit_action(Action::PublishSocialFact {
        actor: agent_owner("agent-a"),
        schema_id: "social.reputation.v1".to_string(),
        subject: agent_owner("agent-b"),
        object: None,
        claim: "agent-b has missing corroboration".to_string(),
        confidence_ppm: 880_000,
        evidence_event_ids: vec![existing_event_ids[0], next_event_id + 2, next_event_id + 1],
        ttl_ticks: None,
        stake: None,
    });
    let rejected = kernel.step().expect("reject event");
    match rejected.kind {
        WorldEventKind::ActionRejected {
            reason: RejectReason::RuleDenied { notes },
        } => {
            assert!(
                notes
                    .iter()
                    .any(|note| note
                        .contains(&format!("evidence event missing: {}", next_event_id + 2))),
                "missing evidence rejection note should keep first missing input order: {notes:?}"
            );
        }
        other => panic!("unexpected reject event: {other:?}"),
    }
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
    assert!(
        fact.challenge
            .as_ref()
            .expect("challenge exists")
            .stake
            .is_none()
    );
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
