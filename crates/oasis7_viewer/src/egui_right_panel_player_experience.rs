use super::egui_right_panel_player_card_motion::{
    build_player_card_transition_snapshot, sync_player_guide_transition, PlayerGuideTransitionState,
};
use super::egui_right_panel_player_guide::{
    build_player_guide_progress_snapshot, build_player_post_onboarding_snapshot, player_goal_badge,
    player_goal_color, player_goal_detail, player_goal_title,
    player_mission_hud_minimap_reserved_bottom, player_onboarding_dismiss,
    player_onboarding_primary_action, player_onboarding_title, render_player_cinematic_intro,
    render_player_guide_progress_lines, render_player_layout_preset_strip,
    render_player_mission_hud, PlayerGuideProgressSnapshot,
};
use super::egui_right_panel_player_micro_loop::{
    build_player_micro_loop_snapshot, build_player_no_progress_diagnosis, PlayerNoProgressDiagnosis,
};
use bevy_egui::egui;
use oasis7::simulator::{RejectReason, ResourceOwner, WorldEvent, WorldEventKind};
use std::collections::BTreeSet;

use crate::event_click_list::event_row_label;
use crate::selection_linking::selection_kind_label;
use crate::{RightPanelLayoutState, ViewerSelection, ViewerState};

const FEEDBACK_TOAST_MAX: usize = 3;
const FEEDBACK_TOAST_TTL_SECS: f64 = 4.2;
const FEEDBACK_TOAST_FADE_SECS: f64 = 0.8;
const PLAYER_ACHIEVEMENT_MAX: usize = 3;
const PLAYER_ACHIEVEMENT_TTL_SECS: f64 = 5.2;
const PLAYER_ACHIEVEMENT_FADE_SECS: f64 = 1.0;
const PLAYER_ACHIEVEMENT_MAX_WIDTH: f32 = 320.0;
const PLAYER_ATMOSPHERE_TOP_ALPHA_BASE: f32 = 0.12;
const PLAYER_ATMOSPHERE_BOTTOM_ALPHA_BASE: f32 = 0.08;
const AGENT_CHATTER_MAX: usize = 4;
const AGENT_CHATTER_TTL_SECS: f64 = 5.0;
const AGENT_CHATTER_FADE_SECS: f64 = 0.9;
const AGENT_CHATTER_MAX_WIDTH: f32 = 320.0;
const PLAYER_GOAL_HINT_MAX_WIDTH: f32 = 320.0;
const PLAYER_ONBOARDING_MAX_WIDTH: f32 = 360.0;
const PLAYER_HUD_MAX_WIDTH: f32 = 760.0;
const PLAYER_STUCK_HINT_SECS: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FeedbackTone {
    Positive,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
struct FeedbackToast {
    id: u64,
    title: &'static str,
    detail: String,
    tone: FeedbackTone,
    expires_at_secs: f64,
}

#[derive(Default)]
pub(crate) struct FeedbackToastState {
    toasts: Vec<FeedbackToast>,
    last_seen_event_id: Option<u64>,
    action_feedback_seen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlayerGuideStep {
    ConnectWorld,
    OpenPanel,
    SelectTarget,
    ExploreAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum PlayerAchievementMilestone {
    WorldConnected,
    PanelOpened,
    FirstSelection,
    FirstEventSeen,
}

#[derive(Clone, Debug)]
struct PlayerAchievementToast {
    id: u64,
    milestone: PlayerAchievementMilestone,
    expires_at_secs: f64,
}

#[derive(Clone, Debug)]
struct PlayerAgentChatterBubble {
    id: u64,
    speaker: String,
    line: String,
    tone: FeedbackTone,
    expires_at_secs: f64,
}

#[derive(Default)]
struct PlayerAgentChatterState {
    bubbles: Vec<PlayerAgentChatterBubble>,
    last_seen_event_id: Option<u64>,
}

#[derive(Default)]
pub(crate) struct PlayerAchievementState {
    unlocked: BTreeSet<PlayerAchievementMilestone>,
    toasts: Vec<PlayerAchievementToast>,
    next_toast_id: u64,
    chatter: PlayerAgentChatterState,
}

#[derive(Default)]
pub(crate) struct PlayerOnboardingState {
    dismissed_step: Option<PlayerGuideStep>,
    guide_transition: PlayerGuideTransitionState,
    no_progress_watch: PlayerNoProgressWatch,
    first_session_started_at_secs: Option<f64>,
    first_session_start_tick: u64,
    first_session_start_event_count: usize,
    first_session_summary_visible: bool,
    first_session_summary_shown: bool,
}

#[derive(Default)]
struct PlayerNoProgressWatch {
    last_tick: u64,
    last_event_count: usize,
    last_progress_at_secs: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlayerHudSnapshot {
    pub role: String,
    pub connection: String,
    pub tick: u64,
    pub events: usize,
    pub selection: String,
    pub objective: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlayerFirstSessionSummarySnapshot {
    pub(super) duration_secs: u64,
    pub(super) tick_gain: u64,
    pub(super) event_gain: usize,
    pub(super) title: &'static str,
    pub(super) detail: String,
    pub(super) next_tip: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlayerAtmosphereSnapshot {
    pub(super) top_alpha: f32,
    pub(super) bottom_alpha: f32,
    pub(super) orb_x_factor: f32,
    pub(super) orb_y_factor: f32,
    pub(super) orb_radius: f32,
}

pub(super) fn feedback_tone_for_event(event: &WorldEventKind) -> FeedbackTone {
    match event {
        WorldEventKind::RuntimeEvent { kind, .. } => {
            runtime_feedback_tone(kind).unwrap_or(FeedbackTone::Info)
        }
        WorldEventKind::ActionRejected { .. } => FeedbackTone::Warning,
        WorldEventKind::FactoryBuilt { .. }
        | WorldEventKind::RecipeScheduled { .. }
        | WorldEventKind::CompoundMined { .. }
        | WorldEventKind::CompoundRefined { .. }
        | WorldEventKind::RadiationHarvested { .. }
        | WorldEventKind::AgentMoved { .. }
        | WorldEventKind::ModuleArtifactSaleCompleted { .. } => FeedbackTone::Positive,
        _ => FeedbackTone::Info,
    }
}

fn feedback_title_for_event(tone: FeedbackTone, locale: crate::i18n::UiLocale) -> &'static str {
    match (tone, locale.is_zh()) {
        (FeedbackTone::Positive, true) => "进展达成",
        (FeedbackTone::Positive, false) => "Progress",
        (FeedbackTone::Warning, true) => "操作受阻",
        (FeedbackTone::Warning, false) => "Action Blocked",
        (FeedbackTone::Info, true) => "世界更新",
        (FeedbackTone::Info, false) => "World Update",
    }
}

pub(super) fn push_feedback_toast(
    feedback: &mut FeedbackToastState,
    event: &WorldEvent,
    now_secs: f64,
    locale: crate::i18n::UiLocale,
) {
    let tone = feedback_tone_for_event(&event.kind);
    let detail = friendly_feedback_detail_for_event(event, locale)
        .unwrap_or_else(|| event_row_label(event, false, locale));
    let detail = super::truncate_observe_text(&detail, 64);
    feedback.toasts.push(FeedbackToast {
        id: event.id,
        title: feedback_title_for_event(tone, locale),
        detail,
        tone,
        expires_at_secs: now_secs + FEEDBACK_TOAST_TTL_SECS,
    });
    while feedback.toasts.len() > FEEDBACK_TOAST_MAX {
        feedback.toasts.remove(0);
    }
}

fn should_show_feedback_toast_for_event(event: &WorldEventKind) -> bool {
    !matches!(
        event,
        WorldEventKind::ActionRejected {
            reason: oasis7::simulator::RejectReason::AgentNotFound { .. }
        }
    )
}

pub(super) fn sync_feedback_toasts(
    feedback: &mut FeedbackToastState,
    state: &ViewerState,
    now_secs: f64,
    locale: crate::i18n::UiLocale,
) {
    feedback
        .toasts
        .retain(|toast| toast.expires_at_secs > now_secs);
    let newest_event_id = state.events.last().map(|event| event.id);
    let Some(newest_event_id) = newest_event_id else {
        return;
    };
    let Some(last_seen) = feedback.last_seen_event_id else {
        feedback.last_seen_event_id = Some(newest_event_id);
        return;
    };
    if newest_event_id <= last_seen {
        return;
    }
    let mut seen_max = last_seen;
    let mut saw_new_event = false;
    for event in state.events.iter().filter(|event| event.id > last_seen) {
        if should_show_feedback_toast_for_event(&event.kind) {
            push_feedback_toast(feedback, event, now_secs, locale);
            saw_new_event = true;
        }
        seen_max = seen_max.max(event.id);
    }
    feedback.last_seen_event_id = Some(seen_max);
    if saw_new_event {
        feedback.action_feedback_seen = true;
    }
}

pub(super) fn player_action_feedback_seen(feedback: &FeedbackToastState) -> bool {
    feedback.action_feedback_seen
}
fn feedback_fill_color(tone: FeedbackTone, alpha: f32) -> egui::Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    match tone {
        FeedbackTone::Positive => {
            egui::Color32::from_rgba_unmultiplied(22, 69, 50, to_u8(232.0 * alpha))
        }
        FeedbackTone::Warning => {
            egui::Color32::from_rgba_unmultiplied(99, 44, 32, to_u8(236.0 * alpha))
        }
        FeedbackTone::Info => {
            egui::Color32::from_rgba_unmultiplied(24, 42, 66, to_u8(224.0 * alpha))
        }
    }
}

pub(super) fn render_feedback_toasts(
    context: &egui::Context,
    feedback: &FeedbackToastState,
    now_secs: f64,
) {
    let mut vertical_offset = 14.0;
    for toast in feedback.toasts.iter().rev() {
        let remaining = (toast.expires_at_secs - now_secs).max(0.0);
        let alpha = if remaining < FEEDBACK_TOAST_FADE_SECS {
            (remaining / FEEDBACK_TOAST_FADE_SECS) as f32
        } else {
            1.0
        };
        egui::Area::new(egui::Id::new(("viewer-feedback-toast", toast.id)))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, vertical_offset))
            .movable(false)
            .interactable(false)
            .show(context, |ui| {
                egui::Frame::group(ui.style())
                    .fill(feedback_fill_color(toast.tone, alpha))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(9))
                    .show(ui, |ui| {
                        ui.set_max_width(360.0);
                        ui.strong(toast.title);
                        ui.small(toast.detail.as_str());
                    });
            });
        vertical_offset += 68.0;
    }
}

pub(super) fn build_player_atmosphere_snapshot(now_secs: f64) -> PlayerAtmosphereSnapshot {
    let pulse = ((now_secs * 0.55).sin() * 0.5 + 0.5) as f32;
    let drift = ((now_secs * 0.32).cos() * 0.5 + 0.5) as f32;
    PlayerAtmosphereSnapshot {
        top_alpha: (PLAYER_ATMOSPHERE_TOP_ALPHA_BASE + 0.05 * pulse).clamp(0.0, 0.28),
        bottom_alpha: (PLAYER_ATMOSPHERE_BOTTOM_ALPHA_BASE + 0.06 * drift).clamp(0.0, 0.25),
        orb_x_factor: 0.74 + 0.08 * ((now_secs * 0.24).sin() as f32),
        orb_y_factor: 0.22 + 0.05 * ((now_secs * 0.18).cos() as f32),
        orb_radius: 120.0 + 42.0 * pulse,
    }
}

pub(super) fn render_player_atmosphere(context: &egui::Context, now_secs: f64) {
    let snapshot = build_player_atmosphere_snapshot(now_secs);
    let rect = context.content_rect();
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }
    let layer = egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("viewer-player-atmosphere"),
    );
    let painter = context.layer_painter(layer);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;

    let top_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.34),
    );
    painter.rect_filled(
        top_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(12, 28, 44, to_u8(snapshot.top_alpha * 255.0)),
    );

    let bottom_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x, rect.max.y - rect.height() * 0.29),
        rect.max,
    );
    painter.rect_filled(
        bottom_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(8, 18, 34, to_u8(snapshot.bottom_alpha * 255.0)),
    );

    let orb_center = egui::pos2(
        rect.min.x + rect.width() * snapshot.orb_x_factor,
        rect.min.y + rect.height() * snapshot.orb_y_factor,
    );
    painter.circle_filled(
        orb_center,
        snapshot.orb_radius,
        egui::Color32::from_rgba_unmultiplied(42, 132, 188, 28),
    );
}

