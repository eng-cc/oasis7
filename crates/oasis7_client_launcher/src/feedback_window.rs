use super::*;
use crate::feedback_entry::{
    collect_recent_logs, submit_feedback_with_fallback, validate_feedback_draft,
    FeedbackDraftIssue, FeedbackKind, FeedbackSubmitResult,
};

fn sanitized_launch_config_snapshot(config: &LaunchConfig) -> Result<serde_json::Value, String> {
    let mut value =
        serde_json::to_value(config).map_err(|err| format!("config serialization error: {err}"))?;
    if let Some(object) = value.as_object_mut() {
        if let Some(token) = object.get_mut("agent_provider_auth_token") {
            if !token.as_str().unwrap_or_default().is_empty() {
                *token = serde_json::Value::String("<redacted>".to_string());
            }
        }
    }
    Ok(value)
}

impl ClientLauncherApp {
    pub(super) fn feedback_kind_label(&self, kind: FeedbackKind) -> &'static str {
        match (kind, self.ui_language) {
            (FeedbackKind::Bug, UiLanguage::ZhCn) => "Bug",
            (FeedbackKind::Bug, UiLanguage::EnUs) => "Bug",
            (FeedbackKind::Suggestion, UiLanguage::ZhCn) => "建议",
            (FeedbackKind::Suggestion, UiLanguage::EnUs) => "Suggestion",
        }
    }

    pub(super) fn feedback_issue_text(&self, issue: FeedbackDraftIssue) -> &'static str {
        match (issue, self.ui_language) {
            (FeedbackDraftIssue::TitleRequired, UiLanguage::ZhCn) => "反馈标题不能为空",
            (FeedbackDraftIssue::TitleRequired, UiLanguage::EnUs) => {
                "Feedback title cannot be empty"
            }
            (FeedbackDraftIssue::DescriptionRequired, UiLanguage::ZhCn) => "反馈描述不能为空",
            (FeedbackDraftIssue::DescriptionRequired, UiLanguage::EnUs) => {
                "Feedback description cannot be empty"
            }
            (FeedbackDraftIssue::OutputDirRequired, UiLanguage::ZhCn) => "反馈目录不能为空",
            (FeedbackDraftIssue::OutputDirRequired, UiLanguage::EnUs) => {
                "Feedback directory cannot be empty"
            }
        }
    }

    pub(super) fn submit_feedback(&mut self) {
        if !self.is_feedback_available() {
            let message = self
                .tr(
                    "反馈提交失败：区块链未就绪",
                    "Feedback submit failed: blockchain is not ready",
                )
                .to_string();
            self.append_log(message.clone());
            self.feedback_submit_state = FeedbackSubmitState::Failed(message);
            return;
        }

        let issues = validate_feedback_draft(&self.feedback_draft);
        if !issues.is_empty() {
            for issue in issues {
                self.append_log(format!(
                    "feedback validation failed: {}",
                    self.feedback_issue_text(issue)
                ));
            }
            self.feedback_submit_state = FeedbackSubmitState::Failed(
                self.tr(
                    "反馈提交失败：请先修复表单必填项",
                    "Feedback submit failed: fix required form fields first",
                )
                .to_string(),
            );
            return;
        }

        let config_snapshot = match sanitized_launch_config_snapshot(&self.config) {
            Ok(value) => value,
            Err(err) => {
                self.feedback_submit_state = FeedbackSubmitState::Failed(format!(
                    "{}: {err}",
                    self.tr(
                        "反馈提交失败：配置序列化错误",
                        "Feedback submit failed: config serialization error"
                    )
                ));
                return;
            }
        };
        let recent_logs = collect_recent_logs(&self.logs);
        match submit_feedback_with_fallback(
            &self.feedback_draft,
            config_snapshot,
            recent_logs,
            self.config.chain_enabled,
            self.config.chain_status_bind.as_str(),
        ) {
            Ok(FeedbackSubmitResult::Distributed {
                feedback_id,
                event_id,
            }) => {
                let message = format!(
                    "{}: feedback_id={feedback_id}, event_id={event_id}",
                    self.tr(
                        "反馈已提交到分布式网络",
                        "Feedback submitted to distributed network",
                    )
                );
                self.append_log(message.clone());
                self.feedback_submit_state = FeedbackSubmitState::Success(message);
            }
            Ok(FeedbackSubmitResult::Local { path, remote_error }) => {
                let fallback = remote_error.is_some();
                if let Some(remote_error) = remote_error {
                    self.append_log(format!(
                        "distributed feedback submit failed, fallback to local file: {remote_error}"
                    ));
                }
                let message = format!(
                    "{}: {}",
                    if fallback {
                        self.tr(
                            "分布式提交失败，已本地保存",
                            "Distributed submit failed; saved locally",
                        )
                    } else {
                        self.tr("反馈已保存", "Feedback saved")
                    },
                    path.display()
                );
                self.append_log(message.clone());
                self.feedback_submit_state = FeedbackSubmitState::Success(message);
            }
            Err(err) => {
                let message = format!(
                    "{}: {err}",
                    self.tr("反馈提交失败", "Feedback submit failed")
                );
                self.append_log(message.clone());
                self.feedback_submit_state = FeedbackSubmitState::Failed(message);
            }
        }
    }

    pub(super) fn show_feedback_window(&mut self, ctx: &egui::Context) {
        if !self.feedback_window_open {
            return;
        }

        let title = self
            .tr("反馈（Bug / 建议）", "Feedback (Bug / Suggestion)")
            .to_string();
        let feedback_bug_label = self.feedback_kind_label(FeedbackKind::Bug).to_string();
        let feedback_suggestion_label = self
            .feedback_kind_label(FeedbackKind::Suggestion)
            .to_string();
        let feedback_desc_hint = self
            .tr(
                "请写复现步骤、预期结果、实际结果",
                "Describe steps, expected result, and actual result",
            )
            .to_string();

        let mut window_open = self.feedback_window_open;
        egui::Window::new(title)
            .open(&mut window_open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("类型", "Type"));
                    egui::ComboBox::from_id_salt("feedback_kind_window")
                        .selected_text(self.feedback_kind_label(self.feedback_draft.kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.feedback_draft.kind,
                                FeedbackKind::Bug,
                                feedback_bug_label.as_str(),
                            );
                            ui.selectable_value(
                                &mut self.feedback_draft.kind,
                                FeedbackKind::Suggestion,
                                feedback_suggestion_label.as_str(),
                            );
                        });
                    ui.label(self.tr("标题", "Title"));
                    ui.text_edit_singleline(&mut self.feedback_draft.title);
                });
                ui.label(self.tr("描述", "Description"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.feedback_draft.description)
                        .desired_rows(4)
                        .hint_text(feedback_desc_hint),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("反馈目录", "Feedback Directory"));
                    ui.text_edit_singleline(&mut self.feedback_draft.output_dir);
                    if ui.button(self.tr("提交反馈", "Submit Feedback")).clicked() {
                        self.submit_feedback();
                    }
                });

                let feedback_issues = validate_feedback_draft(&self.feedback_draft);
                if !feedback_issues.is_empty() {
                    ui.small(
                        egui::RichText::new(self.tr(
                            "提交前请完善必填项：",
                            "Please complete required fields before submit:",
                        ))
                        .color(egui::Color32::from_rgb(196, 84, 84)),
                    );
                    for issue in feedback_issues {
                        ui.small(
                            egui::RichText::new(format!("- {}", self.feedback_issue_text(issue)))
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                }
                match &self.feedback_submit_state {
                    FeedbackSubmitState::Success(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(62, 152, 92)),
                        );
                    }
                    FeedbackSubmitState::Failed(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                    FeedbackSubmitState::None => {}
                }
            });

        self.feedback_window_open = window_open;
    }
}

#[cfg(test)]
mod tests {
    use super::sanitized_launch_config_snapshot;
    use crate::LaunchConfig;

    #[test]
    fn sanitized_launch_config_snapshot_redacts_provider_auth_token() {
        let config = LaunchConfig {
            agent_provider_auth_token: "secret-token".to_string(),
            ..LaunchConfig::default()
        };
        let snapshot = sanitized_launch_config_snapshot(&config).expect("snapshot");
        assert_eq!(
            snapshot
                .get("agent_provider_auth_token")
                .and_then(|value| value.as_str()),
            Some("<redacted>")
        );
    }
}
