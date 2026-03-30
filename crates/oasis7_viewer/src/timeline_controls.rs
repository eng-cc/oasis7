use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use oasis7::simulator::{PowerEvent, WorldEvent, WorldEventKind};
use oasis7::viewer::ViewerControl;

use crate::button_feedback::{mark_step_loading_on_control, StepControlLoadingState};
use crate::i18n::{locale_or_default, UiI18n, UiLocale};
use crate::ui_locale_text::{
    seek_button_label, timeline_insights, timeline_jump_label, timeline_mark_filter_label,
    timeline_mode_label, timeline_status_line,
};
use crate::web_test_api::{
    latest_web_test_api_control_feedback, WebTestApiControlFeedbackSnapshot,
};
use crate::{
    dispatch_viewer_control, viewer_seek_supported, ControlButton, ViewerClient,
    ViewerControlProfileState, ViewerState,
};

const DENSITY_BINS: usize = 16;
const MAX_TICK_LABELS: usize = 4;
const MAX_PEAK_TICKS: usize = 3;

#[derive(Resource, Default)]
pub(super) struct TimelineUiState {
    pub target_tick: u64,
    pub max_tick_seen: u64,
    pub manual_override: bool,
    pub drag_active: bool,
}

#[derive(Resource, Clone, Copy)]
pub(super) struct TimelineMarkFilterState {
    pub show_error: bool,
    pub show_llm: bool,
    pub show_peak: bool,
}

impl Default for TimelineMarkFilterState {
    fn default() -> Self {
        Self {
            show_error: true,
            show_llm: true,
            show_peak: true,
        }
    }
}

impl TimelineMarkFilterState {
    fn toggle(&mut self, kind: TimelineMarkKind) {
        match kind {
            TimelineMarkKind::Error => self.show_error = !self.show_error,
            TimelineMarkKind::Llm => self.show_llm = !self.show_llm,
            TimelineMarkKind::Peak => self.show_peak = !self.show_peak,
        }
    }
}

#[derive(Component)]
pub(super) struct TimelineAdjustButton {
    pub delta: i64,
}

#[derive(Component)]
pub(super) struct TimelineSeekSubmitButton;

#[derive(Component)]
pub(super) struct TimelineBar;

#[derive(Component)]
pub(super) struct TimelineBarFill;

#[derive(Component)]
pub(super) struct TimelineStatusText;

#[derive(Component)]
pub(super) struct TimelineInsightsText;

#[derive(Component)]
pub(super) struct TimelineControlFeedbackText;

#[derive(Component)]
pub(super) struct TimelineRecoveryActionsRow;

