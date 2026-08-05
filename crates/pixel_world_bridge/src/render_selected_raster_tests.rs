use super::fixtures::{
    sample_render_state_with_receipt_target, sample_render_state_with_recommended_target,
};
use super::*;

#[test]
fn bevy_pixel_regression_exports_selected_location_ring_with_world_layers() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "location", "loc-0",
    ));
    let (image, summary) = rasterize_pixel_regression(&mut app);

    assert!(summary.grid_pixels > 0);
    assert!(summary.fragment_pixels > 0);
    assert!(summary.location_pixels > 0);
    assert!(summary.selected_location_cue_pixels > 0);
    assert!(summary.agent_pixels > 0);
    assert!(summary.agent_pixels > summary.selected_location_cue_pixels);
    assert!(
        image.pixels().any(|pixel| pixel.0 == [251, 191, 36, 255]),
        "selected location ring must contribute opaque amber pixels"
    );

    write_pixel_probe_if_requested(&image, &summary);
}

#[test]
fn bevy_pixel_regression_exports_selected_agent_corner_cue_with_world_layers() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let (image, summary) = rasterize_pixel_regression(&mut app);

    assert!(summary.grid_pixels > 0);
    assert!(summary.agent_pixels > 0);
    assert!(summary.agent_core_pixels > 0);
    assert!(summary.selected_agent_cue_pixels > 0);
    assert!(
        image.pixels().any(|pixel| pixel.0 == [251, 191, 36, 255]),
        "selected agent cue must contribute opaque amber pixels"
    );

    write_pixel_probe_if_requested(&image, &summary);
}

#[test]
fn bevy_pixel_regression_exports_only_location_derived_position_cues() {
    let mut derived_state = sample_render_state(12_000.0);
    derived_state.selection = None;
    let mut derived = render_test_app(derived_state);
    let (image, derived_summary) = rasterize_pixel_regression(&mut derived);
    let mut snapshot_state = sample_render_state(12_000.0);
    snapshot_state.selection = None;
    snapshot_state.agents[0].position_source = AgentPositionSource::Snapshot;
    let mut snapshot = render_test_app(snapshot_state);
    let (_, snapshot_summary) = rasterize_pixel_regression(&mut snapshot);

    assert!(
        derived_summary.derived_position_cue_pixels > 0,
        "a location-derived Agent position must contribute a visible hollow cue to the raster"
    );
    assert_eq!(
        snapshot_summary.derived_position_cue_pixels, 0,
        "snapshot positions must not contribute provenance cue pixels"
    );
    write_pixel_probe_if_requested(&image, &derived_summary);
}

#[test]
fn bevy_pixel_regression_exports_visible_receipt_target_pixels() {
    let mut visible = render_test_app(sample_render_state_with_receipt_target(
        Some("accepted"),
        Some("agent-0"),
    ));
    let (_, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut absent = render_test_app(sample_render_state_with_receipt_target(None, None));
    let (_, absent_summary) = rasterize_pixel_regression(&mut absent);

    assert!(
        visible_summary.non_background_pixels > absent_summary.non_background_pixels,
        "an accepted receipt cue must contribute visible raster pixels above its target Agent"
    );
}

#[test]
fn bevy_pixel_regression_exports_recommended_target_wayfinder_pixels() {
    let mut visible = render_test_app(sample_render_state_with_recommended_target(Some("agent-0")));
    let (_, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut absent = render_test_app(sample_render_state_with_recommended_target(None));
    let (_, absent_summary) = rasterize_pixel_regression(&mut absent);

    assert!(
        visible_summary.non_background_pixels > absent_summary.non_background_pixels,
        "the recommended target bracket must contribute visible pixels above its target Agent"
    );
}
