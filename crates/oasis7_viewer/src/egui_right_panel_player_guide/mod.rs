mod hud;
mod post_onboarding;

pub(super) use self::hud::{
    build_player_minimap_points, player_mission_hud_anchor_y, player_mission_hud_compact_mode,
    player_mission_hud_minimap_reserved_bottom, player_mission_hud_show_command_action,
    player_mission_hud_show_minimap, render_player_mission_hud,
    resolve_selected_location_id_for_minimap,
};
pub(super) use self::post_onboarding::build_player_post_onboarding_snapshot;

use crate::web_test_api::WebTestApiControlFeedbackSnapshot;
use crate::{RightPanelLayoutState, ViewerSelection, ViewerState};
use bevy_egui::egui;
use oasis7::simulator::{
    PlayerGameplayGoalKind, PlayerGameplaySnapshot, PlayerGameplayStageId,
    PlayerGameplayStageStatus, WorldEventKind,
};

use super::egui_right_panel_player_experience::PlayerGuideStep;

const PLAYER_CINEMATIC_FADE_IN_TICKS: u64 = 6;
const PLAYER_CINEMATIC_HOLD_END_TICKS: u64 = 28;
const PLAYER_CINEMATIC_FADE_OUT_END_TICKS: u64 = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlayerGuideProgressSnapshot {
    pub(super) connect_world_done: bool,
    pub(super) open_panel_done: bool,
    pub(super) select_target_done: bool,
    pub(super) explore_ready: bool,
}

impl PlayerGuideProgressSnapshot {
    pub(super) fn completed_steps(self) -> usize {
        let steps = [
            self.connect_world_done,
            self.open_panel_done,
            self.select_target_done,
            self.explore_ready,
        ];
        steps.into_iter().filter(|done| *done).count()
    }

    pub(super) fn is_step_complete(self, step: PlayerGuideStep) -> bool {
        match step {
            PlayerGuideStep::ConnectWorld => self.connect_world_done,
            PlayerGuideStep::OpenPanel => self.open_panel_done,
            PlayerGuideStep::SelectTarget => self.select_target_done,
            PlayerGuideStep::ExploreAction => self.explore_ready,
        }
    }
}

pub(super) fn build_player_guide_progress_snapshot(
    status: &crate::ConnectionStatus,
    layout_state: &RightPanelLayoutState,
    selection: &ViewerSelection,
    action_feedback_seen: bool,
) -> PlayerGuideProgressSnapshot {
    let connect_world_done = matches!(status, crate::ConnectionStatus::Connected);
    let open_panel_done = connect_world_done && !layout_state.panel_hidden;
    let select_target_done = open_panel_done && selection.current.is_some();
    PlayerGuideProgressSnapshot {
        connect_world_done,
        open_panel_done,
        select_target_done,
        explore_ready: select_target_done && action_feedback_seen,
    }
}

pub(super) fn player_goal_title(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "等待世界同步",
        (PlayerGuideStep::ConnectWorld, false) => "Waiting For World Sync",
        (PlayerGuideStep::OpenPanel, true) => "展开操作面板",
        (PlayerGuideStep::OpenPanel, false) => "Open Control Panel",
        (PlayerGuideStep::SelectTarget, true) => "选择一个目标",
        (PlayerGuideStep::SelectTarget, false) => "Select A Target",
        (PlayerGuideStep::ExploreAction, true) => "开始推进任务",
        (PlayerGuideStep::ExploreAction, false) => "Advance The Run",
    }
}

pub(super) fn player_goal_detail(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "连接建立后，你将看到实时 Tick 与事件流。",
        (PlayerGuideStep::ConnectWorld, false) => {
            "Once connected, live ticks and events will start flowing."
        }
        (PlayerGuideStep::OpenPanel, true) => "按 Tab 或右上角入口按钮，打开面板查看操作入口。",
        (PlayerGuideStep::OpenPanel, false) => {
            "Press Tab or use the top-right toggle to open the panel."
        }
        (PlayerGuideStep::SelectTarget, true) => "点击场景中的 Agent 或地点，查看详情并触发联动。",
        (PlayerGuideStep::SelectTarget, false) => {
            "Click an agent or location in the scene to inspect and interact."
        }
        (PlayerGuideStep::ExploreAction, true) => {
            "点击“直接指挥 Agent”，发送一次移动/采集/建造指令并观察反馈。"
        }
        (PlayerGuideStep::ExploreAction, false) => {
            "Click \"Command Agent\", send one move/harvest/build command, then watch feedback."
        }
    }
}

pub(super) fn player_goal_color(step: PlayerGuideStep) -> egui::Color32 {
    match step {
        PlayerGuideStep::ConnectWorld => egui::Color32::from_rgb(122, 88, 34),
        PlayerGuideStep::OpenPanel => egui::Color32::from_rgb(44, 92, 152),
        PlayerGuideStep::SelectTarget => egui::Color32::from_rgb(30, 112, 88),
        PlayerGuideStep::ExploreAction => egui::Color32::from_rgb(38, 128, 74),
    }
}

pub(super) fn player_goal_badge(locale: crate::i18n::UiLocale) -> &'static str {
    if locale.is_zh() {
        "下一步目标"
    } else {
        "Next Goal"
    }
}

