use super::*;

fn player_connection_color(status: &crate::ConnectionStatus) -> egui::Color32 {
    match status {
        crate::ConnectionStatus::Connected => egui::Color32::from_rgb(36, 130, 72),
        crate::ConnectionStatus::Connecting => egui::Color32::from_rgb(160, 116, 40),
        crate::ConnectionStatus::Error(_) => egui::Color32::from_rgb(170, 58, 58),
    }
}

fn player_connection_text(
    status: &crate::ConnectionStatus,
    locale: crate::i18n::UiLocale,
) -> &'static str {
    match (status, locale.is_zh()) {
        (crate::ConnectionStatus::Connected, true) => "已连接",
        (crate::ConnectionStatus::Connected, false) => "Connected",
        (crate::ConnectionStatus::Connecting, true) => "连接中",
        (crate::ConnectionStatus::Connecting, false) => "Connecting",
        (crate::ConnectionStatus::Error(_), true) => "连接异常",
        (crate::ConnectionStatus::Error(_), false) => "Connection Error",
    }
}

fn player_selection_text(selection: &ViewerSelection, locale: crate::i18n::UiLocale) -> String {
    let Some(current) = selection.current.as_ref() else {
        return if locale.is_zh() {
            "未选择".to_string()
        } else {
            "None".to_string()
        };
    };
    let id = super::super::truncate_observe_text(&current.id, 16);
    format!("{} {id}", selection_kind_label(current.kind))
}

fn player_identity_text(
    state: &ViewerState,
    post_onboarding_ready: bool,
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> String {
    if let Some(claimer_agent_id) = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.player_gameplay.as_ref())
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .map(|claim| super::super::truncate_observe_text(&claim.claimer_agent_id, 16))
    {
        return if locale.is_zh() {
            format!("{} 的首条产线负责人", claimer_agent_id)
        } else {
            format!("{claimer_agent_id}'s first-line lead")
        };
    }

    if post_onboarding_ready {
        return if locale.is_zh() {
            "首条工业线负责人".to_string()
        } else {
            "First-line lead".to_string()
        };
    }

    match (step, locale.is_zh()) {
        (PlayerGuideStep::ConnectWorld, true) => "新到场玩家".to_string(),
        (PlayerGuideStep::ConnectWorld, false) => "New arrival".to_string(),
        (PlayerGuideStep::OpenPanel, true) | (PlayerGuideStep::SelectTarget, true) => {
            "前线指挥员".to_string()
        }
        (PlayerGuideStep::OpenPanel, false) | (PlayerGuideStep::SelectTarget, false) => {
            "Field commander".to_string()
        }
        (PlayerGuideStep::ExploreAction, true) => "行动指挥员".to_string(),
        (PlayerGuideStep::ExploreAction, false) => "Action commander".to_string(),
    }
}

fn player_objective_text(
    state: &ViewerState,
    post_onboarding_ready: bool,
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
) -> String {
    if post_onboarding_ready {
        return build_player_post_onboarding_snapshot(state, None, locale)
            .title
            .to_string();
    }

    player_goal_title(step, locale).to_string()
}

pub(super) fn player_current_tick(state: &ViewerState) -> u64 {
    state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0)
}

pub(super) fn build_player_hud_snapshot(
    state: &ViewerState,
    selection: &ViewerSelection,
    step: PlayerGuideStep,
    post_onboarding_ready: bool,
    locale: crate::i18n::UiLocale,
) -> PlayerHudSnapshot {
    PlayerHudSnapshot {
        role: player_identity_text(state, post_onboarding_ready, step, locale),
        connection: player_connection_text(&state.status, locale).to_string(),
        tick: player_current_tick(state),
        events: state.events.len(),
        selection: player_selection_text(selection, locale),
        objective: player_objective_text(state, post_onboarding_ready, step, locale),
    }
}

fn render_hud_chip(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    tone: egui::Color32,
    emphasized: bool,
) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(22, 31, 45))
        .stroke(egui::Stroke::new(1.0, tone))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(7))
        .show(ui, |ui| {
            ui.small(egui::RichText::new(label).color(tone));
            if emphasized {
                ui.strong(value);
            } else {
                ui.small(value);
            }
        });
}

