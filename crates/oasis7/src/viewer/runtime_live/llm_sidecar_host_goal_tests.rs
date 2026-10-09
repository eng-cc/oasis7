use super::*;

#[test]
fn runtime_live_host_goal_default_is_bound_not_empty() {
    let snapshot = trusted_provider_goal_snapshot(None).expect("default host goal");
    assert_eq!(snapshot.revision, 1);
    assert_eq!(
        snapshot.short_term_summary,
        runtime_live_phase1_short_term_goal()
    );
    assert!(snapshot.long_term_summary.is_empty());
    assert_eq!(snapshot.provenance, "harness_projection");
    assert_ne!(
        snapshot.digest,
        crate::simulator::GoalSnapshotV1::empty().digest
    );
}

#[test]
fn runtime_live_host_goal_profile_changes_bind_normalized_text_and_revision() {
    let mut profile = AgentPromptProfile::for_agent("agent-1");
    profile.version = 7;
    profile.short_term_goal_override = Some("  Cafe\u{301}  ".to_string());
    profile.long_term_goal_override = Some("  grow factory  ".to_string());
    let first = trusted_provider_goal_snapshot(Some(&profile)).expect("host overrides");
    assert_eq!(first.revision, 7);
    assert_eq!(first.short_term_summary, "Café");
    assert_eq!(first.long_term_summary, "grow factory");

    profile.short_term_goal_override = Some("Café".to_string());
    assert_eq!(
        trusted_provider_goal_snapshot(Some(&profile))
            .unwrap()
            .digest,
        first.digest
    );
    profile.version += 1;
    let revised = trusted_provider_goal_snapshot(Some(&profile)).unwrap();
    assert_ne!(revised.digest, first.digest);
    profile.long_term_goal_override = Some("another goal".to_string());
    assert_ne!(
        trusted_provider_goal_snapshot(Some(&profile))
            .unwrap()
            .digest,
        revised.digest
    );
}

#[test]
fn runtime_live_host_goal_rejects_oversize_text_without_truncation() {
    for long_term in [false, true] {
        let mut profile = AgentPromptProfile::for_agent("agent-1");
        let target = if long_term {
            &mut profile.long_term_goal_override
        } else {
            &mut profile.short_term_goal_override
        };
        *target = Some("界".repeat(171)); // 513 UTF-8 bytes.
        let error = trusted_provider_goal_snapshot(Some(&profile)).expect_err("oversize host goal");
        assert!(error.contains("goal_snapshot_too_large"), "{error}");
    }
}