pub(super) fn player_guide_progress_badge(locale: crate::i18n::UiLocale) -> &'static str {
    if locale.is_zh() {
        "引导进度"
    } else {
        "Guide Progress"
    }
}

pub(super) fn player_onboarding_title(locale: crate::i18n::UiLocale) -> &'static str {
    if locale.is_zh() {
        "新手引导"
    } else {
        "Player Guide"
    }
}

pub(super) fn player_onboarding_primary_action(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "知道了",
        (PlayerGuideStep::ConnectWorld, false) => "Got it",
        (PlayerGuideStep::OpenPanel, true) => "打开面板",
        (PlayerGuideStep::OpenPanel, false) => "Open panel",
        (PlayerGuideStep::SelectTarget, true) => "我来选择",
        (PlayerGuideStep::SelectTarget, false) => "I'll select",
        (PlayerGuideStep::ExploreAction, true) => "继续探索",
        (PlayerGuideStep::ExploreAction, false) => "Keep playing",
    }
}

pub(super) fn player_onboarding_dismiss(locale: crate::i18n::UiLocale) -> &'static str {
    if locale.is_zh() {
        "关闭当前提示"
    } else {
        "Hide this tip"
    }
}

pub(super) fn render_player_guide_progress_lines(
    ui: &mut egui::Ui,
    locale: crate::i18n::UiLocale,
    progress: PlayerGuideProgressSnapshot,
    step: PlayerGuideStep,
    tone: egui::Color32,
) {
    ui.small(format!(
        "{} {}/4",
        player_guide_progress_badge(locale),
        progress.completed_steps()
    ));
    let steps = [
        PlayerGuideStep::ConnectWorld,
        PlayerGuideStep::OpenPanel,
        PlayerGuideStep::SelectTarget,
        PlayerGuideStep::ExploreAction,
    ];
    for item in steps {
        let marker = if progress.is_step_complete(item) {
            "✓"
        } else if item == step {
            "▶"
        } else {
            "·"
        };
        ui.small(
            egui::RichText::new(format!("{marker} {}", player_goal_title(item, locale))).color(
                if item == step {
                    tone
                } else {
                    egui::Color32::from_gray(178)
                },
            ),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlayerLayoutPreset {
    Mission,
    Command,
    Intel,
}

fn player_layout_preset_label(
    preset: PlayerLayoutPreset,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (preset, locale.is_zh()) {
        (PlayerLayoutPreset::Mission, true) => "任务",
        (PlayerLayoutPreset::Mission, false) => "Mission",
        (PlayerLayoutPreset::Command, true) => "指挥",
        (PlayerLayoutPreset::Command, false) => "Command",
        (PlayerLayoutPreset::Intel, true) => "情报",
        (PlayerLayoutPreset::Intel, false) => "Intel",
    }
}

pub(super) fn resolve_player_layout_preset(
    layout_state: &RightPanelLayoutState,
    module_visibility: &crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
) -> PlayerLayoutPreset {
    if !layout_state.panel_hidden
        && module_visibility.show_chat
        && !module_visibility.show_timeline
        && !module_visibility.show_details
    {
        return PlayerLayoutPreset::Command;
    }

    if module_visibility.show_timeline || module_visibility.show_details {
        return PlayerLayoutPreset::Intel;
    }

    PlayerLayoutPreset::Mission
}

pub(super) fn apply_player_layout_preset(
    layout_state: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    preset: PlayerLayoutPreset,
) {
    layout_state.panel_hidden = false;
    layout_state.top_panel_collapsed = false;
    module_visibility.show_controls = false;
    module_visibility.show_overlay = false;
    module_visibility.show_diagnosis = false;

    match preset {
        PlayerLayoutPreset::Mission => {
            module_visibility.show_overview = false;
            module_visibility.show_chat = false;
            module_visibility.show_event_link = false;
            module_visibility.show_timeline = false;
            module_visibility.show_details = false;
        }
        PlayerLayoutPreset::Command => {
            module_visibility.show_overview = false;
            module_visibility.show_chat = true;
            module_visibility.show_event_link = false;
            module_visibility.show_timeline = false;
            module_visibility.show_details = false;
        }
        PlayerLayoutPreset::Intel => {
            module_visibility.show_overview = true;
            module_visibility.show_chat = false;
            module_visibility.show_event_link = true;
            module_visibility.show_timeline = true;
            module_visibility.show_details = true;
        }
    }
}

pub(super) fn render_player_layout_preset_strip(
    context: &egui::Context,
    layout_state: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    if !should_show_player_layout_preset_strip(layout_state.panel_hidden) {
        return;
    }
    let active = resolve_player_layout_preset(layout_state, module_visibility);
    let anchor_y = player_layout_preset_strip_anchor_y(layout_state.panel_hidden);
    let pulse = ((now_secs * 1.5).sin() * 0.5 + 0.5) as f32;
    egui::Area::new(egui::Id::new("viewer-player-layout-strip"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, anchor_y))
        .movable(false)
        .interactable(true)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(16, 24, 37, 214))
                .stroke(egui::Stroke::new(
                    1.0 + 0.4 * pulse,
                    egui::Color32::from_rgb(64, 106, 152),
                ))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.small(if locale.is_zh() {
                        "布局焦点"
                    } else {
                        "Layout Focus"
                    });
                    ui.horizontal_wrapped(|ui| {
                        for preset in [
                            PlayerLayoutPreset::Mission,
                            PlayerLayoutPreset::Command,
                            PlayerLayoutPreset::Intel,
                        ] {
                            if ui
                                .selectable_label(
                                    active == preset,
                                    player_layout_preset_label(preset, locale),
                                )
                                .clicked()
                            {
                                apply_player_layout_preset(layout_state, module_visibility, preset);
                            }
                        }
                    });
                });
        });
}

