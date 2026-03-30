use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::app_bootstrap::ThemeRuntimeState;
use crate::button_feedback::StepControlLoadingState;
use crate::copyable_text::{copy_panel_hint, copy_panel_title, ensure_egui_cjk_font};
use crate::event_click_list::{
    apply_event_click_action, event_row_label, event_window, focus_tick,
};
use crate::i18n::{
    camera_mode_button_label, camera_mode_section_label, copyable_panel_toggle_label,
    experience_mode_label, language_toggle_label, locale_or_default, module_switches_title,
    right_panel_toggle_label, top_controls_label, top_panel_toggle_label, UiI18n,
};
use crate::industry_graph_view_model::{IndustrySemanticZoomLevel, IndustrySemanticZoomState};
use crate::right_panel_module_visibility::RightPanelModuleVisibilityState;
use crate::selection_linking::{
    jump_selection_events_action, locate_focus_event_action, quick_locate_agent_action,
    selection_kind_label,
};
use crate::timeline_controls::{
    timeline_axis_max_public, timeline_mark_filter_label_public, timeline_mark_jump_action,
    timeline_seek_action, TimelineMarkKindPublic, TimelineUiState,
};
use crate::ui_locale_text::{
    localize_agent_activity_block, localize_details_block, localize_economy_dashboard_block,
    localize_events_summary_block, localize_industrial_ops_block, localize_ops_navigation_block,
    localize_world_summary_block, map_link_message_for_locale, overlay_button_label,
    overlay_chunk_legend_label, overlay_chunk_legend_title, overlay_grid_line_width_hint,
    overlay_loading, seek_button_label, status_line, timeline_insights, timeline_jump_label,
    timeline_mode_label, timeline_status_line,
};
use crate::ui_text::{
    agent_activity_summary, build_industry_graph_view_model, economy_dashboard_summary_with_zoom,
    events_summary, industrial_ops_summary_with_zoom, ops_navigation_alert_summary_with_zoom,
    provider_debug_summary, selection_details_summary, world_summary, ProviderDebugFilter,
};
use crate::world_overlay::overlay_status_text_public;
use crate::{
    grid_line_thickness, viewer_seek_supported, CopyableTextPanelState, DiagnosisState,
    EventObjectLinkState, GridLineKind, RenderPerfSummary, RightPanelLayoutState,
    RightPanelWidthState, TimelineMarkFilterState, Viewer3dConfig, ViewerCameraMode, ViewerClient,
    ViewerControlProfileState, ViewerExperienceMode, ViewerSelection, ViewerState,
    WorldOverlayConfig,
};

#[path = "egui_observe_section_card.rs"]
mod egui_observe_section_card;
#[path = "egui_right_panel_chat.rs"]
mod egui_right_panel_chat;
#[path = "egui_right_panel_controls.rs"]
mod egui_right_panel_controls;
#[path = "egui_right_panel_env.rs"]
mod egui_right_panel_env;
#[path = "egui_right_panel_layout.rs"]
mod egui_right_panel_layout;
#[path = "egui_right_panel_player_card_motion.rs"]
mod egui_right_panel_player_card_motion;
#[path = "egui_right_panel_player_entry.rs"]
mod egui_right_panel_player_entry;
#[path = "egui_right_panel_player_experience.rs"]
mod egui_right_panel_player_experience;
#[path = "egui_right_panel_player_guide/mod.rs"]
mod egui_right_panel_player_guide;
#[path = "egui_right_panel_player_micro_loop.rs"]
mod egui_right_panel_player_micro_loop;
#[path = "egui_right_panel_text_sections.rs"]
mod egui_right_panel_text_sections;
#[path = "egui_right_panel_text_utils.rs"]
mod egui_right_panel_text_utils;
#[path = "egui_right_panel_theme_runtime.rs"]
mod egui_right_panel_theme_runtime;