fn player_achievement_badge(locale: crate::i18n::UiLocale) -> &'static str {
    if locale.is_zh() {
        "里程碑解锁"
    } else {
        "Milestone Unlocked"
    }
}

fn player_achievement_title(
    milestone: PlayerAchievementMilestone,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (milestone, locale.is_zh()) {
        (PlayerAchievementMilestone::WorldConnected, true) => "世界连接成功",
        (PlayerAchievementMilestone::WorldConnected, false) => "World Link Established",
        (PlayerAchievementMilestone::PanelOpened, true) => "操作面板已展开",
        (PlayerAchievementMilestone::PanelOpened, false) => "Control Panel Online",
        (PlayerAchievementMilestone::FirstSelection, true) => "首次锁定目标",
        (PlayerAchievementMilestone::FirstSelection, false) => "First Target Locked",
        (PlayerAchievementMilestone::FirstEventSeen, true) => "首次收到世界回应",
        (PlayerAchievementMilestone::FirstEventSeen, false) => "First World Response",
    }
}

fn player_achievement_detail(
    milestone: PlayerAchievementMilestone,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (milestone, locale.is_zh()) {
        (PlayerAchievementMilestone::WorldConnected, true) => "实时 Tick 与事件流已经开始。",
        (PlayerAchievementMilestone::WorldConnected, false) => {
            "Live ticks and events are now flowing."
        }
        (PlayerAchievementMilestone::PanelOpened, true) => "主操作入口已就绪，可随时查看详情。",
        (PlayerAchievementMilestone::PanelOpened, false) => {
            "Control entry is ready, inspect details anytime."
        }
        (PlayerAchievementMilestone::FirstSelection, true) => "你可以围绕该目标推进下一步行动。",
        (PlayerAchievementMilestone::FirstSelection, false) => {
            "You can now plan actions around this target."
        }
        (PlayerAchievementMilestone::FirstEventSeen, true) => "你的操作已在世界中产生反馈。",
        (PlayerAchievementMilestone::FirstEventSeen, false) => {
            "Your actions are now reflected in the world."
        }
    }
}

