use super::*;

impl ViewerRuntimeLiveServer {
    pub(super) fn set_latest_player_gameplay_feedback(
        &mut self,
        feedback: PlayerGameplayRecentFeedback,
    ) {
        self.set_latest_player_gameplay_feedback_with_causality(feedback, None);
    }

    pub(super) fn make_player_gameplay_feedback(
        action: impl Into<String>,
        stage: impl Into<String>,
        effect: impl Into<String>,
        intent_summary: Option<String>,
        target_agent_id: Option<String>,
        reason: Option<String>,
        hint: Option<String>,
        delta_logical_time: u64,
        delta_event_seq: u64,
    ) -> PlayerGameplayRecentFeedback {
        PlayerGameplayRecentFeedback {
            action: action.into(),
            stage: stage.into(),
            effect: effect.into(),
            intent_summary,
            target_agent_id,
            reason,
            hint,
            delta_logical_time,
            delta_event_seq,
        }
    }

    pub(super) fn set_latest_player_gameplay_feedback_with_causality(
        &mut self,
        feedback: PlayerGameplayRecentFeedback,
        causality: Option<PlayerGameplayCausalitySignal>,
    ) {
        if feedback.delta_logical_time > 0 || feedback.delta_event_seq > 0 {
            self.confirm_player_gameplay_progress();
        }
        self.latest_player_gameplay_causality = causality;
        self.latest_player_gameplay_feedback = Some(feedback);
    }

    pub(super) fn record_chain_sync_failure(&mut self, error: &ViewerRuntimeLiveServerError) {
        let reason = match error {
            ViewerRuntimeLiveServerError::Serde(message) => message.clone(),
            ViewerRuntimeLiveServerError::Runtime(err) => format!("{err:?}"),
            ViewerRuntimeLiveServerError::Init(message) => message.clone(),
            ViewerRuntimeLiveServerError::Io(err) => err.to_string(),
        };
        let hint = if reason.contains("execution world is not ready") {
            "wait for the execution world persistence files to appear, or restart/repair the chain runtime bootstrap before refreshing gameplay"
                .to_string()
        } else {
            "repair the chain runtime sync path, then refresh gameplay to confirm the committed world is available"
                .to_string()
        };
        self.set_latest_player_gameplay_feedback(Self::make_player_gameplay_feedback(
            "chain_sync",
            "blocked",
            "committed runtime sync failed before the viewer could observe new world state",
            Some("refresh committed world state".to_string()),
            None,
            Some(reason),
            Some(hint),
            0,
            0,
        ));
    }

    pub(super) fn clear_chain_sync_failure_feedback(&mut self) {
        if self
            .latest_player_gameplay_feedback
            .as_ref()
            .is_some_and(|feedback| feedback.action == "chain_sync")
        {
            self.latest_player_gameplay_feedback = None;
        }
    }

    pub(super) fn confirm_player_gameplay_progress(&mut self) {
        self.confirmed_player_gameplay_progress_time = Some(self.world.state().time);
    }
}
