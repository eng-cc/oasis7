#[test]
fn governance_recovery_drill_alert_rejects_silence_age_overflow_without_mutation() {
    let client = sample_client();
    let alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let alert_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 2,
            alert_cooldown_ms: 200,
        };
    let drill_run_report =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            scheduled_at_ms: 1_000,
            drill_due: true,
            drill_executed: true,
            next_due_at_ms: Some(1_100),
            drill_report: Some(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport {
                    world_id: "w1".to_string(),
                    node_id: "node-a".to_string(),
                    drill_at_ms: 1_000,
                    alert_state: MembershipRevocationDeadLetterReplayRollbackAlertState {
                        last_alert_at_ms: Some(1),
                    },
                    governance_state: MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                        rollback_streak: 3,
                        last_level:
                            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        last_level_updated_at_ms: Some(980),
                    },
                    recent_audits: vec![sample_governance_audit_record(
                        "w1",
                        "node-a",
                        990,
                        MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        3,
                    )],
                    has_emergency_history: true,
                },
            ),
        };

    let error = client
        .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
            "w1",
            "node-a",
            i64::MIN,
            &drill_run_report,
            &alert_policy,
            &alert_state_store,
            &alert_sink,
        )
        .expect_err("silence age underflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("alert silence age overflow"),
        "unexpected error: {message}"
    );

    let stored = alert_state_store
        .load_state("w1", "node-a")
        .expect("load drill alert state after overflow");
    assert_eq!(stored.last_alert_at_ms, None);
    assert!(
        alert_sink.list().expect("list emitted alerts").is_empty(),
        "overflow should not emit alerts"
    );
}

#[test]
fn governance_recovery_drill_alert_rejects_cooldown_age_overflow_without_mutation() {
    let client = sample_client();
    let alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let alert_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 2,
            alert_cooldown_ms: 200,
        };
    alert_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState {
                last_alert_at_ms: Some(1),
            },
        )
        .expect("seed drill alert state");
    let drill_run_report =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            scheduled_at_ms: 1_000,
            drill_due: true,
            drill_executed: true,
            next_due_at_ms: Some(1_100),
            drill_report: Some(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport {
                    world_id: "w1".to_string(),
                    node_id: "node-a".to_string(),
                    drill_at_ms: 1_000,
                    alert_state: MembershipRevocationDeadLetterReplayRollbackAlertState {
                        last_alert_at_ms: None,
                    },
                    governance_state: MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                        rollback_streak: 3,
                        last_level:
                            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        last_level_updated_at_ms: Some(980),
                    },
                    recent_audits: vec![sample_governance_audit_record(
                        "w1",
                        "node-a",
                        990,
                        MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        3,
                    )],
                    has_emergency_history: true,
                },
            ),
        };

    let error = client
        .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
            "w1",
            "node-a",
            i64::MIN,
            &drill_run_report,
            &alert_policy,
            &alert_state_store,
            &alert_sink,
        )
        .expect_err("cooldown age underflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("alert cooldown age overflow"),
        "unexpected error: {message}"
    );

    let stored = alert_state_store
        .load_state("w1", "node-a")
        .expect("load drill alert state after overflow");
    assert_eq!(stored.last_alert_at_ms, Some(1));
    assert!(
        alert_sink.list().expect("list emitted alerts").is_empty(),
        "overflow should not emit alerts"
    );
}