fn player_achievement_color(milestone: PlayerAchievementMilestone) -> egui::Color32 {
    match milestone {
        PlayerAchievementMilestone::WorldConnected => egui::Color32::from_rgb(56, 108, 176),
        PlayerAchievementMilestone::PanelOpened => egui::Color32::from_rgb(72, 146, 204),
        PlayerAchievementMilestone::FirstSelection => egui::Color32::from_rgb(58, 152, 102),
        PlayerAchievementMilestone::FirstEventSeen => egui::Color32::from_rgb(194, 142, 62),
    }
}

fn should_unlock_player_achievement(
    milestone: PlayerAchievementMilestone,
    state: &ViewerState,
    selection: &ViewerSelection,
    layout_state: &RightPanelLayoutState,
) -> bool {
    match milestone {
        PlayerAchievementMilestone::WorldConnected => {
            matches!(state.status, crate::ConnectionStatus::Connected)
        }
        PlayerAchievementMilestone::PanelOpened => !layout_state.panel_hidden,
        PlayerAchievementMilestone::FirstSelection => selection.current.is_some(),
        PlayerAchievementMilestone::FirstEventSeen => !state.events.is_empty(),
    }
}

fn unlock_player_achievement(
    achievements: &mut PlayerAchievementState,
    milestone: PlayerAchievementMilestone,
    now_secs: f64,
) -> bool {
    if !achievements.unlocked.insert(milestone) {
        return false;
    }

    achievements.next_toast_id = achievements.next_toast_id.saturating_add(1);
    achievements.toasts.push(PlayerAchievementToast {
        id: achievements.next_toast_id,
        milestone,
        expires_at_secs: now_secs + PLAYER_ACHIEVEMENT_TTL_SECS,
    });
    while achievements.toasts.len() > PLAYER_ACHIEVEMENT_MAX {
        achievements.toasts.remove(0);
    }
    true
}

