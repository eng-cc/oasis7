use super::*;
use config_ui::StartupGuideTarget;

impl ClientLauncherApp {
    fn resolve_existing_binary_path(default_path: String) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if Path::new(default_path.as_str()).is_file() {
                return default_path;
            }
            if let Ok(current) = std::env::current_exe() {
                return current.to_string_lossy().to_string();
            }
        }
        default_path
    }

    fn startup_error_title(&self, target: StartupGuideTarget) -> &'static str {
        match (target, self.ui_language) {
            (StartupGuideTarget::Game, UiLanguage::ZhCn) => "启动异常：游戏流程需要修复",
            (StartupGuideTarget::Game, UiLanguage::EnUs) => "Startup Error: Game Flow Needs Fix",
            (StartupGuideTarget::Chain, UiLanguage::ZhCn) => "启动异常：区块链流程需要修复",
            (StartupGuideTarget::Chain, UiLanguage::EnUs) => {
                "Startup Error: Blockchain Flow Needs Fix"
            }
        }
    }

    fn startup_error_retry_text(&self, target: StartupGuideTarget) -> &'static str {
        match (target, self.ui_language) {
            (StartupGuideTarget::Game, UiLanguage::ZhCn) => "重试启动游戏",
            (StartupGuideTarget::Game, UiLanguage::EnUs) => "Retry Start Game",
            (StartupGuideTarget::Chain, UiLanguage::ZhCn) => "重试启动区块链",
            (StartupGuideTarget::Chain, UiLanguage::EnUs) => "Retry Start Blockchain",
        }
    }

    fn startup_error_fix_text(&self, target: StartupGuideTarget) -> &'static str {
        match (target, self.ui_language) {
            (StartupGuideTarget::Game, UiLanguage::ZhCn) => "修复游戏配置",
            (StartupGuideTarget::Game, UiLanguage::EnUs) => "Fix Game Config",
            (StartupGuideTarget::Chain, UiLanguage::ZhCn) => "修复区块链配置",
            (StartupGuideTarget::Chain, UiLanguage::EnUs) => "Fix Blockchain Config",
        }
    }

    fn startup_error_autofill_text(&self) -> &'static str {
        self.tr("自动补全默认值", "Autofill Safe Defaults")
    }

    fn stale_chain_recovery_action_text(&self) -> &'static str {
        self.tr("使用 fresh node 恢复", "Recover With Fresh Node")
    }

    fn apply_chain_recovery_config(&mut self) -> Option<WebChainRecoverySnapshot> {
        let recovery = self.chain_recovery.clone()?;
        self.config = recovery.suggested_config.clone();
        self.config.normalize();
        self.config_dirty = false;
        self.append_log(self.tr(
            "已应用 stale execution world 恢复建议，准备使用 fresh node id 重试。",
            "Applied stale execution world recovery suggestion; retrying with a fresh node id.",
        ));
        Some(recovery)
    }

    fn render_startup_error_issue_lines(&self, ui: &mut egui::Ui, issues: &[ConfigIssue]) {
        for issue in issues.iter().take(3) {
            ui.small(format!("- {}", issue.text(self.ui_language)));
        }
        if issues.len() > 3 {
            ui.small(self.tr(
                "...更多问题请在配置引导中查看",
                "...more issues in config guide",
            ));
        }
    }

    fn render_startup_error_actions(
        &mut self,
        ui: &mut egui::Ui,
        target: StartupGuideTarget,
        issues: &[ConfigIssue],
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.startup_error_fix_text(target)).clicked() {
                self.record_guided_quick_action_click();
                match target {
                    StartupGuideTarget::Game => self.open_game_config_guide(),
                    StartupGuideTarget::Chain => self.open_chain_config_guide(),
                }
            }
            if ui.button(self.startup_error_autofill_text()).clicked() {
                self.record_guided_quick_action_click();
                self.apply_safe_defaults_for_startup_target(target);
            }
            if ui.button(self.startup_error_retry_text(target)).clicked() {
                self.record_guided_quick_action_click();
                match target {
                    StartupGuideTarget::Game => self.handle_start_game_click(issues),
                    StartupGuideTarget::Chain => self.handle_start_chain_click(issues),
                }
            }
        });
    }

    fn render_game_startup_error_card(&mut self, ui: &mut egui::Ui, issues: &[ConfigIssue]) {
        let has_runtime_failure = matches!(
            self.status,
            LauncherStatus::InvalidArgs | LauncherStatus::StartFailed | LauncherStatus::QueryFailed
        );
        if issues.is_empty() && !has_runtime_failure {
            return;
        }

        ui.group(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(188, 60, 60),
                self.startup_error_title(StartupGuideTarget::Game),
            );
            if !issues.is_empty() {
                self.render_startup_error_issue_lines(ui, issues);
            } else {
                ui.small(self.tr(
                    "最近一次启动未通过，请先修复后重试。",
                    "Latest startup did not pass. Fix and retry.",
                ));
            }
            self.render_startup_error_actions(ui, StartupGuideTarget::Game, issues);
        });
    }

    fn render_chain_startup_error_card(&mut self, ui: &mut egui::Ui, issues: &[ConfigIssue]) {
        if !self.config.chain_enabled {
            return;
        }
        let has_runtime_failure = matches!(
            self.chain_runtime_status,
            ChainRuntimeStatus::StaleExecutionWorld(_)
                | ChainRuntimeStatus::ConfigError(_)
                | ChainRuntimeStatus::Unreachable(_)
        );
        if issues.is_empty() && !has_runtime_failure {
            return;
        }

        ui.group(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(188, 60, 60),
                self.startup_error_title(StartupGuideTarget::Chain),
            );
            if !issues.is_empty() {
                self.render_startup_error_issue_lines(ui, issues);
            } else if let Some(detail) = self.chain_runtime_status.detail() {
                ui.small(format!(
                    "{}: {detail}",
                    self.tr("链状态异常", "Chain status error")
                ));
            } else {
                ui.small(self.tr(
                    "区块链启动异常，请执行修复后重试。",
                    "Blockchain startup failed. Fix and retry.",
                ));
            }
            if issues.is_empty()
                && matches!(
                    self.chain_runtime_status,
                    ChainRuntimeStatus::StaleExecutionWorld(_)
                )
            {
                if let Some(recovery) = self.chain_recovery.clone() {
                    ui.separator();
                    ui.small(format!(
                        "{}: {} · {}: {}",
                        self.tr("建议 fresh node", "Suggested fresh node"),
                        recovery.fresh_node_id,
                        self.tr("建议状态端口", "Suggested status bind"),
                        recovery.fresh_chain_status_bind,
                    ));
                    if ui.button(self.stale_chain_recovery_action_text()).clicked() {
                        self.record_guided_quick_action_click();
                        if self.apply_chain_recovery_config().is_some() {
                            self.handle_start_chain_click(issues);
                        }
                    }
                }
            }
            self.render_startup_error_actions(ui, StartupGuideTarget::Chain, issues);
        });
    }

    pub(super) fn render_startup_error_cards(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        self.render_chain_startup_error_card(ui, chain_required_issues);
        self.render_game_startup_error_card(ui, game_required_issues);
    }

    pub(super) fn apply_safe_defaults_for_startup_target(&mut self, target: StartupGuideTarget) {
        let defaults = LaunchConfig::default();
        match target {
            StartupGuideTarget::Game => {
                self.config.scenario = defaults.scenario;
                self.config.live_bind = defaults.live_bind;
                self.config.web_bind = defaults.web_bind;
                self.config.viewer_host = defaults.viewer_host;
                self.config.viewer_port = defaults.viewer_port;
                self.config.viewer_static_dir = defaults.viewer_static_dir;
                self.config.launcher_bin =
                    Self::resolve_existing_binary_path(defaults.launcher_bin);
                self.status = LauncherStatus::Idle;
                self.append_log(self.tr(
                    "已为游戏启动链路自动补全安全默认值。",
                    "Autofilled safe defaults for game startup flow.",
                ));
            }
            StartupGuideTarget::Chain => {
                self.config.chain_enabled = true;
                self.config.chain_runtime_bin =
                    Self::resolve_existing_binary_path(defaults.chain_runtime_bin);
                self.config.chain_status_bind = defaults.chain_status_bind;
                self.config.chain_node_id = defaults.chain_node_id;
                self.config.chain_node_role = defaults.chain_node_role;
                self.config.chain_node_tick_ms = defaults.chain_node_tick_ms;
                self.config.chain_pos_slot_duration_ms = defaults.chain_pos_slot_duration_ms;
                self.config.chain_pos_ticks_per_slot = defaults.chain_pos_ticks_per_slot;
                self.config.chain_pos_proposal_tick_phase = defaults.chain_pos_proposal_tick_phase;
                self.config.chain_pos_max_past_slot_lag = defaults.chain_pos_max_past_slot_lag;
                self.config.normalize();
                self.chain_runtime_status = if chain_runtime_effectively_enabled(&self.config) {
                    ChainRuntimeStatus::NotStarted
                } else {
                    ChainRuntimeStatus::Disabled
                };
                self.chain_recovery = None;
                self.append_log(self.tr(
                    "已为区块链启动链路自动补全安全默认值。",
                    "Autofilled safe defaults for blockchain startup flow.",
                ));
            }
        }
    }
}