use egui_observe_section_card::render_observe_section_card;
use egui_right_panel_chat::{render_chat_section, AgentChatDraftState};
#[cfg(test)]
use egui_right_panel_controls::send_control_request;
use egui_right_panel_controls::{
    render_control_buttons, render_module_toggle_button, ControlPanelUiState,
};
#[cfg(test)]
use egui_right_panel_env::env_toggle_enabled;
use egui_right_panel_env::{
    is_ops_nav_panel_enabled, is_product_style_enabled, is_product_style_motion_enabled,
};
use egui_right_panel_layout::{
    adaptive_chat_panel_default_width, adaptive_chat_panel_max_width_for_side_layout,
    adaptive_main_panel_max_width_for_layout, adaptive_main_panel_min_width,
    adaptive_panel_default_width, is_compact_chat_layout, panel_toggle_shortcut_pressed,
    player_chat_panel_max_width_for_side_layout, player_main_panel_max_width_for_layout,
    should_show_chat_panel, total_right_panel_width,
};
#[cfg(test)]
use egui_right_panel_layout::{adaptive_chat_panel_max_width, adaptive_panel_max_width};
use egui_right_panel_player_entry::render_hidden_panel_entry;
#[cfg(test)]
use egui_right_panel_player_experience::build_player_hud_snapshot;
#[cfg(test)]
use egui_right_panel_player_experience::{
    dismiss_player_onboarding_step, feedback_action_feedback_seen, feedback_last_seen_event_id,
    feedback_toast_cap, feedback_toast_detail, feedback_toast_ids, feedback_toast_len,
    feedback_toast_snapshot, feedback_tone_for_event, player_achievement_is_unlocked,
    player_achievement_popup_cap, player_achievement_popup_len,
    player_achievement_popup_milestones, push_feedback_toast, should_show_player_goal_hint,
    should_show_player_onboarding_card, FeedbackTone, PlayerAchievementMilestone, PlayerGuideStep,
};
use egui_right_panel_player_experience::{
    player_action_feedback_seen, render_feedback_toasts, render_player_experience_layers,
    sync_feedback_toasts, FeedbackToastState, PlayerAchievementState, PlayerOnboardingState,
};
use egui_right_panel_player_guide::{apply_player_layout_preset, PlayerLayoutPreset};
use egui_right_panel_text_sections::render_text_sections;
use egui_right_panel_text_utils::{rejection_event_count, truncate_observe_text};
use egui_right_panel_theme_runtime::render_theme_runtime_section;

const MAIN_PANEL_DEFAULT_WIDTH: f32 = 320.0;
const MAIN_PANEL_MIN_WIDTH: f32 = 240.0;
const MAIN_PANEL_COMPACT_MIN_WIDTH: f32 = 160.0;
const MAIN_PANEL_MAX_WIDTH_RATIO: f32 = 0.6;
const CHAT_PANEL_DEFAULT_WIDTH: f32 = 360.0;
const CHAT_PANEL_MIN_WIDTH: f32 = 280.0;
const CHAT_PANEL_MAX_WIDTH_RATIO: f32 = 0.65;
const MIN_INTERACTION_VIEWPORT_WIDTH: f32 = 240.0;
const CHAT_SIDE_PANEL_COMPACT_BREAKPOINT: f32 =
    MAIN_PANEL_MIN_WIDTH + CHAT_PANEL_MIN_WIDTH + MIN_INTERACTION_VIEWPORT_WIDTH;
const EVENT_ROW_LIMIT: usize = 10;
const MAX_TICK_LABELS: usize = 4;
const EVENT_ROW_LABEL_MAX_CHARS: usize = 72;
const OPS_NAV_PANEL_ENV: &str = "OASIS7_VIEWER_SHOW_OPS_NAV";
const PRODUCT_STYLE_ENV: &str = "OASIS7_VIEWER_PRODUCT_STYLE";
const PRODUCT_STYLE_MOTION_ENV: &str = "OASIS7_VIEWER_PRODUCT_STYLE_MOTION";
const PANEL_ENTRY_CARD_MAX_WIDTH: f32 = 280.0;

#[derive(SystemParam)]
pub(super) struct RightPanelParams<'w, 's> {
    panel_width: ResMut<'w, RightPanelWidthState>,
    layout_state: ResMut<'w, RightPanelLayoutState>,
    experience_mode: Res<'w, ViewerExperienceMode>,
    camera_mode: ResMut<'w, ViewerCameraMode>,
    i18n: Option<ResMut<'w, UiI18n>>,
    copyable_panel_state: ResMut<'w, CopyableTextPanelState>,
    module_visibility: ResMut<'w, RightPanelModuleVisibilityState>,
    overlay_config: ResMut<'w, WorldOverlayConfig>,
    industry_zoom: ResMut<'w, IndustrySemanticZoomState>,
    state: Res<'w, ViewerState>,
    selection: ResMut<'w, ViewerSelection>,
    render_perf: Option<Res<'w, RenderPerfSummary>>,
    viewer_3d_config: Option<Res<'w, Viewer3dConfig>>,
    viewer_3d_assets: Option<Res<'w, crate::Viewer3dAssets>>,
    font_assets: Res<'w, Assets<Font>>,
    loading: ResMut<'w, StepControlLoadingState>,
    client: Option<Res<'w, ViewerClient>>,
    control_profile: Option<Res<'w, ViewerControlProfileState>>,
    chat_focus_signal: ResMut<'w, crate::ChatInputFocusSignal>,
    timeline: ResMut<'w, TimelineUiState>,
    timeline_filters: ResMut<'w, TimelineMarkFilterState>,
    theme_runtime: ResMut<'w, ThemeRuntimeState>,
    diagnosis_state: Res<'w, DiagnosisState>,
    link_state: ResMut<'w, EventObjectLinkState>,
    scene: Res<'w, crate::Viewer3dScene>,
    transforms: Query<'w, 's, (&'static mut Transform, Option<&'static crate::BaseScale>)>,
}