pub(super) fn sync_player_achievements(
    achievements: &mut PlayerAchievementState,
    state: &ViewerState,
    selection: &ViewerSelection,
    layout_state: &RightPanelLayoutState,
    now_secs: f64,
) {
    achievements
        .toasts
        .retain(|toast| toast.expires_at_secs > now_secs);

    let milestones = [
        PlayerAchievementMilestone::WorldConnected,
        PlayerAchievementMilestone::PanelOpened,
        PlayerAchievementMilestone::FirstSelection,
        PlayerAchievementMilestone::FirstEventSeen,
    ];

    for milestone in milestones {
        if achievements.unlocked.contains(&milestone) {
            continue;
        }
        if should_unlock_player_achievement(milestone, state, selection, layout_state) {
            unlock_player_achievement(achievements, milestone, now_secs);
            break;
        }
    }
}

pub(super) fn render_player_achievement_popups(
    context: &egui::Context,
    achievements: &PlayerAchievementState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let mut vertical_offset = 228.0;
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    for toast in achievements.toasts.iter().rev() {
        let remaining = (toast.expires_at_secs - now_secs).max(0.0);
        let alpha = if remaining < PLAYER_ACHIEVEMENT_FADE_SECS {
            (remaining / PLAYER_ACHIEVEMENT_FADE_SECS) as f32
        } else {
            1.0
        };
        let tone = player_achievement_color(toast.milestone);
        let fill = egui::Color32::from_rgba_unmultiplied(16, 25, 20, to_u8(224.0 * alpha));
        let stroke = egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(
                tone.r(),
                tone.g(),
                tone.b(),
                to_u8(236.0 * alpha),
            ),
        );
        egui::Area::new(egui::Id::new(("viewer-player-achievement", toast.id)))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, vertical_offset))
            .movable(false)
            .interactable(false)
            .show(context, |ui| {
                egui::Frame::group(ui.style())
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(egui::CornerRadius::same(9))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_max_width(PLAYER_ACHIEVEMENT_MAX_WIDTH);
                        ui.small(egui::RichText::new(player_achievement_badge(locale)).color(tone));
                        ui.strong(player_achievement_title(toast.milestone, locale));
                        ui.small(player_achievement_detail(toast.milestone, locale));
                    });
            });
        vertical_offset += 86.0;
    }
}

fn owner_agent_id(owner: &ResourceOwner) -> Option<&str> {
    match owner {
        ResourceOwner::Agent { agent_id } => Some(agent_id.as_str()),
        ResourceOwner::Location { .. } => None,
    }
}

fn is_player_focus_noise(event: &WorldEventKind) -> bool {
    matches!(
        event,
        WorldEventKind::ActionRejected {
            reason: RejectReason::AgentNotFound { .. }
        }
    )
}

