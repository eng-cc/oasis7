use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Navigation {
    pub(super) request_id: String,
    pub(super) mode: String,
    target: Option<Selection>,
}

fn available(state: &RenderState, target: &FocusTarget) -> bool {
    match target.kind.as_str() {
        "agent" => state
            .agents
            .iter()
            .any(|agent| agent.id == target.id && agent.pos.is_some()),
        "location" => state
            .locations
            .iter()
            .any(|location| location.id == target.id),
        "module_visual" => state
            .module_visual_entities
            .iter()
            .any(|entity| entity.id == target.id),
        _ => false,
    }
}

pub(super) fn prepare_camera(
    runtime: &mut BevyRuntimeState,
    state: Option<&RenderState>,
    version: u64,
) -> bool {
    let navigation_changed = state
        .and_then(|state| state.navigation.as_ref())
        .is_some_and(|next| {
            runtime
                .render_state
                .as_ref()
                .and_then(|old| old.navigation.as_ref())
                .is_none_or(|old| old.request_id != next.request_id)
        });
    let overview = navigation_changed
        && state
            .and_then(|state| state.navigation.as_ref())
            .is_some_and(|nav| nav.mode == "overview");
    if state.is_some_and(|state| state.exploration_mode)
        && runtime.render_state.is_some()
        && !overview
    {
        runtime.camera_fit_version = version;
    }
    navigation_changed
}

pub(super) fn apply(runtime: &mut BevyRuntimeState, state: Option<&RenderState>, moved: bool) {
    let Some(state) = state else {
        return;
    };
    if runtime
        .render_state
        .as_ref()
        .is_none_or(|old| !old.exploration_mode)
    {
        runtime.active_follow_target = None;
        runtime.pending_focus_target = None;
    }
    // Losing a trusted location pauses tracking permanently until a new command.
    if runtime
        .active_follow_target
        .as_ref()
        .is_some_and(|target| !available(state, target))
    {
        runtime.active_follow_target = None;
        runtime.pending_focus_target = None;
    }
    let previous = runtime
        .render_state
        .as_ref()
        .and_then(|state| state.navigation.as_ref());
    let command = state.navigation.as_ref().filter(|command| {
        !command.request_id.is_empty()
            && previous.is_none_or(|old| old.request_id != command.request_id)
    });
    if let Some(command) = command {
        runtime.active_follow_target = None;
        runtime.pending_focus_target = None;
        match command.mode.as_str() {
            "overview" => {
                runtime.camera_fit_version = 0;
                runtime.last_canvas_size = None;
                runtime.camera_user_override = false;
            }
            "locate" | "follow" => {
                let target = command
                    .target
                    .as_ref()
                    .map(|target| FocusTarget {
                        kind: target.kind.clone(),
                        id: target.id.clone(),
                    })
                    .filter(|target| available(state, target));
                runtime.pending_focus_target = target.clone();
                if command.mode == "follow" {
                    runtime.active_follow_target = target;
                }
            }
            _ => {}
        }
        runtime.hit_regions_dirty = true;
    } else if moved {
        runtime.pending_focus_target = runtime.active_follow_target.clone();
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