pub(super) fn render_right_side_panel_egui(
    mut contexts: EguiContexts,
    mut cjk_font_initialized: Local<bool>,
    mut chat_draft: Local<AgentChatDraftState>,
    mut control_panel: Local<ControlPanelUiState>,
    mut feedback_toast_state: Local<FeedbackToastState>,
    mut player_achievement_state: Local<PlayerAchievementState>,
    mut onboarding_state: Local<PlayerOnboardingState>,
    mut player_layout_initialized: Local<bool>,
    mut module_switches_expanded: Local<bool>,
    mut provider_debug_filter: Local<ProviderDebugFilter>,
    params: RightPanelParams,
) {
    let RightPanelParams {
        mut panel_width,
        mut layout_state,
        experience_mode,
        mut camera_mode,
        mut i18n,
        mut copyable_panel_state,
        mut module_visibility,
        mut overlay_config,
        mut industry_zoom,
        state,
        mut selection,
        render_perf,
        viewer_3d_config,
        viewer_3d_assets,
        font_assets,
        mut loading,
        client,
        control_profile,
        mut chat_focus_signal,
        mut timeline,
        mut timeline_filters,
        mut theme_runtime,
        diagnosis_state,
        mut link_state,
        scene,
        mut transforms,
    } = params;

    let locale = locale_or_default(i18n.as_deref());

    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    if let Some(viewer_3d_assets) = viewer_3d_assets.as_deref() {
        ensure_egui_cjk_font(
            context,
            &mut cjk_font_initialized,
            &font_assets,
            &viewer_3d_assets.label_font,
        );
    }
    let now_secs = context.input(|input| input.time);
    let player_mode_enabled = *experience_mode == ViewerExperienceMode::Player;
    if player_mode_enabled && !*player_layout_initialized {
        apply_player_layout_preset(
            layout_state.as_mut(),
            module_visibility.as_mut(),
            PlayerLayoutPreset::Mission,
        );
        copyable_panel_state.visible = module_visibility.show_details;
        *module_switches_expanded = false;
        *player_layout_initialized = true;
    } else if !player_mode_enabled {
        *player_layout_initialized = false;
        *module_switches_expanded = true;
    }
    if player_mode_enabled {
        sync_feedback_toasts(&mut feedback_toast_state, &state, now_secs, locale);
    }

    if panel_toggle_shortcut_pressed(context) {
        layout_state.panel_hidden = !layout_state.panel_hidden;
    }

    if copyable_panel_state.visible != module_visibility.show_details {
        copyable_panel_state.visible = module_visibility.show_details;
    }
    chat_focus_signal.wants_ime_focus = false;
    if player_mode_enabled {
        render_player_experience_layers(
            context,
            &state,
            &selection,
            client.as_deref(),
            control_profile.as_deref(),
            layout_state.as_mut(),
            module_visibility.as_mut(),
            &mut onboarding_state,
            &mut player_achievement_state,
            player_action_feedback_seen(&feedback_toast_state),
            locale,
            now_secs,
        );
    }

    if layout_state.panel_hidden {
        panel_width.width_px = 0.0;
        render_hidden_panel_entry(
            context,
            *experience_mode,
            locale,
            now_secs,
            layout_state.as_mut(),
            module_visibility.as_mut(),
        );
        if layout_state.panel_hidden {
            if player_mode_enabled {
                render_feedback_toasts(context, &feedback_toast_state, now_secs);
            }
            return;
        }
    }

    let available_width = context.available_rect().width();
    let show_chat_panel_requested =
        should_show_chat_panel(layout_state.as_ref(), module_visibility.show_chat);
    let compact_chat_layout = is_compact_chat_layout(available_width);
    let chat_max_width = if player_mode_enabled {
        player_chat_panel_max_width_for_side_layout(available_width)
    } else {
        adaptive_chat_panel_max_width_for_side_layout(available_width)
    };
    let show_chat_side_panel =
        show_chat_panel_requested && !compact_chat_layout && chat_max_width >= CHAT_PANEL_MIN_WIDTH;
    let chat_panel_width = if show_chat_side_panel {
        let default_chat_width = adaptive_chat_panel_default_width(available_width);
        let chat_response = egui::SidePanel::right("viewer-chat-side-panel")
            .resizable(true)
            .default_width(default_chat_width.min(chat_max_width))
            .width_range(CHAT_PANEL_MIN_WIDTH..=chat_max_width)
            .show(context, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.heading(if locale.is_zh() { "对话" } else { "Chat" });
                chat_focus_signal.wants_ime_focus =
                    render_chat_section(ui, locale, &state, client.as_deref(), &mut chat_draft);
            });
        chat_response.response.rect.width()
    } else {
        0.0
    };

    let panel_min_width = adaptive_main_panel_min_width(available_width);
    let default_panel_width = adaptive_panel_default_width(available_width).max(panel_min_width);
    let panel_max_width = if player_mode_enabled {
        player_main_panel_max_width_for_layout(available_width, chat_panel_width)
    } else {
        adaptive_main_panel_max_width_for_layout(available_width, chat_panel_width)
            .max(panel_min_width)
    };
    let mut hide_panel_requested = false;
    let panel_response = egui::SidePanel::right("viewer-right-side-panel")
        .resizable(true)
        .default_width(default_panel_width)
        .width_range(panel_min_width..=panel_max_width)
        .show(context, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            ui.horizontal_wrapped(|ui| {
                if ui.button(right_panel_toggle_label(true, locale)).clicked() {
                    hide_panel_requested = true;
                }

                if ui
                    .button(top_panel_toggle_label(
                        layout_state.top_panel_collapsed,
                        locale,
                    ))
                    .clicked()
                {
                    layout_state.top_panel_collapsed = !layout_state.top_panel_collapsed;
                }

                if ui.button(language_toggle_label(locale)).clicked() {
                    if let Some(i18n) = i18n.as_deref_mut() {
                        i18n.locale = i18n.locale.toggled();
                    }
                }

                ui.label(experience_mode_label(*experience_mode, locale));

                ui.separator();
                ui.label(camera_mode_section_label(locale));

                let is_two_d = *camera_mode == ViewerCameraMode::TwoD;
                if ui
                    .selectable_label(
                        is_two_d,
                        camera_mode_button_label(ViewerCameraMode::TwoD, locale),
                    )
                    .clicked()
                {
                    *camera_mode = ViewerCameraMode::TwoD;
                }

                if ui
                    .selectable_label(
                        !is_two_d,
                        camera_mode_button_label(ViewerCameraMode::ThreeD, locale),
                    )
                    .clicked()
                {
                    *camera_mode = ViewerCameraMode::ThreeD;
                }

                if ui
                    .button(copyable_panel_toggle_label(
                        copyable_panel_state.visible,
                        locale,
                    ))
                    .clicked()
                {
                    copyable_panel_state.visible = !copyable_panel_state.visible;
                    module_visibility.show_details = copyable_panel_state.visible;
                }

                ui.label(top_controls_label(locale));
            });

            if layout_state.top_panel_collapsed {
                return;
            }

            ui.separator();
            if player_mode_enabled {
                if ui
                    .button(if *module_switches_expanded {
                        if locale.is_zh() {
                            "收起模块开关"
                        } else {
                            "Hide Module Switches"
                        }
                    } else if locale.is_zh() {
                        "展开模块开关"
                    } else {
                        "Show Module Switches"
                    })
                    .clicked()
                {
                    *module_switches_expanded = !*module_switches_expanded;
                }
            }
            if !player_mode_enabled || *module_switches_expanded {
                ui.strong(module_switches_title(locale));
                ui.horizontal_wrapped(|ui| {
                    render_module_toggle_button(
                        ui,
                        "controls",
                        &mut module_visibility.show_controls,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "overview",
                        &mut module_visibility.show_overview,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "chat",
                        &mut module_visibility.show_chat,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "overlay",
                        &mut module_visibility.show_overlay,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "diagnosis",
                        &mut module_visibility.show_diagnosis,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "event_link",
                        &mut module_visibility.show_event_link,
                        locale,
                    );
                    render_module_toggle_button(
                        ui,
                        "timeline",
                        &mut module_visibility.show_timeline,
                        locale,
                    );

                    let mut details_visible = module_visibility.show_details;
                    render_module_toggle_button(ui, "details", &mut details_visible, locale);
                    module_visibility.show_details = details_visible;
                    copyable_panel_state.visible = details_visible;
                });
            }

            if module_visibility.show_controls {
                ui.separator();
                render_control_buttons(
                    ui,
                    locale,
                    player_mode_enabled,
                    &state,
                    loading.as_mut(),
                    &mut control_panel,
                    client.as_deref(),
                    control_profile.as_deref(),
                );
                render_theme_runtime_section(ui, locale, theme_runtime.as_mut());
            }

            if module_visibility.show_chat && !show_chat_side_panel {
                ui.separator();
                if compact_chat_layout {
                    ui.label(if locale.is_zh() {
                        "窄屏模式：对话已内联到主面板"
                    } else {
                        "Compact mode: chat is embedded in main panel"
                    });
                }
                ui.heading(if locale.is_zh() { "对话" } else { "Chat" });
                chat_focus_signal.wants_ime_focus =
                    render_chat_section(ui, locale, &state, client.as_deref(), &mut chat_draft);
            }

            if module_visibility.show_overview {
                ui.separator();
                render_overview_section(
                    ui,
                    locale,
                    &state,
                    &selection,
                    timeline.as_ref(),
                    render_perf.as_deref(),
                );
            }

            if module_visibility.show_overlay {
                ui.separator();
                render_overlay_section(
                    ui,
                    locale,
                    *camera_mode,
                    &state,
                    &viewer_3d_config,
                    overlay_config.as_mut(),
                    industry_zoom.as_mut(),
                );
            }

            if module_visibility.show_diagnosis {
                ui.separator();
                ui.strong(if locale.is_zh() {
                    "诊断"
                } else {
                    "Diagnosis"
                });
                ui.add(
                    egui::Label::new(diagnosis_state.text.as_str())
                        .wrap()
                        .selectable(true),
                );
            }

            if module_visibility.show_event_link {
                ui.separator();
                ui.strong(if locale.is_zh() {
                    "事件联动"
                } else {
                    "Event Link"
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(crate::ui_locale_text::quick_locate_agent_label(locale))
                        .clicked()
                    {
                        if let Some(config) = viewer_3d_config.as_deref() {
                            quick_locate_agent_action(
                                &scene,
                                config,
                                selection.as_mut(),
                                link_state.as_mut(),
                                &mut transforms,
                            );
                        } else {
                            link_state.message = "Link: viewer config unavailable".to_string();
                        }
                    }

                    if ui
                        .button(crate::ui_locale_text::locate_focus_label(locale))
                        .clicked()
                    {
                        if let Some(config) = viewer_3d_config.as_deref() {
                            locate_focus_event_action(
                                &state,
                                &scene,
                                config,
                                selection.as_mut(),
                                link_state.as_mut(),
                                &mut transforms,
                                Some(timeline.as_mut()),
                            );
                        } else {
                            link_state.message = "Link: viewer config unavailable".to_string();
                        }
                    }

                    if ui
                        .button(crate::ui_locale_text::jump_selection_label(locale))
                        .clicked()
                    {
                        jump_selection_events_action(
                            &state,
                            &selection,
                            link_state.as_mut(),
                            Some(timeline.as_mut()),
                        );
                    }
                });
                ui.add(
                    egui::Label::new(map_link_message_for_locale(&link_state.message, locale))
                        .wrap()
                        .selectable(true),
                );
            }

            if module_visibility.show_timeline {
                ui.separator();
                ui.strong(if locale.is_zh() {
                    "时间轴"
                } else {
                    "Timeline"
                });
                render_timeline_section(
                    ui,
                    locale,
                    &state,
                    timeline.as_mut(),
                    timeline_filters.as_mut(),
                    client.as_deref(),
                    control_profile.as_deref(),
                );
            }

            if module_visibility.show_details {
                ui.separator();
                ui.heading(copy_panel_title(locale));
                ui.add(egui::Label::new(copy_panel_hint(locale)).wrap());

                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_text_sections(
                        ui,
                        locale,
                        &state,
                        &selection,
                        timeline.as_ref(),
                        &viewer_3d_config,
                        industry_zoom.level,
                        &mut *provider_debug_filter,
                    );

                    ui.separator();
                    ui.strong(if locale.is_zh() {
                        "事件行"
                    } else {
                        "Event Rows"
                    });

                    let focus = focus_tick(&state, Some(timeline.as_ref()));
                    let (rows, focused_event_id) =
                        event_window(&state.events, focus, EVENT_ROW_LIMIT);

                    if rows.is_empty() {
                        ui.label(crate::ui_locale_text::event_links_empty(locale));
                        return;
                    }

                    for event in rows {
                        let line =
                            event_row_label(event, focused_event_id == Some(event.id), locale);
                        let line_preview = truncate_observe_text(&line, EVENT_ROW_LABEL_MAX_CHARS);
                        let mut response = ui.add(egui::Button::new(line_preview.as_str()));
                        if line_preview != line {
                            response = response.on_hover_text(line.as_str());
                        }
                        if response.clicked() {
                            if let Some(config) = viewer_3d_config.as_deref() {
                                apply_event_click_action(
                                    event.id,
                                    &state,
                                    &scene,
                                    config,
                                    selection.as_mut(),
                                    &mut transforms,
                                    link_state.as_mut(),
                                    Some(timeline.as_mut()),
                                );
                            } else {
                                link_state.message = "Link: viewer config unavailable".to_string();
                            }
                        }
                    }
                });
            }
        });

    if hide_panel_requested {
        layout_state.panel_hidden = true;
        panel_width.width_px = 0.0;
        return;
    }

    panel_width.width_px =
        total_right_panel_width(panel_response.response.rect.width(), chat_panel_width);

    if player_mode_enabled {
        render_feedback_toasts(context, &feedback_toast_state, now_secs);
    }
}