pub(super) fn player_entry_card_style(now_secs: f64) -> (egui::Color32, egui::Stroke) {
    let pulse = ((now_secs * 2.0).sin() * 0.5 + 0.5) as f32;
    let fill = egui::Color32::from_rgb(
        18,
        (28.0 + pulse * 8.0).round() as u8,
        (40.0 + pulse * 12.0).round() as u8,
    );
    let stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgb(
            (58.0 + pulse * 32.0).round() as u8,
            (106.0 + pulse * 28.0).round() as u8,
            (152.0 + pulse * 40.0).round() as u8,
        ),
    );
    (fill, stroke)
}

pub(super) fn render_player_compact_hud(
    context: &egui::Context,
    state: &ViewerState,
    selection: &ViewerSelection,
    step: PlayerGuideStep,
    post_onboarding_ready: bool,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let snapshot = build_player_hud_snapshot(state, selection, step, post_onboarding_ready, locale);
    let objective_color = player_goal_color(step);
    let pulse = ((now_secs * 1.6).sin() * 0.5 + 0.5) as f32;
    let accent = egui::Color32::from_rgba_unmultiplied(
        objective_color.r(),
        objective_color.g(),
        objective_color.b(),
        (136.0 + 72.0 * pulse) as u8,
    );

    egui::Area::new(egui::Id::new("viewer-player-compact-hud"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 10.0))
        .movable(false)
        .interactable(false)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(12, 20, 30))
                .stroke(egui::Stroke::new(1.0, accent))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_max_width(PLAYER_HUD_MAX_WIDTH);
                    ui.horizontal_wrapped(|ui| {
                        render_hud_chip(
                            ui,
                            if locale.is_zh() { "身份" } else { "Role" },
                            snapshot.role.as_str(),
                            egui::Color32::from_rgb(206, 176, 92),
                            false,
                        );
                        render_hud_chip(
                            ui,
                            if locale.is_zh() { "连接" } else { "Conn" },
                            snapshot.connection.as_str(),
                            player_connection_color(&state.status),
                            false,
                        );
                        render_hud_chip(
                            ui,
                            if locale.is_zh() { "Tick" } else { "Tick" },
                            snapshot.tick.to_string().as_str(),
                            egui::Color32::from_rgb(112, 160, 224),
                            true,
                        );
                        render_hud_chip(
                            ui,
                            if locale.is_zh() { "事件" } else { "Events" },
                            snapshot.events.to_string().as_str(),
                            egui::Color32::from_rgb(114, 188, 166),
                            false,
                        );
                        render_hud_chip(
                            ui,
                            if locale.is_zh() { "目标" } else { "Target" },
                            snapshot.selection.as_str(),
                            egui::Color32::from_rgb(152, 178, 232),
                            false,
                        );
                        render_hud_chip(
                            ui,
                            if locale.is_zh() {
                                "当前目标"
                            } else {
                                "Objective"
                            },
                            snapshot.objective.as_str(),
                            objective_color,
                            false,
                        );
                    });
                });
        });
}

pub(super) fn should_show_player_onboarding_card(
    onboarding: &PlayerOnboardingState,
    step: PlayerGuideStep,
) -> bool {
    onboarding.dismissed_step != Some(step)
}

pub(super) fn dismiss_player_onboarding_step(
    onboarding: &mut PlayerOnboardingState,
    step: PlayerGuideStep,
) {
    onboarding.dismissed_step = Some(step);
}

pub(super) fn should_show_player_goal_hint(
    onboarding: &PlayerOnboardingState,
    step: PlayerGuideStep,
    layout_state: &RightPanelLayoutState,
) -> bool {
    layout_state.panel_hidden && !should_show_player_onboarding_card(onboarding, step)
}

pub(super) fn sync_player_stuck_hint_state(
    onboarding: &mut PlayerOnboardingState,
    state: &ViewerState,
    now_secs: f64,
) -> Option<f64> {
    if !matches!(state.status, crate::ConnectionStatus::Connected) {
        onboarding.no_progress_watch = PlayerNoProgressWatch::default();
        return None;
    }
    let watch = &mut onboarding.no_progress_watch;
    let current_tick = player_current_tick(state);
    let current_event_count = state.events.len();
    let Some(last_progress_at) = watch.last_progress_at_secs else {
        watch.last_tick = current_tick;
        watch.last_event_count = current_event_count;
        watch.last_progress_at_secs = Some(now_secs);
        return None;
    };

    let tick_advanced = current_tick > watch.last_tick;
    let events_advanced = current_event_count > watch.last_event_count;
    if tick_advanced || events_advanced {
        watch.last_tick = current_tick;
        watch.last_event_count = current_event_count;
        watch.last_progress_at_secs = Some(now_secs);
        return None;
    }

    let idle_secs = (now_secs - last_progress_at).max(0.0);
    if idle_secs >= PLAYER_STUCK_HINT_SECS {
        Some(idle_secs)
    } else {
        None
    }
}

