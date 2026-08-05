use super::super::fixtures::{
    sample_render_state_with_receipt_target, sample_render_state_with_recommended_target,
};
use super::super::*;

fn recommended_target_cue_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&PixelWorldRecommendedTargetCue>();
    query.iter(world).count()
}

#[test]
fn recommended_target_wayfinder_is_reused_then_removed_without_hit_regions() {
    let mut app = render_test_app(sample_render_state_with_recommended_target(Some("agent-0")));
    let hit_regions_before = hit_regions(&mut app);
    assert_eq!(recommended_target_cue_count(&mut app), 3);

    app.update();
    assert_eq!(recommended_target_cue_count(&mut app), 3);
    assert_eq!(hit_regions(&mut app), hit_regions_before);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = Some(sample_render_state_with_recommended_target(None));
    app.update();
    assert_eq!(recommended_target_cue_count(&mut app), 0);
    assert_eq!(hit_regions(&mut app), hit_regions_before);
}

#[test]
fn recommended_target_wayfinder_yields_to_receipt_target_for_the_same_agent() {
    let mut state = sample_render_state_with_receipt_target(Some("accepted"), Some("agent-0"));
    state.recommended_target = Some(RecommendedTarget {
        agent_id: "agent-0".to_string(),
    });
    let mut app = render_test_app(state);

    assert_eq!(recommended_target_cue_count(&mut app), 0);
    let world = app.world_mut();
    let mut receipt_query = world.query::<&PixelWorldReceiptTargetCue>();
    assert!(
        receipt_query.iter(world).next().is_some(),
        "the receipt cue remains the higher-priority explicit feedback"
    );
}
