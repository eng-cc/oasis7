use super::*;

impl ClientLauncherApp {
    pub(super) fn render_onboarding_understand_step(
        &mut self,
        ui: &mut egui::Ui,
        game_running: bool,
        chain_running: bool,
    ) {
        ui.label(
            egui::RichText::new(self.tr("推荐顺序", "Recommended Order"))
                .strong()
                .size(14.0),
        );
        ui.horizontal_wrapped(|ui| {
            Self::modal_status_chip(ui, self.tr("01 启动链", "01 Chain"), Self::task_blue());
            Self::modal_status_chip(ui, self.tr("02 启动游戏", "02 Game"), Self::task_blue());
            Self::modal_status_chip(ui, self.tr("03 打开页面", "03 Page"), Self::task_blue());
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            Self::modal_status_chip(
                ui,
                if chain_running {
                    self.tr("区块链 已就绪", "Chain Ready")
                } else {
                    self.tr("区块链 未启动", "Chain Not Started")
                },
                if chain_running {
                    egui::Color32::from_rgb(62, 152, 92)
                } else {
                    egui::Color32::from_rgb(116, 119, 124)
                },
            );
            Self::modal_status_chip(
                ui,
                if game_running {
                    self.tr("游戏 运行中", "Game Running")
                } else {
                    self.tr("游戏 未启动", "Game Not Started")
                },
                if game_running {
                    egui::Color32::from_rgb(62, 152, 92)
                } else {
                    egui::Color32::from_rgb(116, 119, 124)
                },
            );
        });
    }

    pub(super) fn render_onboarding_fix_config_step(
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
        ui.small(format!(
            "{}: game={} chain={}",
            self.tr("必填问题计数", "Required issue count"),
            game_required_issues.len(),
            chain_issue_count
        ));

        if game_required_issues.is_empty() && chain_issue_count == 0 {
            Self::modal_banner(
                ui,
                self.tr(
                    "当前必填配置已通过，可进入下一步。",
                    "Required configuration is valid. You can continue.",
                ),
                egui::Color32::from_rgb(62, 152, 92),
            );
        } else {
            Self::modal_banner(
                ui,
                self.tr("请先修复下列阻断项：", "Please fix blocking issues first:"),
                egui::Color32::from_rgb(188, 60, 60),
            );
            for issue in game_required_issues {
                ui.small(format!(
                    "- [{}] {}",
                    self.tr("游戏", "Game"),
                    issue.text(self.ui_language)
                ));
            }
            if self.config.chain_enabled {
                for issue in chain_required_issues {
                    ui.small(format!(
                        "- [{}] {}",
                        self.tr("区块链", "Blockchain"),
                        issue.text(self.ui_language)
                    ));
                }
            }
        }

        ui.horizontal_wrapped(|ui| {
            if Self::modal_primary_button(ui, self.tr("打开配置引导", "Open Configuration Guide"))
                .clicked()
            {
                if !game_required_issues.is_empty() {
                    self.handle_start_game_click(game_required_issues);
                } else if self.config.chain_enabled && !chain_required_issues.is_empty() {
                    self.handle_start_chain_click(chain_required_issues);
                } else {
                    self.config_window_open = true;
                }
            }
            if Self::modal_secondary_button(ui, self.tr("打开高级配置", "Open Advanced Config"))
                .clicked()
            {
                self.config_window_open = true;
            }
        });
    }

    pub(super) fn render_onboarding_launch_step(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
        game_running: bool,
        chain_running: bool,
    ) {
        ui.label(self.tr("执行启动动作：", "Execute launch actions:"));
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.config.chain_enabled && !chain_running,
                    egui::Button::new(
                        egui::RichText::new(self.tr("启动区块链", "Start Blockchain")).strong(),
                    )
                    .fill(egui::Color32::from_rgb(42, 169, 87)),
                )
                .clicked()
            {
                self.handle_start_chain_click(chain_required_issues);
            }

            if ui
                .add_enabled(
                    !game_running,
                    egui::Button::new(
                        egui::RichText::new(self.tr("启动游戏", "Start Game")).strong(),
                    )
                    .fill(egui::Color32::from_rgb(42, 169, 87)),
                )
                .clicked()
            {
                self.handle_start_game_click(game_required_issues);
            }

            if Self::modal_secondary_button(ui, self.tr("打开游戏页", "Open Game Page")).clicked()
            {
                let url = self.current_game_url();
                if let Err(err) = open_browser(url.as_str()) {
                    self.append_log(format!("open browser failed: {err}"));
                } else {
                    self.append_log(format!("open browser: {url}"));
                }
            }
        });

        ui.separator();
        let ready_for_finish = game_running && (!self.config.chain_enabled || chain_running);
        if ready_for_finish {
            Self::modal_banner(
                ui,
                self.tr(
                    "启动链路已就绪，可以完成引导。",
                    "Startup flow is ready. You can finish onboarding.",
                ),
                egui::Color32::from_rgb(62, 152, 92),
            );
        } else {
            Self::modal_banner(
                ui,
                self.tr(
                    "提示：建议区块链与游戏都启动成功后再完成引导。",
                    "Tip: finish onboarding after blockchain and game are started.",
                ),
                egui::Color32::from_rgb(201, 146, 44),
            );
        }
    }

    pub(super) fn show_guidance_insights_window(&mut self, ctx: &egui::Context) {
        if !self.guidance_insights_open {
            return;
        }

        let mut keep_open = self.guidance_insights_open;
        egui::Window::new(self.tr("引导洞察", "Guidance Insights"))
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .default_height(320.0)
            .open(&mut keep_open)
            .show(ctx, |ui| {
                ui.label(self.tr("本地计数（重启后保留）", "Local counters (persisted)"));
                ui.separator();
                ui.small(format!(
                    "{}: {}",
                    self.tr("引导打开次数", "Onboarding Opened"),
                    self.ux_state.onboarding_opened_count
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("引导跳过次数", "Onboarding Skipped"),
                    self.ux_state.onboarding_skipped_count
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("引导完成次数", "Onboarding Completed"),
                    self.ux_state.onboarding_completed_count
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("演示模式启动次数", "Demo Mode Runs"),
                    self.ux_state.demo_mode_runs_count
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("快捷动作点击次数", "Quick Action Clicks"),
                    self.ux_state.quick_action_click_count
                ));
            });

        self.guidance_insights_open = keep_open;
    }
}