#[derive(Component)]
pub(super) struct TimelineRecoveryActionLabel {
    kind: TimelineRecoveryActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineRecoveryActionKind {
    Play,
    StepX8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineMarkKind {
    Error,
    Llm,
    Peak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimelineMarkKindPublic {
    Error,
    Llm,
    Peak,
}

#[derive(Component)]
pub(super) struct TimelineMarkJumpButton {
    kind: TimelineMarkKind,
}

#[derive(Component)]
pub(super) struct TimelineMarkFilterButton {
    kind: TimelineMarkKind,
}

#[derive(Component)]
pub(super) struct TimelineMarkFilterLabel;

#[derive(Component)]
pub(super) struct TimelineMarkJumpLabel {
    kind: TimelineMarkKind,
}

#[derive(Component)]
pub(super) struct TimelineSeekLabel;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineKeyInsights {
    error_ticks: Vec<u64>,
    llm_ticks: Vec<u64>,
    resource_peak_ticks: Vec<u64>,
    density_sparkline: String,
}

pub(super) fn spawn_timeline_controls(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    locale: UiLocale,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.13, 0.14, 0.18)),
            BorderColor::all(Color::srgb(0.24, 0.26, 0.3)),
        ))
        .with_children(|timeline| {
            timeline.spawn((
                Text::new(timeline_status_line(
                    0,
                    0,
                    0,
                    timeline_mode_label(false, false, locale),
                    locale,
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.9, 0.95)),
                TimelineStatusText,
            ));

            timeline.spawn((
                Text::new(timeline_insights(
                    0,
                    0,
                    0,
                    "-".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                    true,
                    true,
                    true,
                    "················",
                    locale,
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.8, 0.9)),
                TimelineInsightsText,
            ));

            timeline.spawn((
                Text::new(timeline_control_feedback_summary(None, locale)),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.84, 0.88)),
                TimelineControlFeedbackText,
            ));

            timeline
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(24.0),
                        column_gap: Val::Px(6.0),
                        row_gap: Val::Px(6.0),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        display: Display::None,
                        ..default()
                    },
                    TimelineRecoveryActionsRow,
                ))
                .with_children(|row| {
                    spawn_recovery_button(
                        row,
                        &font,
                        timeline_recovery_action_label(TimelineRecoveryActionKind::Play, locale),
                        TimelineRecoveryActionKind::Play,
                        ViewerControl::Play,
                    );
                    spawn_recovery_button(
                        row,
                        &font,
                        timeline_recovery_action_label(TimelineRecoveryActionKind::StepX8, locale),
                        TimelineRecoveryActionKind::StepX8,
                        ViewerControl::Step { count: 8 },
                    );
                });

            timeline
                .spawn(Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(24.0),
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|filters| {
                    spawn_mark_filter_button(filters, &font, TimelineMarkKind::Error, locale);
                    spawn_mark_filter_button(filters, &font, TimelineMarkKind::Llm, locale);
                    spawn_mark_filter_button(filters, &font, TimelineMarkKind::Peak, locale);
                });

            timeline
                .spawn(Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(24.0),
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|marks| {
                    spawn_mark_jump_button(
                        marks,
                        &font,
                        timeline_jump_label("err", locale),
                        TimelineMarkKind::Error,
                        Color::srgb(0.42, 0.2, 0.2),
                    );
                    spawn_mark_jump_button(
                        marks,
                        &font,
                        timeline_jump_label("llm", locale),
                        TimelineMarkKind::Llm,
                        Color::srgb(0.2, 0.32, 0.42),
                    );
                    spawn_mark_jump_button(
                        marks,
                        &font,
                        timeline_jump_label("peak", locale),
                        TimelineMarkKind::Peak,
                        Color::srgb(0.32, 0.28, 0.16),
                    );
                });

            timeline
                .spawn(Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(28.0),
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|buttons| {
                    spawn_adjust_button(buttons, &font, "-100", -100);
                    spawn_adjust_button(buttons, &font, "-10", -10);
                    spawn_adjust_button(buttons, &font, "-1", -1);
                    spawn_adjust_button(buttons, &font, "+1", 1);
                    spawn_adjust_button(buttons, &font, "+10", 10);
                    spawn_adjust_button(buttons, &font, "+100", 100);

                    buttons
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                height: Val::Px(24.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                min_width: Val::Px(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.18, 0.28, 0.22)),
                            TimelineSeekSubmitButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(seek_button_label(locale)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                TimelineSeekLabel,
                            ));
                        });
                });

            timeline
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(14.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.24)),
                    RelativeCursorPosition::default(),
                    TimelineBar,
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.45, 0.62, 0.95)),
                        TimelineBarFill,
                    ));
                });
        });
}

fn spawn_adjust_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    delta: i64,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(44.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.26)),
            TimelineAdjustButton { delta },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_recovery_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: TimelineRecoveryActionKind,
    control: ViewerControl,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(124.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.22, 0.14)),
            ControlButton { control },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TimelineRecoveryActionLabel { kind },
            ));
        });
}

fn spawn_mark_filter_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    kind: TimelineMarkKind,
    locale: UiLocale,
) {
    let enabled = true;
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(78.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(mark_filter_background(kind, enabled)),
            TimelineMarkFilterButton { kind },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(mark_filter_label(kind, enabled, locale)),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TimelineMarkFilterLabel,
            ));
        });
}

fn spawn_mark_jump_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: TimelineMarkKind,
    background: Color,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(88.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(background),
            TimelineMarkJumpButton { kind },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TimelineMarkJumpLabel { kind },
            ));
        });
}

