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
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr("1) 游戏配置", "1) Game Configuration"));
            ui.colored_label(
                self.preflight_state_color(state),
                self.preflight_state_text(state),
            );
            if state == PreflightCheckState::Blocked {
                ui.small(format!(
                    "{}={}",
                    self.tr("问题数", "issues"),
                    game_required_issues.len()
                ));
                if ui.button(self.tr("修复", "Fix")).clicked() {
                    self.record_guided_quick_action_click();
                    self.open_game_config_guide();
                }
                if ui
                    .button(self.tr("自动补默认值", "Autofill Defaults"))
                    .clicked()
                {
                    self.record_guided_quick_action_click();
                    self.apply_safe_defaults_for_startup_target(StartupGuideTarget::Game);
                }
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
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr("2) 区块链配置", "2) Blockchain Configuration"));
            ui.colored_label(
                self.preflight_state_color(state),
                self.preflight_state_text(state),
            );
            if !self.config.chain_enabled {
                if ui
                    .button(self.tr("启用区块链", "Enable Blockchain"))
                    .clicked()
                {
                    self.record_guided_quick_action_click();
                    self.config.chain_enabled = true;
                    self.config.normalize();
                    self.chain_runtime_status = if chain_runtime_effectively_enabled(&self.config) {
                        ChainRuntimeStatus::NotStarted
                    } else {
                        ChainRuntimeStatus::Disabled
                    };
                }
                return;
            }
            if state == PreflightCheckState::Blocked {
                ui.small(format!(
                    "{}={}",
                    self.tr("问题数", "issues"),
                    chain_required_issues.len()
                ));
                if ui.button(self.tr("修复", "Fix")).clicked() {
                    self.record_guided_quick_action_click();
                    self.open_chain_config_guide();
                }
                if ui
                    .button(self.tr("自动补默认值", "Autofill Defaults"))
                    .clicked()
                {
                    self.record_guided_quick_action_click();
                    self.apply_safe_defaults_for_startup_target(StartupGuideTarget::Chain);
                }
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
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr(
                "3) 链状态依赖（反馈/转账/浏览器）",
                "3) Chain Dependency (Feedback/Transfer/Explorer)",
            ));
            ui.colored_label(
                self.preflight_state_color(state),
                self.preflight_state_text(state),
            );
            if state == PreflightCheckState::Blocked {
                if ui
                    .button(self.tr("启动区块链", "Start Blockchain"))
                    .clicked()
                {
                    self.record_guided_quick_action_click();
                    self.handle_start_chain_click(chain_required_issues);
                }
                if ui
                    .button(self.tr("重试状态探测", "Retry Status Probe"))
                    .clicked()
                {
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
            ui.label(self.tr("启动前体检（Preflight）", "Startup Preflight Checklist"));
            self.render_preflight_game_config_row(ui, game_required_issues);
            self.render_preflight_chain_config_row(ui, chain_required_issues);
            self.render_preflight_chain_runtime_row(ui, chain_required_issues);
        });
    }
}