pub(super) fn should_show_player_layout_preset_strip(panel_hidden: bool) -> bool {
    panel_hidden
}

pub(super) fn player_layout_preset_strip_anchor_y(panel_hidden: bool) -> f32 {
    if should_show_player_layout_preset_strip(panel_hidden) {
        74.0
    } else {
        0.0
    }
}

fn player_current_tick(state: &crate::ViewerState) -> u64 {
    state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0)
}

pub(super) fn player_control_stage_label(
    stage: &str,
    locale: crate::i18n::UiLocale,
) -> &'static str {
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

pub(super) fn player_control_stage_color(stage: &str) -> egui::Color32 {
    match stage {
        "completed_advanced" | "applied" => egui::Color32::from_rgb(78, 182, 108),
        "completed_no_progress" => egui::Color32::from_rgb(224, 176, 92),
        "blocked" => egui::Color32::from_rgb(226, 128, 98),
        "executing" | "received" => egui::Color32::from_rgb(118, 168, 236),
        _ => egui::Color32::from_rgb(186, 206, 238),
    }
}

pub(super) fn player_control_stage_shows_recovery_actions(stage: &str) -> bool {
    matches!(stage, "completed_no_progress")
}

pub(super) fn player_cinematic_intro_alpha(status: &crate::ConnectionStatus, tick: u64) -> f32 {
    if !matches!(status, crate::ConnectionStatus::Connected)
        || tick > PLAYER_CINEMATIC_FADE_OUT_END_TICKS
    {
        return 0.0;
    }
    if tick <= PLAYER_CINEMATIC_FADE_IN_TICKS {
        ((tick + 1) as f32 / (PLAYER_CINEMATIC_FADE_IN_TICKS + 1) as f32).clamp(0.0, 1.0)
    } else if tick <= PLAYER_CINEMATIC_HOLD_END_TICKS {
        1.0
    } else {
        (1.0 - (tick - PLAYER_CINEMATIC_HOLD_END_TICKS) as f32
            / (PLAYER_CINEMATIC_FADE_OUT_END_TICKS - PLAYER_CINEMATIC_HOLD_END_TICKS) as f32)
            .clamp(0.0, 1.0)
    }
}

fn player_cinematic_subtitle(step: PlayerGuideStep, locale: crate::i18n::UiLocale) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "世界链路建立中，准备接入前哨视角。",
        (PlayerGuideStep::ConnectWorld, false) => {
            "World link is stabilizing. Preparing outpost feed."
        }
        (PlayerGuideStep::OpenPanel, true) => "先展开指挥面板，领取第一条任务线。",
        (PlayerGuideStep::OpenPanel, false) => {
            "Open the control panel to claim your first mission loop."
        }
        (PlayerGuideStep::SelectTarget, true) => "锁定一个目标，你的行动将立刻改变世界。",
        (PlayerGuideStep::SelectTarget, false) => {
            "Lock a target. Your next action will change this world."
        }
        (PlayerGuideStep::ExploreAction, true) => "保持节奏推进任务，连续反馈会持续强化。",
        (PlayerGuideStep::ExploreAction, false) => {
            "Keep the loop moving. Feedback intensity will ramp up."
        }
    }
}