fn chatter_line_for_event(
    event: &WorldEvent,
    locale: crate::i18n::UiLocale,
) -> Option<(String, String, FeedbackTone)> {
    if is_player_focus_noise(&event.kind) {
        return None;
    }
    match &event.kind {
        WorldEventKind::AgentMoved { agent_id, to, .. } => Some((
            super::truncate_observe_text(agent_id, 14),
            if locale.is_zh() {
                format!("已移动至 {}", super::truncate_observe_text(to, 14))
            } else {
                format!("Moved to {}", super::truncate_observe_text(to, 14))
            },
            FeedbackTone::Positive,
        )),
        WorldEventKind::RadiationHarvested {
            agent_id, amount, ..
        } => Some((
            super::truncate_observe_text(agent_id, 14),
            if locale.is_zh() {
                format!("采集辐照 +{amount}")
            } else {
                format!("Harvested radiation +{amount}")
            },
            FeedbackTone::Positive,
        )),
        WorldEventKind::CompoundMined {
            owner,
            compound_mass_g,
            ..
        } => owner_agent_id(owner).map(|agent_id| {
            (
                super::truncate_observe_text(agent_id, 14),
                if locale.is_zh() {
                    format!("开采复合物 {compound_mass_g}g")
                } else {
                    format!("Mined compound {compound_mass_g}g")
                },
                FeedbackTone::Positive,
            )
        }),
        WorldEventKind::CompoundRefined {
            owner,
            hardware_output,
            ..
        } => owner_agent_id(owner).map(|agent_id| {
            (
                super::truncate_observe_text(agent_id, 14),
                if locale.is_zh() {
                    format!("精炼产出硬件 +{hardware_output}")
                } else {
                    format!("Refined hardware +{hardware_output}")
                },
                FeedbackTone::Positive,
            )
        }),
        WorldEventKind::FactoryBuilt {
            owner,
            factory_kind,
            ..
        } => owner_agent_id(owner).map(|agent_id| {
            (
                super::truncate_observe_text(agent_id, 14),
                if locale.is_zh() {
                    format!("建成 {}", super::truncate_observe_text(factory_kind, 18))
                } else {
                    format!("Built {}", super::truncate_observe_text(factory_kind, 18))
                },
                FeedbackTone::Info,
            )
        }),
        WorldEventKind::RecipeScheduled {
            owner, recipe_id, ..
        } => owner_agent_id(owner).map(|agent_id| {
            (
                super::truncate_observe_text(agent_id, 14),
                if locale.is_zh() {
                    format!("启动配方 {}", super::truncate_observe_text(recipe_id, 18))
                } else {
                    format!(
                        "Started recipe {}",
                        super::truncate_observe_text(recipe_id, 18)
                    )
                },
                FeedbackTone::Info,
            )
        }),
        WorldEventKind::RuntimeEvent { kind, domain_kind } => {
            runtime_feedback_line(kind, domain_kind.as_deref(), locale).map(|line| {
                (
                    runtime_feedback_speaker(domain_kind.as_deref(), locale),
                    line,
                    runtime_feedback_tone(kind).unwrap_or(FeedbackTone::Info),
                )
            })
        }
        WorldEventKind::ActionRejected { .. } => Some((
            if locale.is_zh() {
                "系统".to_string()
            } else {
                "System".to_string()
            },
            super::truncate_observe_text(&event_row_label(event, false, locale), 58),
            FeedbackTone::Warning,
        )),
        _ => None,
    }
}

fn friendly_feedback_detail_for_event(
    event: &WorldEvent,
    locale: crate::i18n::UiLocale,
) -> Option<String> {
    match &event.kind {
        WorldEventKind::RuntimeEvent { kind, domain_kind } => {
            runtime_feedback_line(kind, domain_kind.as_deref(), locale)
        }
        _ => chatter_line_for_event(event, locale).map(|(_, line, _)| line),
    }
}

fn runtime_feedback_tone(kind: &str) -> Option<FeedbackTone> {
    match kind {
        "runtime.economy.factory_production_blocked" => Some(FeedbackTone::Warning),
        "runtime.economy.factory_built"
        | "runtime.economy.recipe_completed"
        | "runtime.economy.factory_production_resumed" => Some(FeedbackTone::Positive),
        "runtime.economy.recipe_started" => Some(FeedbackTone::Info),
        _ => None,
    }
}

fn runtime_feedback_speaker(summary: Option<&str>, locale: crate::i18n::UiLocale) -> String {
    let fallback = if locale.is_zh() {
        "工业线"
    } else {
        "Industry"
    };
    summary
        .and_then(|value| runtime_summary_value(value, "factory"))
        .or_else(|| summary.and_then(|value| runtime_summary_value(value, "builder")))
        .or_else(|| summary.and_then(|value| runtime_summary_value(value, "requester")))
        .map(|value| super::truncate_observe_text(value, 14))
        .unwrap_or_else(|| fallback.to_string())
}

