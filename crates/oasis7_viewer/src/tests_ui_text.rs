use super::*;
use crate::i18n::{locale_or_default, UiLocale};
use crate::ui_locale_text::{selection_line, status_line};
use crate::ui_text::{
    agent_activity_summary, events_summary, provider_debug_summary, selection_details_summary,
    world_summary, ProviderDebugFilter,
};

pub(super) fn default_locale() -> UiLocale {
    locale_or_default(None)
}

pub(super) fn build_status_text(state: &ViewerState, locale: UiLocale) -> String {
    status_line(&state.status, locale)
}

pub(super) fn build_summary_text(
    state: &ViewerState,
    viewer_3d_config: Option<&Viewer3dConfig>,
    locale: UiLocale,
) -> String {
    ui_locale_text::localize_world_summary_block(
        world_summary(
            state.snapshot.as_ref(),
            state.metrics.as_ref(),
            viewer_3d_config.map(|config| &config.physical),
        ),
        locale,
    )
}

pub(super) fn build_events_text(
    state: &ViewerState,
    timeline: Option<&TimelineUiState>,
    locale: UiLocale,
) -> String {
    let focus_tick = timeline.and_then(|timeline| {
        if timeline.manual_override || timeline.drag_active {
            Some(timeline.target_tick)
        } else {
            None
        }
    });
    ui_locale_text::localize_events_summary_block(events_summary(&state.events, focus_tick), locale)
}

#[allow(dead_code)]
pub(super) fn build_selection_text(selection: &ViewerSelection, locale: UiLocale) -> String {
    selection_line(selection, locale)
}

pub(super) fn build_agent_activity_text(state: &ViewerState, locale: UiLocale) -> String {
    ui_locale_text::localize_agent_activity_block(
        agent_activity_summary(state.snapshot.as_ref(), &state.events),
        locale,
    )
}

pub(super) fn build_selection_details_text(
    selection: &ViewerSelection,
    state: &ViewerState,
    viewer_3d_config: Option<&Viewer3dConfig>,
    locale: UiLocale,
) -> String {
    let reference_radiation_area_m2 = viewer_3d_config
        .map(|config| config.physical.reference_radiation_area_m2)
        .unwrap_or(1.0);
    ui_locale_text::localize_details_block(
        selection_details_summary(
            selection,
            state.snapshot.as_ref(),
            &state.events,
            &state.decision_traces,
            reference_radiation_area_m2,
        ),
        locale,
    )
}

pub(super) fn build_provider_debug_text(
    state: &ViewerState,
    filter: ProviderDebugFilter,
) -> String {
    provider_debug_summary(&state.decision_traces, filter)
}
