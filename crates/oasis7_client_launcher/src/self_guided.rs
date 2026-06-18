use super::*;

const ONBOARDING_STEP_TOTAL: usize = 3;

#[path = "self_guided_storage.rs"]
mod storage;

use self::storage::current_unix_ms;
pub(super) use self::storage::load_launcher_ux_state;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) use self::storage::UX_STATE_PATH;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct LauncherUxState {
    pub(super) onboarding_completed: bool,
    pub(super) onboarding_dismissed: bool,
    pub(super) expert_mode: bool,
    pub(super) last_successful_config: Option<LaunchConfig>,
    pub(super) last_successful_saved_at_unix_ms: Option<i64>,
    pub(super) onboarding_opened_count: u64,
    pub(super) onboarding_skipped_count: u64,
    pub(super) onboarding_completed_count: u64,
    pub(super) demo_mode_runs_count: u64,
    pub(super) quick_action_click_count: u64,
}

impl Default for LauncherUxState {
    fn default() -> Self {
        Self {
            onboarding_completed: false,
            onboarding_dismissed: false,
            expert_mode: false,
            last_successful_config: None,
            last_successful_saved_at_unix_ms: None,
            onboarding_opened_count: 0,
            onboarding_skipped_count: 0,
            onboarding_completed_count: 0,
            demo_mode_runs_count: 0,
            quick_action_click_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OnboardingStep {
    Understand,
    FixConfig,
    Launch,
}

impl OnboardingStep {
    fn index(self) -> usize {
        match self {
            Self::Understand => 1,
            Self::FixConfig => 2,
            Self::Launch => 3,
        }
    }

    fn previous(self) -> Option<Self> {
        match self {
            Self::Understand => None,
            Self::FixConfig => Some(Self::Understand),
            Self::Launch => Some(Self::FixConfig),
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            Self::Understand => Some(Self::FixConfig),
            Self::FixConfig => Some(Self::Launch),
            Self::Launch => None,
        }
    }

    fn title(self, language: UiLanguage) -> &'static str {
        match (self, language) {
            (Self::Understand, UiLanguage::ZhCn) => "步骤 1/3：认识启动器主流程",
            (Self::Understand, UiLanguage::EnUs) => "Step 1/3: Understand the Main Flow",
            (Self::FixConfig, UiLanguage::ZhCn) => "步骤 2/3：检查并修复配置",
            (Self::FixConfig, UiLanguage::EnUs) => "Step 2/3: Check and Fix Configuration",
            (Self::Launch, UiLanguage::ZhCn) => "步骤 3/3：启动并进入游戏页面",
            (Self::Launch, UiLanguage::EnUs) => "Step 3/3: Start and Open Game Page",
        }
    }

    fn body(self, language: UiLanguage) -> &'static str {
        match (self, language) {
            (Self::Understand, UiLanguage::ZhCn) => {
                "先关注 3 张任务卡：启动区块链、启动游戏、打开游戏页面。完成后即可进入转账/浏览器等高级能力。"
            }
            (Self::Understand, UiLanguage::EnUs) => {
                "Focus on three cards first: Start Blockchain, Start Game, and Open Game Page. Advanced actions come after that."
            }
            (Self::FixConfig, UiLanguage::ZhCn) => {
                "如果启动失败，优先使用“配置引导”修复必填字段，而不是手动猜测原因。"
            }
            (Self::FixConfig, UiLanguage::EnUs) => {
                "If startup fails, use the configuration guide to fix required fields before trying advanced actions."
            }
            (Self::Launch, UiLanguage::ZhCn) => {
                "当区块链和游戏都运行后，点击“打开游戏页”验证闭环，再尝试反馈/转账/浏览器。"
            }
            (Self::Launch, UiLanguage::EnUs) => {
                "After blockchain and game are running, open the game page to verify the loop, then try Feedback/Transfer/Explorer."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NextTaskHint {
    FixChainConfig,
    StartChain,
    FixGameConfig,
    StartGame,
    OpenGamePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DisabledActionCta {
    EnableChain,
    FixChainConfig,
    StartChain,
    RetryChainStatus,
    FixGameConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigGuideTargetHint {
    Game,
    Chain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DemoModePhase {
    Idle,
    StartChainRequested,
    WaitChainReady,
    StartGameRequested,
    WaitGameRunning,
    Done,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GuidanceCounter {
    OnboardingOpened,
    OnboardingSkipped,
    OnboardingCompleted,
    DemoRuns,
    QuickActionClicks,
}

pub(super) fn resolve_next_task_hint(
    chain_enabled: bool,
    game_required_issues: &[ConfigIssue],
    chain_required_issues: &[ConfigIssue],
    game_running: bool,
    chain_running: bool,
) -> NextTaskHint {
    if chain_enabled && !chain_required_issues.is_empty() {
        return NextTaskHint::FixChainConfig;
    }
    if !game_required_issues.is_empty() {
        return NextTaskHint::FixGameConfig;
    }
    if chain_enabled && !chain_running {
        return NextTaskHint::StartChain;
    }
    if !game_running {
        return NextTaskHint::StartGame;
    }
    NextTaskHint::OpenGamePage
}

pub(super) fn resolve_config_guide_target(
    chain_enabled: bool,
    game_required_issues: &[ConfigIssue],
    chain_required_issues: &[ConfigIssue],
) -> Option<ConfigGuideTargetHint> {
    if chain_enabled && !chain_required_issues.is_empty() {
        return Some(ConfigGuideTargetHint::Chain);
    }
    if !game_required_issues.is_empty() {
        return Some(ConfigGuideTargetHint::Game);
    }
    None
}

#[cfg(test)]
pub(super) fn resolve_primary_disabled_cta(
    chain_enabled: bool,
    game_required_issues: &[ConfigIssue],
    chain_required_issues: &[ConfigIssue],
    chain_running: bool,
) -> Option<DisabledActionCta> {
    if !chain_enabled {
        return Some(DisabledActionCta::EnableChain);
    }
    if !chain_required_issues.is_empty() {
        return Some(DisabledActionCta::FixChainConfig);
    }
    if !chain_running {
        return Some(DisabledActionCta::StartChain);
    }
    if !game_required_issues.is_empty() {
        return Some(DisabledActionCta::FixGameConfig);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OnboardingState {
    pub(super) open: bool,
    pub(super) step: OnboardingStep,
    pub(super) auto_open_checked: bool,
    pub(super) completed: bool,
    pub(super) dismissed: bool,
}

impl OnboardingState {
    pub(super) fn from_persisted(completed: bool, dismissed: bool) -> Self {
        Self {
            open: false,
            step: OnboardingStep::Understand,
            auto_open_checked: false,
            completed,
            dismissed,
        }
    }
}

#[cfg(test)]
#[path = "self_guided_tests.rs"]
mod tests;

#[cfg(not(test))]
pub(super) use self::storage::save_launcher_ux_state;

impl ClientLauncherApp {
    pub(super) fn is_expert_mode(&self) -> bool {
        self.ux_state.expert_mode
    }

    pub(super) fn set_expert_mode(&mut self, enabled: bool) {
        self.ux_state.expert_mode = enabled;
        #[cfg(not(test))]
        {
            if let Err(err) = save_launcher_ux_state(&self.ux_state) {
                self.append_log(format!(
                    "{}: {err}",
                    self.tr(
                        "保存专家模式状态失败（已降级为会话内状态）",
                        "Persist expert mode failed (fallback to session-only)"
                    )
                ));
            }
        }
        self.append_log(if enabled {
            self.tr("已切换为专家模式。", "Switched to expert mode.")
                .to_string()
        } else {
            self.tr("已切换为引导模式。", "Switched to guided mode.")
                .to_string()
        });
    }

    pub(super) fn persist_ux_state_or_log(&mut self, _context_zh: &str, _context_en: &str) {
        #[cfg(not(test))]
        {
            if let Err(err) = save_launcher_ux_state(&self.ux_state) {
                self.append_log(format!("{}: {err}", self.tr(_context_zh, _context_en)));
            }
        }
    }

    pub(super) fn increment_guidance_counter(&mut self, counter: GuidanceCounter) {
        match counter {
            GuidanceCounter::OnboardingOpened => self.ux_state.onboarding_opened_count += 1,
            GuidanceCounter::OnboardingSkipped => self.ux_state.onboarding_skipped_count += 1,
            GuidanceCounter::OnboardingCompleted => self.ux_state.onboarding_completed_count += 1,
            GuidanceCounter::DemoRuns => self.ux_state.demo_mode_runs_count += 1,
            GuidanceCounter::QuickActionClicks => self.ux_state.quick_action_click_count += 1,
        }
        self.persist_ux_state_or_log(
            "保存引导计数失败（已降级为会话内状态）",
            "Persist guidance counters failed (fallback to session-only)",
        );
    }

    pub(super) fn record_guided_quick_action_click(&mut self) {
        self.increment_guidance_counter(GuidanceCounter::QuickActionClicks);
    }

    pub(super) fn maybe_save_last_successful_config_profile(&mut self, game_running: bool) {
        if !game_running {
            return;
        }
        if self
            .ux_state
            .last_successful_config
            .as_ref()
            .is_some_and(|config| config == &self.config)
        {
            return;
        }
        self.ux_state.last_successful_config = Some(self.config.clone());
        self.ux_state.last_successful_saved_at_unix_ms = Some(current_unix_ms());
        self.persist_ux_state_or_log(
            "保存最近成功配置失败（已降级为会话内状态）",
            "Persist last successful config failed (fallback to session-only)",
        );
        self.append_log(self.tr(
            "已保存最近成功配置画像。",
            "Saved last successful configuration profile.",
        ));
    }

    pub(super) fn restore_last_successful_config_profile(&mut self) {
        let Some(saved_config) = self.ux_state.last_successful_config.clone() else {
            self.append_log(self.tr(
                "恢复最近成功配置失败：暂无可用画像。",
                "Restore failed: no successful profile is available.",
            ));
            return;
        };
        self.config = saved_config;
        self.config.normalize();
        self.chain_runtime_status = if chain_runtime_effectively_enabled(&self.config) {
            ChainRuntimeStatus::NotStarted
        } else {
            ChainRuntimeStatus::Disabled
        };
        self.append_log(self.tr(
            "已恢复最近成功配置，请按需重新启动区块链与游戏。",
            "Restored last successful configuration. Restart blockchain/game if needed.",
        ));
    }

    pub(super) fn clear_last_successful_config_profile(&mut self) {
        self.ux_state.last_successful_config = None;
        self.ux_state.last_successful_saved_at_unix_ms = None;
        self.persist_ux_state_or_log(
            "清空最近成功配置失败（已降级为会话内状态）",
            "Clear saved profile failed (fallback to session-only)",
        );
        self.append_log(self.tr(
            "已清空最近成功配置画像。",
            "Cleared last successful configuration profile.",
        ));
    }

    pub(super) fn demo_mode_phase_text(&self) -> &'static str {
        match (self.demo_mode_phase, self.ui_language) {
            (DemoModePhase::Idle, UiLanguage::ZhCn) => "未启动",
            (DemoModePhase::Idle, UiLanguage::EnUs) => "Idle",
            (DemoModePhase::StartChainRequested, UiLanguage::ZhCn) => "准备启动区块链",
            (DemoModePhase::StartChainRequested, UiLanguage::EnUs) => "Prepare Chain Start",
            (DemoModePhase::WaitChainReady, UiLanguage::ZhCn) => "等待区块链就绪",
            (DemoModePhase::WaitChainReady, UiLanguage::EnUs) => "Waiting Chain Ready",
            (DemoModePhase::StartGameRequested, UiLanguage::ZhCn) => "准备启动游戏",
            (DemoModePhase::StartGameRequested, UiLanguage::EnUs) => "Prepare Game Start",
            (DemoModePhase::WaitGameRunning, UiLanguage::ZhCn) => "等待游戏运行",
            (DemoModePhase::WaitGameRunning, UiLanguage::EnUs) => "Waiting Game Running",
            (DemoModePhase::Done, UiLanguage::ZhCn) => "已完成",
            (DemoModePhase::Done, UiLanguage::EnUs) => "Done",
            (DemoModePhase::Failed, UiLanguage::ZhCn) => "失败",
            (DemoModePhase::Failed, UiLanguage::EnUs) => "Failed",
        }
    }

    fn apply_demo_mode_safe_defaults(&mut self) {
        self.config.scenario = DEFAULT_SCENARIO.to_string();
        self.config.live_bind = DEFAULT_LIVE_BIND.to_string();
        self.config.web_bind = DEFAULT_WEB_BIND.to_string();
        self.config.viewer_host = DEFAULT_VIEWER_HOST.to_string();
        self.config.viewer_port = DEFAULT_VIEWER_PORT.to_string();
        self.config.chain_enabled = true;
        self.config.chain_status_bind = DEFAULT_CHAIN_STATUS_BIND.to_string();
        self.config.chain_node_id = default_chain_node_id();
        self.config.chain_node_role = DEFAULT_CHAIN_NODE_ROLE.to_string();
        self.config.chain_node_tick_ms = DEFAULT_CHAIN_NODE_TICK_MS.to_string();
        self.config.chain_pos_slot_duration_ms = DEFAULT_CHAIN_POS_SLOT_DURATION_MS.to_string();
        self.config.chain_pos_ticks_per_slot = DEFAULT_CHAIN_POS_TICKS_PER_SLOT.to_string();
        self.config.chain_pos_proposal_tick_phase =
            DEFAULT_CHAIN_POS_PROPOSAL_TICK_PHASE.to_string();
        self.config.auto_open_browser = false;
        self.config.normalize();
    }

    pub(super) fn start_demo_mode_one_click(&mut self) {
        self.apply_demo_mode_safe_defaults();
        self.demo_mode_phase = if chain_runtime_effectively_enabled(&self.config) {
            DemoModePhase::StartChainRequested
        } else {
            DemoModePhase::StartGameRequested
        };
        self.increment_guidance_counter(GuidanceCounter::DemoRuns);
        self.append_log(self.tr(
            "演示模式：已应用安全默认配置，准备启动游戏。",
            "Demo mode: safe defaults applied, preparing game startup.",
        ));
    }

    pub(super) fn reset_demo_mode(&mut self) {
        self.demo_mode_phase = DemoModePhase::Idle;
        self.append_log(self.tr("已重置演示模式状态。", "Demo mode state reset."));
    }

    pub(super) fn advance_demo_mode(
        &mut self,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        match self.demo_mode_phase {
            DemoModePhase::Idle | DemoModePhase::Done | DemoModePhase::Failed => {}
            DemoModePhase::StartChainRequested => {
                if chain_running {
                    self.demo_mode_phase = DemoModePhase::StartGameRequested;
                    self.append_log(self.tr(
                        "演示模式：区块链已就绪，进入游戏启动步骤。",
                        "Demo mode: blockchain ready, moving to game start.",
                    ));
                    return;
                }
                if !chain_required_issues.is_empty() {
                    self.demo_mode_phase = DemoModePhase::Failed;
                    self.open_chain_config_guide();
                    self.append_log(self.tr(
                        "演示模式失败：区块链配置仍有阻断项，已打开配置引导。",
                        "Demo mode failed: chain configuration is blocked. Guide opened.",
                    ));
                    return;
                }
                self.start_chain_process();
                self.demo_mode_phase = DemoModePhase::WaitChainReady;
                self.append_log(self.tr(
                    "演示模式：已触发区块链启动，等待就绪。",
                    "Demo mode: blockchain start requested, waiting for ready.",
                ));
            }
            DemoModePhase::WaitChainReady => {
                if chain_running {
                    self.demo_mode_phase = DemoModePhase::StartGameRequested;
                    self.append_log(self.tr(
                        "演示模式：区块链已就绪，进入游戏启动步骤。",
                        "Demo mode: blockchain ready, moving to game start.",
                    ));
                } else if matches!(
                    self.chain_runtime_status,
                    ChainRuntimeStatus::ConfigError(_)
                        | ChainRuntimeStatus::Unreachable(_)
                        | ChainRuntimeStatus::Disabled
                ) {
                    self.demo_mode_phase = DemoModePhase::Failed;
                    self.append_log(self.tr(
                        "演示模式失败：区块链启动异常，请检查日志与配置。",
                        "Demo mode failed: blockchain startup error, check logs/config.",
                    ));
                }
            }
            DemoModePhase::StartGameRequested => {
                if game_running {
                    self.demo_mode_phase = DemoModePhase::Done;
                    self.append_log(self.tr(
                        "演示模式完成：游戏已运行，可打开游戏页面。",
                        "Demo mode completed: game is running, open game page.",
                    ));
                    return;
                }
                if !game_required_issues.is_empty() {
                    self.demo_mode_phase = DemoModePhase::Failed;
                    self.open_game_config_guide();
                    self.append_log(self.tr(
                        "演示模式失败：游戏配置仍有阻断项，已打开配置引导。",
                        "Demo mode failed: game configuration is blocked. Guide opened.",
                    ));
                    return;
                }
                self.start_process();
                self.demo_mode_phase = DemoModePhase::WaitGameRunning;
                self.append_log(self.tr(
                    "演示模式：已触发游戏启动，等待运行状态。",
                    "Demo mode: game start requested, waiting for running state.",
                ));
            }
            DemoModePhase::WaitGameRunning => {
                if game_running {
                    self.demo_mode_phase = DemoModePhase::Done;
                    self.append_log(self.tr(
                        "演示模式完成：游戏已运行，可打开游戏页面。",
                        "Demo mode completed: game is running, open game page.",
                    ));
                } else if matches!(
                    self.status,
                    LauncherStatus::InvalidArgs | LauncherStatus::QueryFailed
                ) {
                    self.demo_mode_phase = DemoModePhase::Failed;
                    self.append_log(self.tr(
                        "演示模式失败：游戏启动异常，请检查日志与配置。",
                        "Demo mode failed: game startup error, check logs/config.",
                    ));
                }
            }
        }
    }

    pub(super) fn next_task_hint_text(&self, hint: NextTaskHint) -> &'static str {
        match (hint, self.ui_language) {
            (NextTaskHint::FixChainConfig, UiLanguage::ZhCn) => {
                "下一步：先修复区块链配置，再启动区块链"
            }
            (NextTaskHint::FixChainConfig, UiLanguage::EnUs) => {
                "Next: fix blockchain configuration before starting blockchain"
            }
            (NextTaskHint::StartChain, UiLanguage::ZhCn) => "下一步：启动区块链",
            (NextTaskHint::StartChain, UiLanguage::EnUs) => "Next: start blockchain",
            (NextTaskHint::FixGameConfig, UiLanguage::ZhCn) => "下一步：先修复游戏配置，再启动游戏",
            (NextTaskHint::FixGameConfig, UiLanguage::EnUs) => {
                "Next: fix game configuration before starting game"
            }
            (NextTaskHint::StartGame, UiLanguage::ZhCn) => "下一步：启动游戏",
            (NextTaskHint::StartGame, UiLanguage::EnUs) => "Next: start game",
            (NextTaskHint::OpenGamePage, UiLanguage::ZhCn) => "下一步：打开游戏页验证闭环",
            (NextTaskHint::OpenGamePage, UiLanguage::EnUs) => "Next: open game page to verify",
        }
    }

    fn config_guide_button_text(&self, target: ConfigGuideTargetHint) -> &'static str {
        match (target, self.ui_language) {
            (ConfigGuideTargetHint::Game, UiLanguage::ZhCn) => "修复游戏配置（配置引导）",
            (ConfigGuideTargetHint::Game, UiLanguage::EnUs) => "Fix Game Config (Guide)",
            (ConfigGuideTargetHint::Chain, UiLanguage::ZhCn) => "修复区块链配置（配置引导）",
            (ConfigGuideTargetHint::Chain, UiLanguage::EnUs) => "Fix Blockchain Config (Guide)",
        }
    }

    pub(super) fn render_task_flow_cards(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        ui.label(
            egui::RichText::new(self.tr("任务流（推荐顺序）", "Guided Task Flow"))
                .strong()
                .size(14.0),
        );
        let hint = resolve_next_task_hint(
            self.config.chain_enabled,
            game_required_issues,
            chain_required_issues,
            game_running,
            chain_running,
        );
        ui.small(egui::RichText::new(self.next_task_hint_text(hint)).color(Self::task_blue()));

        if let Some(target) = resolve_config_guide_target(
            self.config.chain_enabled,
            game_required_issues,
            chain_required_issues,
        ) {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .small_button(self.config_guide_button_text(target))
                    .clicked()
                {
                    self.record_guided_quick_action_click();
                    match target {
                        ConfigGuideTargetHint::Game => self.open_game_config_guide(),
                        ConfigGuideTargetHint::Chain => self.open_chain_config_guide(),
                    }
                    self.append_log(self.tr(
                        "已从任务流直达配置引导。",
                        "Opened configuration guide directly from task flow.",
                    ));
                }
                if !self.is_expert_mode()
                    && ui
                        .small_button(self.tr("重置新手引导", "Reset Onboarding"))
                        .clicked()
                {
                    self.record_guided_quick_action_click();
                    self.reset_onboarding();
                }
            });
        }

        self.render_chain_task_card(ui, chain_required_issues, chain_running);
        ui.add_space(6.0);
        self.render_game_task_card(ui, game_required_issues, game_running);
        ui.add_space(6.0);
        self.render_page_task_card(ui, game_required_issues, game_running);
    }

    pub(super) fn task_blue() -> egui::Color32 {
        egui::Color32::from_rgb(74, 116, 168)
    }

    fn task_card_fill() -> egui::Color32 {
        egui::Color32::from_rgb(252, 253, 250)
    }

    fn render_task_card_frame(
        ui: &mut egui::Ui,
        step: &str,
        icon: &str,
        title: &str,
        status: &str,
        status_color: egui::Color32,
        body: &str,
        add_actions: impl FnOnce(&mut egui::Ui),
    ) {
        egui::Frame::group(ui.style())
            .fill(Self::task_card_fill())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(70.0);
                let available_width = ui.available_width();
                if available_width < 292.0 {
                    ui.horizontal_top(|ui| {
                        ui.label(
                            egui::RichText::new(step)
                                .strong()
                                .size(15.0)
                                .color(Self::task_blue()),
                        );
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(236, 244, 252))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(199, 215, 235),
                            ))
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(icon)
                                        .strong()
                                        .size(12.0)
                                        .color(Self::task_blue()),
                                );
                            });
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(title).strong().size(13.0));
                            ui.small(body);
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(252, 253, 250))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(218, 224, 217),
                            ))
                            .inner_margin(egui::Margin::symmetric(5, 2))
                            .show(ui, |ui| {
                                ui.small(egui::RichText::new(status).strong().color(status_color));
                            });
                        add_actions(ui);
                    });
                    return;
                }
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(36.0);
                        ui.label(
                            egui::RichText::new(step)
                                .strong()
                                .size(16.0)
                                .color(Self::task_blue()),
                        );
                        ui.label(egui::RichText::new("|").size(16.0).color(Self::task_blue()));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(42.0);
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(236, 244, 252))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(199, 215, 235),
                            ))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(26.0, 26.0));
                                ui.label(
                                    egui::RichText::new(icon)
                                        .strong()
                                        .size(13.0)
                                        .color(Self::task_blue()),
                                );
                            });
                    });
                    ui.vertical(|ui| {
                        ui.set_width(142.0);
                        ui.label(egui::RichText::new(title).strong().size(13.0));
                        ui.small(body);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(104.0);
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(252, 253, 250))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(218, 224, 217),
                            ))
                            .inner_margin(egui::Margin::symmetric(5, 2))
                            .show(ui, |ui| {
                                ui.small(egui::RichText::new(status).strong().color(status_color));
                            });
                        ui.horizontal_wrapped(add_actions);
                    });
                });
            });
    }

    fn task_primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(
                egui::RichText::new(label)
                    .strong()
                    .size(11.5)
                    .color(egui::Color32::WHITE),
            )
            .fill(egui::Color32::from_rgb(42, 169, 87))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 142, 74)))
            .min_size(egui::vec2(96.0, 28.0)),
        )
    }

    fn task_secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(egui::RichText::new(label).size(11.5))
                .fill(egui::Color32::from_rgb(247, 250, 253))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(218, 226, 236),
                ))
                .min_size(egui::vec2(96.0, 28.0)),
        )
    }

    fn render_chain_task_card(
        &mut self,
        ui: &mut egui::Ui,
        chain_required_issues: &[ConfigIssue],
        chain_running: bool,
    ) {
        if !self.config.chain_enabled {
            Self::render_task_card_frame(
                ui,
                "01",
                "🔗",
                self.tr("启动区块链", "Start Blockchain"),
                self.tr("状态：已关闭", "Status: Disabled"),
                egui::Color32::from_rgb(116, 119, 124),
                self.tr("链功能已关闭。", "Blockchain is disabled."),
                |_| {},
            );
            return;
        }

        if chain_running {
            Self::render_task_card_frame(
                ui,
                "01",
                "🔗",
                self.tr("启动区块链", "Start Blockchain"),
                self.tr("状态：就绪/启动中", "Status: Ready/Starting"),
                egui::Color32::from_rgb(62, 152, 92),
                self.tr(
                    "链节点已进入可用路径。",
                    "Blockchain node is on the ready path.",
                ),
                |_| {},
            );
            return;
        }

        if !chain_required_issues.is_empty() {
            Self::render_task_card_frame(
                ui,
                "01",
                "🔗",
                self.tr("启动区块链", "Start Blockchain"),
                self.tr("状态：配置阻断", "Status: Config Blocked"),
                egui::Color32::from_rgb(188, 60, 60),
                self.tr(
                    "先修复链配置再启动节点。",
                    "Fix chain configuration before starting the node.",
                ),
                |ui| {
                    if Self::task_primary_button(ui, self.tr("修复区块链配置", "Fix Chain Config"))
                        .clicked()
                    {
                        self.handle_start_chain_click(chain_required_issues);
                    }
                },
            );
            return;
        }

        Self::render_task_card_frame(
            ui,
            "01",
            "🔗",
            self.tr("启动区块链", "Start Blockchain"),
            self.tr("状态：待启动", "Status: Pending"),
            egui::Color32::from_rgb(201, 146, 44),
            self.tr("启动本地链运行时。", "Start the local chain runtime."),
            |ui| {
                if Self::task_primary_button(ui, self.tr("启动区块链", "Start Blockchain"))
                    .clicked()
                {
                    self.start_chain_process();
                }
            },
        );
    }

    fn render_game_task_card(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        game_running: bool,
    ) {
        if game_running {
            Self::render_task_card_frame(
                ui,
                "02",
                "▶",
                self.tr("启动游戏", "Start Game"),
                self.tr("状态：运行中", "Status: Running"),
                egui::Color32::from_rgb(62, 152, 92),
                self.tr("游戏服务正在运行。", "The game service is running."),
                |_| {},
            );
            return;
        }

        if !game_required_issues.is_empty() {
            Self::render_task_card_frame(
                ui,
                "02",
                "▶",
                self.tr("启动游戏", "Start Game"),
                self.tr("状态：配置阻断", "Status: Config Blocked"),
                egui::Color32::from_rgb(188, 60, 60),
                self.tr(
                    "修复游戏配置后再启动。",
                    "Fix game configuration before starting.",
                ),
                |ui| {
                    if Self::task_primary_button(ui, self.tr("修复游戏配置", "Fix Game Config"))
                        .clicked()
                    {
                        self.handle_start_game_click(game_required_issues);
                    }
                },
            );
            return;
        }

        let chain_waiting = self.config.chain_enabled
            && !matches!(
                self.chain_runtime_status,
                ChainRuntimeStatus::Starting | ChainRuntimeStatus::Ready
            );
        Self::render_task_card_frame(
            ui,
            "02",
            "▶",
            self.tr("启动游戏", "Start Game"),
            if chain_waiting {
                self.tr("状态：建议先启动链", "Status: Chain First")
            } else {
                self.tr("状态：待启动", "Status: Pending")
            },
            egui::Color32::from_rgb(201, 146, 44),
            self.tr("启动客户端游戏流程。", "Start the client game flow."),
            |ui| {
                if Self::task_primary_button(ui, self.tr("启动游戏", "Start Game")).clicked() {
                    self.start_process();
                }
            },
        );
    }

    fn render_page_task_card(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        game_running: bool,
    ) {
        if game_running {
            Self::render_task_card_frame(
                ui,
                "03",
                "↗",
                self.tr("打开游戏页", "Open Game Page"),
                self.tr("状态：可打开", "Status: Ready"),
                egui::Color32::from_rgb(62, 152, 92),
                self.tr(
                    "打开浏览器验证游戏画面。",
                    "Open the browser and verify the game view.",
                ),
                |ui| {
                    if Self::task_primary_button(ui, self.tr("打开游戏页", "Open Game Page"))
                        .clicked()
                    {
                        let url = self.current_game_url();
                        if let Err(err) = open_browser(url.as_str()) {
                            self.append_log(format!("open browser failed: {err}"));
                        } else {
                            self.append_log(format!("open browser: {url}"));
                        }
                    }
                },
            );
        } else {
            Self::render_task_card_frame(
                ui,
                "03",
                "↗",
                self.tr("打开游戏页", "Open Game Page"),
                self.tr("状态：等待游戏启动", "Status: Waiting"),
                egui::Color32::from_rgb(201, 146, 44),
                self.tr(
                    "游戏运行后才能打开页面。",
                    "Open the page after the game is running.",
                ),
                |ui| {
                    if Self::task_secondary_button(ui, self.tr("先启动游戏", "Start Game First"))
                        .clicked()
                    {
                        self.handle_start_game_click(game_required_issues);
                    }
                },
            );
        }
    }

    pub(super) fn maybe_open_onboarding_on_first_visit(
        &mut self,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        if self.is_expert_mode() {
            self.onboarding_state.auto_open_checked = true;
            return;
        }
        if self.onboarding_state.auto_open_checked {
            return;
        }
        self.onboarding_state.auto_open_checked = true;
        if self.onboarding_state.completed || self.onboarding_state.dismissed {
            return;
        }

        if !game_required_issues.is_empty()
            || (self.config.chain_enabled && !chain_required_issues.is_empty())
        {
            self.onboarding_state.step = OnboardingStep::FixConfig;
        } else if game_running || chain_running {
            self.onboarding_state.step = OnboardingStep::Launch;
        } else {
            self.onboarding_state.step = OnboardingStep::Understand;
        }
        self.onboarding_state.open = true;
        self.increment_guidance_counter(GuidanceCounter::OnboardingOpened);
        self.append_log(self.tr(
            "已自动打开首次引导（3 步）。",
            "First-run onboarding (3 steps) opened automatically.",
        ));
    }

    pub(super) fn open_onboarding_manual(&mut self) {
        self.onboarding_state.dismissed = false;
        self.ux_state.onboarding_dismissed = false;
        self.persist_ux_state_or_log(
            "保存引导状态失败（已降级为会话内状态）",
            "Persist onboarding state failed (fallback to session-only)",
        );
        self.onboarding_state.open = true;
        self.increment_guidance_counter(GuidanceCounter::OnboardingOpened);
    }

    pub(super) fn reset_onboarding(&mut self) {
        self.set_onboarding_completed(false, false);
        self.onboarding_state.open = true;
        self.onboarding_state.step = OnboardingStep::Understand;
        self.onboarding_state.auto_open_checked = true;
        self.onboarding_state.dismissed = false;
        self.append_log(self.tr("已重置引导状态。", "Onboarding state has been reset."));
    }

    fn set_onboarding_completed(&mut self, completed: bool, skipped: bool) {
        self.onboarding_state.completed = completed;
        self.onboarding_state.dismissed = false;
        self.onboarding_state.open = false;
        self.ux_state.onboarding_completed = completed;
        self.ux_state.onboarding_dismissed = false;
        #[cfg(not(test))]
        {
            if let Err(err) = save_launcher_ux_state(&self.ux_state) {
                self.append_log(format!(
                    "{}: {err}",
                    self.tr(
                        "保存引导状态失败（已降级为会话内状态）",
                        "Persist onboarding state failed (fallback to session-only)"
                    )
                ));
            }
        }

        if completed {
            if skipped {
                self.increment_guidance_counter(GuidanceCounter::OnboardingSkipped);
            } else {
                self.increment_guidance_counter(GuidanceCounter::OnboardingCompleted);
            }
            self.append_log(if skipped {
                self.tr("已跳过首次引导。", "Onboarding skipped.")
                    .to_string()
            } else {
                self.tr("首次引导已完成。", "Onboarding completed.")
                    .to_string()
            });
        }
    }

    pub(super) fn show_onboarding_window(
        &mut self,
        ctx: &egui::Context,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        if !self.onboarding_state.open {
            return;
        }

        let mut keep_open = self.onboarding_state.open;
        let mut request_skip = false;
        let mut request_complete = false;

        let step = self.onboarding_state.step;
        let title = self.tr("首次引导（可随时跳过）", "First-Run Onboarding (Skippable)");
        let progress_label = format!(
            "{} {}/{}",
            self.tr("进度", "Progress"),
            step.index(),
            ONBOARDING_STEP_TOTAL
        );
        let step_title = step.title(self.ui_language);
        let step_body = step.body(self.ui_language);
        let window_size = Self::modal_window_size(ctx, 700.0, 460.0);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size(window_size)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                Self::modal_header(
                    ui,
                    step_title,
                    step_body,
                    Some((
                        progress_label.as_str(),
                        egui::Color32::from_rgb(74, 116, 168),
                    )),
                );
                ui.add_space(8.0);

                Self::modal_card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for index in 1..=ONBOARDING_STEP_TOTAL {
                            let active = index == step.index();
                            Self::modal_status_chip(
                                ui,
                                match index {
                                    1 => self.tr("01 理解", "01 Understand"),
                                    2 => self.tr("02 配置", "02 Configure"),
                                    _ => self.tr("03 启动", "03 Launch"),
                                },
                                if active {
                                    egui::Color32::from_rgb(74, 116, 168)
                                } else {
                                    egui::Color32::from_rgb(116, 119, 124)
                                },
                            );
                        }
                    });
                    ui.add_space(8.0);

                    match step {
                        OnboardingStep::Understand => {
                            self.render_onboarding_understand_step(ui, game_running, chain_running);
                        }
                        OnboardingStep::FixConfig => {
                            self.render_onboarding_fix_config_step(
                                ui,
                                game_required_issues,
                                chain_required_issues,
                            );
                        }
                        OnboardingStep::Launch => {
                            self.render_onboarding_launch_step(
                                ui,
                                game_required_issues,
                                chain_required_issues,
                                game_running,
                                chain_running,
                            );
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(previous) = step.previous() {
                        if Self::modal_secondary_button(ui, self.tr("上一步", "Back")).clicked()
                        {
                            self.onboarding_state.step = previous;
                        }
                    }

                    if let Some(next) = step.next() {
                        if Self::modal_primary_button(ui, self.tr("下一步", "Next")).clicked() {
                            self.onboarding_state.step = next;
                        }
                    } else if Self::modal_primary_button(
                        ui,
                        self.tr("完成引导", "Finish Onboarding"),
                    )
                    .clicked()
                    {
                        request_complete = true;
                    }

                    if Self::modal_secondary_button(ui, self.tr("跳过（稍后再看）", "Skip for now"))
                        .clicked()
                    {
                        request_skip = true;
                    }
                });
            });

        if request_complete {
            self.set_onboarding_completed(true, false);
            keep_open = false;
        }
        if request_skip {
            self.dismiss_onboarding_with_reminder();
            keep_open = false;
        }

        self.onboarding_state.open = keep_open;
    }
}