pub(super) fn render_player_cinematic_intro(
    context: &egui::Context,
    state: &crate::ViewerState,
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let tick = player_current_tick(state);
    let alpha = player_cinematic_intro_alpha(&state.status, tick);
    if alpha <= 0.01 {
        return;
    }
    let pulse = ((now_secs * 1.6).sin() * 0.5 + 0.5) as f32;
    let tone = player_goal_color(step);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    egui::Area::new(egui::Id::new("viewer-player-cinematic-intro"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 56.0))
        .movable(false)
        .interactable(false)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(
                    8,
                    16,
                    24,
                    to_u8(226.0 * alpha),
                ))
                .stroke(egui::Stroke::new(
                    1.0 + 0.6 * pulse,
                    egui::Color32::from_rgba_unmultiplied(
                        tone.r(),
                        tone.g(),
                        tone.b(),
                        to_u8((136.0 + 92.0 * pulse) * alpha),
                    ),
                ))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(560.0);
                    ui.vertical_centered(|ui| {
                        ui.small(
                            egui::RichText::new(if locale.is_zh() {
                                "沉浸开场"
                            } else {
                                "Immersive Intro"
                            })
                            .color(tone),
                        );
                        ui.strong(if locale.is_zh() {
                            "前哨部署完成"
                        } else {
                            "Outpost Deployment Ready"
                        });
                        ui.label(player_cinematic_subtitle(step, locale));
                        ui.small(if locale.is_zh() {
                            "按 Tab 可随时展开控制面板"
                        } else {
                            "Press Tab to open the control panel anytime"
                        });
                    });
                });
        });
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlayerMissionLoopSnapshot {
    pub(super) completed_steps: usize,
    pub(super) title: &'static str,
    pub(super) objective: &'static str,
    pub(super) completion_condition: &'static str,
    pub(super) eta: &'static str,
    pub(super) short_goals: [PlayerShortGoalSnapshot; 2],
    pub(super) action_label: &'static str,
    pub(super) action_opens_panel: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlayerShortGoalSnapshot {
    pub(super) label: &'static str,
    pub(super) complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlayerRewardFeedbackSnapshot {
    pub(super) badge: &'static str,
    pub(super) title: &'static str,
    pub(super) detail: String,
    pub(super) complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlayerPostOnboardingStatus {
    Active,
    Blocked,
    BranchReady,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlayerPostOnboardingSnapshot {
    pub(super) status: PlayerPostOnboardingStatus,
    pub(super) title: &'static str,
    pub(super) objective: String,
    pub(super) progress_detail: String,
    pub(super) progress_percent: u8,
    pub(super) blocker_detail: Option<String>,
    pub(super) next_step: String,
    pub(super) branch_hint: Option<String>,
    pub(super) action_label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlayerMiniMapPoint {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) selected: bool,
}

pub(super) fn build_player_mission_loop_snapshot(
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    locale: crate::i18n::UiLocale,
) -> PlayerMissionLoopSnapshot {
    let (action_label, action_opens_panel) = match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => ("执行下一步：确认连接状态", false),
        (PlayerGuideStep::ConnectWorld, false) => ("Do next step: Check connection", false),
        (PlayerGuideStep::OpenPanel, true) => ("执行下一步：打开面板", true),
        (PlayerGuideStep::OpenPanel, false) => ("Do next step: Open panel", true),
        (PlayerGuideStep::SelectTarget, true) => ("执行下一步：切换任务视图并选目标", false),
        (PlayerGuideStep::SelectTarget, false) => {
            ("Do next step: Switch to mission view and select", false)
        }
        (PlayerGuideStep::ExploreAction, true) => ("执行下一步：打开指挥并开始推进", false),
        (PlayerGuideStep::ExploreAction, false) => ("Do next step: Open command and play", false),
    };
    let short_goals = build_player_short_goals(step, progress, locale);
    PlayerMissionLoopSnapshot {
        completed_steps: progress.completed_steps(),
        title: if locale.is_zh() {
            "主任务：建立行动闭环"
        } else {
            "Mission: Build Action Loop"
        },
        objective: player_goal_action_sentence(step, locale),
        completion_condition: player_goal_completion_condition(step, locale),
        eta: player_goal_eta(step, locale),
        short_goals,
        action_label,
        action_opens_panel,
    }
}

fn build_player_short_goals(
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    locale: crate::i18n::UiLocale,
) -> [PlayerShortGoalSnapshot; 2] {
    let (labels, done) = match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => (
            ["建立世界连接", "展开操作面板"],
            [progress.connect_world_done, progress.open_panel_done],
        ),
        (PlayerGuideStep::ConnectWorld, false) => (
            ["Connect to world", "Open control panel"],
            [progress.connect_world_done, progress.open_panel_done],
        ),
        (PlayerGuideStep::OpenPanel, true) => (
            ["展开操作面板", "锁定一个目标"],
            [progress.open_panel_done, progress.select_target_done],
        ),
        (PlayerGuideStep::OpenPanel, false) => (
            ["Open control panel", "Lock one target"],
            [progress.open_panel_done, progress.select_target_done],
        ),
        (PlayerGuideStep::SelectTarget, true) => (
            ["锁定一个目标", "发送首条指令"],
            [progress.select_target_done, progress.explore_ready],
        ),
        (PlayerGuideStep::SelectTarget, false) => (
            ["Lock one target", "Send first order"],
            [progress.select_target_done, progress.explore_ready],
        ),
        (PlayerGuideStep::ExploreAction, true) => (
            ["发送首条指令", "确认世界反馈"],
            [progress.explore_ready, progress.explore_ready],
        ),
        (PlayerGuideStep::ExploreAction, false) => (
            ["Send first order", "Confirm world feedback"],
            [progress.explore_ready, progress.explore_ready],
        ),
    };

    [
        PlayerShortGoalSnapshot {
            label: labels[0],
            complete: done[0],
        },
        PlayerShortGoalSnapshot {
            label: labels[1],
            complete: done[1],
        },
    ]
}

pub(super) fn build_player_reward_feedback_snapshot(
    progress: PlayerGuideProgressSnapshot,
    locale: crate::i18n::UiLocale,
) -> PlayerRewardFeedbackSnapshot {
    let completed_steps = progress.completed_steps();
    match (completed_steps, locale.is_zh()) {
        (4, true) => PlayerRewardFeedbackSnapshot {
            badge: "任务奖励",
            title: "闭环达成",
            detail: "你已打通完整上手路径，可持续推进行动循环。".to_string(),
            complete: true,
        },
        (4, false) => PlayerRewardFeedbackSnapshot {
            badge: "Reward",
            title: "Loop Completed",
            detail: "You finished the onboarding loop and unlocked the full play rhythm."
                .to_string(),
            complete: true,
        },
        (_, true) => PlayerRewardFeedbackSnapshot {
            badge: "进度奖励",
            title: "节奏提升中",
            detail: format!("已完成 {completed_steps}/4 步，继续推进可触发闭环达成反馈。"),
            complete: false,
        },
        (_, false) => PlayerRewardFeedbackSnapshot {
            badge: "Progress Reward",
            title: "Momentum Building",
            detail: format!(
                "{completed_steps}/4 steps completed. Keep pushing to trigger completion feedback."
            ),
            complete: false,
        },
    }
}

fn player_goal_action_sentence(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "等待连接状态变为“已连接”",
        (PlayerGuideStep::ConnectWorld, false) => "Wait until connection status becomes Connected",
        (PlayerGuideStep::OpenPanel, true) => "打开右侧操作面板，进入可操作状态",
        (PlayerGuideStep::OpenPanel, false) => "Open the right control panel to unlock actions",
        (PlayerGuideStep::SelectTarget, true) => "在场景中选择 1 个 Agent 或地点",
        (PlayerGuideStep::SelectTarget, false) => "Select one agent or location in the scene",
        (PlayerGuideStep::ExploreAction, true) => "发送 1 次指令并确认世界出现新反馈",
        (PlayerGuideStep::ExploreAction, false) => {
            "Send one command and confirm new world feedback"
        }
    }
}

fn post_onboarding_summary_value<'a>(summary: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=");
    let start = summary.find(needle.as_str())?;
    let value_start = start + needle.len();
    let rest = &summary[value_start..];
    let value_end = rest.find(' ').unwrap_or(rest.len());
    let value = rest[..value_end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn localized_post_onboarding_title_for_goal(
    goal_kind: PlayerGameplayGoalKind,
    status: PlayerPostOnboardingStatus,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (goal_kind, status, locale.is_zh()) {
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, true) => {
            "下一阶段：第一次扩产取舍"
        }
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, false) => {
            "Next Stage: Choose the First Expansion Tradeoff"
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, true) => "下一阶段：选择中循环方向",
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, false) => {
            "Next Stage: Choose Your Mid-loop Path"
        }
        (PlayerGameplayGoalKind::RecoverCapability, _, true) => "PostOnboarding：恢复持续能力",
        (PlayerGameplayGoalKind::RecoverCapability, _, false) => {
            "PostOnboarding: Recover Sustainable Capability"
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, true) => "PostOnboarding：稳定第一条产线",
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, false) => {
            "PostOnboarding: Stabilize Your First Line"
        }
        (PlayerGameplayGoalKind::StartFactoryRun, _, true) => "PostOnboarding：启动第一座工厂",
        (PlayerGameplayGoalKind::StartFactoryRun, _, false) => {
            "PostOnboarding: Start Your First Factory Run"
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, true) => {
            "PostOnboarding：把资源流变成产出"
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, false) => {
            "PostOnboarding: Turn Material Flow Into Output"
        }
        (PlayerGameplayGoalKind::EstablishFirstCapability, _, true) => {
            "PostOnboarding：建立第一项持续能力"
        }
        (PlayerGameplayGoalKind::EstablishFirstCapability, _, false) => {
            "PostOnboarding: Establish Your First Sustainable Capability"
        }
        (_, PlayerPostOnboardingStatus::BranchReady, true) => "下一阶段：选择中循环方向",
        (_, PlayerPostOnboardingStatus::BranchReady, false) => {
            "Next Stage: Choose Your Mid-loop Path"
        }
        (_, PlayerPostOnboardingStatus::Blocked, true) => "PostOnboarding：恢复持续能力",
        (_, PlayerPostOnboardingStatus::Blocked, false) => {
            "PostOnboarding: Recover Sustainable Capability"
        }
        (_, _, true) => "PostOnboarding：建立第一项持续能力",
        (_, _, false) => "PostOnboarding: Establish Your First Sustainable Capability",
    }
}

