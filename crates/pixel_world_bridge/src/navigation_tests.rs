use super::*;

fn state(mode: &str, request_id: &str) -> RenderState {
    let mut state = crate::lib_tests::sample_render_state_for_camera("agent");
    state.exploration_mode = true;
    state.navigation = Some(Navigation {
        mode: mode.into(),
        request_id: request_id.into(),
        target: Some(Selection {
            kind: "agent".into(),
            id: "agent-0".into(),
        }),
    });
    state
}

fn update(runtime: &mut BevyRuntimeState, state: RenderState, version: u64) {
    apply_external_render_snapshot(
        runtime,
        true,
        RenderSnapshot::Changed {
            version,
            state: Some(Box::new(state)),
        },
    );
}

#[test]
fn selection_in_exploration_does_not_move_camera() {
    let mut runtime = BevyRuntimeState::default();
    let mut state = state("stop", "0");
    state.navigation = None;
    runtime.camera_user_override = true;
    update(&mut runtime, state, 1);
    assert!(runtime.pending_focus_target.is_none());
    assert!(runtime.active_follow_target.is_none());
    assert!(runtime.camera_user_override);
}

#[test]
fn same_target_can_be_located_followed_stopped_and_located_again() {
    let mut runtime = BevyRuntimeState::default();
    update(&mut runtime, state("locate", "1"), 1);
    assert!(runtime.pending_focus_target.is_some());
    assert!(runtime.active_follow_target.is_none());
    update(&mut runtime, state("follow", "2"), 2);
    assert!(runtime.active_follow_target.is_some());
    update(&mut runtime, state("stop", "3"), 3);
    assert!(runtime.pending_focus_target.is_none());
    assert!(runtime.active_follow_target.is_none());
    update(&mut runtime, state("locate", "4"), 4);
    assert!(runtime.pending_focus_target.is_some());
}

#[test]
fn missing_trusted_position_pauses_follow_without_automatic_resume() {
    let mut runtime = BevyRuntimeState::default();
    update(&mut runtime, state("follow", "1"), 1);
    let mut missing = state("follow", "1");
    missing.agents[0].pos = None;
    update(&mut runtime, missing, 2);
    assert!(runtime.active_follow_target.is_none());
    update(&mut runtime, state("follow", "1"), 3);
    assert!(runtime.active_follow_target.is_none());
    assert!(runtime.pending_focus_target.is_none());
    update(&mut runtime, state("follow", "2"), 4);
    assert!(runtime.active_follow_target.is_some());
}

#[test]
fn manual_camera_override_does_not_replay_same_navigation_request() {
    let mut runtime = BevyRuntimeState::default();
    update(&mut runtime, state("follow", "1"), 1);
    runtime.active_follow_target = None;
    runtime.pending_focus_target = None;
    update(&mut runtime, state("follow", "1"), 2);
    assert!(runtime.active_follow_target.is_none());
    assert!(runtime.pending_focus_target.is_none());
}

#[test]
fn changing_selection_keeps_camera_fit_and_overview_explicitly_resets_it() {
    let mut runtime = BevyRuntimeState::default();
    update(&mut runtime, state("stop", "1"), 1);
    runtime.camera_fit_version = 1;
    let mut next = state("stop", "1");
    next.selection = Some(Selection {
        kind: "location".into(),
        id: "loc-0".into(),
    });
    update(&mut runtime, next, 2);
    assert_eq!(runtime.camera_fit_version, 2);
    assert!(runtime.pending_focus_target.is_none());
    update(&mut runtime, state("overview", "2"), 3);
    assert_eq!(runtime.camera_fit_version, 0);
    assert!(runtime.needs_reconcile);
}