pub(super) fn build_player_stuck_hint(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
    idle_secs: f64,
) -> String {
    build_player_stuck_hint_with_diagnosis(step, locale, idle_secs, None)
}

pub(super) fn build_player_stuck_hint_with_diagnosis(
    step: PlayerGuideStep,
    locale: crate::i18n::UiLocale,
    idle_secs: f64,
    diagnosis: Option<&PlayerNoProgressDiagnosis>,
) -> String {
    let idle_secs = idle_secs.round() as u64;
    let base = match (step, locale.is_zh()) {
        (PlayerGuideStep::ExploreAction, true) => {
            format!("检测到 {idle_secs} 秒无进展：点击“执行下一步”或“直接指挥 Agent”恢复推进")
        }
        (PlayerGuideStep::ExploreAction, false) => format!(
            "No progress for {idle_secs}s: click \"Do next step\" or \"Command Agent\" to resume"
        ),
        (_, true) => {
            format!("检测到 {idle_secs} 秒无进展：先完成当前主任务步骤再继续")
        }
        (_, false) => {
            format!("No progress for {idle_secs}s: complete the current main step to recover")
        }
    };
    match diagnosis {
        Some(diagnosis) if locale.is_zh() => {
            format!(
                "{base}｜原因：{}｜建议：{}",
                diagnosis.reason, diagnosis.suggestion
            )
        }
        Some(diagnosis) => {
            format!(
                "{base} | Cause: {} | Next: {}",
                diagnosis.reason, diagnosis.suggestion
            )
        }
        None => base,
    }
}

pub(super) fn sync_player_first_session_summary_state(
    onboarding: &mut PlayerOnboardingState,
    state: &ViewerState,
    progress: PlayerGuideProgressSnapshot,
    now_secs: f64,
) {
    if onboarding.first_session_started_at_secs.is_none()
        && matches!(state.status, crate::ConnectionStatus::Connected)
    {
        onboarding.first_session_started_at_secs = Some(now_secs);
        onboarding.first_session_start_tick = player_current_tick(state);
        onboarding.first_session_start_event_count = state.events.len();
    }

    if progress.explore_ready && !onboarding.first_session_summary_shown {
        onboarding.first_session_summary_visible = true;
        onboarding.first_session_summary_shown = true;
    }
}

pub(super) fn dismiss_player_first_session_summary(onboarding: &mut PlayerOnboardingState) {
    onboarding.first_session_summary_visible = false;
}

#[cfg(test)]
pub(super) fn player_first_session_summary_visible(onboarding: &PlayerOnboardingState) -> bool {
    onboarding.first_session_summary_visible
}

pub(super) fn build_player_first_session_summary_snapshot(
    onboarding: &PlayerOnboardingState,
    state: &ViewerState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) -> Option<PlayerFirstSessionSummarySnapshot> {
    if !onboarding.first_session_summary_visible {
        return None;
    }
    let started_at = onboarding.first_session_started_at_secs?;
    let duration_secs = (now_secs - started_at).max(0.0).round() as u64;
    let tick_gain = player_current_tick(state).saturating_sub(onboarding.first_session_start_tick);
    let event_gain = state
        .events
        .len()
        .saturating_sub(onboarding.first_session_start_event_count);
    Some(match locale.is_zh() {
        true => PlayerFirstSessionSummarySnapshot {
            duration_secs,
            tick_gain,
            event_gain,
            title: "首局回顾：已进入 PostOnboarding 阶段",
            detail: format!(
                "用时约 {} 秒，世界推进 +{} tick，新增 {} 条反馈事件；你现在开始负责首条工业线。",
                duration_secs, tick_gain, event_gain
            ),
            next_tip:
                "下一阶段目标：先看首屏主目标，再追首个产出奖励、一次恢复确认，或当前阻塞的真实代价。"
                    .to_string(),
        },
        false => PlayerFirstSessionSummarySnapshot {
            duration_secs,
            tick_gain,
            event_gain,
            title: "First Session Recap: PostOnboarding unlocked",
            detail: format!(
                "About {}s, world advanced +{} ticks, and {} new feedback events; you now own the first industrial line.",
                duration_secs, tick_gain, event_gain
            ),
            next_tip:
                "Next stage goal: read the first screen first, then chase the first output reward, one confirmed recovery, or the real cost behind the current blocker."
                    .to_string(),
        },
    })
}

