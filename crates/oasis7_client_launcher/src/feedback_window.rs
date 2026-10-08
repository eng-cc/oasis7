use super::*;
use crate::feedback_entry::{
    FeedbackDraftIssue, FeedbackKind, FeedbackSubmitResult, collect_recent_logs,
    submit_feedback_with_fallback, validate_feedback_draft,
};

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

        let recent_logs = collect_recent_logs(&self.logs);
        match submit_feedback_with_fallback(&self.feedback_draft, &self.config, recent_logs) {
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
        let mut request_close = false;
        let window_size = Self::modal_window_size(ctx, 620.0, 440.0);
        egui::Window::new(title)
            .open(&mut window_open)
            .resizable(true)
            .default_size(window_size)
            .show(ctx, |ui| {
                Self::modal_header(
                    ui,
                    self.tr("反馈", "Feedback"),
                    self.tr(
                        "记录问题、建议和近期运行上下文。",
                        "Capture issues, suggestions, and recent runtime context.",
                    ),
                    Some((
                        if self.is_feedback_available() {
                            self.tr("可提交", "Ready")
                        } else {
                            self.tr("链未就绪", "Chain Pending")
                        },
                        if self.is_feedback_available() {
                            egui::Color32::from_rgb(62, 152, 92)
                        } else {
                            egui::Color32::from_rgb(201, 146, 44)
                        },
                    )),
                );
                ui.add_space(8.0);

                Self::modal_card(ui, |ui| {
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.feedback_draft.title)
                                .desired_width((ui.available_width() - 8.0).max(180.0)),
                        );
                    });
                    ui.add_space(6.0);
                    ui.label(self.tr("描述", "Description"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.feedback_draft.description)
                            .desired_rows(5)
                            .desired_width(ui.available_width())
                            .hint_text(feedback_desc_hint),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("反馈目录", "Feedback Directory"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.feedback_draft.output_dir)
                                .desired_width((ui.available_width() - 8.0).max(220.0)),
                        );
                    });
                });
                ui.add_space(8.0);

                let feedback_issues = validate_feedback_draft(&self.feedback_draft);
                if !feedback_issues.is_empty() {
                    let summary = feedback_issues
                        .iter()
                        .map(|issue| self.feedback_issue_text(*issue))
                        .collect::<Vec<_>>()
                        .join(" / ");
                    Self::modal_banner(
                        ui,
                        format!(
                            "{} {}",
                            self.tr(
                                "提交前请完善必填项：",
                                "Please complete required fields before submit:"
                            ),
                            summary
                        )
                        .as_str(),
                        egui::Color32::from_rgb(196, 84, 84),
                    );
                }
                match &self.feedback_submit_state {
                    FeedbackSubmitState::Success(message) => {
                        Self::modal_banner(
                            ui,
                            message.as_str(),
                            egui::Color32::from_rgb(62, 152, 92),
                        );
                    }
                    FeedbackSubmitState::Failed(message) => {
                        Self::modal_banner(
                            ui,
                            message.as_str(),
                            egui::Color32::from_rgb(196, 84, 84),
                        );
                    }
                    FeedbackSubmitState::None => {}
                }
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if Self::modal_primary_button(ui, self.tr("提交反馈", "Submit Feedback"))
                        .clicked()
                    {
                        self.submit_feedback();
                    }
                    if Self::modal_secondary_button(ui, self.tr("关闭", "Close")).clicked() {
                        request_close = true;
                    }
                });
            });

        if request_close {
            window_open = false;
        }
        self.feedback_window_open = window_open;
    }
}

#[cfg(test)]
mod tests {
    use crate::feedback_entry::{FeedbackDraft, FeedbackKind, submit_feedback_report};
    use crate::{ChainRuntimeStatus, ClientLauncherApp, LaunchConfig};

