use super::*;

pub(super) fn selection_focus_position(
    render_state: &RenderState,
    focus_target: &FocusTarget,
) -> Option<Position> {
    match focus_target.kind.as_str() {
        "agent" => render_state
            .agents
            .iter()
            .find(|agent| agent.id == focus_target.id)
            .and_then(|agent| agent.pos.clone()),
        "location" => render_state
            .locations
            .iter()
            .find(|location| location.id == focus_target.id)
            .map(|location| location.pos.clone()),
        "world_event" => render_state
            .world_events
            .iter()
            .find(|event| event.id == focus_target.id)
            .map(|event| event.pos.clone()),
        "module_visual" => render_state
            .module_visual_entities
            .iter()
            .find(|entity| entity.id == focus_target.id)
            .map(|entity| entity.pos.clone()),
        _ => None,
    }
}
pub(super) fn maybe_focus_selected_entity(runtime: &mut BevyRuntimeState, width: f64, height: f64) {
    let Some(focus_target) = runtime.pending_focus_target.clone() else {
        return;
    };
    let Some(render_state) = runtime.render_state.as_ref() else {
        runtime.pending_focus_target = None;
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        runtime.pending_focus_target = None;
        return;
    };
    let Some(position) = selection_focus_position(render_state, &focus_target) else {
        runtime.pending_focus_target = None;
        return;
    };
    let Some((selected_x, selected_y)) = to_canvas_point(
        &position,
        world_bounds,
        width,
        height,
        &CameraState::default(),
    ) else {
        runtime.pending_focus_target = None;
        return;
    };
    let centered_x = selected_x - (width / 2.0);
    let centered_y = selected_y - (height / 2.0);
    runtime.camera.pan_x_px = -(centered_x * runtime.camera.zoom.max(0.5));
    runtime.camera.pan_y_px = -(centered_y * runtime.camera.zoom.max(0.5));
    runtime.camera_user_override = true;
    runtime.camera_fit_version = runtime.render_version;
    runtime.pending_focus_target = None;
    let _ = camera_viewport::emit(runtime);
}