fn localized_post_onboarding_objective_for_goal(
    goal_kind: PlayerGameplayGoalKind,
    status: PlayerPostOnboardingStatus,
    locale: crate::i18n::UiLocale,
) -> String {
    match (goal_kind, status, locale.is_zh()) {
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, true) => {
            "第一条产线已经够稳，可以开始第一次扩产取舍：补吞吐、补韧性，或拉开物流覆盖。"
                .to_string()
        }
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, false) => {
            "The first line is stable enough to grow. Choose whether the next investment should buy more throughput, stronger resilience, or wider logistics reach.".to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, true) => {
            "第一项持续工业能力已建立，开始把它扩张成稳定组织能力。".to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, false) => {
            "Your first sustainable industrial capability is online. Turn it into stable organizational momentum.".to_string()
        }
        (PlayerGameplayGoalKind::RecoverCapability, _, true) => {
            "优先恢复被阻塞的产线或能力链，而不是重复单次动作。".to_string()
        }
        (PlayerGameplayGoalKind::RecoverCapability, _, false) => {
            "Recover the blocked line or capability chain instead of repeating one-off actions."
                .to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, true) => {
            "让第一条生产线连续推进，直到出现稳定产出或明确阻塞原因。".to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, false) => {
            "Keep your first production line moving until it produces stable output or exposes a clear blocker."
                .to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, _, true) => {
            "把已建成的工厂推进成真正运转的持续能力。".to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, _, false) => {
            "Turn the factory you built into a running, repeatable capability.".to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, true) => {
            "不要停留在一次性采集，继续把资源推进到可见产出。".to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, false) => {
            "Do not stop at one-off harvesting; push the resource flow into visible output."
                .to_string()
        }
        (_, _, true) => {
            "首局行动闭环已完成，下一步不是重复教程，而是做出第一项持续工业成果。"
                .to_string()
        }
        (_, _, false) => {
            "The first-session action loop is complete. The next step is not to repeat the tutorial, but to create your first sustainable industrial result."
                .to_string()
        }
    }
}

