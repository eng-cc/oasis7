use super::*;

#[cfg(not(target_arch = "wasm32"))]
use std::env;

#[cfg(not(target_arch = "wasm32"))]
impl ClientLauncherApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn screenshot_modal_name() -> Option<String> {
        env::var(OASIS7_CLIENT_LAUNCHER_SCREENSHOT_MODAL_ENV)
            .ok()
            .map(|raw| raw.trim().to_ascii_lowercase())
            .filter(|raw| !raw.is_empty())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn screenshot_modal_preview_width(modal: &str, available_width: f32) -> f32 {
        let desired_width = match modal {
            "transfer" | "chain_transfer" => 640.0,
            "explorer" | "blockchain_explorer" => 760.0,
            "peer" | "peer_details" | "p2p" => 700.0,
            _ => 820.0,
        };
        available_width.min(desired_width)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn render_screenshot_modal_preview(
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