fn render_overview_section(
    ui: &mut egui::Ui,
    locale: crate::i18n::UiLocale,
    state: &ViewerState,
    selection: &ViewerSelection,
    timeline: &TimelineUiState,
    perf_summary: Option<&RenderPerfSummary>,
) {
    let current_tick = state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0);

    let selection_value = selection
        .current
        .as_ref()
        .map(|current| format!("{} {}", selection_kind_label(current.kind), current.id))
        .unwrap_or_else(|| {
            if locale.is_zh() {
                "(无)".to_string()
            } else {
                "(none)".to_string()
            }
        });

    let rejected_events = rejection_event_count(&state.events);
    let (connection_text, connection_color) = connection_signal(&state.status, locale);
    let (health_text, health_color) = health_signal(rejected_events, locale);
    let (mode_text, mode_color) = mode_signal(timeline, locale);

    ui.horizontal_wrapped(|ui| {
        render_status_badge(ui, &connection_text, connection_color);
        render_status_badge(ui, &health_text, health_color);
        render_status_badge(ui, &mode_text, mode_color);
    });

    let chips = [
        (
            if locale.is_zh() { "Tick" } else { "Tick" },
            current_tick.to_string(),
        ),
        (
            if locale.is_zh() { "事件" } else { "Events" },
            state.events.len().to_string(),
        ),
        (
            if locale.is_zh() { "轨迹" } else { "Traces" },
            state.decision_traces.len().to_string(),
        ),
        (
            if locale.is_zh() {
                "选择"
            } else {
                "Selection"
            },
            truncate_observe_text(&selection_value, 18),
        ),
    ];

    ui.horizontal_wrapped(|ui| {
        for (label, value) in chips {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.small(label);
                ui.label(value);
            });
        }
    });

    ui.add(egui::Label::new(status_line(&state.status, locale)).selectable(true));

    if let Some(perf) = perf_summary {
        let hotspot = crate::render_perf_summary::infer_perf_hotspot(perf);
        let frame_line = if locale.is_zh() {
            format!(
                "渲染: avg/p95 {:.1}/{:.1} ms",
                perf.frame_ms_avg, perf.frame_ms_p95
            )
        } else {
            format!(
                "Render: avg/p95 {:.1}/{:.1} ms",
                perf.frame_ms_avg, perf.frame_ms_p95
            )
        };
        let entity_line = if locale.is_zh() {
            format!(
                "对象:{} 标签:{} 覆盖层:{} 事件窗:{}",
                perf.world_entities,
                perf.visible_labels,
                perf.overlay_entities,
                perf.event_window_size
            )
        } else {
            format!(
                "Entities:{} Labels:{} Overlays:{} EventWindow:{}",
                perf.world_entities,
                perf.visible_labels,
                perf.overlay_entities,
                perf.event_window_size
            )
        };
        let budget_line = if locale.is_zh() {
            if perf.auto_degrade_active {
                "预算状态: 自动降级触发".to_string()
            } else {
                "预算状态: 稳定".to_string()
            }
        } else if perf.auto_degrade_active {
            "Budget: auto degrade active".to_string()
        } else {
            "Budget: stable".to_string()
        };
        let hotspot_line = if locale.is_zh() {
            format!("瓶颈: {}", hotspot.as_str())
        } else {
            format!("Hotspot: {}", hotspot.as_str())
        };
        let runtime_line = if locale.is_zh() {
            format!(
                "Runtime: {}/{} tick/decide/action/callback {:.1}/{:.1}/{:.1}/{:.1} ms",
                perf.runtime_health.as_str(),
                perf.runtime_bottleneck.as_str(),
                perf.runtime_tick_p95_ms,
                perf.runtime_decision_p95_ms,
                perf.runtime_action_execution_p95_ms,
                perf.runtime_callback_p95_ms
            )
        } else {
            format!(
                "Runtime: {}/{} tick/decision/action/callback {:.1}/{:.1}/{:.1}/{:.1} ms",
                perf.runtime_health.as_str(),
                perf.runtime_bottleneck.as_str(),
                perf.runtime_tick_p95_ms,
                perf.runtime_decision_p95_ms,
                perf.runtime_action_execution_p95_ms,
                perf.runtime_callback_p95_ms
            )
        };

        ui.add(egui::Label::new(frame_line).selectable(true));
        ui.add(egui::Label::new(entity_line).selectable(true));
        ui.add(egui::Label::new(budget_line).selectable(true));
        ui.add(egui::Label::new(hotspot_line).selectable(true));
        ui.add(egui::Label::new(runtime_line).selectable(true));
    }
}

