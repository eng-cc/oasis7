use super::*;
use crate::self_guided::DisabledActionCta;

pub(super) fn resolve_disabled_cta_plan(
    chain_status: &ChainRuntimeStatus,
    chain_enabled: bool,
    game_required_issues: &[ConfigIssue],
    chain_required_issues: &[ConfigIssue],
) -> (Option<DisabledActionCta>, Option<DisabledActionCta>) {
    if !chain_enabled {
        return (Some(DisabledActionCta::EnableChain), None);
    }
    if !chain_required_issues.is_empty() {
        return (
            Some(DisabledActionCta::FixChainConfig),
            Some(DisabledActionCta::RetryChainStatus),
        );
    }

    match chain_status {
        ChainRuntimeStatus::NotStarted => (
            Some(DisabledActionCta::StartChain),
            Some(DisabledActionCta::RetryChainStatus),
        ),
        ChainRuntimeStatus::Starting => (
            Some(DisabledActionCta::RetryChainStatus),
            Some(DisabledActionCta::StartChain),
        ),
        ChainRuntimeStatus::StaleExecutionWorld(_) => (
            Some(DisabledActionCta::StartChain),
            Some(DisabledActionCta::RetryChainStatus),
        ),
        ChainRuntimeStatus::Unreachable(_) => (
            Some(DisabledActionCta::RetryChainStatus),
            Some(DisabledActionCta::StartChain),
        ),
        ChainRuntimeStatus::ConfigError(_) => (
            Some(DisabledActionCta::FixChainConfig),
            Some(DisabledActionCta::RetryChainStatus),
        ),
        ChainRuntimeStatus::Disabled => (Some(DisabledActionCta::EnableChain), None),
        ChainRuntimeStatus::Ready => {
            if !game_required_issues.is_empty() {
                (Some(DisabledActionCta::FixGameConfig), None)
            } else {
                (None, None)
            }
        }
    }
}