pub(super) fn sync_timeline_state_from_world(
    mut timeline: ResMut<TimelineUiState>,
    state: Res<ViewerState>,
) {
    if !state.is_changed() {
        return;
    }

    let current_tick = current_tick_from_state(&state);
    timeline.max_tick_seen = timeline.max_tick_seen.max(current_tick);

    if !timeline.manual_override && !timeline.drag_active {
        timeline.target_tick = current_tick;
    }
}

pub(super) fn handle_timeline_adjust_buttons(
    mut interactions: Query<
        (&Interaction, &TimelineAdjustButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut timeline: ResMut<TimelineUiState>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        timeline.manual_override = true;
        if button.delta < 0 {
            timeline.target_tick = timeline.target_tick.saturating_sub((-button.delta) as u64);
        } else {
            timeline.target_tick = timeline.target_tick.saturating_add(button.delta as u64);
        }
    }
}

pub(super) fn handle_timeline_mark_filter_buttons(
    mut filters: ResMut<TimelineMarkFilterState>,
    mut interactions: Query<
        (&Interaction, &TimelineMarkFilterButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button) in &mut interactions {
        if *interaction == Interaction::Pressed {
            filters.toggle(button.kind);
        }
    }
}

pub(super) fn handle_timeline_mark_jump_buttons(
    state: Res<ViewerState>,
    mut timeline: ResMut<TimelineUiState>,
    mark_filters: Option<Res<TimelineMarkFilterState>>,
    mut interactions: Query<
        (&Interaction, &TimelineMarkJumpButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        timeline_mark_jump_action(
            &state,
            &mut timeline,
            mark_filters.as_deref(),
            timeline_mark_kind_public(button.kind),
        );
    }
}

pub(super) fn handle_timeline_seek_submit(
    mut interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<TimelineSeekSubmitButton>,
        ),
    >,
    client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
    mut timeline: ResMut<TimelineUiState>,
) {
    for interaction in &mut interactions {
        if *interaction == Interaction::Pressed {
            timeline_seek_action(&mut timeline, client.as_deref(), control_profile.as_deref());
        }
    }
}

pub(super) fn timeline_seek_action(
    timeline: &mut TimelineUiState,
    client: Option<&ViewerClient>,
    control_profile: Option<&ViewerControlProfileState>,
) {
    if let Some(client) = client {
        let _ = dispatch_viewer_control(
            client,
            control_profile,
            ViewerControl::Seek {
                tick: timeline.target_tick,
            },
            None,
        );
    }
    timeline.manual_override = false;
    timeline.drag_active = false;
}

pub(super) fn timeline_mark_jump_action(
    state: &ViewerState,
    timeline: &mut TimelineUiState,
    mark_filters: Option<&TimelineMarkFilterState>,
    kind: TimelineMarkKindPublic,
) {
    let axis_max = timeline_axis_max(timeline, current_tick_from_state(state));
    let insights = apply_mark_filters(
        build_timeline_key_insights(&state.events, &state.decision_traces, axis_max),
        mark_filters,
    );

    let ticks = match kind {
        TimelineMarkKindPublic::Error => insights.error_ticks.as_slice(),
        TimelineMarkKindPublic::Llm => insights.llm_ticks.as_slice(),
        TimelineMarkKindPublic::Peak => insights.resource_peak_ticks.as_slice(),
    };

    if let Some(next_tick) = select_next_mark_tick(ticks, timeline.target_tick) {
        timeline.target_tick = next_tick;
        timeline.manual_override = true;
        timeline.drag_active = false;
    }
}

pub(super) fn timeline_mark_filter_label_public(
    kind: TimelineMarkKindPublic,
    enabled: bool,
    locale: UiLocale,
) -> String {
    mark_filter_label(timeline_mark_kind_internal(kind), enabled, locale)
}

pub(super) fn timeline_axis_max_public(timeline: &TimelineUiState, current_tick: u64) -> u64 {
    timeline_axis_max(timeline, current_tick)
}

fn timeline_mark_kind_public(kind: TimelineMarkKind) -> TimelineMarkKindPublic {
    match kind {
        TimelineMarkKind::Error => TimelineMarkKindPublic::Error,
        TimelineMarkKind::Llm => TimelineMarkKindPublic::Llm,
        TimelineMarkKind::Peak => TimelineMarkKindPublic::Peak,
    }
}