fn render_status_badge(ui: &mut egui::Ui, text: &str, fill: egui::Color32) {
    ui.add(egui::Label::new(
        egui::RichText::new(format!("  {text}  "))
            .color(egui::Color32::WHITE)
            .background_color(fill),
    ));
}

fn connection_signal(
    status: &crate::ConnectionStatus,
    locale: crate::i18n::UiLocale,
) -> (String, egui::Color32) {
    match status {
        crate::ConnectionStatus::Connected => (
            if locale.is_zh() {
                "连接正常"
            } else {
                "Conn OK"
            }
            .to_string(),
            egui::Color32::from_rgb(36, 130, 72),
        ),
        crate::ConnectionStatus::Connecting => (
            if locale.is_zh() {
                "连接中"
            } else {
                "Connecting"
            }
            .to_string(),
            egui::Color32::from_rgb(144, 108, 36),
        ),
        crate::ConnectionStatus::Error(_) => (
            if locale.is_zh() {
                "连接异常"
            } else {
                "Conn Error"
            }
            .to_string(),
            egui::Color32::from_rgb(160, 52, 52),
        ),
    }
}

fn health_signal(rejected_events: usize, locale: crate::i18n::UiLocale) -> (String, egui::Color32) {
    if rejected_events == 0 {
        (
            if locale.is_zh() {
                "健康:正常"
            } else {
                "Health: OK"
            }
            .to_string(),
            egui::Color32::from_rgb(32, 112, 64),
        )
    } else if rejected_events <= 2 {
        (
            if locale.is_zh() {
                format!("健康:告警{}", rejected_events)
            } else {
                format!("Health: Warn {rejected_events}")
            },
            egui::Color32::from_rgb(150, 110, 32),
        )
    } else {
        (
            if locale.is_zh() {
                format!("健康:高风险{}", rejected_events)
            } else {
                format!("Health: High {rejected_events}")
            },
            egui::Color32::from_rgb(154, 48, 48),
        )
    }
}