pub(super) fn render_player_first_session_summary(
    context: &egui::Context,
    onboarding: &mut PlayerOnboardingState,
    state: &ViewerState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let Some(summary) =
        build_player_first_session_summary_snapshot(onboarding, state, locale, now_secs)
    else {
        return;
    };
    let mut close_clicked = false;
    egui::Area::new(egui::Id::new("viewer-player-first-session-summary"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 86.0))
        .movable(false)
        .interactable(true)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(22, 34, 30, 236))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(86, 174, 126),
                ))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(460.0);
                    ui.small(
                        egui::RichText::new(if locale.is_zh() {
                            "首局结算"
                        } else {
                            "Session Summary"
                        })
                        .color(egui::Color32::from_rgb(142, 220, 174)),
                    );
                    ui.strong(summary.title);
                    ui.small(summary.detail.as_str());
                    ui.small(summary.next_tip.as_str());
                    ui.horizontal_wrapped(|ui| {
                        close_clicked = ui
                            .button(if locale.is_zh() {
                                "进入下一阶段"
                            } else {
                                "Enter Next Stage"
                            })
                            .clicked();
                    });
                });
        });
    if close_clicked {
        dismiss_player_first_session_summary(onboarding);
    }
}

#[cfg(test)]
pub(super) fn feedback_toast_cap() -> usize {
    FEEDBACK_TOAST_MAX
}

#[cfg(test)]
pub(super) fn feedback_toast_len(feedback: &FeedbackToastState) -> usize {
    feedback.toasts.len()
}

#[cfg(test)]
pub(super) fn feedback_toast_ids(feedback: &FeedbackToastState) -> Vec<u64> {
    feedback.toasts.iter().map(|toast| toast.id).collect()
}

#[cfg(test)]
pub(super) fn feedback_last_seen_event_id(feedback: &FeedbackToastState) -> Option<u64> {
    feedback.last_seen_event_id
}

#[cfg(test)]
pub(super) fn feedback_action_feedback_seen(feedback: &FeedbackToastState) -> bool {
    feedback.action_feedback_seen
}

#[cfg(test)]
pub(super) fn feedback_toast_snapshot(
    feedback: &FeedbackToastState,
    index: usize,
) -> Option<(u64, FeedbackTone, &'static str)> {
    feedback
        .toasts
        .get(index)
        .map(|toast| (toast.id, toast.tone, toast.title))
}

#[cfg(test)]
pub(super) fn feedback_toast_detail(feedback: &FeedbackToastState, index: usize) -> Option<String> {
    feedback.toasts.get(index).map(|toast| toast.detail.clone())
}

#[cfg(test)]
pub(super) fn player_achievement_popup_cap() -> usize {
    PLAYER_ACHIEVEMENT_MAX
}

#[cfg(test)]
pub(super) fn player_achievement_popup_len(achievements: &PlayerAchievementState) -> usize {
    achievements.toasts.len()
}

#[cfg(test)]
pub(super) fn player_achievement_popup_milestones(
    achievements: &PlayerAchievementState,
) -> Vec<PlayerAchievementMilestone> {
    achievements
        .toasts
        .iter()
        .map(|toast| toast.milestone)
        .collect()
}

#[cfg(test)]
pub(super) fn player_achievement_is_unlocked(
    achievements: &PlayerAchievementState,
    milestone: PlayerAchievementMilestone,
) -> bool {
    achievements.unlocked.contains(&milestone)
}

#[cfg(test)]
pub(super) fn player_agent_chatter_cap() -> usize {
    AGENT_CHATTER_MAX
}

#[cfg(test)]
pub(super) fn player_agent_chatter_len(achievements: &PlayerAchievementState) -> usize {
    achievements.chatter.bubbles.len()
}