fn timeline_mark_kind_internal(kind: TimelineMarkKindPublic) -> TimelineMarkKind {
    match kind {
        TimelineMarkKindPublic::Error => TimelineMarkKind::Error,
        TimelineMarkKindPublic::Llm => TimelineMarkKind::Llm,
        TimelineMarkKindPublic::Peak => TimelineMarkKind::Peak,
    }
}

pub(super) fn handle_timeline_bar_drag(
    state: Res<ViewerState>,
    mut timeline: ResMut<TimelineUiState>,
    interactions: Query<(&Interaction, &RelativeCursorPosition), With<TimelineBar>>,
) {
    let current_tick = current_tick_from_state(&state);
    for (interaction, relative) in &interactions {
        if *interaction == Interaction::Pressed {
            timeline.drag_active = true;
            timeline.manual_override = true;
            if let Some(cursor) = relative.normalized {
                let axis_max = timeline_axis_max(&timeline, current_tick);
                timeline.target_tick = normalized_x_to_tick(cursor.x, axis_max);
            }
        } else if timeline.drag_active {
            timeline.drag_active = false;
        }
    }
}

pub(super) fn update_timeline_ui(
    state: Res<ViewerState>,
    timeline: Res<TimelineUiState>,
    control_profile: Option<Res<ViewerControlProfileState>>,
    i18n: Option<Res<UiI18n>>,
    mark_filters: Option<Res<TimelineMarkFilterState>>,
    mut last_feedback_digest: Local<Option<String>>,
    mut queries: ParamSet<(
        Query<&mut Text, With<TimelineStatusText>>,
        Query<&mut Text, With<TimelineInsightsText>>,
        Query<&mut Node, With<TimelineBarFill>>,
        Query<(&TimelineMarkJumpLabel, &mut Text)>,
        Query<&mut Text, With<TimelineSeekLabel>>,
        Query<&mut Text, With<TimelineControlFeedbackText>>,
        Query<
            (
                &mut Node,
                Has<TimelineRecoveryActionsRow>,
                Has<TimelineSeekSubmitButton>,
            ),
            Or<(
                With<TimelineRecoveryActionsRow>,
                With<TimelineSeekSubmitButton>,
            )>,
        >,
        Query<(&TimelineRecoveryActionLabel, &mut Text)>,
    )>,
) {
    let control_feedback = latest_web_test_api_control_feedback();
    let feedback_digest = control_feedback_digest(control_feedback.as_ref());
    let feedback_changed = *last_feedback_digest != feedback_digest;

    let filter_changed = mark_filters
        .as_ref()
        .map(|filters| filters.is_changed())
        .unwrap_or(false);
    let locale_changed = i18n
        .as_ref()
        .map(|value| value.is_changed())
        .unwrap_or(false);
    if !state.is_changed()
        && !timeline.is_changed()
        && !filter_changed
        && !locale_changed
        && !feedback_changed
    {
        return;
    }
    *last_feedback_digest = feedback_digest;

    let locale = locale_or_default(i18n.as_deref());
    let seek_supported = viewer_seek_supported(control_profile.as_deref());

    let current_tick = current_tick_from_state(&state);
    let axis_max = timeline_axis_max(&timeline, current_tick);
    let mode_label = timeline_mode_label(timeline.drag_active, timeline.manual_override, locale);

    if let Ok(mut text) = queries.p0().single_mut() {
        text.0 = timeline_status_line(
            current_tick,
            timeline.target_tick,
            axis_max,
            mode_label,
            locale,
        );
    }

    if let Ok(mut text) = queries.p1().single_mut() {
        let filters = mark_filters.as_ref().map(|filters| filters.as_ref());
        let key = apply_mark_filters(
            build_timeline_key_insights(&state.events, &state.decision_traces, axis_max),
            filters,
        );
        let filter_state = filters.copied().unwrap_or_default();
        text.0 = timeline_insights(
            key.error_ticks.len(),
            key.llm_ticks.len(),
            key.resource_peak_ticks.len(),
            format_tick_list(&key.error_ticks, MAX_TICK_LABELS),
            format_tick_list(&key.llm_ticks, MAX_TICK_LABELS),
            format_tick_list(&key.resource_peak_ticks, MAX_TICK_LABELS),
            filter_state.show_error,
            filter_state.show_llm,
            filter_state.show_peak,
            &key.density_sparkline,
            locale,
        );
    }

    let progress = if axis_max == 0 {
        0.0
    } else {
        ((timeline.target_tick as f32) / (axis_max as f32) * 100.0).clamp(0.0, 100.0)
    };

    for mut fill in &mut queries.p2() {
        fill.width = Val::Percent(progress);
    }

    for (jump, mut text) in &mut queries.p3() {
        text.0 = match jump.kind {
            TimelineMarkKind::Error => timeline_jump_label("err", locale).to_string(),
            TimelineMarkKind::Llm => timeline_jump_label("llm", locale).to_string(),
            TimelineMarkKind::Peak => timeline_jump_label("peak", locale).to_string(),
        };
    }

    for mut text in &mut queries.p4() {
        text.0 = seek_button_label(locale).to_string();
    }

    if let Ok(mut text) = queries.p5().single_mut() {
        text.0 = timeline_control_feedback_summary(control_feedback.as_ref(), locale);
    }

    let show_recovery = timeline_should_show_recovery_actions(control_feedback.as_ref());
    for (mut node, is_recovery_row, is_seek_button) in &mut queries.p6() {
        if is_recovery_row {
            node.display = if show_recovery {
                Display::Flex
            } else {
                Display::None
            };
        }
        if is_seek_button {
            node.display = if seek_supported {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    for (label, mut text) in &mut queries.p7() {
        text.0 = timeline_recovery_action_label(label.kind, locale).to_string();
    }
}

fn control_feedback_digest(feedback: Option<&WebTestApiControlFeedbackSnapshot>) -> Option<String> {
    feedback.map(|feedback| {
        format!(
            "{}|{}|{:?}|{:?}|{}|{}|{}|{}",
            feedback.action,
            feedback.stage,
            feedback.reason,
            feedback.hint,
            feedback.effect,
            feedback.delta_logical_time,
            feedback.delta_event_seq,
            feedback.delta_trace_count
        )
    })
}

fn timeline_control_feedback_summary(
    feedback: Option<&WebTestApiControlFeedbackSnapshot>,
    locale: UiLocale,
) -> String {
    let Some(feedback) = feedback else {
        return if locale.is_zh() {
            "控制反馈: 无（发生阻塞时会在此给出恢复建议）".to_string()
        } else {
            "Control feedback: none (recovery hints will appear here when blocked)".to_string()
        };
    };

    let mut summary = if locale.is_zh() {
        format!(
            "控制反馈: {} · {} | 增量 tick +{} event +{} trace +{} | {}",
            feedback.action,
            timeline_control_stage_label(feedback.stage.as_str(), locale),
            feedback.delta_logical_time,
            feedback.delta_event_seq,
            feedback.delta_trace_count,
            feedback.effect,
        )
    } else {
        format!(
            "Control: {} · {} | delta tick +{} event +{} trace +{} | {}",
            feedback.action,
            timeline_control_stage_label(feedback.stage.as_str(), locale),
            feedback.delta_logical_time,
            feedback.delta_event_seq,
            feedback.delta_trace_count,
            feedback.effect,
        )
    };

    if let Some(reason) = feedback.reason.as_deref() {
        summary.push_str(" | ");
        summary.push_str(reason);
    }
    if let Some(hint) = feedback.hint.as_deref() {
        summary.push_str(" | ");
        summary.push_str(hint);
    }
    summary
}

fn timeline_should_show_recovery_actions(
    feedback: Option<&WebTestApiControlFeedbackSnapshot>,
) -> bool {
    feedback.is_some_and(|feedback| timeline_stage_shows_recovery_actions(feedback.stage.as_str()))
}

fn timeline_control_stage_label(stage: &str, locale: UiLocale) -> &'static str {
    match (stage, locale.is_zh()) {
        ("received", true) => "已接收",
        ("received", false) => "Received",
        ("executing", true) => "执行中",
        ("executing", false) => "Executing",
        ("completed_advanced", true) | ("applied", true) => "已完成（有推进）",
        ("completed_advanced", false) | ("applied", false) => "Completed (advanced)",
        ("completed_no_progress", true) => "已完成（无推进）",
        ("completed_no_progress", false) => "Completed (no progress)",
        ("blocked", true) => "已阻断",
        ("blocked", false) => "Blocked",
        (_, true) => "处理中",
        (_, false) => "Pending",
    }
}

fn timeline_stage_shows_recovery_actions(stage: &str) -> bool {
    matches!(stage, "completed_no_progress")
}

fn timeline_recovery_action_label(
    kind: TimelineRecoveryActionKind,
    locale: UiLocale,
) -> &'static str {
    match (kind, locale.is_zh()) {
        (TimelineRecoveryActionKind::Play, true) => "恢复：play",
        (TimelineRecoveryActionKind::Play, false) => "Recover: play",
        (TimelineRecoveryActionKind::StepX8, true) => "重试：step x8",
        (TimelineRecoveryActionKind::StepX8, false) => "Retry: step x8",
    }
}

pub(super) fn normalized_x_to_tick(normalized_x: f32, axis_max: u64) -> u64 {
    if axis_max == 0 {
        return 0;
    }
    let ratio = (normalized_x + 0.5).clamp(0.0, 1.0);
    (ratio * axis_max as f32).round() as u64
}

fn current_tick_from_state(state: &ViewerState) -> u64 {
    state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0)
}