fn localized_post_onboarding_progress_detail_for_goal(
    goal_kind: PlayerGameplayGoalKind,
    status: PlayerPostOnboardingStatus,
    locale: crate::i18n::UiLocale,
) -> String {
    match (goal_kind, status, locale.is_zh()) {
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, true) => {
            "阶段进展：已越过 bootstrap，第一次扩产建议已经解锁。".to_string()
        }
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, _, false) => {
            "Stage progress: bootstrap is complete and the first expansion tradeoff is now unlocked."
                .to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, true) => {
            "阶段进展：已完成首个可见产出/稳定产线里程碑。".to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, _, false) => {
            "Stage progress: your first visible output or stable line milestone is complete."
                .to_string()
        }
        (PlayerGameplayGoalKind::RecoverCapability, _, true) => {
            "阶段进展：你已经进入经营阶段，但当前主线被阻塞。".to_string()
        }
        (PlayerGameplayGoalKind::RecoverCapability, _, false) => {
            "Stage progress: you are in the management phase, but the primary line is blocked."
                .to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, true) => {
            "阶段进展：首条产线已启动，接下来重点看输出与停机原因。".to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, _, false) => {
            "Stage progress: the first line is running; now watch for output and stoppage reasons."
                .to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, _, true) => {
            "阶段进展：工厂已就绪，还差一次可见的生产推进。".to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, _, false) => {
            "Stage progress: the factory is ready; one visible production push remains."
                .to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, true) => {
            "阶段进展：基础资源已经动起来，接下来要形成第一项持续能力。".to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, _, false) => {
            "Stage progress: base resources are moving; now convert them into the first sustainable capability."
                .to_string()
        }
        (_, PlayerPostOnboardingStatus::Blocked, true) => {
            "阶段进展：你已经进入经营阶段，但当前主线被阻塞。".to_string()
        }
        (_, PlayerPostOnboardingStatus::Blocked, false) => {
            "Stage progress: you are in the management phase, but the primary line is blocked."
                .to_string()
        }
        (_, _, true) => "阶段进展：你已从“会操作”进入“会经营”的起点。".to_string(),
        (_, _, false) => {
            "Stage progress: you have moved from 'can operate' into the start of 'can manage'."
                .to_string()
        }
    }
}

fn localized_post_onboarding_next_step_for_goal(
    goal_kind: PlayerGameplayGoalKind,
    locale: crate::i18n::UiLocale,
) -> String {
    match (goal_kind, locale.is_zh()) {
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, true) => {
            "下一步：保持 Command 视图，再推进 1~2 次，并在扩吞吐、补上游韧性或拓展物流覆盖之间做第一次取舍。"
                .to_string()
        }
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, false) => {
            "Next: stay in Command view, advance 1-2 more times, and choose between more throughput, stronger upstream resilience, or wider logistics reach."
                .to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, true) => {
            "下一步：保持 Command 视图，继续扩产、推进治理提案，或为关键节点补防护。"
                .to_string()
        }
        (PlayerGameplayGoalKind::ChooseMidLoopPath, false) => {
            "Next: stay in Command view and either expand production, push governance, or secure a critical node."
                .to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, true) => {
            "下一步：保持 Command 视图，再推进 1~2 次，并观察是否出现产出、恢复或阻塞反馈。"
                .to_string()
        }
        (PlayerGameplayGoalKind::StabilizeFirstLine, false) => {
            "Next: stay in Command view, advance 1-2 more times, and watch for output, recovery, or blocker feedback."
                .to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, true) => {
            "下一步：切到 Command 视图并继续推进，直到工厂启动配方、产出结果或返回阻塞原因。"
                .to_string()
        }
        (PlayerGameplayGoalKind::StartFactoryRun, false) => {
            "Next: switch to Command view and keep advancing until the factory starts a recipe, yields output, or returns a blocker."
                .to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, true) => {
            "下一步：继续在 Command 视图推进采集、精炼、建厂或首个配方，直到出现稳定产出。"
                .to_string()
        }
        (PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput, false) => {
            "Next: keep using Command view to harvest, refine, build, or start the first recipe until stable output appears."
                .to_string()
        }
        (_, true) => {
            "下一步：保持 Command 视图，再推进 2~3 次，优先追首个工业产出、首条稳定产线或一次明确的恢复反馈。"
                .to_string()
        }
        (_, false) => {
            "Next: stay in Command view and advance 2-3 more times, prioritizing the first industrial output, the first stable line, or one clear recovery signal."
                .to_string()
        }
    }
}

