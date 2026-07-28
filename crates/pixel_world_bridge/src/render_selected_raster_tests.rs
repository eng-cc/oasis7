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