fn mode_signal(
    timeline: &TimelineUiState,
    locale: crate::i18n::UiLocale,
) -> (String, egui::Color32) {
    if timeline.manual_override || timeline.drag_active {
        (
            if locale.is_zh() {
                "观察:手动"
            } else {
                "View: Manual"
            }
            .to_string(),
            egui::Color32::from_rgb(125, 96, 28),
        )
    } else {
        (
            if locale.is_zh() {
                "观察:实时"
            } else {
                "View: Live"
            }
            .to_string(),
            egui::Color32::from_rgb(38, 94, 148),
        )
    }
}

fn render_overlay_section(
    ui: &mut egui::Ui,
    locale: crate::i18n::UiLocale,
    camera_mode: ViewerCameraMode,
    state: &ViewerState,
    viewer_3d_config: &Option<Res<Viewer3dConfig>>,
    overlay_config: &mut WorldOverlayConfig,
    industry_zoom: &mut IndustrySemanticZoomState,
) {
    ui.strong(if locale.is_zh() {
        "语义缩放"
    } else {
        "Semantic Zoom"
    });
    ui.horizontal_wrapped(|ui| {
        for level in IndustrySemanticZoomLevel::ALL {
            let selected = industry_zoom.level == level;
            if ui
                .selectable_label(selected, semantic_zoom_label(level, locale))
                .clicked()
            {
                industry_zoom.level = level;
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(overlay_button_label("chunk", locale)).clicked() {
            overlay_config.show_chunk_overlay = !overlay_config.show_chunk_overlay;
        }
        if ui.button(overlay_button_label("heat", locale)).clicked() {
            overlay_config.show_resource_heatmap = !overlay_config.show_resource_heatmap;
        }
        if ui.button(overlay_button_label("flow", locale)).clicked() {
            overlay_config.show_flow_overlay = !overlay_config.show_flow_overlay;
        }
    });

    let text = if let Some(config) = viewer_3d_config.as_deref() {
        overlay_status_text_public(
            state.snapshot.as_ref(),
            &state.events,
            *overlay_config,
            config.effective_cm_to_unit(),
            locale,
            industry_zoom.level,
        )
    } else {
        overlay_loading(locale).to_string()
    };

    ui.add(egui::Label::new(text).wrap().selectable(true));

    ui.add_space(4.0);
    ui.strong(overlay_chunk_legend_title(locale));
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(
            egui::Color32::from_rgba_premultiplied(76, 107, 168, 180),
            format!("● {}", overlay_chunk_legend_label("unexplored", locale)),
        );
        ui.colored_label(
            egui::Color32::from_rgba_premultiplied(61, 199, 112, 196),
            format!("● {}", overlay_chunk_legend_label("generated", locale)),
        );
        ui.colored_label(
            egui::Color32::from_rgba_premultiplied(158, 102, 71, 196),
            format!("● {}", overlay_chunk_legend_label("exhausted", locale)),
        );
        ui.colored_label(
            egui::Color32::from_rgba_premultiplied(77, 87, 97, 140),
            format!("● {}", overlay_chunk_legend_label("world_grid", locale)),
        );
    });

    let world_thickness = grid_line_thickness(GridLineKind::World, camera_mode);
    let chunk_thickness = grid_line_thickness(GridLineKind::Chunk, camera_mode);
    ui.add(
        egui::Label::new(overlay_grid_line_width_hint(
            locale,
            camera_mode,
            world_thickness,
            chunk_thickness,
        ))
        .wrap()
        .selectable(true),
    );
}