fn timeline_axis_max(timeline: &TimelineUiState, current_tick: u64) -> u64 {
    timeline
        .max_tick_seen
        .max(current_tick)
        .max(timeline.target_tick)
}

fn select_next_mark_tick(ticks: &[u64], current_target: u64) -> Option<u64> {
    ticks
        .iter()
        .copied()
        .find(|tick| *tick > current_target)
        .or_else(|| ticks.first().copied())
}

fn apply_mark_filters(
    mut insights: TimelineKeyInsights,
    filters: Option<&TimelineMarkFilterState>,
) -> TimelineKeyInsights {
    let filters = filters.copied().unwrap_or_default();
    if !filters.show_error {
        insights.error_ticks.clear();
    }
    if !filters.show_llm {
        insights.llm_ticks.clear();
    }
    if !filters.show_peak {
        insights.resource_peak_ticks.clear();
    }
    insights
}

fn mark_filter_background(kind: TimelineMarkKind, enabled: bool) -> Color {
    if !enabled {
        return Color::srgb(0.16, 0.16, 0.18);
    }
    match kind {
        TimelineMarkKind::Error => Color::srgb(0.52, 0.22, 0.22),
        TimelineMarkKind::Llm => Color::srgb(0.22, 0.4, 0.52),
        TimelineMarkKind::Peak => Color::srgb(0.42, 0.36, 0.18),
    }
}