fn runtime_feedback_line(
    kind: &str,
    summary: Option<&str>,
    locale: crate::i18n::UiLocale,
) -> Option<String> {
    let summary = summary?;
    let factory = runtime_summary_label(summary, "factory", "factory");
    let recipe = runtime_summary_label(summary, "recipe", "recipe");
    let outputs = runtime_summary_label(summary, "outputs", "outputs");
    let reason = runtime_summary_label(summary, "reason", "reason");
    let detail = runtime_summary_value(summary, "detail");
    let previous_reason = runtime_summary_value(summary, "previous_reason");

    let line = match (kind, locale.is_zh()) {
        ("runtime.economy.recipe_started", true) => {
            format!("指令已接收：{} 正在执行 {}", factory, recipe)
        }
        ("runtime.economy.recipe_started", false) => {
            format!("Order accepted: {factory} is executing {recipe}")
        }
        ("runtime.economy.recipe_completed", true) => {
            format!("奖励已到账：{} 产出 {}", factory, outputs)
        }
        ("runtime.economy.recipe_completed", false) => {
            format!("Reward earned: {factory} produced {outputs}")
        }
        ("runtime.economy.factory_production_blocked", true) => match detail {
            Some(detail) if detail != reason => {
                format!("代价已显现：{} 停机，{} ({detail})", factory, reason)
            }
            _ => format!("代价已显现：{} 停机，{}", factory, reason),
        },
        ("runtime.economy.factory_production_blocked", false) => match detail {
            Some(detail) if detail != reason => {
                format!("Cost surfaced: {factory} is blocked by {reason} ({detail})")
            }
            _ => format!("Cost surfaced: {factory} is blocked by {reason}"),
        },
        ("runtime.economy.factory_production_resumed", true) => match previous_reason {
            Some(previous_reason) if previous_reason != "none" => {
                format!(
                    "恢复已确认：{} 恢复 {}，已解除 {}",
                    factory, recipe, previous_reason
                )
            }
            _ => format!("恢复已确认：{} 恢复 {}", factory, recipe),
        },
        ("runtime.economy.factory_production_resumed", false) => match previous_reason {
            Some(previous_reason) if previous_reason != "none" => {
                format!("Recovery confirmed: {factory} resumed {recipe} after {previous_reason}",)
            }
            _ => {
                format!("Recovery confirmed: {factory} resumed {recipe}")
            }
        },
        ("runtime.economy.factory_built", true) => {
            format!("能力已解锁：{} 已就绪", factory)
        }
        ("runtime.economy.factory_built", false) => {
            format!("Capability unlocked: {factory} is ready")
        }
        _ => return None,
    };

    Some(line)
}

fn runtime_summary_label(summary: &str, key: &str, fallback: &str) -> String {
    runtime_summary_value(summary, key)
        .unwrap_or(fallback)
        .to_string()
}

fn runtime_summary_value<'a>(summary: &'a str, key: &str) -> Option<&'a str> {
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

fn chatter_stroke_color(tone: FeedbackTone, alpha: f32) -> egui::Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    match tone {
        FeedbackTone::Positive => {
            egui::Color32::from_rgba_unmultiplied(72, 180, 118, to_u8(236.0 * alpha))
        }
        FeedbackTone::Warning => {
            egui::Color32::from_rgba_unmultiplied(214, 104, 82, to_u8(232.0 * alpha))
        }
        FeedbackTone::Info => {
            egui::Color32::from_rgba_unmultiplied(108, 166, 230, to_u8(224.0 * alpha))
        }
    }
}

fn push_agent_chatter_bubble(
    achievements: &mut PlayerAchievementState,
    event: &WorldEvent,
    speaker: String,
    line: String,
    tone: FeedbackTone,
    now_secs: f64,
) {
    achievements.chatter.bubbles.push(PlayerAgentChatterBubble {
        id: event.id,
        speaker: super::truncate_observe_text(&speaker, 14),
        line: super::truncate_observe_text(&line, 68),
        tone,
        expires_at_secs: now_secs + AGENT_CHATTER_TTL_SECS,
    });
    while achievements.chatter.bubbles.len() > AGENT_CHATTER_MAX {
        achievements.chatter.bubbles.remove(0);
    }
}

pub(super) fn sync_agent_chatter_bubbles(
    achievements: &mut PlayerAchievementState,
    state: &ViewerState,
    now_secs: f64,
    locale: crate::i18n::UiLocale,
) {
    achievements
        .chatter
        .bubbles
        .retain(|bubble| bubble.expires_at_secs > now_secs);
    let newest_event_id = state.events.last().map(|event| event.id);
    let Some(newest_event_id) = newest_event_id else {
        return;
    };
    let Some(last_seen) = achievements.chatter.last_seen_event_id else {
        achievements.chatter.last_seen_event_id = Some(newest_event_id);
        return;
    };
    if newest_event_id <= last_seen {
        return;
    }
    let mut seen_max = last_seen;
    for event in state.events.iter().filter(|event| event.id > last_seen) {
        if let Some((speaker, line, tone)) = chatter_line_for_event(event, locale) {
            push_agent_chatter_bubble(achievements, event, speaker, line, tone, now_secs);
        }
        seen_max = seen_max.max(event.id);
    }
    achievements.chatter.last_seen_event_id = Some(seen_max);
}

