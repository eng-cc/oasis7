use super::*;

pub(super) fn requeue_follow_target_after_resize(
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
) {
    let canvas_size = (
        width.round().max(0.0) as u32,
        height.round().max(0.0) as u32,
    );
    if runtime.last_canvas_size.replace(canvas_size) != Some(canvas_size)
        && let Some(follow_target) = runtime.active_follow_target.clone()
    {
        // Direct pan or zoom clears the follow target. Only an active selection
        // is re-centred when the embedded canvas changes size.
        runtime.pending_focus_target = Some(follow_target);
    }
}