fn mark_filter_label(kind: TimelineMarkKind, enabled: bool, locale: UiLocale) -> String {
    let key = match kind {
        TimelineMarkKind::Error => "err",
        TimelineMarkKind::Llm => "llm",
        TimelineMarkKind::Peak => "peak",
    };
    timeline_mark_filter_label(key, enabled, locale)
}

fn build_timeline_key_insights(
    events: &[WorldEvent],
    decision_traces: &[oasis7::simulator::AgentDecisionTrace],
    axis_max: u64,
) -> TimelineKeyInsights {
    let error_ticks = collect_error_ticks(events);
    let llm_ticks = collect_llm_ticks(decision_traces);
    let resource_peak_ticks = collect_resource_peak_ticks(events, MAX_PEAK_TICKS);
    let density_sparkline = event_density_sparkline(events, axis_max, DENSITY_BINS);
    TimelineKeyInsights {
        error_ticks,
        llm_ticks,
        resource_peak_ticks,
        density_sparkline,
    }
}

fn collect_error_ticks(events: &[WorldEvent]) -> Vec<u64> {
    let mut ticks = Vec::new();
    for event in events {
        if matches!(event.kind, WorldEventKind::ActionRejected { .. }) {
            ticks.push(event.time);
        }
    }
    dedup_sorted_ticks(ticks)
}

fn collect_llm_ticks(decision_traces: &[oasis7::simulator::AgentDecisionTrace]) -> Vec<u64> {
    let ticks: Vec<u64> = decision_traces.iter().map(|trace| trace.time).collect();
    dedup_sorted_ticks(ticks)
}

