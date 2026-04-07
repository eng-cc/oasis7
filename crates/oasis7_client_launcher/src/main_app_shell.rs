use super::*;

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

impl eframe::App for ClientLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_process();
        self.poll_chain_process();
        self.maybe_auto_start_chain();
        self.update_chain_runtime_status();
        #[cfg(target_arch = "wasm32")]
        launcher_test_hook_web::sync_launcher_test_hook(self);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(self.tr("oasis7 客户端启动器", "oasis7 Client Launcher"));
                ui.separator();
                ui.label(format!(
                    "{}: {}",
                    self.tr("游戏", "Game"),
                    self.status.text(self.ui_language)
                ));
                ui.separator();
                let chain_status = format!(
                    "{}: {}",
                    self.tr("区块链", "Blockchain"),
                    self.chain_runtime_status.text(self.ui_language)
                );
                let response =
                    ui.colored_label(self.chain_runtime_status.color(), chain_status.as_str());
                if let Some(detail) = self.chain_runtime_status.detail() {
                    response.on_hover_text(detail);
                }
                if is_openclaw_local_http_mode(&self.config) {
                    ui.separator();
                    let provider_status = match &self.openclaw_provider_check_status {
                        OpenClawProviderCheckStatus::Disabled => OpenClawProviderCheckStatus::Idle,
                        other => other.clone(),
                    };
                    let provider_label = format!(
                        "{}: {}",
                        self.tr("OpenClaw", "OpenClaw"),
                        provider_status.text(self.ui_language)
                    );
                    let response =
                        ui.colored_label(provider_status.color(), provider_label.as_str());
                    if let Some(detail) = provider_status.detail() {
                        response.on_hover_text(detail);
                    }
                }
                ui.separator();
                ui.label(self.tr("语言", "Language"));
                egui::ComboBox::from_id_salt("launcher_language")
                    .selected_text(self.ui_language.display_name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.ui_language,
                            UiLanguage::ZhCn,
                            UiLanguage::ZhCn.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.ui_language,
                            UiLanguage::EnUs,
                            UiLanguage::EnUs.display_name(),
                        );
                    });
                ui.separator();
                let mut expert_mode = self.is_expert_mode();
                if ui
                    .checkbox(&mut expert_mode, self.tr("专家模式", "Expert Mode"))
                    .changed()
                {
                    self.set_expert_mode(expert_mode);
                }
            });
        });

        let game_required_issues = collect_required_config_issues(&self.config);
        let chain_required_issues = collect_chain_required_config_issues(&self.config);
        let game_running = matches!(self.status, LauncherStatus::Running);
        let chain_running = matches!(
            self.chain_runtime_status,
            ChainRuntimeStatus::Starting | ChainRuntimeStatus::Ready
        );
        self.maybe_save_last_successful_config_profile(game_running);
        let can_click_start_game = !game_running;
        let can_click_start_chain = self.config.chain_enabled && !chain_running;
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

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_startup_preflight_checklist(
                ui,
                &game_required_issues,
                &chain_required_issues,
            );
            ui.separator();

            self.render_onboarding_reminder_banner(ui);
            if self.should_show_onboarding_reminder() {
                ui.separator();
            }

            self.render_startup_error_cards(ui, &game_required_issues, &chain_required_issues);
            ui.separator();

            self.render_config_validation_summary(
                ui,
                &game_required_issues,
                &chain_required_issues,
            );
            ui.separator();
            self.render_chain_p2p_summary(ui);

            ui.separator();
            if self.is_expert_mode() {
                ui.small(self.tr(
                    "专家模式已开启：已隐藏新手任务流卡片。",
                    "Expert mode enabled: guided task cards are hidden.",
                ));
            } else {
                self.render_task_flow_cards(
                    ui,
                    &game_required_issues,
                    &chain_required_issues,
                    game_running,
                    chain_running,
                );
            }
            ui.separator();

            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        can_click_start_game,
                        egui::Button::new(self.tr("启动游戏", "Start Game")),
                    )
                    .clicked()
                {
                    self.handle_start_game_click(&game_required_issues);
                }
                if ui
                    .add_enabled(
                        game_running,
                        egui::Button::new(self.tr("停止游戏", "Stop Game")),
                    )
                    .clicked()
                {
                    self.stop_process();
                }
                if ui
                    .add_enabled(
                        can_click_start_chain,
                        egui::Button::new(self.tr("启动区块链", "Start Blockchain")),
                    )
                    .clicked()
                {
                    self.handle_start_chain_click(&chain_required_issues);
                }
                if ui
                    .add_enabled(
                        chain_running,
                        egui::Button::new(self.tr("停止区块链", "Stop Blockchain")),
                    )
                    .clicked()
                {
                    self.stop_chain_process();
                }
                if ui.button(self.tr("高级配置", "Advanced Config")).clicked() {
                    self.config_window_open = true;
                }
                if !self.is_expert_mode() {
                    if ui.button(self.tr("新手引导", "Onboarding")).clicked() {
                        self.open_onboarding_manual();
                    }
                    if ui.button(self.tr("重置引导", "Reset Guide")).clicked() {
                        self.reset_onboarding();
                    }
                }
                let has_saved_profile = self.ux_state.last_successful_config.is_some();
                if ui
                    .add_enabled(
                        has_saved_profile,
                        egui::Button::new(
                            self.tr("恢复最近成功配置", "Restore Last Successful Config"),
                        ),
                    )
                    .clicked()
                {
                    self.restore_last_successful_config_profile();
                }
                if ui
                    .add_enabled(
                        has_saved_profile,
                        egui::Button::new(self.tr("清空成功配置", "Clear Saved Config")),
                    )
                    .clicked()
                {
                    self.clear_last_successful_config_profile();
                }
                if let Some(saved_at) = self.ux_state.last_successful_saved_at_unix_ms {
                    ui.small(format!(
                        "{}={saved_at}",
                        self.tr("最近成功配置时间戳", "Saved Profile Timestamp")
                    ));
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
                        egui::Button::new(self.tr("演示模式一键启动", "Demo Mode One-Click Start")),
                    )
                    .clicked()
                {
                    self.start_demo_mode_one_click();
                }
                if matches!(
                    self.demo_mode_phase,
                    DemoModePhase::Done | DemoModePhase::Failed
                ) && ui
                    .button(self.tr("重置演示状态", "Reset Demo State"))
                    .clicked()
                {
                    self.reset_demo_mode();
                }
                ui.small(format!(
                    "{}={}",
                    self.tr("演示模式状态", "Demo Mode Status"),
                    self.demo_mode_phase_text()
                ));
                if ui
                    .button(self.tr("引导洞察", "Guidance Insights"))
                    .clicked()
                {
                    self.guidance_insights_open = true;
                }
                if ui.button(self.tr("打开游戏页", "Open Game Page")).clicked() {
                    let url = self.current_game_url();
                    if let Err(err) = open_browser(url.as_str()) {
                        self.append_log(format!("open browser failed: {err}"));
                    } else {
                        self.append_log(format!("open browser: {url}"));
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button(self.tr("设置", "Settings")).clicked() {
                        self.llm_settings_panel.open();
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("反馈", "Feedback")),
                        )
                        .clicked()
                    {
                        self.feedback_window_open = true;
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("转账", "Transfer")),
                        )
                        .clicked()
                    {
                        self.transfer_window_open = true;
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("浏览器", "Explorer")),
                        )
                        .clicked()
                    {
                        self.explorer_window_open = true;
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    if ui.button(self.tr("设置", "Settings")).clicked() {
                        self.llm_settings_panel.open();
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("反馈", "Feedback")),
                        )
                        .clicked()
                    {
                        self.feedback_window_open = true;
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("转账", "Transfer")),
                        )
                        .clicked()
                    {
                        self.transfer_window_open = true;
                    }
                    if ui
                        .add_enabled(
                            self.is_feedback_available(),
                            egui::Button::new(self.tr("浏览器", "Explorer")),
                        )
                        .clicked()
                    {
                        self.explorer_window_open = true;
                    }
                }
                if ui.button(self.tr("清空日志", "Clear Logs")).clicked() {
                    self.logs.clear();
                }
            });
            self.render_disabled_action_ctas(
                ui,
                &game_required_issues,
                &chain_required_issues,
                chain_running,
            );

            let url = self.current_game_url();
            ui.label(format!("{}: {url}", self.tr("游戏地址", "Game URL")));

            ui.separator();
            ui.label(self.tr("日志（stdout/stderr）", "Logs (stdout/stderr)"));

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.label(line);
                    }
                });
        });

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
        self.show_explorer_window(ctx);
        ctx.request_repaint_after(Duration::from_millis(120));
    }
}
