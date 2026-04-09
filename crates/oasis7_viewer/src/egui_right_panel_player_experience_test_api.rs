use super::{
    egui_right_panel_player_experience_hud, FeedbackToastState, FeedbackTone,
    PlayerAchievementMilestone, PlayerAchievementState, PlayerOnboardingState,
};

pub(crate) fn feedback_toast_cap() -> usize {
    egui_right_panel_player_experience_hud::feedback_toast_cap()
}

pub(crate) fn feedback_toast_len(feedback: &FeedbackToastState) -> usize {
    egui_right_panel_player_experience_hud::feedback_toast_len(feedback)
}

pub(crate) fn feedback_toast_ids(feedback: &FeedbackToastState) -> Vec<u64> {
    egui_right_panel_player_experience_hud::feedback_toast_ids(feedback)
}

pub(crate) fn feedback_last_seen_event_id(feedback: &FeedbackToastState) -> Option<u64> {
    egui_right_panel_player_experience_hud::feedback_last_seen_event_id(feedback)
}

pub(crate) fn feedback_action_feedback_seen(feedback: &FeedbackToastState) -> bool {
    egui_right_panel_player_experience_hud::feedback_action_feedback_seen(feedback)
}

pub(crate) fn feedback_toast_snapshot(
    feedback: &FeedbackToastState,
    index: usize,
) -> Option<(u64, FeedbackTone, &'static str)> {
    egui_right_panel_player_experience_hud::feedback_toast_snapshot(feedback, index)
}

pub(crate) fn feedback_toast_detail(feedback: &FeedbackToastState, index: usize) -> Option<String> {
    egui_right_panel_player_experience_hud::feedback_toast_detail(feedback, index)
}

pub(crate) fn player_achievement_popup_cap() -> usize {
    egui_right_panel_player_experience_hud::player_achievement_popup_cap()
}

pub(crate) fn player_achievement_popup_len(achievements: &PlayerAchievementState) -> usize {
    egui_right_panel_player_experience_hud::player_achievement_popup_len(achievements)
}

pub(crate) fn player_achievement_popup_milestones(
    achievements: &PlayerAchievementState,
) -> Vec<PlayerAchievementMilestone> {
    egui_right_panel_player_experience_hud::player_achievement_popup_milestones(achievements)
}

pub(crate) fn player_achievement_is_unlocked(
    achievements: &PlayerAchievementState,
    milestone: PlayerAchievementMilestone,
) -> bool {
    egui_right_panel_player_experience_hud::player_achievement_is_unlocked(achievements, milestone)
}

pub(crate) fn player_agent_chatter_cap() -> usize {
    egui_right_panel_player_experience_hud::player_agent_chatter_cap()
}

pub(crate) fn player_agent_chatter_len(achievements: &PlayerAchievementState) -> usize {
    egui_right_panel_player_experience_hud::player_agent_chatter_len(achievements)
}

pub(crate) fn player_agent_chatter_last_seen_event_id(
    achievements: &PlayerAchievementState,
) -> Option<u64> {
    egui_right_panel_player_experience_hud::player_agent_chatter_last_seen_event_id(achievements)
}

pub(crate) fn player_agent_chatter_ids(achievements: &PlayerAchievementState) -> Vec<u64> {
    egui_right_panel_player_experience_hud::player_agent_chatter_ids(achievements)
}

pub(crate) fn player_agent_chatter_snapshot(
    achievements: &PlayerAchievementState,
    index: usize,
) -> Option<(u64, FeedbackTone, String, String)> {
    egui_right_panel_player_experience_hud::player_agent_chatter_snapshot(achievements, index)
}

pub(crate) fn player_first_session_summary_visible(onboarding: &PlayerOnboardingState) -> bool {
    egui_right_panel_player_experience_hud::player_first_session_summary_visible(onboarding)
}