fn collect_resource_peak_ticks(events: &[WorldEvent], max_ticks: usize) -> Vec<u64> {
    let mut weighted_ticks: Vec<(i64, u64)> = events
        .iter()
        .filter_map(|event| event_resource_weight(event).map(|weight| (weight, event.time)))
        .collect();
    weighted_ticks.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));

    let mut selected = Vec::new();
    for (_, tick) in weighted_ticks {
        if !selected.contains(&tick) {
            selected.push(tick);
            if selected.len() >= max_ticks {
                break;
            }
        }
    }
    selected.sort_unstable();
    selected
}

fn event_resource_weight(event: &WorldEvent) -> Option<i64> {
    match &event.kind {
        WorldEventKind::ResourceTransferred { amount, .. } => Some(amount.abs()),
        WorldEventKind::RadiationHarvested { amount, .. } => Some(amount.abs()),
        WorldEventKind::CompoundRefined {
            electricity_cost,
            hardware_output,
            ..
        } => Some(electricity_cost.abs().saturating_add(hardware_output.abs())),
        WorldEventKind::Power(power_event) => power_event_weight(power_event),
        _ => None,
    }
}

fn power_event_weight(power_event: &PowerEvent) -> Option<i64> {
    match power_event {
        PowerEvent::PowerGenerated { amount, .. } => Some(amount.abs()),
        PowerEvent::PowerConsumed { amount, .. } => Some(amount.abs()),
        PowerEvent::PowerTransferred { amount, loss, .. } => {
            Some(amount.abs().saturating_add(loss.abs()))
        }
        PowerEvent::PowerCharged { amount, .. } => Some(amount.abs()),
        _ => None,
    }
}

fn event_density_sparkline(events: &[WorldEvent], axis_max: u64, bins: usize) -> String {
    if bins == 0 {
        return String::new();
    }

    let mut counts = vec![0_u32; bins];
    for event in events {
        let idx = tick_to_bin(event.time.min(axis_max), axis_max, bins);
        counts[idx] = counts[idx].saturating_add(1);
    }

    let max_count = counts.iter().copied().max().unwrap_or(0);
    if max_count == 0 {
        return "·".repeat(bins);
    }

    const LEVELS: [char; 9] = ['·', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    counts
        .iter()
        .map(|count| {
            if *count == 0 {
                return LEVELS[0];
            }
            let scaled = ((*count as f32 / max_count as f32) * 8.0).ceil() as usize;
            LEVELS[scaled.clamp(1, 8)]
        })
        .collect()
}

fn tick_to_bin(tick: u64, axis_max: u64, bins: usize) -> usize {
    if axis_max == 0 || bins <= 1 {
        return 0;
    }
    let ratio = (tick as f32 / axis_max as f32).clamp(0.0, 1.0);
    (ratio * (bins.saturating_sub(1)) as f32).round() as usize
}

fn dedup_sorted_ticks(mut ticks: Vec<u64>) -> Vec<u64> {
    ticks.sort_unstable();
    ticks.dedup();
    ticks
}

fn format_tick_list(ticks: &[u64], max_items: usize) -> String {
    if ticks.is_empty() {
        return "-".to_string();
    }
    let shown: Vec<String> = ticks
        .iter()
        .take(max_items)
        .map(|tick| tick.to_string())
        .collect();
    if ticks.len() > max_items {
        format!("{}+{}", shown.join(","), ticks.len() - max_items)
    } else {
        shown.join(",")
    }
}

pub(super) fn handle_control_buttons(
    mut interactions: Query<(&Interaction, &ControlButton), (Changed<Interaction>, With<Button>)>,
    client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
    state: Res<ViewerState>,
    mut loading: ResMut<StepControlLoadingState>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if matches!(button.control, ViewerControl::Step { .. }) && loading.pending {
            continue;
        }

        mark_step_loading_on_control(&button.control, &state, &mut loading);
        if let Some(client) = client.as_deref() {
            let _ = dispatch_viewer_control(
                client,
                control_profile.as_deref(),
                button.control.clone(),
                None,
            );
        }
    }
}

#[cfg(test)]
#[path = "timeline_controls_tests.rs"]
mod tests;