#[cfg(test)]
pub(super) fn player_agent_chatter_last_seen_event_id(
    achievements: &PlayerAchievementState,
) -> Option<u64> {
    achievements.chatter.last_seen_event_id
}

#[cfg(test)]
pub(super) fn player_agent_chatter_ids(achievements: &PlayerAchievementState) -> Vec<u64> {
    achievements
        .chatter
        .bubbles
        .iter()
        .map(|bubble| bubble.id)
        .collect()
}

#[cfg(test)]
pub(super) fn player_agent_chatter_snapshot(
    achievements: &PlayerAchievementState,
    index: usize,
) -> Option<(u64, FeedbackTone, String, String)> {
    achievements.chatter.bubbles.get(index).map(|bubble| {
        (
            bubble.id,
            bubble.tone,
            bubble.speaker.clone(),
            bubble.line.clone(),
        )
    })
}

pub(super) fn render_player_goal_hint(
    context: &egui::Context,
    onboarding: &PlayerOnboardingState,
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let tone = player_goal_color(step);
    let motion =
        build_player_card_transition_snapshot(&onboarding.guide_transition, step, now_secs, 0.8);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    egui::Area::new(egui::Id::new("viewer-player-next-goal"))
        .anchor(
            egui::Align2::LEFT_BOTTOM,
            egui::vec2(14.0, -14.0 + motion.slide_px),
        )
        .movable(false)
        .interactable(false)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(
                    15,
                    20,
                    30,
                    to_u8(224.0 * motion.alpha),
                ))
                .stroke(egui::Stroke::new(
                    1.0 + 0.4 * motion.pulse,
                    egui::Color32::from_rgba_unmultiplied(
                        tone.r(),
                        tone.g(),
                        tone.b(),
                        to_u8((152.0 + 84.0 * motion.pulse) * motion.alpha),
                    ),
                ))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(9))
                .show(ui, |ui| {
                    ui.set_max_width(PLAYER_GOAL_HINT_MAX_WIDTH);
                    ui.small(egui::RichText::new(player_goal_badge(locale)).color(tone));
                    ui.strong(player_goal_title(step, locale));
                    ui.small(player_goal_detail(step, locale));
                    render_player_guide_progress_lines(ui, locale, progress, step, tone);
                });
        });
}

pub(super) fn render_player_onboarding_card(
    context: &egui::Context,
    onboarding: &mut PlayerOnboardingState,
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    layout_state: &mut RightPanelLayoutState,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    if !should_show_player_onboarding_card(onboarding, step) {
        return;
    }

    let tone = player_goal_color(step);
    let motion =
        build_player_card_transition_snapshot(&onboarding.guide_transition, step, now_secs, 1.2);
    let to_u8 = |value: f32| (value.clamp(0.0, 255.0)) as u8;
    let mut primary_clicked = false;
    let mut dismiss_clicked = false;
    egui::Area::new(egui::Id::new("viewer-player-onboarding"))
        .anchor(
            egui::Align2::LEFT_TOP,
            egui::vec2(14.0, 14.0 - motion.slide_px),
        )
        .movable(false)
        .interactable(true)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(
                    19,
                    26,
                    38,
                    to_u8(236.0 * motion.alpha),
                ))
                .stroke(egui::Stroke::new(
                    1.0 + 0.45 * motion.pulse,
                    egui::Color32::from_rgba_unmultiplied(
                        tone.r(),
                        tone.g(),
                        tone.b(),
                        to_u8((150.0 + 90.0 * motion.pulse) * motion.alpha),
                    ),
                ))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(PLAYER_ONBOARDING_MAX_WIDTH);
                    ui.small(
                        egui::RichText::new(player_onboarding_title(locale))
                            .strong()
                            .color(tone),
                    );
                    ui.strong(player_goal_title(step, locale));
                    ui.label(player_goal_detail(step, locale));
                    render_player_guide_progress_lines(ui, locale, progress, step, tone);
                    ui.horizontal_wrapped(|ui| {
                        primary_clicked = ui
                            .button(player_onboarding_primary_action(step, locale))
                            .clicked();
                        dismiss_clicked = ui.button(player_onboarding_dismiss(locale)).clicked();
                    });
                });
        });

    if primary_clicked && step == PlayerGuideStep::OpenPanel {
        layout_state.panel_hidden = false;
    }

    if primary_clicked || dismiss_clicked {
        dismiss_player_onboarding_step(onboarding, step);
    }
}
