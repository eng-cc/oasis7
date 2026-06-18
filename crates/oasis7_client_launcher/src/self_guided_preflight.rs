use super::*;
use config_ui::StartupGuideTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreflightCheckState {
    Pass,
    Blocked,
}

pub(super) fn resolve_chain_runtime_preflight_state(
    chain_enabled: bool,
    chain_status: &ChainRuntimeStatus,
) -> PreflightCheckState {
    if !chain_enabled {
        return PreflightCheckState::Blocked;
    }
    if matches!(chain_status, ChainRuntimeStatus::Ready) {
        PreflightCheckState::Pass
    } else {
        PreflightCheckState::Blocked
    }
}

impl ClientLauncherApp {
    fn preflight_state_text(&self, state: PreflightCheckState) -> &'static str {
        match (state, self.ui_language) {
            (PreflightCheckState::Pass, UiLanguage::ZhCn) => "通过",
            (PreflightCheckState::Pass, UiLanguage::EnUs) => "Pass",
            (PreflightCheckState::Blocked, UiLanguage::ZhCn) => "阻断",
            (PreflightCheckState::Blocked, UiLanguage::EnUs) => "Blocked",
        }
    }

    fn preflight_state_color(&self, state: PreflightCheckState) -> egui::Color32 {
        match state {
            PreflightCheckState::Pass => egui::Color32::from_rgb(62, 152, 92),
            PreflightCheckState::Blocked => egui::Color32::from_rgb(188, 60, 60),
        }
    }

    fn preflight_row_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(219, 226, 236),
            ))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add_contents(ui);
            });
    }

    fn render_preflight_state_pill(
        ui: &mut egui::Ui,
        text: &str,
        color: egui::Color32,
    ) -> egui::Response {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(247, 251, 247))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(218, 224, 217),
            ))
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.small(egui::RichText::new(text).strong().color(color));
            })
            .response
    }

    fn preflight_icon(ui: &mut egui::Ui, state: PreflightCheckState) {
        let (icon, fill, color) = match state {
            PreflightCheckState::Pass => (
                "✓",
                egui::Color32::from_rgb(232, 247, 237),
                egui::Color32::from_rgb(46, 150, 82),
            ),
            PreflightCheckState::Blocked => (
                "!",
                egui::Color32::from_rgb(255, 246, 224),
                egui::Color32::from_rgb(221, 139, 23),
            ),
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(217, 225, 235),
            ))
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(icon).strong().size(17.0).color(color));
            });
    }

    fn preflight_action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(egui::RichText::new(label).size(10.0))
                .fill(egui::Color32::from_rgb(255, 255, 255))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(213, 222, 232),
                ))
                .min_size(egui::vec2(56.0, 24.0)),
        )
    }

    fn render_preflight_game_config_row(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
    ) {
        let state = if game_required_issues.is_empty() {
            PreflightCheckState::Pass
        } else {
            PreflightCheckState::Blocked
        };
        Self::preflight_row_frame(ui, |ui| {
            ui.horizontal(|ui| {
                Self::preflight_icon(ui, state);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(self.tr("游戏配置", "Game Configuration"))
                            .strong()
                            .size(12.0),
                    );
                    ui.horizontal(|ui| {
                        ui.small(self.tr("状态", "Status"));
                        Self::render_preflight_state_pill(
                            ui,
                            self.preflight_state_text(state),
                            self.preflight_state_color(state),
                        );
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state == PreflightCheckState::Blocked
                        && Self::preflight_action_button(ui, self.tr("修复", "Fix")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.open_game_config_guide();
                    }
                });
            });
            if state == PreflightCheckState::Blocked {
                ui.small(format!(
                    "{}={}",
                    self.tr("问题数", "issues"),
                    game_required_issues.len()
                ));
                ui.horizontal(|ui| {
                    if Self::preflight_action_button(ui, self.tr("自动补默", "Autofill")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.apply_safe_defaults_for_startup_target(StartupGuideTarget::Game);
                    }
                });
            } else {
                ui.small(self.tr("必要字段已通过。", "Required fields passed."));
            }
        });
    }

    fn render_preflight_chain_config_row(
        &mut self,
        ui: &mut egui::Ui,
        chain_required_issues: &[ConfigIssue],
    ) {
        let state = if self.config.chain_enabled && chain_required_issues.is_empty() {
            PreflightCheckState::Pass
        } else {
            PreflightCheckState::Blocked
        };
        Self::preflight_row_frame(ui, |ui| {
            ui.horizontal(|ui| {
                Self::preflight_icon(ui, state);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(self.tr("区块链配置", "Blockchain Configuration"))
                            .strong()
                            .size(12.0),
                    );
                    ui.horizontal(|ui| {
                        ui.small(self.tr("状态", "Status"));
                        Self::render_preflight_state_pill(
                            ui,
                            self.preflight_state_text(state),
                            self.preflight_state_color(state),
                        );
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.config.chain_enabled
                        && Self::preflight_action_button(ui, self.tr("启用", "Enable")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.config.chain_enabled = true;
                        self.config.normalize();
                        self.chain_runtime_status =
                            if chain_runtime_effectively_enabled(&self.config) {
                                ChainRuntimeStatus::NotStarted
                            } else {
                                ChainRuntimeStatus::Disabled
                            };
                    } else if state == PreflightCheckState::Blocked
                        && Self::preflight_action_button(ui, self.tr("修复", "Fix")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.open_chain_config_guide();
                    }
                });
            });
            if !self.config.chain_enabled {
                ui.small(self.tr("链功能当前未启用。", "Blockchain is currently disabled."));
                return;
            }
            if state == PreflightCheckState::Blocked {
                ui.small(format!(
                    "{}={}",
                    self.tr("问题数", "issues"),
                    chain_required_issues.len()
                ));
                ui.horizontal(|ui| {
                    if Self::preflight_action_button(ui, self.tr("自动补默", "Autofill")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.apply_safe_defaults_for_startup_target(StartupGuideTarget::Chain);
                    }
                });
            } else {
                ui.small(self.tr("链配置满足启动要求。", "Chain config is launch-ready."));
            }
        });
    }

    fn render_preflight_chain_runtime_row(
        &mut self,
        ui: &mut egui::Ui,
        chain_required_issues: &[ConfigIssue],
    ) {
        let state = resolve_chain_runtime_preflight_state(
            self.config.chain_enabled,
            &self.chain_runtime_status,
        );
        Self::preflight_row_frame(ui, |ui| {
            ui.horizontal(|ui| {
                Self::preflight_icon(ui, state);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(self.tr("链状态依赖", "Chain Dependency"))
                            .strong()
                            .size(12.0),
                    );
                    ui.horizontal(|ui| {
                        ui.small(self.tr("状态", "Status"));
                        Self::render_preflight_state_pill(
                            ui,
                            self.preflight_state_text(state),
                            self.preflight_state_color(state),
                        );
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state == PreflightCheckState::Blocked
                        && Self::preflight_action_button(ui, self.tr("启动链", "Start")).clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.handle_start_chain_click(chain_required_issues);
                    }
                });
            });
            ui.small(self.tr(
                "反馈/转账/浏览器依赖链就绪。",
                "Feedback/transfer/explorer depend on chain readiness.",
            ));
            if state == PreflightCheckState::Blocked {
                if Self::preflight_action_button(ui, self.tr("重试探测", "Retry")).clicked() {
                    self.record_guided_quick_action_click();
                    self.trigger_state_refresh();
                    self.update_chain_runtime_status();
                }
            }
        });
    }

    pub(super) fn render_startup_preflight_checklist(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(self.tr("启动前检查", "Preflight Checklist"))
                    .strong()
                    .size(14.0),
            );
            self.render_preflight_game_config_row(ui, game_required_issues);
            self.render_preflight_chain_config_row(ui, chain_required_issues);
            self.render_preflight_chain_runtime_row(ui, chain_required_issues);
        });
    }
}