fn render_timeline_section(
    ui: &mut egui::Ui,
    locale: crate::i18n::UiLocale,
    state: &ViewerState,
    timeline: &mut TimelineUiState,
    filters: &mut TimelineMarkFilterState,
    client: Option<&ViewerClient>,
    control_profile: Option<&ViewerControlProfileState>,
) {
    let current_tick = state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0);

    let axis_max = timeline_axis_max_public(timeline, current_tick);
    let mode = timeline_mode_label(timeline.drag_active, timeline.manual_override, locale);
    ui.add(
        egui::Label::new(timeline_status_line(
            current_tick,
            timeline.target_tick,
            axis_max,
            mode,
            locale,
        ))
        .wrap()
        .selectable(true),
    );

    ui.horizontal_wrapped(|ui| {
        if ui
            .button(timeline_mark_filter_label_public(
                TimelineMarkKindPublic::Error,
                filters.show_error,
                locale,
            ))
            .clicked()
        {
            filters.show_error = !filters.show_error;
        }

        if ui
            .button(timeline_mark_filter_label_public(
                TimelineMarkKindPublic::Llm,
                filters.show_llm,
                locale,
            ))
            .clicked()
        {
            filters.show_llm = !filters.show_llm;
        }

        if ui
            .button(timeline_mark_filter_label_public(
                TimelineMarkKindPublic::Peak,
                filters.show_peak,
                locale,
            ))
            .clicked()
        {
            filters.show_peak = !filters.show_peak;
        }
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(timeline_jump_label("err", locale)).clicked() {
            timeline_mark_jump_action(
                state,
                timeline,
                Some(filters),
                TimelineMarkKindPublic::Error,
            );
        }
        if ui.button(timeline_jump_label("llm", locale)).clicked() {
            timeline_mark_jump_action(state, timeline, Some(filters), TimelineMarkKindPublic::Llm);
        }
        if ui.button(timeline_jump_label("peak", locale)).clicked() {
            timeline_mark_jump_action(state, timeline, Some(filters), TimelineMarkKindPublic::Peak);
        }
    });

    let slider_response =
        ui.add(egui::Slider::new(&mut timeline.target_tick, 0..=axis_max.max(1)).text("tick"));
    if slider_response.changed() {
        timeline.manual_override = true;
    }

    ui.horizontal_wrapped(|ui| {
        if ui.button("-10").clicked() {
            timeline.target_tick = timeline.target_tick.saturating_sub(10);
            timeline.manual_override = true;
        }
        if ui.button("-1").clicked() {
            timeline.target_tick = timeline.target_tick.saturating_sub(1);
            timeline.manual_override = true;
        }
        if ui.button("+1").clicked() {
            timeline.target_tick = timeline.target_tick.saturating_add(1);
            timeline.manual_override = true;
        }
        if ui.button("+10").clicked() {
            timeline.target_tick = timeline.target_tick.saturating_add(10);
            timeline.manual_override = true;
        }
        if viewer_seek_supported(control_profile) && ui.button(seek_button_label(locale)).clicked()
        {
            timeline_seek_action(timeline, client, control_profile);
        }
    });

    let insights = timeline_insights(
        0,
        0,
        0,
        "-".to_string(),
        "-".to_string(),
        "-".to_string(),
        filters.show_error,
        filters.show_llm,
        filters.show_peak,
        "················",
        locale,
    );
    ui.add(egui::Label::new(insights).wrap().selectable(true));
}

fn semantic_zoom_label(
    level: IndustrySemanticZoomLevel,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (level, locale.is_zh()) {
        (IndustrySemanticZoomLevel::World, true) => "世界",
        (IndustrySemanticZoomLevel::Region, true) => "区域",
        (IndustrySemanticZoomLevel::Node, true) => "节点",
        (IndustrySemanticZoomLevel::World, false) => "World",
        (IndustrySemanticZoomLevel::Region, false) => "Region",
        (IndustrySemanticZoomLevel::Node, false) => "Node",
    }
}

#[cfg(test)]
#[path = "egui_right_panel_tests.rs"]
mod tests;