    fn assert_unavailable_feedback_submission_does_not_write_bundle(
        chain_enabled: bool,
        status: ChainRuntimeStatus,
        case: &str,
    ) {
        let output_dir = std::env::temp_dir().join(format!(
            "oasis7-feedback-no-write-{case}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let mut app = ClientLauncherApp::default();
        app.config.chain_enabled = chain_enabled;
        app.chain_runtime_status = status;
        app.feedback_draft.output_dir = output_dir.to_string_lossy().to_string();
        app.feedback_draft.title = "unavailable feedback".to_string();
        app.feedback_draft.description = "valid draft must still be gated".to_string();

        app.submit_feedback();

        assert!(
            !output_dir.exists(),
            "unavailable feedback submission wrote a local bundle"
        );
    }

    #[test]
    fn feedback_disabled_submit_does_not_write_local_bundle() {
        assert_unavailable_feedback_submission_does_not_write_bundle(
            false,
            ChainRuntimeStatus::Ready,
            "disabled",
        );
    }

    #[test]
    fn feedback_not_ready_submit_does_not_write_local_bundle() {
        assert_unavailable_feedback_submission_does_not_write_bundle(
            true,
            ChainRuntimeStatus::Starting,
            "starting",
        );
    }

    #[test]
    fn feedback_bundle_excludes_sensitive_config_and_log_values() {
        let config = LaunchConfig {
            agent_provider_url:
                "https://user:password@provider.private.test/v1?api_key=endpoint-secret".to_string(),
            agent_provider_auth_token: "configured-secret-token".to_string(),
            chain_status_bind: "chain.private.test:5121".to_string(),
            viewer_static_dir: "/Users/private-user/private/game-build".to_string(),
            launcher_bin: "/Users/private-user/private/oasis7-launcher".to_string(),
            ..LaunchConfig::default()
        };
        let temp_dir = std::env::temp_dir().join(format!(
            "oasis7-feedback-privacy-witness-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let draft = FeedbackDraft {
            kind: FeedbackKind::Bug,
            title: "Feedback bundle privacy regression".to_string(),
            description: "User-authored token=literal-user-text remains unchanged".to_string(),
            output_dir: temp_dir.to_string_lossy().to_string(),
        };
        let logs = vec![
            "[stderr] launcher started successfully".to_string(),
            format!(
                "[stderr] provider request failed: {} token={}",
                config.agent_provider_url, config.agent_provider_auth_token
            ),
            "[stderr] Authorization: Bearer log-bearer-secret".to_string(),
            "[stderr] previous bundle at /Users/another-user/private/feedback/report.json"
                .to_string(),
            "[stderr] prior feedback/20261008T173012Z-bug.json saved".to_string(),
            "[stderr] OPENAI_API_KEY=openai-env-secret GITHUB_TOKEN=github-env-secret".to_string(),
            "[stderr] AWS_SECRET_ACCESS_KEY=aws-secret-access-value OAUTH_CLIENT_SECRET=oauth-client-secret-value".to_string(),
            "[stderr] accessToken=camel-case-access-secret".to_string(),
            r#"[stderr] api_key="prefix-\"escaped-secret\"-suffix" safe-context=retained"#.to_string(),
            "[stderr] client_secret=\"unterminated-secret".to_string(),
        ];

        let path = submit_feedback_report(&draft, &config, logs)
            .expect("feedback bundle should be written");
        let bundle = std::fs::read_to_string(&path).expect("feedback bundle should be readable");

        for (label, sensitive_value) in [
            ("provider URL", config.agent_provider_url.as_str()),
            (
                "configured credential",
                config.agent_provider_auth_token.as_str(),
            ),
            ("chain endpoint", config.chain_status_bind.as_str()),
            ("viewer path", config.viewer_static_dir.as_str()),
            ("launcher path", config.launcher_bin.as_str()),
            ("URL query credential", "endpoint-secret"),
            ("generic bearer credential", "log-bearer-secret"),
            (
                "previous feedback bundle path",
                "/Users/another-user/private/feedback/report.json",
            ),
            (
                "relative feedback bundle path",
                "feedback/20261008T173012Z-bug.json",
            ),
            ("OpenAI environment key", "openai-env-secret"),
            ("GitHub token", "github-env-secret"),
            ("AWS secret access key", "aws-secret-access-value"),
            ("OAuth client secret", "oauth-client-secret-value"),
            ("camelCase token", "camel-case-access-secret"),
            ("escaped quoted secret", "escaped-secret"),
            ("unterminated quoted secret", "unterminated-secret"),
        ] {
            assert!(
                !bundle.contains(sensitive_value),
                "feedback bundle leaked {label}"
            );
        }
        assert!(bundle.contains("launcher started successfully"));
        assert!(bundle.contains("User-authored token=literal-user-text remains unchanged"));
        assert!(bundle.contains("<redacted>"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