fn localized_post_onboarding_branch_hint_for_goal(
    goal_kind: PlayerGameplayGoalKind,
    locale: crate::i18n::UiLocale,
) -> String {
    match (goal_kind, locale.is_zh()) {
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, true) => {
            "已解锁取舍：扩产吞吐 / 上游韧性 / 物流覆盖".to_string()
        }
        (PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff, false) => {
            "Tradeoffs unlocked: throughput expansion / input resilience / logistics reach"
                .to_string()
        }
        (_, true) => "已解锁分支：生产扩张 / 治理影响 / 冲突安全".to_string(),
        (_, false) => {
            "Branches unlocked: production expansion / governance influence / conflict security"
                .to_string()
        }
    }
}

fn post_onboarding_blocker_detail(
    reason: &str,
    detail: &str,
    locale: crate::i18n::UiLocale,
) -> String {
    let normalized = format!("{reason} {detail}");
    if normalized.contains("material_shortage") || normalized.contains("missing_input") {
        if locale.is_zh() {
            "主阻塞：缺料，当前产线拿不到继续推进所需的输入。".to_string()
        } else {
            "Primary blocker: missing materials. The current line cannot get the inputs it needs."
                .to_string()
        }
    } else if normalized.contains("electricity")
        || normalized.contains("power")
        || normalized.contains("energy")
    {
        if locale.is_zh() {
            "主阻塞：缺电/能源不足，当前能力链无法持续运转。".to_string()
        } else {
            "Primary blocker: insufficient power or energy. The capability chain cannot keep running."
                .to_string()
        }
    } else if normalized.contains("logistics") {
        if locale.is_zh() {
            "主阻塞：物流阻塞，资源没能按节奏流到目标节点。".to_string()
        } else {
            "Primary blocker: logistics jam. Resources are not reaching the target node in time."
                .to_string()
        }
    } else if normalized.contains("governance") {
        if locale.is_zh() {
            "主阻塞：治理限制，当前行为被制度或权限约束挡住。".to_string()
        } else {
            "Primary blocker: governance restriction. Rules or permissions are blocking progress."
                .to_string()
        }
    } else if normalized.contains("war") || normalized.contains("crisis") {
        if locale.is_zh() {
            "主阻塞：危机/冲突压力，当前应先保全与恢复。".to_string()
        } else {
            "Primary blocker: crisis or conflict pressure. Stabilization must come before expansion."
                .to_string()
        }
    } else if locale.is_zh() {
        format!("主阻塞：{reason}")
    } else {
        format!("Primary blocker: {reason}")
    }
}

fn post_onboarding_blocker_next_step(
    reason: &str,
    detail: &str,
    locale: crate::i18n::UiLocale,
) -> String {
    let normalized = format!("{reason} {detail}");
    if normalized.contains("material_shortage") || normalized.contains("missing_input") {
        if locale.is_zh() {
            "建议下一步：补齐上游原料或继续推进采集/精炼，再观察产线是否恢复。".to_string()
        } else {
            "Next: replenish upstream materials or keep harvesting/refining, then check whether the line resumes."
                .to_string()
        }
    } else if normalized.contains("electricity")
        || normalized.contains("power")
        || normalized.contains("energy")
    {
        if locale.is_zh() {
            "建议下一步：先补能源，再继续推进工厂或配方。".to_string()
        } else {
            "Next: restore energy first, then continue advancing the factory or recipe.".to_string()
        }
    } else if normalized.contains("logistics") {
        if locale.is_zh() {
            "建议下一步：重新推进运输/位置相关操作，先打通物流路径。".to_string()
        } else {
            "Next: advance movement or transport-related actions and reopen the logistics path."
                .to_string()
        }
    } else if normalized.contains("governance") {
        if locale.is_zh() {
            "建议下一步：切换到治理/规则相关面板，确认限制来源后再继续推进。".to_string()
        } else {
            "Next: inspect governance or rules-related panels, identify the restriction, and then continue."
                .to_string()
        }
    } else if normalized.contains("war") || normalized.contains("crisis") {
        if locale.is_zh() {
            "建议下一步：优先保全节点、处理危机，再回到扩张主线。".to_string()
        } else {
            "Next: secure the node and handle the crisis first, then return to expansion."
                .to_string()
        }
    } else if locale.is_zh() {
        "建议下一步：继续在 Command 视图推进 1 步，并观察新的阻塞或恢复反馈。".to_string()
    } else {
        "Next: advance one more step in Command view and watch for new blocker or recovery feedback."
            .to_string()
    }
}