pub(super) fn render_agent_chatter_bubbles(
    context: &egui::Context,
    achievements: &PlayerAchievementState,
    reserved_bottom_px: f32,
    now_secs: f64,
) {
    let mut vertical_offset = 14.0 + reserved_bottom_px.max(0.0);
    for bubble in achievements.chatter.bubbles.iter().rev() {
        let remaining = (bubble.expires_at_secs - now_secs).max(0.0);
        let alpha = if remaining < AGENT_CHATTER_FADE_SECS {
            (remaining / AGENT_CHATTER_FADE_SECS) as f32
        } else {
            1.0
        };
        let accent = chatter_stroke_color(bubble.tone, alpha);
        egui::Area::new(egui::Id::new(("viewer-agent-chatter", bubble.id)))
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                egui::vec2(-14.0, -vertical_offset),
            )
            .movable(false)
            .interactable(false)
            .show(context, |ui| {
                egui::Frame::group(ui.style())
                    .fill(feedback_fill_color(bubble.tone, 0.82 * alpha))
                    .stroke(egui::Stroke::new(1.0, accent))
                    .corner_radius(egui::CornerRadius::same(9))
                    .inner_margin(egui::Margin::same(9))
                    .show(ui, |ui| {
                        ui.set_max_width(AGENT_CHATTER_MAX_WIDTH);
                        ui.small(egui::RichText::new(bubble.speaker.as_str()).color(accent));
                        ui.label(bubble.line.as_str());
                    });
            });
        vertical_offset += 74.0;
    }
}

pub(super) fn resolve_player_guide_step(
    status: &crate::ConnectionStatus,
    layout_state: &RightPanelLayoutState,
    selection: &ViewerSelection,
) -> PlayerGuideStep {
    if !matches!(status, crate::ConnectionStatus::Connected) {
        PlayerGuideStep::ConnectWorld
    } else if layout_state.panel_hidden {
        PlayerGuideStep::OpenPanel
    } else if selection.current.is_none() {
        PlayerGuideStep::SelectTarget
    } else {
        PlayerGuideStep::ExploreAction
    }
}

#[path = "egui_right_panel_player_experience_hud.rs"]
mod egui_right_panel_player_experience_hud;
#[cfg(test)]
#[path = "egui_right_panel_player_experience_test_api.rs"]
mod egui_right_panel_player_experience_test_api;
#[cfg(test)]
pub(super) use egui_right_panel_player_experience_test_api::{
    feedback_action_feedback_seen, feedback_last_seen_event_id, feedback_toast_cap,
    feedback_toast_detail, feedback_toast_ids, feedback_toast_len, feedback_toast_snapshot,
    player_achievement_is_unlocked, player_achievement_popup_cap, player_achievement_popup_len,
    player_achievement_popup_milestones, player_agent_chatter_cap, player_agent_chatter_ids,
    player_agent_chatter_last_seen_event_id, player_agent_chatter_len,
    player_agent_chatter_snapshot, player_first_session_summary_visible,
};

use egui_right_panel_player_experience_hud::{
    render_player_compact_hud, render_player_first_session_summary, render_player_goal_hint,
    render_player_onboarding_card,
};

#[cfg(test)]
pub(super) fn build_player_hud_snapshot(
    state: &ViewerState,
    selection: &ViewerSelection,
    step: PlayerGuideStep,
    post_onboarding_ready: bool,
    locale: crate::i18n::UiLocale,
) -> PlayerHudSnapshot {
    egui_right_panel_player_experience_hud::build_player_hud_snapshot(
        state,
        selection,
        step,
        post_onboarding_ready,
        locale,
    )
}

pub(super) fn should_show_player_onboarding_card(
    onboarding: &PlayerOnboardingState,
    step: PlayerGuideStep,
) -> bool {
    egui_right_panel_player_experience_hud::should_show_player_onboarding_card(onboarding, step)
}

#[cfg(test)]
pub(super) fn dismiss_player_onboarding_step(
    onboarding: &mut PlayerOnboardingState,
    step: PlayerGuideStep,
) {
    egui_right_panel_player_experience_hud::dismiss_player_onboarding_step(onboarding, step);
}

pub(super) fn should_show_player_goal_hint(
    onboarding: &PlayerOnboardingState,
    step: PlayerGuideStep,
    layout_state: &RightPanelLayoutState,
) -> bool {
    egui_right_panel_player_experience_hud::should_show_player_goal_hint(
        onboarding,
        step,
        layout_state,
    )
}

pub(super) fn player_entry_card_style(now_secs: f64) -> (egui::Color32, egui::Stroke) {
    egui_right_panel_player_experience_hud::player_entry_card_style(now_secs)
}

