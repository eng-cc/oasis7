use super::*;
use std::fmt::Display;

impl Drop for ClientLauncherApp {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(mut running) = self.running.take() {
                let _ = stop_child_process(&mut running.child);
            }
        }
    }
}

impl ClientLauncherApp {
    fn ui_ok_color() -> egui::Color32 {
        egui::Color32::from_rgb(62, 152, 92)
    }

    fn ui_error_color() -> egui::Color32 {
        egui::Color32::from_rgb(188, 60, 60)
    }

    fn ui_idle_color() -> egui::Color32 {
        egui::Color32::from_rgb(116, 119, 124)
    }

    fn ui_accent_color() -> egui::Color32 {
        egui::Color32::from_rgb(39, 113, 198)
    }

    fn dashboard_bg() -> egui::Color32 {
        egui::Color32::from_rgb(247, 249, 251)
    }

    fn card_fill() -> egui::Color32 {
        egui::Color32::from_rgb(255, 255, 255)
    }

    fn card_stroke() -> egui::Stroke {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(224, 229, 236))
    }

    fn launcher_status_color(&self) -> egui::Color32 {
        match self.status {
            LauncherStatus::Running => Self::ui_ok_color(),
            LauncherStatus::Idle | LauncherStatus::Stopped => Self::ui_idle_color(),
            LauncherStatus::InvalidArgs
            | LauncherStatus::StartFailed
            | LauncherStatus::StopFailed
            | LauncherStatus::QueryFailed
            | LauncherStatus::Exited(_) => Self::ui_error_color(),
        }
    }

    fn open_current_game_page(&mut self) {
        let url = self.current_game_url();
        if let Err(err) = open_browser(url.as_str()) {
            self.append_log(format!("open browser failed: {err}"));
        } else {
            self.append_log(format!("open browser: {url}"));
        }
    }

    fn compact_middle(value: &str, max_chars: usize) -> String {
        let char_count = value.chars().count();
        if char_count <= max_chars {
            return value.to_string();
        }
        let keep = max_chars.saturating_sub(1);
        let head = keep / 2;
        let tail = keep.saturating_sub(head);
        let prefix: String = value.chars().take(head).collect();
        let suffix: String = value
            .chars()
            .rev()
            .take(tail)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}…{suffix}")
    }

    fn render_status_chip(
        ui: &mut egui::Ui,
        label: &str,
        value: impl Display,
        color: egui::Color32,
    ) -> egui::Response {
        let value = value.to_string();
        let text = if label.is_empty() {
            value
        } else {
            format!("{label}  {value}")
        };
        let fill = if color == Self::ui_ok_color() {
            egui::Color32::from_rgb(229, 246, 235)
        } else if color == Self::ui_error_color() {
            egui::Color32::from_rgb(252, 235, 232)
        } else if color == egui::Color32::from_rgb(201, 146, 44) {
            egui::Color32::from_rgb(253, 244, 224)
        } else {
            egui::Color32::from_rgb(238, 244, 252)
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("• {text}"))
                        .strong()
                        .size(10.5)
                        .color(color),
                );
            })
            .response
    }

    fn render_top_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            Self::render_launcher_logo(ui);
            ui.label(egui::RichText::new("oasis7 Launcher").strong().size(14.0));
            ui.label(
                egui::RichText::new("v1.3.0")
                    .size(10.5)
                    .color(Self::ui_idle_color()),
            );
            ui.add_space(6.0);
            Self::render_status_chip(
                ui,
                self.tr("▶ 游戏", "▶ Game"),
                self.status.text(self.ui_language),
                self.launcher_status_color(),
            );
            ui.add_space(2.0);
            let response = Self::render_status_chip(
                ui,
                self.tr("◇ 区块链", "◇ Chain"),
                self.chain_runtime_status.text(self.ui_language),
                self.chain_runtime_status.color(),
            );
            if let Some(detail) = self.chain_runtime_status.detail() {
                response.on_hover_text(detail);
            }
            if is_provider_http_mode(&self.config) {
                ui.add_space(2.0);
                let provider_status = match &self.provider_check_status {
                    ProviderCheckStatus::Disabled => ProviderCheckStatus::Idle,
                    other => other.clone(),
                };
                let response = Self::render_status_chip(
                    ui,
                    self.tr("○ Provider", "○ Provider"),
                    provider_status.text(self.ui_language),
                    provider_status.color(),
                );
                if let Some(detail) = provider_status.detail() {
                    response.on_hover_text(detail);
                }
            }
            ui.add_space(8.0);
            if Self::toolbar_item(ui, self.ui_language.display_name()).clicked() {
                self.ui_language = match self.ui_language {
                    UiLanguage::ZhCn => UiLanguage::EnUs,
                    UiLanguage::EnUs => UiLanguage::ZhCn,
                };
            }
            let expert_mode = self.is_expert_mode();
            let expert_label = if expert_mode {
                self.tr("专家 ON", "Expert ON")
            } else {
                self.tr("专家 OFF", "Expert OFF")
            };
            if Self::toolbar_toggle(ui, expert_label, expert_mode).clicked() {
                self.set_expert_mode(!expert_mode);
            }
            if Self::toolbar_item(ui, self.tr("设置", "Settings")).clicked() {
                self.llm_settings_panel.open();
            }
            Self::toolbar_item(ui, self.tr("关于", "About"))
                .on_hover_text(self.tr("关于启动器", "About launcher"));
        });
    }

    fn render_launcher_header(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.render_top_status_bar(ui);
            });
    }

    fn render_launcher_logo(ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        painter.circle_filled(center, 11.0, egui::Color32::from_rgb(230, 242, 255));
        painter.circle_stroke(
            center,
            11.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(82, 133, 184)),
        );
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "O7",
            egui::FontId::proportional(10.5),
            egui::Color32::from_rgb(36, 91, 150),
        );
    }

    fn toolbar_item(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(egui::RichText::new(label).size(10.5))
                .fill(Self::card_fill())
                .stroke(Self::card_stroke())
                .min_size(egui::vec2(44.0, 22.0)),
        )
    }

    fn toolbar_toggle(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
        let fill = if enabled {
            egui::Color32::from_rgb(229, 246, 235)
        } else {
            Self::card_fill()
        };
        ui.add(
            egui::Button::new(egui::RichText::new(label).size(10.5))
                .fill(fill)
                .stroke(Self::card_stroke())
                .min_size(egui::vec2(58.0, 22.0)),
        )
    }

    fn render_next_action_deck(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        let hint = resolve_next_task_hint(
            self.config.chain_enabled,
            game_required_issues,
            chain_required_issues,
            game_running,
            chain_running,
        );
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(self.tr("下一步", "Next Action"))
                            .strong()
                            .color(egui::Color32::from_rgb(56, 86, 101)),
                    );
                    ui.label(
                        egui::RichText::new(self.next_task_hint_text(hint))
                            .strong()
                            .size(17.0),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    match hint {
                        NextTaskHint::FixChainConfig => {
                            if ui
                                .button(self.tr("修复区块链配置", "Fix Chain Config"))
                                .clicked()
                            {
                                self.record_guided_quick_action_click();
                                self.open_chain_config_guide();
                            }
                        }
                        NextTaskHint::StartChain => {
                            if ui
                                .button(self.tr("启动区块链", "Start Blockchain"))
                                .clicked()
                            {
                                self.record_guided_quick_action_click();
                                self.handle_start_chain_click(chain_required_issues);
                            }
                        }
                        NextTaskHint::FixGameConfig => {
                            if ui
                                .button(self.tr("修复游戏配置", "Fix Game Config"))
                                .clicked()
                            {
                                self.record_guided_quick_action_click();
                                self.open_game_config_guide();
                            }
                        }
                        NextTaskHint::StartGame => {
                            if ui.button(self.tr("启动游戏", "Start Game")).clicked() {
                                self.record_guided_quick_action_click();
                                self.handle_start_game_click(game_required_issues);
                            }
                        }
                        NextTaskHint::OpenGamePage => {
                            if ui.button(self.tr("打开游戏页", "Open Game Page")).clicked() {
                                self.record_guided_quick_action_click();
                                self.open_current_game_page();
                            }
                        }
                    }
                    let demo_running = matches!(
                        self.demo_mode_phase,
                        DemoModePhase::StartChainRequested
                            | DemoModePhase::WaitChainReady
                            | DemoModePhase::StartGameRequested
                            | DemoModePhase::WaitGameRunning
                    );
                    if ui
                        .add_enabled(
                            !demo_running,
                            egui::Button::new(
                                self.tr("演示模式一键启动", "Demo Mode One-Click Start"),
                            ),
                        )
                        .clicked()
                    {
                        self.record_guided_quick_action_click();
                        self.start_demo_mode_one_click();
                    }
                    ui.small(format!(
                        "{}={}",
                        self.tr("演示模式", "Demo"),
                        self.demo_mode_phase_text()
                    ));
                });
            });
        });
    }

    fn render_runtime_overview_panel(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(self.tr("系统健康", "System Health"))
                    .strong()
                    .size(15.0),
            );
            let game_url = self.current_game_url();
            let has_saved_profile = self.ux_state.last_successful_config.is_some();
            let tile_width = 116.0;
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("P2P 网络", "P2P Network"), |ui| {
                            self.render_health_p2p_body(ui);
                        });
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("可观测性", "Observability"), |ui| {
                            self.render_health_observability_body(ui);
                        });
                    },
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("配置", "Configuration"), |ui| {
                            self.render_health_config_body(
                                ui,
                                game_required_issues,
                                chain_required_issues,
                            );
                        });
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("配置档案", "Profile"), |ui| {
                            ui.small(format!(
                                "{}: {}",
                                self.tr("游戏地址", "Game URL"),
                                Self::compact_middle(game_url.as_str(), 18)
                            ))
                            .on_hover_text(game_url);
                            ui.small(format!(
                                "{}: {}",
                                self.tr("最近配置", "Profile"),
                                if has_saved_profile {
                                    self.tr("已保存", "Saved")
                                } else {
                                    self.tr("无", "None")
                                }
                            ));
                            ui.small(format!(
                                "open={} done={} quick={}",
                                self.ux_state.onboarding_opened_count,
                                self.ux_state.onboarding_completed_count,
                                self.ux_state.quick_action_click_count
                            ));
                        });
                    },
                );
            });
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(250, 252, 255))
                .stroke(Self::card_stroke())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.small(self.tr("引导洞察", "Guidance Insights"));
                        ui.separator();
                        ui.small(format!(
                            "{} {}",
                            self.ux_state.onboarding_opened_count,
                            self.tr("会话", "sessions")
                        ));
                        ui.separator();
                        ui.small(format!(
                            "{} {}",
                            self.ux_state.quick_action_click_count,
                            self.tr("快捷操作", "quick actions")
                        ));
                    });
                });
        });
    }

    fn render_health_p2p_body(&self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            Self::health_metric_pair(
                ui,
                self.tr("节点", "Peers"),
                "0",
                Self::ui_idle_color(),
                self.tr("延迟", "Latency"),
                "--",
            );
            Self::render_empty_sparkline(ui);
            Self::health_state_chip(ui, self.tr("已禁用", "Disabled"), Self::ui_idle_color());
            return;
        }

        if let Some(status) = self.chain_p2p_status.as_ref() {
            let applied_mode = status
                .applied_effective_user_mode
                .as_deref()
                .unwrap_or(status.effective_user_mode.as_str());
            let reachable = status
                .detected_reachability
                .as_deref()
                .unwrap_or(self.tr("未探测", "Unknown"));
            Self::health_metric_pair(
                ui,
                self.tr("模式", "Mode"),
                self.chain_user_mode_label(applied_mode),
                Self::ui_ok_color(),
                self.tr("可达", "Reach"),
                reachable,
            );
            ui.small(format!(
                "relay={} probe={}",
                self.p2p_probe_bool_text(status.relay_available),
                self.p2p_probe_bool_text(status.probe_stable)
            ));
        } else {
            Self::health_metric_pair(
                ui,
                self.tr("节点", "Peers"),
                "--",
                Self::ui_idle_color(),
                self.tr("延迟", "Latency"),
                "--",
            );
            Self::render_empty_sparkline(ui);
            Self::health_state_chip(ui, self.tr("等待节点", "Waiting"), Self::ui_idle_color());
        }
    }

    fn render_health_observability_body(&self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            Self::health_metric_pair(
                ui,
                self.tr("指标", "Metrics"),
                "0",
                Self::ui_idle_color(),
                self.tr("告警", "Alerts"),
                "0",
            );
            Self::render_empty_bars(ui);
            Self::health_state_chip(ui, self.tr("已禁用", "Disabled"), Self::ui_idle_color());
            return;
        }

        if let Some(status) = self.chain_observability_status.as_ref() {
            let status_color = self.observability_status_color(status.status.as_str());
            Self::health_metric_pair(
                ui,
                self.tr("Peer", "Peers"),
                status.connected_peer_count,
                status_color,
                self.tr("Heads", "Heads"),
                status.known_peer_heads,
            );
            ui.small(format!(
                "{}: {}",
                self.tr("网络落后高度", "Network Lag"),
                status.network_height_lag
            ));
            Self::health_state_chip(
                ui,
                self.observability_status_text(status.status.as_str()),
                status_color,
            );
        } else {
            Self::health_metric_pair(
                ui,
                self.tr("Peer", "Peers"),
                "--",
                Self::ui_idle_color(),
                self.tr("告警", "Alerts"),
                "--",
            );
            Self::render_empty_bars(ui);
            Self::health_state_chip(ui, self.tr("等待遥测", "Waiting"), Self::ui_idle_color());
        }
    }

    fn render_health_config_body(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        let chain_issue_count = if self.config.chain_enabled {
            chain_required_issues.len()
        } else {
            0
        };
        let has_issue = !game_required_issues.is_empty() || chain_issue_count > 0;
        let issue_count = game_required_issues.len() + chain_issue_count;
        Self::health_metric_pair(
            ui,
            self.tr("问题", "Issues"),
            issue_count,
            if has_issue {
                Self::ui_error_color()
            } else {
                Self::ui_ok_color()
            },
            self.tr("游戏", "Game"),
            game_required_issues.len(),
        );
        if ui
            .small_button(self.tr("高级配置", "Advanced Config"))
            .clicked()
        {
            self.config_window_open = true;
        }
        if has_issue {
            Self::health_state_chip(
                ui,
                self.tr("存在配置问题", "Config Issues"),
                Self::ui_error_color(),
            );
            ui.small(self.tr("查看具体字段。", "Inspect fields."));
        } else {
            Self::health_state_chip(ui, self.tr("配置通过", "Config OK"), Self::ui_ok_color());
        }
    }

    fn health_metric_pair(
        ui: &mut egui::Ui,
        primary_label: &str,
        primary_value: impl Display,
        primary_color: egui::Color32,
        secondary_label: &str,
        secondary_value: impl Display,
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(primary_value.to_string())
                        .strong()
                        .size(16.0)
                        .color(primary_color),
                );
                ui.small(primary_label);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(secondary_value.to_string())
                        .strong()
                        .size(13.0),
                );
                ui.small(secondary_label);
            });
        });
    }

    fn health_state_chip(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
        let fill = if color == Self::ui_ok_color() {
            egui::Color32::from_rgb(229, 246, 235)
        } else if color == Self::ui_error_color() {
            egui::Color32::from_rgb(252, 235, 232)
        } else {
            egui::Color32::from_rgb(240, 244, 248)
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.small(egui::RichText::new(text).strong().color(color));
            });
    }

    fn metric_tile(ui: &mut egui::Ui, title: &str, add_body: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(ui.style())
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(title).strong().size(11.5));
                add_body(ui);
            });
    }

    fn render_empty_sparkline(ui: &mut egui::Ui) {
        let width = ui.available_width().clamp(72.0, 112.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(199, 213, 228));
        let points = [
            (0.00, 0.70),
            (0.18, 0.48),
            (0.34, 0.62),
            (0.50, 0.38),
            (0.68, 0.56),
            (0.84, 0.44),
            (1.00, 0.52),
        ];
        for pair in points.windows(2) {
            let a =
                rect.left_top() + egui::vec2(rect.width() * pair[0].0, rect.height() * pair[0].1);
            let b =
                rect.left_top() + egui::vec2(rect.width() * pair[1].0, rect.height() * pair[1].1);
            painter.line_segment([a, b], stroke);
        }
    }

    fn render_empty_bars(ui: &mut egui::Ui) {
        let graph_width = ui.available_width().clamp(72.0, 112.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(graph_width, 24.0), egui::Sense::hover());
        let painter = ui.painter();
        let fill = egui::Color32::from_rgb(207, 224, 242);
        for index in 0..14 {
            let width = 4.0;
            let gap = 3.0;
            let x = rect.left() + index as f32 * (width + gap);
            let height = 6.0 + ((index * 7) % 17) as f32;
            let bar = egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - height),
                egui::vec2(width, height),
            );
            painter.rect_filled(bar, 1.0, fill);
        }
    }

    fn render_launcher_command_deck(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        let can_click_start_game = !game_running;
        let can_click_start_chain = self.config.chain_enabled && !chain_running;
        let has_saved_profile = self.ux_state.last_successful_config.is_some();

        egui::Frame::group(ui.style())
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::same(9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(self.tr("快捷命令", "Command Dock"))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    if Self::command_tile(
                        ui,
                        "🔗",
                        self.tr("启动链", "Chain"),
                        can_click_start_chain,
                    )
                    .clicked()
                    {
                        self.handle_start_chain_click(chain_required_issues);
                    }
                    if Self::command_tile(ui, "▶", self.tr("游戏", "Game"), can_click_start_game)
                        .clicked()
                    {
                        self.handle_start_game_click(game_required_issues);
                    }
                    if Self::command_tile(ui, "↗", self.tr("页面", "Page"), true).clicked() {
                        self.open_current_game_page();
                    }
                    if Self::command_tile(ui, "⚙", self.tr("配置", "Config"), true).clicked() {
                        self.config_window_open = true;
                    }
                    if !self.is_expert_mode() {
                        if Self::command_tile(ui, "?", self.tr("引导", "Guide"), true).clicked() {
                            self.open_onboarding_manual();
                        }
                    }
                    if Self::command_tile(ui, "◇", self.tr("洞察", "Insights"), true).clicked()
                    {
                        self.guidance_insights_open = true;
                    }
                    ui.menu_button(self.tr("更多", "More"), |ui| {
                        if ui
                            .add_enabled(
                                chain_running,
                                egui::Button::new(self.tr("停止区块链", "Stop Chain")),
                            )
                            .clicked()
                        {
                            self.stop_chain_process();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                game_running,
                                egui::Button::new(self.tr("停止游戏", "Stop Game")),
                            )
                            .clicked()
                        {
                            self.stop_process();
                            ui.close();
                        }
                        if ui.button(self.tr("重置引导", "Reset Guide")).clicked() {
                            self.reset_onboarding();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                has_saved_profile,
                                egui::Button::new(self.tr("恢复配置", "Restore Config")),
                            )
                            .clicked()
                        {
                            self.restore_last_successful_config_profile();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                has_saved_profile,
                                egui::Button::new(self.tr("清空配置", "Clear Config")),
                            )
                            .clicked()
                        {
                            self.clear_last_successful_config_profile();
                            ui.close();
                        }
                        let can_reset_demo = matches!(
                            self.demo_mode_phase,
                            DemoModePhase::Done | DemoModePhase::Failed
                        );
                        if ui
                            .add_enabled(
                                can_reset_demo,
                                egui::Button::new(self.tr("重置演示", "Reset Demo")),
                            )
                            .clicked()
                        {
                            self.reset_demo_mode();
                            ui.close();
                        }
                        if ui.button(self.tr("设置", "Settings")).clicked() {
                            self.llm_settings_panel.open();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.is_feedback_available(),
                                egui::Button::new(self.tr("反馈", "Feedback")),
                            )
                            .clicked()
                        {
                            self.feedback_window_open = true;
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.is_feedback_available(),
                                egui::Button::new(self.tr("转账", "Transfer")),
                            )
                            .clicked()
                        {
                            self.transfer_window_open = true;
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.is_feedback_available(),
                                egui::Button::new(self.tr("浏览器", "Explorer")),
                            )
                            .clicked()
                        {
                            self.explorer_window_open = true;
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.config.chain_enabled,
                                egui::Button::new(self.tr("Peer 窗口", "Peers")),
                            )
                            .clicked()
                        {
                            self.peer_details_window_open = true;
                            ui.close();
                        }
                        if ui.button(self.tr("清空日志", "Clear Log")).clicked() {
                            self.logs.clear();
                            ui.close();
                        }
                    });
                });
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "↻  {}",
                                self.tr("一键修复", "One-Click Repair")
                            ))
                            .size(12.0),
                        )
                        .fill(egui::Color32::from_rgb(255, 255, 255))
                        .stroke(Self::card_stroke())
                        .min_size(egui::vec2(ui.available_width(), 26.0)),
                    )
                    .clicked()
                {
                    self.open_game_config_guide();
                }
            });
    }

    fn command_tile(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool) -> egui::Response {
        let text = egui::RichText::new(format!("{icon}\n{label}"))
            .size(10.0)
            .color(if enabled {
                Self::ui_accent_color()
            } else {
                Self::ui_idle_color()
            });
        ui.add_enabled(
            enabled,
            egui::Button::new(text)
                .fill(Self::card_fill())
                .stroke(Self::card_stroke())
                .min_size(egui::vec2(50.0, 46.0)),
        )
    }

    fn render_launcher_footer_band(&self, ui: &mut egui::Ui) {
        let game_url = self.current_game_url();
        let profile = self.config.agent_provider_profile.trim();
        let profile = if profile.is_empty() {
            self.tr("default", "default")
        } else {
            profile
        };
        egui::Frame::new()
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(9, 5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("● {}", self.tr("就绪", "Ready")))
                            .strong()
                            .size(10.5)
                            .color(Self::ui_ok_color()),
                    );
                    ui.add_space(34.0);
                    ui.small(format!("{}: {profile}", self.tr("配置", "Profile")));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("网络", "Network"),
                        self.config.chain_network_tier.trim()
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("游戏地址", "Game URL"),
                        Self::compact_middle(game_url.as_str(), 34)
                    ))
                    .on_hover_text(game_url);
                    ui.separator();
                    ui.small(format!(
                        "{}  {}",
                        self.tr("本地时间", "Local Time"),
                        self.tr("强 Strong", "Strong")
                    ));
                });
            });
    }

    fn render_launcher_logs_panel(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style())
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::same(9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(self.tr("运行日志", "Runtime Log"))
                            .strong()
                            .size(14.0),
                    );
                    Self::log_filter_chip(ui, self.tr("全部", "All"), true);
                    Self::log_filter_chip(ui, self.tr("信息", "Info"), false);
                    Self::log_filter_chip(ui, self.tr("警告", "Warn"), false);
                    Self::log_filter_chip(ui, self.tr("错误", "Error"), false);
                });
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.set_height(16.0);
                    ui.small(egui::RichText::new("LEVEL").color(Self::ui_idle_color()));
                    ui.add_space(36.0);
                    ui.small(egui::RichText::new("SOURCE").color(Self::ui_idle_color()));
                    ui.add_space(48.0);
                    ui.small(egui::RichText::new("MESSAGE").color(Self::ui_idle_color()));
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(112.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.logs {
                            ui.horizontal(|ui| {
                                let level = if line.contains("ERROR") || line.contains("error") {
                                    ("ERR", Self::ui_error_color())
                                } else if line.contains("WARN") || line.contains("warn") {
                                    ("WARN", egui::Color32::from_rgb(201, 146, 44))
                                } else {
                                    ("INFO", Self::ui_ok_color())
                                };
                                let (source, message) = Self::log_source_and_message(line.as_str());
                                ui.set_height(22.0);
                                Self::render_log_badge(ui, level.0, level.1);
                                ui.add_space(4.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(82.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.small(Self::compact_middle(source, 15));
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.small(Self::compact_middle(message, 58))
                                            .on_hover_text(line);
                                    },
                                );
                            });
                        }
                    });
            });
    }

    fn render_log_badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
        let fill = if color == Self::ui_ok_color() {
            egui::Color32::from_rgb(229, 246, 235)
        } else if color == Self::ui_error_color() {
            egui::Color32::from_rgb(252, 235, 232)
        } else {
            egui::Color32::from_rgb(253, 244, 224)
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.small(egui::RichText::new(label).strong().color(color));
            });
    }

    fn log_source_and_message(line: &str) -> (&str, &str) {
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let source = &rest[..end];
                let message = rest[end + 1..].trim();
                return (source, message);
            }
        }
        if line.contains("control plane") {
            ("control", line)
        } else if line.contains("launcher") {
            ("launcher", line)
        } else {
            ("system", line)
        }
    }

    fn log_filter_chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
        let fill = if active {
            egui::Color32::from_rgb(232, 242, 255)
        } else {
            Self::card_fill()
        };
        ui.add(
            egui::Button::new(egui::RichText::new(label).size(10.0))
                .fill(fill)
                .stroke(Self::card_stroke())
                .min_size(egui::vec2(34.0, 20.0)),
        )
    }

    fn render_normal_dashboard(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        let width = ui.available_width();
        if width >= 840.0 {
            let left_width = (width * 0.30).clamp(236.0, 292.0);
            let right_width = (width * 0.30).clamp(236.0, 292.0);
            let middle_width = (width - left_width - right_width - 32.0).max(300.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_width);
                    self.render_startup_preflight_checklist(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                    );
                });
                ui.vertical(|ui| {
                    ui.set_width(middle_width);
                    self.render_task_flow_cards(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                        game_running,
                        chain_running,
                    );
                });
                ui.vertical(|ui| {
                    ui.set_width(right_width);
                    self.render_runtime_overview_panel(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                    );
                });
            });
        } else if width >= 700.0 {
            self.render_task_flow_cards(
                ui,
                game_required_issues,
                chain_required_issues,
                game_running,
                chain_running,
            );
            ui.add_space(6.0);
            ui.horizontal_top(|ui| {
                let column_width = (width - 8.0) / 2.0;
                ui.vertical(|ui| {
                    ui.set_width(column_width);
                    self.render_startup_preflight_checklist(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                    );
                });
                ui.vertical(|ui| {
                    ui.set_width(column_width);
                    self.render_runtime_overview_panel(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                    );
                });
            });
        } else {
            self.render_task_flow_cards(
                ui,
                game_required_issues,
                chain_required_issues,
                game_running,
                chain_running,
            );
            ui.add_space(6.0);
            self.render_startup_preflight_checklist(
                ui,
                game_required_issues,
                chain_required_issues,
            );
            ui.add_space(6.0);
            self.render_runtime_overview_panel(ui, game_required_issues, chain_required_issues);
        }
    }

    fn render_bottom_dashboard(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        let width = ui.available_width();
        if width >= 820.0 {
            let dock_width = (width * 0.61).clamp(430.0, 620.0);
            let log_width = (width - dock_width - 10.0).max(300.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(dock_width);
                    self.render_launcher_command_deck(
                        ui,
                        game_required_issues,
                        chain_required_issues,
                        game_running,
                        chain_running,
                    );
                    if self.is_expert_mode() {
                        self.render_disabled_action_ctas(
                            ui,
                            game_required_issues,
                            chain_required_issues,
                            chain_running,
                        );
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width(log_width);
                    self.render_launcher_logs_panel(ui);
                });
            });
        } else {
            self.render_launcher_command_deck(
                ui,
                game_required_issues,
                chain_required_issues,
                game_running,
                chain_running,
            );
            if self.is_expert_mode() {
                self.render_disabled_action_ctas(
                    ui,
                    game_required_issues,
                    chain_required_issues,
                    chain_running,
                );
            }
            ui.add_space(6.0);
            self.render_launcher_logs_panel(ui);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn screenshot_modal_name() -> Option<String> {
        env::var(OASIS7_CLIENT_LAUNCHER_SCREENSHOT_MODAL_ENV)
            .ok()
            .map(|raw| raw.trim().to_ascii_lowercase())
            .filter(|raw| !raw.is_empty())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn screenshot_modal_preview_width(modal: &str, available_width: f32) -> f32 {
        let desired_width = match modal {
            "transfer" | "chain_transfer" => 640.0,
            "explorer" | "blockchain_explorer" => 760.0,
            "peer" | "peer_details" | "p2p" => 700.0,
            _ => 820.0,
        };
        available_width.min(desired_width)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_screenshot_modal_preview(
        &mut self,
        ui: &mut egui::Ui,
        modal: &str,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) -> bool {
        match modal {
            "startup" | "startup_guide" | "guide" => {
                Self::modal_header(
                    ui,
                    self.tr("启动引导（游戏）", "Startup Guide (Game)"),
                    self.tr(
                        "检测到游戏启动前有必填问题，请直接在此窗口补齐。",
                        "Required game settings are missing. Fill them directly in this window.",
                    ),
                    Some((
                        self.tr("需修复", "Needs Fix"),
                        egui::Color32::from_rgb(188, 60, 60),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("待修复问题", "Issues to Fix"))
                            .strong()
                            .size(14.0),
                    );
                    for issue in game_required_issues {
                        ui.horizontal_wrapped(|ui| {
                            Self::modal_status_chip(
                                ui,
                                self.tr("阻断", "Blocked"),
                                egui::Color32::from_rgb(188, 60, 60),
                            );
                            ui.small(issue.text(self.ui_language));
                        });
                    }
                });
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("直接编辑下列字段", "Edit Fields Directly"))
                            .strong()
                            .size(14.0),
                    );
                    let field_ids = self.collect_issue_fields(game_required_issues);
                    for field_id in &field_ids {
                        let stack_text_fields = ui.available_width() <= 560.0;
                        let _ = self.render_config_field_by_id(ui, field_id, stack_text_fields);
                    }
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let _ = Self::modal_secondary_button(
                        ui,
                        self.tr("打开高级配置", "Open Advanced Config"),
                    );
                    let _ = Self::modal_primary_button(ui, self.tr("关闭", "Close"));
                });
                true
            }
            "onboarding" | "first_run" => {
                Self::modal_header(
                    ui,
                    self.tr("理解启动顺序", "Understand Startup Order"),
                    self.tr(
                        "先确认链、游戏、页面三段启动路径。",
                        "Confirm the chain, game, and page launch path first.",
                    ),
                    Some((
                        self.tr("进度 1/3", "Progress 1/3"),
                        egui::Color32::from_rgb(74, 116, 168),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        Self::modal_status_chip(
                            ui,
                            self.tr("01 理解", "01 Understand"),
                            Self::task_blue(),
                        );
                        Self::modal_status_chip(
                            ui,
                            self.tr("02 配置", "02 Configure"),
                            egui::Color32::from_rgb(116, 119, 124),
                        );
                        Self::modal_status_chip(
                            ui,
                            self.tr("03 启动", "03 Launch"),
                            egui::Color32::from_rgb(116, 119, 124),
                        );
                    });
                    ui.add_space(8.0);
                    self.render_onboarding_understand_step(ui, game_running, chain_running);
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let _ = Self::modal_primary_button(ui, self.tr("下一步", "Next"));
                    let _ = Self::modal_secondary_button(
                        ui,
                        self.tr("跳过（稍后再看）", "Skip for now"),
                    );
                });
                true
            }
            "feedback" => {
                Self::modal_header(
                    ui,
                    self.tr("反馈", "Feedback"),
                    self.tr(
                        "记录问题、建议和近期运行上下文。",
                        "Capture issues, suggestions, and recent runtime context.",
                    ),
                    Some((
                        self.tr("链未就绪", "Chain Pending"),
                        egui::Color32::from_rgb(201, 146, 44),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("类型", "Type"));
                        Self::modal_status_chip(ui, "Bug", Self::task_blue());
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
                            .desired_width(ui.available_width()),
                    );
                });
                ui.add_space(8.0);
                Self::modal_banner(
                    ui,
                    self.tr(
                        "提交前请完善必填项：反馈标题不能为空 / 反馈描述不能为空",
                        "Please complete required fields before submit: title and description are required",
                    ),
                    egui::Color32::from_rgb(196, 84, 84),
                );
                ui.add_space(8.0);
                let _ = Self::modal_primary_button(ui, self.tr("提交反馈", "Submit Feedback"));
                true
            }
            "settings" | "llm_settings" => {
                Self::modal_header(
                    ui,
                    self.tr("设置中心", "Settings Center"),
                    self.tr(
                        "管理启动器、链运行时和 LLM 连接配置。",
                        "Manage launcher, chain runtime, and LLM connection settings.",
                    ),
                    Some((
                        self.tr("本地配置", "Local Config"),
                        egui::Color32::from_rgb(74, 116, 168),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("游戏与显示", "Game & Viewer"))
                            .strong()
                            .size(14.0),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("场景", "Scenario"));
                        ui.text_edit_singleline(&mut self.config.scenario);
                        ui.label(self.tr("游戏页面主机", "Viewer Host"));
                        ui.text_edit_singleline(&mut self.config.viewer_host);
                    });
                    ui.horizontal_wrapped(|ui| {
                        let llm_label = self.tr("启用 LLM", "Enable LLM");
                        let browser_label = self.tr("自动打开浏览器", "Open Browser Automatically");
                        ui.checkbox(&mut self.config.llm_enabled, llm_label);
                        ui.checkbox(&mut self.config.auto_open_browser, browser_label);
                    });
                });
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("LLM 连接配置", "LLM Connection"))
                            .strong()
                            .size(14.0),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label("API Key");
                        ui.label(egui::RichText::new("••••••••").monospace());
                        ui.label("Model");
                        ui.label(egui::RichText::new("model").monospace());
                    });
                });
                ui.add_space(8.0);
                let _ = Self::modal_primary_button(ui, self.tr("保存到 config.toml", "Save"));
                true
            }
            "transfer" | "chain_transfer" => {
                Self::modal_header(
                    ui,
                    self.tr("链上转账", "On-Chain Transfer"),
                    self.tr(
                        "选择账户、金额和 nonce 后提交链上转账。",
                        "Choose accounts, amount, and nonce before submitting an on-chain transfer.",
                    ),
                    Some((
                        self.tr("可提交", "Ready"),
                        egui::Color32::from_rgb(62, 152, 92),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("提交转账", "Submit Transfer"))
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("转出账户", "From Account"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.transfer_draft.from_account_id)
                                .desired_width(260.0),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("转入账户", "To Account"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.transfer_draft.to_account_id)
                                .desired_width(260.0),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("金额", "Amount"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.transfer_draft.amount)
                                .desired_width(140.0),
                        );
                        ui.label(self.tr("Nonce 模式", "Nonce Mode"));
                        Self::modal_status_chip(
                            ui,
                            self.tr("自动", "Auto"),
                            egui::Color32::from_rgb(62, 152, 92),
                        );
                    });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let _ =
                            Self::modal_primary_button(ui, self.tr("提交转账", "Submit Transfer"));
                        let _ = Self::modal_secondary_button(
                            ui,
                            self.tr("刷新账户/历史", "Refresh Accounts/History"),
                        );
                    });
                });
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("状态追踪", "Transfer Status"))
                            .strong()
                            .size(14.0),
                    );
                    ui.small(self.tr(
                        "提交后会在这里展示 action 状态和确认进度。",
                        "After submit, action status and confirmation progress appear here.",
                    ));
                });
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(self.tr("转账历史", "Transfer History"))
                            .strong()
                            .size(14.0),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("账户过滤", "Account Filter"));
                        ui.text_edit_singleline(
                            &mut self.transfer_panel_state.history_account_filter,
                        );
                        let _ =
                            Self::modal_secondary_button(ui, self.tr("应用过滤", "Apply Filter"));
                        let _ =
                            Self::modal_secondary_button(ui, self.tr("清空过滤", "Clear Filters"));
                    });
                    ui.small(self.tr("暂无转账历史", "No transfer history"));
                });
                true
            }
            "explorer" | "blockchain_explorer" => {
                Self::modal_header(
                    ui,
                    self.tr("区块链浏览器", "Blockchain Explorer"),
                    self.tr(
                        "浏览链健康、区块、交易、账户和内存池状态。",
                        "Browse chain health, blocks, transactions, accounts, and mempool state.",
                    ),
                    Some((
                        self.tr("链已就绪", "Chain Ready"),
                        egui::Color32::from_rgb(62, 152, 92),
                    )),
                );
                ui.add_space(8.0);
                self.render_explorer_command_deck(ui);
                ui.add_space(6.0);
                self.render_overview(ui);
                ui.add_space(6.0);
                self.render_tabs(ui);
                true
            }
            "peer" | "peer_details" | "p2p" => {
                Self::modal_header(
                    ui,
                    self.tr("P2P Peer 明细", "P2P Peer Details"),
                    self.tr(
                        "查看本地 peer、已连接 peer、路径和来源细节。",
                        "Inspect local peer, connected peers, path kind, and discovery details.",
                    ),
                    Some((
                        self.tr("链已启用", "Chain Enabled"),
                        egui::Color32::from_rgb(62, 152, 92),
                    )),
                );
                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                    self.render_chain_peer_details_panel(ui, false);
                });
                true
            }
            _ => {
                let _ = chain_required_issues;
                false
            }
        }
    }
}