impl ClientLauncherApp {
    fn disabled_cta_text(&self, cta: DisabledActionCta) -> &'static str {
        match (cta, self.ui_language) {
            (DisabledActionCta::EnableChain, UiLanguage::ZhCn) => "启用区块链功能",
            (DisabledActionCta::EnableChain, UiLanguage::EnUs) => "Enable Blockchain",
            (DisabledActionCta::FixChainConfig, UiLanguage::ZhCn) => "修复区块链配置",
            (DisabledActionCta::FixChainConfig, UiLanguage::EnUs) => "Fix Chain Config",
            (DisabledActionCta::StartChain, UiLanguage::ZhCn) => "先启动区块链",
            (DisabledActionCta::StartChain, UiLanguage::EnUs) => "Start Blockchain First",
            (DisabledActionCta::RetryChainStatus, UiLanguage::ZhCn) => "立即重试状态探测",
            (DisabledActionCta::RetryChainStatus, UiLanguage::EnUs) => "Retry Chain Status Now",
            (DisabledActionCta::FixGameConfig, UiLanguage::ZhCn) => "修复游戏配置",
            (DisabledActionCta::FixGameConfig, UiLanguage::EnUs) => "Fix Game Config",
        }
    }

    fn disabled_cta_status_hint(&self) -> Option<&'static str> {
        match (self.chain_runtime_status.clone(), self.ui_language) {
            (ChainRuntimeStatus::Starting, UiLanguage::ZhCn) => {
                Some("区块链启动中：状态会自动重试，你也可以立即手动探测。")
            }
            (ChainRuntimeStatus::Starting, UiLanguage::EnUs) => {
                Some("Blockchain is starting: auto-retry is running, or probe now manually.")
            }
            (ChainRuntimeStatus::StaleExecutionWorld(_), UiLanguage::ZhCn) => {
                Some("检测到旧执行世界冲突：建议使用 fresh node 恢复后重试。")
            }
            (ChainRuntimeStatus::StaleExecutionWorld(_), UiLanguage::EnUs) => Some(
                "Stale execution world detected: recover with a fresh node, then retry.",
            ),
            (ChainRuntimeStatus::Unreachable(_), UiLanguage::ZhCn) => {
                Some("区块链当前不可达：可先重试状态探测，再决定是否重新启动。")
            }
            (ChainRuntimeStatus::Unreachable(_), UiLanguage::EnUs) => Some(
                "Blockchain is unreachable: retry status probe first, then decide whether to restart.",
            ),
            (ChainRuntimeStatus::NotStarted, UiLanguage::ZhCn) => {
                Some("区块链未启动：先启动区块链后，反馈/转账/浏览器将自动解锁。")
            }
            (ChainRuntimeStatus::NotStarted, UiLanguage::EnUs) => Some(
                "Blockchain is not started: start it first to unlock Feedback/Transfer/Explorer.",
            ),
            (ChainRuntimeStatus::ConfigError(_), UiLanguage::ZhCn) => {
                Some("区块链配置异常：建议先修复配置，再重试状态探测。")
            }
            (ChainRuntimeStatus::ConfigError(_), UiLanguage::EnUs) => Some(
                "Blockchain has config errors: fix config first, then retry status probe.",
            ),
            _ => None,
        }
    }

    fn handle_disabled_action_cta(
        &mut self,
        cta: DisabledActionCta,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        match cta {
            DisabledActionCta::EnableChain => {
                self.config.chain_enabled = true;
                self.config.normalize();
                self.chain_runtime_status = if chain_runtime_effectively_enabled(&self.config) {
                    ChainRuntimeStatus::NotStarted
                } else {
                    ChainRuntimeStatus::Disabled
                };
                self.append_log(self.tr(
                    "已启用区块链功能，请继续启动区块链。",
                    "Blockchain enabled. Continue with Start Blockchain.",
                ));
            }
            DisabledActionCta::FixChainConfig => {
                self.open_chain_config_guide();
                self.append_log(self.tr(
                    "已打开区块链配置引导，请先修复后再试。",
                    "Blockchain configuration guide opened. Fix it and retry.",
                ));
            }
            DisabledActionCta::StartChain => {
                self.handle_start_chain_click(chain_required_issues);
            }
            DisabledActionCta::RetryChainStatus => {
                self.trigger_state_refresh();
                self.update_chain_runtime_status();
                self.append_log(self.tr(
                    "已触发区块链状态重试探测。",
                    "Triggered blockchain status retry probe.",
                ));
            }
            DisabledActionCta::FixGameConfig => {
                self.open_game_config_guide();
                self.append_log(self.tr(
                    "已打开游戏配置引导，请先修复后再试。",
                    "Game configuration guide opened. Fix it and retry.",
                ));
                if !game_required_issues.is_empty() {
                    self.handle_start_game_click(game_required_issues);
                }
            }
        }
    }

    pub(super) fn render_disabled_action_ctas(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        _chain_running: bool,
    ) {
        if self.is_feedback_available() {
            return;
        }

        if let Some(hint) = self.feedback_unavailable_hint() {
            ui.small(egui::RichText::new(hint).color(egui::Color32::from_rgb(158, 134, 76)));
        }
        if let Some(hint) = self.disabled_cta_status_hint() {
            ui.small(egui::RichText::new(hint).color(egui::Color32::from_rgb(74, 116, 168)));
        }

        let (primary, secondary) = resolve_disabled_cta_plan(
            &self.chain_runtime_status,
            self.config.chain_enabled,
            game_required_issues,
            chain_required_issues,
        );

        ui.horizontal_wrapped(|ui| {
            if let Some(cta) = primary {
                if ui.button(self.disabled_cta_text(cta)).clicked() {
                    self.record_guided_quick_action_click();
                    self.handle_disabled_action_cta(
                        cta,
                        game_required_issues,
                        chain_required_issues,
                    );
                }
            }

            if let Some(cta) = secondary {
                if Some(cta) != primary && ui.button(self.disabled_cta_text(cta)).clicked() {
                    self.record_guided_quick_action_click();
                    self.handle_disabled_action_cta(
                        cta,
                        game_required_issues,
                        chain_required_issues,
                    );
                }
            }
        });
    }
}