pub(super) fn sync_player_stuck_hint_state(
    onboarding: &mut PlayerOnboardingState,
    state: &ViewerState,
    now_secs: f64,
) -> Option<f64> {
    egui_right_panel_player_experience_hud::sync_player_stuck_hint_state(
        onboarding, state, now_secs,
    )
}

#[cfg(test)]
pub(super) fn build_player_stuck_hint(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
    idle_secs: f64,
) -> String {
    egui_right_panel_player_experience_hud::build_player_stuck_hint(step, locale, idle_secs)
}

pub(super) fn build_player_stuck_hint_with_diagnosis(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
    idle_secs: f64,
    diagnosis: Option<&PlayerNoProgressDiagnosis>,
) -> String {
    egui_right_panel_player_experience_hud::build_player_stuck_hint_with_diagnosis(
        step, locale, idle_secs, diagnosis,
    )
}

pub(super) fn sync_player_first_session_summary_state(
    onboarding: &mut PlayerOnboardingState,
    state: &ViewerState,
    progress: PlayerGuideProgressSnapshot,
    now_secs: f64,
) {
    egui_right_panel_player_experience_hud::sync_player_first_session_summary_state(
        onboarding, state, progress, now_secs,
    )
}

#[cfg(test)]
pub(super) fn dismiss_player_first_session_summary(onboarding: &mut PlayerOnboardingState) {
    egui_right_panel_player_experience_hud::dismiss_player_first_session_summary(onboarding)
}

#[cfg(test)]
pub(super) fn build_player_first_session_summary_snapshot(
    onboarding: &PlayerOnboardingState,
    state: &ViewerState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) -> Option<PlayerFirstSessionSummarySnapshot> {
    egui_right_panel_player_experience_hud::build_player_first_session_summary_snapshot(
        onboarding, state, locale, now_secs,
    )
}

pub(super) fn render_player_experience_layers(
    context: &egui::Context,
    state: &ViewerState,
    selection: &ViewerSelection,
    client: Option<&crate::ViewerClient>,
    control_profile: Option<&crate::ViewerControlProfileState>,
    layout_state: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    onboarding: &mut PlayerOnboardingState,
    achievements: &mut PlayerAchievementState,
    action_feedback_seen: bool,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    render_player_atmosphere(context, now_secs);
    render_player_layout_preset_strip(context, layout_state, module_visibility, locale, now_secs);
    sync_player_achievements(achievements, state, selection, layout_state, now_secs);
    sync_agent_chatter_bubbles(achievements, state, now_secs, locale);
    let guide_step = resolve_player_guide_step(&state.status, layout_state, selection);
    let guide_progress = build_player_guide_progress_snapshot(
        &state.status,
        layout_state,
        selection,
        action_feedback_seen,
    );
    let stuck_idle_secs = sync_player_stuck_hint_state(onboarding, state, now_secs);
    let control_feedback = crate::web_test_api::latest_web_test_api_control_feedback();
    let micro_loop_snapshot = build_player_micro_loop_snapshot(state, locale);
    let stuck_diagnosis = stuck_idle_secs.map(|_| {
        build_player_no_progress_diagnosis(control_feedback.as_ref(), &micro_loop_snapshot, locale)
    });
    let stuck_hint = stuck_idle_secs.map(|idle| {
        build_player_stuck_hint_with_diagnosis(guide_step, locale, idle, stuck_diagnosis.as_ref())
    });
    sync_player_first_session_summary_state(onboarding, state, guide_progress, now_secs);
    let onboarding_visible =
        !guide_progress.explore_ready && should_show_player_onboarding_card(onboarding, guide_step);
    sync_player_guide_transition(&mut onboarding.guide_transition, guide_step, now_secs);
    render_player_cinematic_intro(context, state, guide_step, locale, now_secs);
    render_player_compact_hud(
        context,
        state,
        selection,
        guide_step,
        guide_progress.explore_ready,
        locale,
        now_secs,
    );
    render_player_mission_hud(
        context,
        state,
        selection,
        client,
        control_feedback.as_ref(),
        control_profile,
        layout_state,
        module_visibility,
        onboarding_visible,
        guide_step,
        guide_progress,
        stuck_hint.as_deref(),
        stuck_diagnosis.as_ref(),
        locale,
        now_secs,
    );
    render_player_achievement_popups(context, achievements, locale, now_secs);
    render_agent_chatter_bubbles(
        context,
        achievements,
        player_mission_hud_minimap_reserved_bottom(layout_state.panel_hidden),
        now_secs,
    );
    render_player_first_session_summary(context, onboarding, state, locale, now_secs);
    if !guide_progress.explore_ready
        && should_show_player_goal_hint(onboarding, guide_step, layout_state)
    {
        render_player_goal_hint(
            context,
            onboarding,
            guide_step,
            guide_progress,
            locale,
            now_secs,
        );
    }
    if onboarding_visible {
        render_player_onboarding_card(
            context,
            onboarding,
            guide_step,
            guide_progress,
            layout_state,
            locale,
            now_secs,
        );
    }
}