#[test]
fn governance_archive_tiered_offload_drill_alert_orchestration_runs_end_to_end() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let drill_schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    let drill_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    rollback_alert_state_store
        .save_alert_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackAlertState {
                last_alert_at_ms: None,
            },
        )
        .expect("save rollback alert state");
    rollback_governance_state_store
        .save_governance_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                rollback_streak: 3,
                last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                last_level_updated_at_ms: Some(980),
            },
        )
        .expect("save governance state");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &hot_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            700,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append audit 1");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &hot_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            900,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            3,
        ),
    )
    .expect("append audit 2");

    let retention_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
            max_records: 2,
            max_age_ms: 10_000,
        };
    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 1,
            offload_min_age_ms: 200,
            max_offload_records: 10,
        };
    let drill_schedule_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 100,
            recent_audit_limit: 5,
        };
    let drill_alert_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 2,
            alert_cooldown_ms: 500,
        };
    let run_report = client
        .run_revocation_dead_letter_replay_rollback_governance_archive_tiered_offload_with_drill_schedule_and_alert(
            "w1",
            "node-a",
            1_000,
            &retention_policy,
            &offload_policy,
            &drill_schedule_policy,
            &drill_alert_policy,
            &hot_store,
            &cold_store,
            &drill_schedule_state_store,
            &drill_alert_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &hot_store,
            &alert_sink,
        )
        .expect("run orchestration");
    assert_eq!(run_report.prune_report.before, 2);
    assert!(run_report.offload_report.offloaded >= 1);
    assert!(run_report.drill_run_report.drill_executed);
    assert!(run_report.drill_alert_report.alert_emitted);

    let alerts = alert_sink.list().expect("list alerts");
    assert_eq!(alerts.len(), 1);
}

#[test]
fn governance_tiered_offload_rejects_invalid_policy() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let invalid_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 0,
            offload_min_age_ms: 10,
            max_offload_records: 1,
        };
    let error = client
        .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
            "w1",
            "node-a",
            1_000,
            &invalid_policy,
            &hot_store,
            &cold_store,
        )
        .expect_err("invalid offload policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("hot_max_records must be positive"),
        "unexpected error: {message}"
    );
}

#[test]
fn governance_recovery_drill_alert_rejects_invalid_policy() {
    let client = sample_client();
    let alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let drill_run_report =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            scheduled_at_ms: 1_000,
            drill_due: false,
            drill_executed: false,
            next_due_at_ms: Some(1_100),
            drill_report: None,
        };
    let invalid_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 0,
            alert_cooldown_ms: 100,
        };
    let error = client
        .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
            "w1",
            "node-a",
            1_000,
            &drill_run_report,
            &invalid_policy,
            &alert_state_store,
            &alert_sink,
        )
        .expect_err("invalid alert policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("rollback_streak_threshold must be positive"),
        "unexpected error: {message}"
    );
}

#[test]
fn governance_audit_aggregate_query_filters_levels_and_min_time() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    for (tier, node_id, audited_at_ms, level, streak) in [
        (
            "hot",
            "node-a",
            700,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
        (
            "hot",
            "node-a",
            980,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            3,
        ),
        (
            "hot",
            "node-b",
            960,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            1,
        ),
        (
            "cold",
            "node-a",
            940,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            2,
        ),
        (
            "cold",
            "node-b",
            910,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    ] {
        let store = if tier == "hot" {
            &hot_store
        } else {
            &cold_store
        };
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            store,
            "w1",
            node_id,
            &sample_governance_audit_record("w1", node_id, audited_at_ms, level, streak),
        )
        .expect("append aggregate audit sample");
    }

    let policy = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy {
        include_hot: true,
        include_cold: true,
        max_records: 10,
        min_audited_at_ms: Some(930),
        levels: vec![
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
        ],
    };
    let report = client
        .query_revocation_dead_letter_replay_rollback_governance_audit_archive_aggregated(
            "w1",
            &["node-a".to_string(), "node-b".to_string()],
            &policy,
            &hot_store,
            &cold_store,
        )
        .expect("aggregate query");
    assert_eq!(report.world_id, "w1");
    assert_eq!(report.queried_node_count, 2);
    assert_eq!(report.scanned_hot, 3);
    assert_eq!(report.scanned_cold, 2);
    assert_eq!(report.returned, 3);
    assert_eq!(report.records[0].audit.audited_at_ms, 980);
    assert_eq!(
        report.records[0].tier,
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier::Hot
    );
    assert_eq!(report.records[1].audit.audited_at_ms, 960);
    assert_eq!(report.records[2].audit.audited_at_ms, 940);
}