pub(crate) fn player_post_onboarding_status_color(
    status: PlayerPostOnboardingStatus,
) -> egui::Color32 {
    match status {
        PlayerPostOnboardingStatus::Active => egui::Color32::from_rgb(86, 144, 214),
        PlayerPostOnboardingStatus::Blocked => egui::Color32::from_rgb(224, 148, 92),
        PlayerPostOnboardingStatus::BranchReady => egui::Color32::from_rgb(74, 176, 108),
    }
}

pub(crate) fn player_post_onboarding_status_label(
    status: PlayerPostOnboardingStatus,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (status, locale.is_zh()) {
        (PlayerPostOnboardingStatus::Active, true) => "阶段推进中",
        (PlayerPostOnboardingStatus::Active, false) => "Stage Active",
        (PlayerPostOnboardingStatus::Blocked, true) => "阶段受阻",
        (PlayerPostOnboardingStatus::Blocked, false) => "Stage Blocked",
        (PlayerPostOnboardingStatus::BranchReady, true) => "分支已解锁",
        (PlayerPostOnboardingStatus::BranchReady, false) => "Branch Ready",
    }
}

fn player_goal_completion_condition(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "完成条件：状态栏显示“已连接”",
        (PlayerGuideStep::ConnectWorld, false) => "Completion: connection chip shows Connected",
        (PlayerGuideStep::OpenPanel, true) => "完成条件：右侧面板可见",
        (PlayerGuideStep::OpenPanel, false) => "Completion: right panel is visible",
        (PlayerGuideStep::SelectTarget, true) => "完成条件：目标栏出现选中对象",
        (PlayerGuideStep::SelectTarget, false) => "Completion: target chip shows a selected object",
        (PlayerGuideStep::ExploreAction, true) => "完成条件：你的操作后新增至少 1 条世界反馈",
        (PlayerGuideStep::ExploreAction, false) => {
            "Completion: at least one new world feedback appears"
        }
    }
}

fn player_goal_eta(step: PlayerGuideStep, locale: crate::i18n::UiLocale) -> &'static str {
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "预计耗时：约 10 秒",
        (PlayerGuideStep::ConnectWorld, false) => "ETA: about 10s",
        (PlayerGuideStep::OpenPanel, true) => "预计耗时：约 5 秒",
        (PlayerGuideStep::OpenPanel, false) => "ETA: about 5s",
        (PlayerGuideStep::SelectTarget, true) => "预计耗时：约 10 秒",
        (PlayerGuideStep::SelectTarget, false) => "ETA: about 10s",
        (PlayerGuideStep::ExploreAction, true) => "预计耗时：约 20 秒",
        (PlayerGuideStep::ExploreAction, false) => "ETA: about 20s",
    }
}

pub(super) fn build_player_mission_remaining_hint(
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    state: &crate::ViewerState,
    locale: crate::i18n::UiLocale,
) -> String {
    let current_tick = player_current_tick(state);
    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => {
            if progress.connect_world_done {
                "剩余：已完成连接，可进入下一步".to_string()
            } else {
                "剩余：等待状态栏出现“已连接”".to_string()
            }
        }
        (PlayerGuideStep::ConnectWorld, false) => {
            if progress.connect_world_done {
                "Remaining: connection done, proceed to next step".to_string()
            } else {
                "Remaining: wait until the status chip shows Connected".to_string()
            }
        }
        (PlayerGuideStep::OpenPanel, true) => {
            if progress.open_panel_done {
                "剩余：面板已展开，继续锁定目标".to_string()
            } else {
                "剩余：展开右侧面板".to_string()
            }
        }
        (PlayerGuideStep::OpenPanel, false) => {
            if progress.open_panel_done {
                "Remaining: panel opened, proceed to target selection".to_string()
            } else {
                "Remaining: open the right panel".to_string()
            }
        }
        (PlayerGuideStep::SelectTarget, true) => {
            if progress.select_target_done {
                "剩余：目标已锁定，继续发出首条指令".to_string()
            } else {
                "剩余：在场景里选中 1 个 Agent 或地点".to_string()
            }
        }
        (PlayerGuideStep::SelectTarget, false) => {
            if progress.select_target_done {
                "Remaining: target locked, send your first command".to_string()
            } else {
                "Remaining: select one agent or location in the scene".to_string()
            }
        }
        (PlayerGuideStep::ExploreAction, true) => {
            let remaining_tick = 20_u64.saturating_sub(current_tick);
            if !progress.explore_ready {
                "剩余：发送指令后至少出现 1 条新的世界反馈".to_string()
            } else if remaining_tick > 0 {
                format!("剩余：再推进约 {remaining_tick} tick（目标 tick=20）")
            } else {
                "剩余：首局主循环目标已达成".to_string()
            }
        }
        (PlayerGuideStep::ExploreAction, false) => {
            let remaining_tick = 20_u64.saturating_sub(current_tick);
            if !progress.explore_ready {
                "Remaining: trigger at least one new world feedback after your command".to_string()
            } else if remaining_tick > 0 {
                format!("Remaining: advance about {remaining_tick} more ticks (goal tick=20)")
            } else {
                "Remaining: first-session loop target reached".to_string()
            }
        }
    }
}
