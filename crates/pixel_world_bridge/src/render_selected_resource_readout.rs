use super::*;

const EMPTY_RESOURCE_READOUT: &str = "No resources reported";
const RESOURCE_READOUT_LAYER_Z: f32 = 5.0;
const RESOURCE_READOUT_ABOVE_SELECTION_PX: f64 = 36.0;
const RESOURCE_READOUT_MAX_CHARS: usize = 48;
const RESOURCE_READOUT_TOP_MARGIN_PX: f64 = 12.0;
const RESOURCE_READOUT_BOTTOM_HUD_CLEARANCE_PX: f64 = 72.0;
const RESOURCE_READOUT_HORIZONTAL_MARGIN_PX: f64 = 12.0;
const RESOURCE_READOUT_GLYPH_ADVANCE_PX: f64 = 6.6;

/// The selected entity's resource text, kept separate from status badges and
/// interaction entities so player-facing state cannot alter world input.
#[derive(Component)]
pub(crate) struct PixelWorldSelectedResourceReadout {
    pub(crate) target_kind: String,
    pub(crate) target_id: String,
    pub(crate) display: String,
}

pub(super) fn reconcile_selected_resource_readout(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing_readouts: &Query<(Entity, &PixelWorldSelectedResourceReadout)>,
    width: f64,
    height: f64,
) {
    let selected = runtime.render_state.as_ref().and_then(|render_state| {
        let selection = render_state.selection.as_ref()?;
        match selection.kind.as_str() {
            "agent" => render_state
                .agents
                .iter()
                .find(|agent| agent.id == selection.id)
                .map(|agent| {
                    let (canvas_x, canvas_y) = render_state
                        .world_bounds
                        .as_ref()
                        .and_then(|world_bounds| {
                            agent.pos.as_ref().and_then(|pos| {
                                to_canvas_point(pos, world_bounds, width, height, &runtime.camera)
                            })
                        })
                        .unwrap_or_else(|| {
                            fallback_point_for_entity(&agent.id, width, height, &runtime.camera)
                        });
                    (
                        "agent",
                        agent.id.as_str(),
                        agent.resource_summary.as_str(),
                        canvas_x,
                        canvas_y,
                    )
                }),
            "location" => render_state
                .locations
                .iter()
                .find(|location| location.id == selection.id)
                .map(|location| {
                    let (canvas_x, canvas_y) = render_state
                        .world_bounds
                        .as_ref()
                        .and_then(|world_bounds| {
                            to_canvas_point(
                                &location.pos,
                                world_bounds,
                                width,
                                height,
                                &runtime.camera,
                            )
                        })
                        .unwrap_or_else(|| {
                            fallback_point_for_entity(&location.id, width, height, &runtime.camera)
                        });
                    (
                        "location",
                        location.id.as_str(),
                        location.resource_summary.as_str(),
                        canvas_x,
                        canvas_y,
                    )
                }),
            _ => None,
        }
    });

    let Some((target_kind, target_id, resource_summary, canvas_x, canvas_y)) = selected else {
        for (entity, _) in existing_readouts.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let display = player_facing_resource_summary(resource_summary);
    let readout = PixelWorldSelectedResourceReadout {
        target_kind: target_kind.to_string(),
        target_id: target_id.to_string(),
        display,
    };
    let visuals = resource_readout_visuals(
        &readout.display,
        canvas_x,
        resource_readout_canvas_y(canvas_y, height),
        width,
        height,
    );

    let mut existing = existing_readouts.iter();
    if let Some((entity, _)) = existing.next() {
        commands.entity(entity).insert((readout, visuals));
    } else {
        commands.spawn((readout, visuals));
    }
    for (entity, _) in existing {
        commands.entity(entity).despawn();
    }
}

pub(super) fn despawn_selected_resource_readouts(
    commands: &mut Commands,
    existing_readouts: &Query<(Entity, &PixelWorldSelectedResourceReadout)>,
) {
    for (entity, _) in existing_readouts.iter() {
        commands.entity(entity).despawn();
    }
}

fn player_facing_resource_summary(resource_summary: &str) -> String {
    let trimmed = resource_summary.trim();
    if trimmed.is_empty() || trimmed == "-" || is_empty_amounts_container(trimmed) {
        EMPTY_RESOURCE_READOUT.to_string()
    } else {
        resource_summary.to_string()
    }
}

fn is_empty_amounts_container(resource_summary: &str) -> bool {
    resource_summary
        .chars()
        .filter(|character| !character.is_whitespace())
        .eq("amounts:{}".chars())
}

fn resource_readout_canvas_y(selection_canvas_y: f64, canvas_height: f64) -> f64 {
    let maximum_y = (canvas_height - RESOURCE_READOUT_BOTTOM_HUD_CLEARANCE_PX)
        .max(RESOURCE_READOUT_TOP_MARGIN_PX);
    (selection_canvas_y - RESOURCE_READOUT_ABOVE_SELECTION_PX)
        .clamp(RESOURCE_READOUT_TOP_MARGIN_PX, maximum_y)
}

fn resource_readout_visuals(
    display: &str,
    canvas_x: f64,
    canvas_y: f64,
    width: f64,
    height: f64,
) -> (Text2d, TextFont, TextColor, Transform) {
    let visible_display = visible_resource_summary(display, width);
    (
        Text2d::new(visible_display.clone()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb_u8(226, 232, 240)),
        Transform::from_translation(to_bevy_translation(
            resource_readout_canvas_x(canvas_x, &visible_display, width),
            canvas_y,
            width,
            height,
            RESOURCE_READOUT_LAYER_Z,
        )),
    )
}

fn resource_readout_canvas_x(
    selection_canvas_x: f64,
    visible_display: &str,
    canvas_width: f64,
) -> f64 {
    let half_width = resource_readout_text_width(visible_display) / 2.0;
    let minimum_x = half_width + RESOURCE_READOUT_HORIZONTAL_MARGIN_PX;
    let maximum_x = canvas_width - minimum_x;
    if minimum_x <= maximum_x {
        selection_canvas_x.clamp(minimum_x, maximum_x)
    } else {
        canvas_width / 2.0
    }
}

fn resource_readout_text_width(display: &str) -> f64 {
    display.chars().count() as f64 * RESOURCE_READOUT_GLYPH_ADVANCE_PX
}

fn visible_resource_summary(display: &str, canvas_width: f64) -> String {
    let characters = display.chars().collect::<Vec<_>>();
    let max_visible_chars = ((canvas_width - RESOURCE_READOUT_HORIZONTAL_MARGIN_PX * 2.0)
        / RESOURCE_READOUT_GLYPH_ADVANCE_PX)
        .floor()
        .max(1.0) as usize;
    let payload_limit = RESOURCE_READOUT_MAX_CHARS.min(max_visible_chars);
    if characters.len() <= payload_limit {
        return display.to_string();
    }

    let abbreviated = characters
        .iter()
        .take(RESOURCE_READOUT_MAX_CHARS.min(max_visible_chars.saturating_sub(1)))
        .collect::<String>();
    format!("{abbreviated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_readout_visual_uses_text_layer_and_bounds_long_content() {
        let (text, font, color, transform) = resource_readout_visuals(
            "water=12, ore=5, carbon=9, oxygen=4, hydrogen=7, silicon=3",
            480.0,
            resource_readout_canvas_y(290.0, 540.0),
            960.0,
            540.0,
        );

        assert!(text.0.ends_with('…'));
        assert!(text.0.chars().count() <= RESOURCE_READOUT_MAX_CHARS + 1);
        assert_eq!(font.font_size, FontSize::Px(11.0));
        assert_eq!(color.0, Color::srgb_u8(226, 232, 240));
        assert_eq!(transform.translation.z, RESOURCE_READOUT_LAYER_Z);
        assert_eq!(transform.translation.x, 0.0);
        assert_eq!(transform.translation.y, 16.0);
    }

    #[test]
    fn resource_readout_normalizes_empty_amounts_and_clamps_above_the_selection() {
        assert_eq!(
            player_facing_resource_summary(" amounts : { } "),
            EMPTY_RESOURCE_READOUT
        );
        assert_eq!(resource_readout_canvas_y(20.0, 390.0), 12.0);
        assert_eq!(resource_readout_canvas_y(380.0, 390.0), 318.0);
        assert_eq!(resource_readout_canvas_y(220.0, 390.0), 184.0);
    }

    #[test]
    fn resource_readout_keeps_long_text_inside_390px_left_and_right_edges() {
        let display = "water=12, ore=5, carbon=9, oxygen=4, hydrogen=7, silicon=3";
        let visible = visible_resource_summary(display, 390.0);
        let half_width = resource_readout_text_width(&visible) / 2.0;

        for selection_x in [0.0, 390.0] {
            let safe_x = resource_readout_canvas_x(selection_x, &visible, 390.0);
            assert!(safe_x - half_width >= RESOURCE_READOUT_HORIZONTAL_MARGIN_PX);
            assert!(safe_x + half_width <= 390.0 - RESOURCE_READOUT_HORIZONTAL_MARGIN_PX);
        }
        assert_eq!(visible.chars().count(), RESOURCE_READOUT_MAX_CHARS + 1);
    }
}