#[test]
fn governance_audit_aggregate_query_rejects_invalid_policy() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let invalid_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy {
            include_hot: false,
            include_cold: false,
            max_records: 10,
            min_audited_at_ms: None,
            levels: Vec::new(),
        };
    let error = client
        .query_revocation_dead_letter_replay_rollback_governance_audit_archive_aggregated(
            "w1",
            &["node-a".to_string()],
            &invalid_policy,
            &hot_store,
            &cold_store,
        )
        .expect_err("invalid aggregate query policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("requires include_hot or include_cold"),
        "unexpected error: {message}"
    );
}

#[test]
fn governance_recovery_drill_alert_event_bus_file_round_trip() {
    let root = temp_membership_dir("governance-recovery-drill-alert-event-bus");
    fs::create_dir_all(&root).expect("create temp dir");
    let bus =
        FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus::new(
            &root,
        )
        .expect("create event bus");
    let event =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            event_at_ms: 1_000,
            outcome: MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted,
            reasons: vec!["emergency_history_detected".to_string()],
            severity: Some(MembershipRevocationAlertSeverity::Critical),
        };
    bus.publish("w1", "node-a", &event).expect("publish event");
    let listed = bus.list("w1", "node-a").expect("list events");
    assert_eq!(listed, vec![event]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn governance_recovery_drill_alert_event_bus_file_lists_cold_and_hot_records() {
    let root = temp_membership_dir("governance-recovery-drill-alert-event-bus-tiered");
    fs::create_dir_all(&root).expect("create temp dir");
    let bus =
        FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus::new(
            &root,
        )
        .expect("create event bus");

    let total = 4_100_i64;
    for event_at_ms in 0..total {
        bus.publish(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent {
                world_id: "w1".to_string(),
                node_id: "node-a".to_string(),
                event_at_ms,
                outcome:
                    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted,
                reasons: vec![format!("reason-{event_at_ms}")],
                severity: Some(MembershipRevocationAlertSeverity::Warn),
            },
        )
        .expect("publish event");
    }

    let listed = bus.list("w1", "node-a").expect("list events");
    assert_eq!(listed.len(), total as usize);
    assert_eq!(listed.first().map(|event| event.event_at_ms), Some(0));
    assert_eq!(
        listed.last().map(|event| event.event_at_ms),
        Some(total - 1)
    );
    assert!(
        root.join("w1.node-a.revocation-dead-letter-replay-rollback-governance-recovery-drill-alert-event.cold.refs.jsonl")
            .exists(),
        "tiered offload should create cold refs when hot window overflows"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn governance_archive_tiered_offload_drill_alert_event_bus_orchestration_publishes_event() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let drill_schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    let drill_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let event_bus =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus::new();

    rollback_alert_state_store
        .save_alert_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackAlertState {
                last_alert_at_ms: None,
            },
        )
        .expect("save rollback alert state");
    rollback_governance_state_store
        .save_governance_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                rollback_streak: 3,
                last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                last_level_updated_at_ms: Some(980),
            },
        )
        .expect("save governance state");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &hot_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            900,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            3,
        ),
    )
    .expect("append audit");

    let retention_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
            max_records: 2,
            max_age_ms: 10_000,
        };
    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 1,
            offload_min_age_ms: 200,
            max_offload_records: 10,
        };
    let drill_schedule_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 100,
            recent_audit_limit: 5,
        };
    let drill_alert_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 2,
            alert_cooldown_ms: 500,
        };

    let run_report = client
        .run_revocation_dead_letter_replay_rollback_governance_archive_tiered_offload_with_drill_schedule_alert_and_event_bus(
            "w1",
            "node-a",
            1_000,
            &retention_policy,
            &offload_policy,
            &drill_schedule_policy,
            &drill_alert_policy,
            &hot_store,
            &cold_store,
            &drill_schedule_state_store,
            &drill_alert_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &hot_store,
            &alert_sink,
            &event_bus,
        )
        .expect("run orchestration with event bus");
    assert!(run_report.run_report.drill_alert_report.alert_emitted);
    assert_eq!(
        run_report.alert_event.outcome,
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted
    );
    let events = event_bus
        .list("w1", "node-a")
        .expect("list event bus records");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].outcome,
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted
    );
}
