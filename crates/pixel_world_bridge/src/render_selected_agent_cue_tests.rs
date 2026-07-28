use super::*;

const SELECTED_AGENT_CUE_COLOR: Color = Color::srgb_u8(251, 191, 36);

#[derive(Clone, Copy, Debug, PartialEq)]
struct CueSegmentProbe {
    x: f32,
    y: f32,
    z: f32,
    width: f32,
    height: f32,
}

fn selected_agent_cue_segments(app: &mut App) -> Vec<CueSegmentProbe> {
    let world = app.world_mut();
    let mut query = world.query::<(&Sprite, &Transform)>();
    let mut segments = query
        .iter(world)
        .filter_map(|(sprite, transform)| {
            let size = sprite.custom_size?;
            (sprite.color == SELECTED_AGENT_CUE_COLOR
                && transform.translation.z > AGENT_LAYER_Z + SELECTED_ENTITY_LAYER_Z_OFFSET)
                .then_some(CueSegmentProbe {
                    x: transform.translation.x,
                    y: transform.translation.y,
                    z: transform.translation.z,
                    width: size.x,
                    height: size.y,
                })
        })
        .collect::<Vec<_>>();
    segments.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then(left.y.total_cmp(&right.y))
            .then(left.width.total_cmp(&right.width))
            .then(left.height.total_cmp(&right.height))
    });
    segments
}

fn normalized_hit_regions(app: &mut App) -> Vec<(String, String, i64, i64, i64, i64)> {
    let mut regions = hit_regions(app)
        .into_iter()
        .map(|region| {
            (
                region.kind.to_string(),
                region.id,
                (region.left * 1_000.0).round() as i64,
                (region.top * 1_000.0).round() as i64,
                (region.right * 1_000.0).round() as i64,
                (region.bottom * 1_000.0).round() as i64,
            )
        })
        .collect::<Vec<_>>();
    regions.sort();
    regions
}

#[test]
fn selected_agent_corner_frame_has_exactly_four_amber_l_cues_above_the_core() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let summary = visual_probe_summary(&mut app);
    let selected = summary
        .agents
        .iter()
        .find(|agent| agent.id == "agent-0")
        .expect("selected agent");
    let core = summary
        .agent_cores
        .iter()
        .find(|agent| agent.id == "agent-0")
        .expect("selected agent core");
    let segments = selected_agent_cue_segments(&mut app);

    assert_eq!(
        segments.len(),
        8,
        "four L-shaped corner cues require one horizontal and one vertical segment per corner"
    );
    for x_sign in [-1.0_f32, 1.0] {
        for y_sign in [-1.0_f32, 1.0] {
            let corner = segments
                .iter()
                .filter(|segment| {
                    (segment.x - selected.x).signum() == x_sign
                        && (segment.y - selected.y).signum() == y_sign
                })
                .collect::<Vec<_>>();
            assert_eq!(corner.len(), 2, "each of the four corners has one L cue");
            assert!(
                corner.iter().any(|segment| segment.width > segment.height),
                "each L cue has a horizontal tick"
            );
            assert!(
                corner.iter().any(|segment| segment.height > segment.width),
                "each L cue has a vertical tick"
            );
        }
    }
    assert!(
        segments.iter().all(|segment| segment.z > core.z),
        "corner cues render above the selected agent core"
    );
    let selected_half_size = selected.size_px / 2.0;
    assert!(
        segments.iter().all(|segment| {
            (segment.x - selected.x).abs() > selected_half_size
                || (segment.y - selected.y).abs() > selected_half_size
        }),
        "corner cues remain outside the selected agent body"
    );
}

#[test]
fn selected_agent_corner_frame_is_absent_for_unselected_or_non_agent_selection() {
    for (kind, id) in [("location", "loc-0"), ("agent", "missing-agent")] {
        let mut app = render_test_app(sample_render_state_with_beacon_candidates(kind, id));
        assert!(
            selected_agent_cue_segments(&mut app).is_empty(),
            "{kind}:{id} must not receive selected-agent corner cues"
        );
    }
}

#[test]
fn selected_agent_corner_frame_reuses_geometry_across_repeat_and_agent_reorder() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let initial = selected_agent_cue_segments(&mut app);
    assert_eq!(initial.len(), 8);

    app.update();
    assert_eq!(
        selected_agent_cue_segments(&mut app),
        initial,
        "unchanged reconcile keeps stable cue geometry"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let render_state = runtime.render_state.as_mut().expect("render state");
        render_state.agents.reverse();
        runtime.render_version += 1;
    }
    app.update();
    assert_eq!(
        selected_agent_cue_segments(&mut app),
        initial,
        "DTO reorder must not move or duplicate the selected agent cue"
    );
}

#[test]
fn selected_agent_corner_frame_cleans_up_without_changing_hit_regions() {
    let mut selected_app = render_test_app(sample_render_state_with_beacon_candidates(
        "agent", "agent-0",
    ));
    let selected_regions = normalized_hit_regions(&mut selected_app);
    assert_eq!(selected_agent_cue_segments(&mut selected_app).len(), 8);

    let mut unselected_app = render_test_app({
        let mut state = sample_render_state_with_beacon_candidates("agent", "agent-0");
        state.selection = None;
        state
    });
    assert_eq!(
        normalized_hit_regions(&mut unselected_app),
        selected_regions,
        "visual selection cues must not change interaction hit regions"
    );

    {
        let mut runtime = selected_app.world_mut().resource_mut::<BevyRuntimeState>();
        let render_state = runtime.render_state.as_mut().expect("render state");
        render_state.selection = None;
        runtime.render_version += 1;
    }
    selected_app.update();
    assert!(
        selected_agent_cue_segments(&mut selected_app).is_empty(),
        "deselection removes every corner cue"
    );

    {
        let mut runtime = selected_app.world_mut().resource_mut::<BevyRuntimeState>();
        let render_state = runtime.render_state.as_mut().expect("render state");
        render_state.selection = Some(Selection {
            kind: "agent".to_string(),
            id: "agent-0".to_string(),
        });
        render_state.agents.retain(|agent| agent.id != "agent-0");
        runtime.render_version += 1;
        runtime.hit_regions_dirty = true;
    }
    selected_app.update();
    assert!(
        selected_agent_cue_segments(&mut selected_app).is_empty(),
        "removing the selected agent leaves no stale corner cue"
    );
}