impl eframe::App for ClientLauncherApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let color = Self::dashboard_bg();
        color.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_process();
        self.poll_chain_process();
        self.maybe_auto_start_chain();
        self.update_chain_runtime_status();
        #[cfg(target_arch = "wasm32")]
        launcher_test_hook_web::sync_launcher_test_hook(self);

        let game_required_issues = collect_required_config_issues(&self.config);
        let chain_required_issues = collect_chain_required_config_issues(&self.config);
        let game_running = matches!(self.status, LauncherStatus::Running);
        let chain_running = matches!(
            self.chain_runtime_status,
            ChainRuntimeStatus::Starting | ChainRuntimeStatus::Ready
        );
        self.maybe_save_last_successful_config_profile(game_running);
        self.maybe_open_onboarding_on_first_visit(
            &game_required_issues,
            &chain_required_issues,
            game_running,
            chain_running,
        );
        if self.onboarding_state.completed {
            self.maybe_open_startup_guide_on_first_check(
                &game_required_issues,
                &chain_required_issues,
            );
        }
        self.advance_demo_mode(
            &game_required_issues,
            &chain_required_issues,
            game_running,
            chain_running,
        );

        #[cfg(not(target_arch = "wasm32"))]
        let mut rendered_screenshot_modal_preview = false;
        #[cfg(target_arch = "wasm32")]
        let rendered_screenshot_modal_preview = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Self::dashboard_bg())
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                self.render_launcher_header(ui);
                ui.add_space(6.0);
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(modal) = Self::screenshot_modal_name() {
                    ui.horizontal_top(|ui| {
                        let preview_width =
                            Self::screenshot_modal_preview_width(modal.as_str(), ui.available_width());
                        let side_space = ((ui.available_width() - preview_width) / 2.0).max(0.0);
                        ui.add_space(side_space);
                        ui.vertical(|ui| {
                            ui.set_width(preview_width);
                            rendered_screenshot_modal_preview = self.render_screenshot_modal_preview(
                                ui,
                                modal.as_str(),
                                &game_required_issues,
                                &chain_required_issues,
                                game_running,
                                chain_running,
                            );
                        });
                    });
                    if rendered_screenshot_modal_preview {
                        return;
                    }
                }
                self.render_onboarding_reminder_banner(ui);
                if self.should_show_onboarding_reminder() {
                    ui.add_space(6.0);
            }

            if self.is_expert_mode() {
                self.render_next_action_deck(
                    ui,
                    &game_required_issues,
                    &chain_required_issues,
                    game_running,
                    chain_running,
                );
                ui.add_space(6.0);
                ui.small(self.tr(
                    "专家模式已开启：任务卡片收起，但下一步和状态仍保留。",
                    "Expert mode enabled: task cards are hidden, while next action and status remain visible.",
                ));
                self.render_startup_error_cards(ui, &game_required_issues, &chain_required_issues);
                ui.add_space(6.0);
                ui.collapsing(self.tr("完整运行诊断", "Full Runtime Diagnostics"), |ui| {
                    self.render_config_validation_summary(
                        ui,
                        &game_required_issues,
                        &chain_required_issues,
                    );
                    self.render_chain_p2p_summary(ui);
                    self.render_chain_observability_summary(ui);
                });
                ui.add_space(6.0);
            } else {
                self.render_normal_dashboard(
                    ui,
                    &game_required_issues,
                    &chain_required_issues,
                    game_running,
                    chain_running,
                );
            }
            ui.add_space(6.0);

            self.render_bottom_dashboard(
                ui,
                &game_required_issues,
                &chain_required_issues,
                game_running,
                chain_running,
            );
            ui.add_space(6.0);
            self.render_launcher_footer_band(ui);
            });

        if !rendered_screenshot_modal_preview {
            self.show_config_window(ctx, &game_required_issues, &chain_required_issues);
            self.show_onboarding_window(
                ctx,
                &game_required_issues,
                &chain_required_issues,
                game_running,
                chain_running,
            );
            self.show_guidance_insights_window(ctx);
            self.show_startup_guide_window(ctx, &game_required_issues, &chain_required_issues);
            self.llm_settings_panel
                .show(ctx, self.ui_language, &mut self.config);
            self.show_feedback_window(ctx);
            self.show_transfer_window(ctx);
            self.show_peer_details_window(ctx);
            self.show_explorer_window(ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(120));
    }
}
