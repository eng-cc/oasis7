use super::*;

impl ClientLauncherApp {
    pub(crate) fn show_explorer_window(&mut self, ctx: &egui::Context) {
        if !self.explorer_window_open {
            return;
        }

        if self.explorer_panel_state.overview.is_none() {
            self.explorer_panel_state.pending_overview_refresh = true;
        }
        if self.explorer_panel_state.blocks.is_empty() {
            self.explorer_panel_state.pending_blocks_refresh = true;
        }
        if self.explorer_panel_state.txs.is_empty() {
            self.explorer_panel_state.pending_txs_refresh = true;
        }
        self.maybe_request_explorer_panel_data();

        let title = self.tr("区块链浏览器", "Blockchain Explorer").to_string();
        let mut window_open = self.explorer_window_open;
        egui::Window::new(title)
            .open(&mut window_open)
            .resizable(true)
            .default_size(egui::vec2(1240.0, 780.0))
            .show(ctx, |ui| {
                self.render_explorer_command_deck(ui);
                ui.add_space(6.0);
                self.render_overview(ui);
                ui.add_space(6.0);
                self.render_tabs(ui);
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.explorer_panel_state.active_tab {
                        ExplorerTab::Blocks => self.render_blocks_tab(ui),
                        ExplorerTab::Txs => self.render_txs_tab(ui),
                        ExplorerTab::Search => self.render_search_tab(ui),
                        ExplorerTab::Address => self.render_address_tab(ui),
                        ExplorerTab::Contracts => self.render_contracts_tab(ui),
                        ExplorerTab::Assets => self.render_assets_tab(ui),
                        ExplorerTab::Mempool => self.render_mempool_tab(ui),
                    });
            });

        self.explorer_window_open = window_open;
    }

    fn render_explorer_command_deck(&mut self, ui: &mut egui::Ui) {
        let title = self.tr("Explorer Deck", "Explorer Deck");
        let subtitle = self.tr(
            "主链级浏览器操作台：先判断链健康，再进入明细核查。",
            "Mainnet-grade operator deck: check chain health first, then drill into detail.",
        );
        let refresh_text = self.tr("刷新当前视图", "Refresh Current View");
        let reset_text = self.tr("重置当前筛选", "Reset Current Filters");
        let idle_text = self.tr("空闲", "Idle");
        let inflight_text = self.tr("请求中", "In Flight");
        let chain_ready_text = self.tr("链已就绪", "Chain Ready");
        let chain_not_ready_text = self.tr("链未就绪", "Chain Not Ready");
        let shortcuts_text = self.tr("快捷入口", "Quick Shortcuts");
        let my_account_tip = self.tr(
            "先在转账里选一个转出账户，即可启用“我的账户”。",
            "Choose a sender in Transfer to enable My Account.",
        );
        let request_inflight = self.web_request_inflight_for(WebRequestDomain::ExplorerQuery);
        let feedback_available = self.is_feedback_available();

        Self::explorer_card(ui, title, subtitle, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(refresh_text).clicked() {
                    self.explorer_panel_state.pending_overview_refresh = true;
                    match self.explorer_panel_state.active_tab {
                        ExplorerTab::Blocks => {
                            self.explorer_panel_state.pending_blocks_refresh = true
                        }
                        ExplorerTab::Txs => self.explorer_panel_state.pending_txs_refresh = true,
                        ExplorerTab::Search => {
                            self.explorer_panel_state.pending_search_refresh = true
                        }
                        ExplorerTab::Address
                        | ExplorerTab::Contracts
                        | ExplorerTab::Assets
                        | ExplorerTab::Mempool => self
                            .schedule_explorer_p1_tab_refresh(self.explorer_panel_state.active_tab),
                    }
                    self.explorer_panel_state.last_poll_at = Some(Instant::now());
                }
                if ui.button(reset_text).clicked() {
                    self.reset_explorer_active_tab_state();
                }
                Self::explorer_status_chip(
                    ui,
                    if request_inflight {
                        inflight_text
                    } else {
                        idle_text
                    },
                    if request_inflight {
                        egui::Color32::from_rgb(201, 146, 44)
                    } else {
                        egui::Color32::from_rgb(130, 130, 130)
                    },
                );
                Self::explorer_status_chip(
                    ui,
                    if feedback_available {
                        chain_ready_text
                    } else {
                        chain_not_ready_text
                    },
                    if feedback_available {
                        egui::Color32::from_rgb(62, 152, 92)
                    } else {
                        egui::Color32::from_rgb(196, 84, 84)
                    },
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.strong(shortcuts_text);
                if ui
                    .button(self.explorer_shortcut_text(ExplorerQuickShortcut::LatestBlock))
                    .clicked()
                {
                    self.apply_explorer_quick_shortcut(ExplorerQuickShortcut::LatestBlock);
                }
                if ui
                    .button(self.explorer_shortcut_text(ExplorerQuickShortcut::RecentTxs))
                    .clicked()
                {
                    self.apply_explorer_quick_shortcut(ExplorerQuickShortcut::RecentTxs);
                }
                let my_account_available = self.explorer_my_account_candidate().is_some();
                if ui
                    .add_enabled(
                        my_account_available,
                        egui::Button::new(
                            self.explorer_shortcut_text(ExplorerQuickShortcut::MyAccount),
                        ),
                    )
                    .clicked()
                {
                    self.apply_explorer_quick_shortcut(ExplorerQuickShortcut::MyAccount);
                }
                if !my_account_available {
                    ui.small(my_account_tip);
                }
            });

            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.small(self.tr("术语", "Glossary"));
                self.render_glossary_term_chip(ui, GlossaryTerm::Slot);
                self.render_glossary_term_chip(ui, GlossaryTerm::Mempool);
                self.render_glossary_term_chip(ui, GlossaryTerm::ActionId);
            });
        });
    }

    fn render_overview(&mut self, ui: &mut egui::Ui) {
        if let Some(overview) = self.explorer_panel_state.overview.as_ref() {
            let title = self.tr("链健康概览", "Chain Health");
            let subtitle = self.tr(
                "对标主链浏览器的第一屏：先判断高度、身份、哈希和状态分布。",
                "Mainnet-style first screen: read heights, identity, hashes, and status mix first.",
            );
            Self::explorer_card(ui, title, subtitle, |ui| {
                ui.columns(4, |cols| {
                    Self::explorer_metric_card(
                        &mut cols[0],
                        self.tr("最新高度", "Latest Height"),
                        overview.latest_height.to_string(),
                        Some(self.tr("head", "head").to_string()),
                        egui::Color32::from_rgb(74, 116, 168),
                    );
                    Self::explorer_metric_card(
                        &mut cols[1],
                        self.tr("已提交高度", "Committed"),
                        overview.committed_height.to_string(),
                        Some(self.tr("local", "local").to_string()),
                        egui::Color32::from_rgb(62, 152, 92),
                    );
                    Self::explorer_metric_card(
                        &mut cols[2],
                        self.tr("网络高度", "Network"),
                        overview.network_committed_height.to_string(),
                        Some(self.tr("network", "network").to_string()),
                        egui::Color32::from_rgb(81, 104, 132),
                    );
                    Self::explorer_metric_card(
                        &mut cols[3],
                        self.tr("追踪记录", "Tracked"),
                        overview.tracked_records.to_string(),
                        Some(format!(
                            "{} {}",
                            self.tr("总交易", "Total"),
                            overview.transfer_total
                        )),
                        egui::Color32::from_rgb(112, 121, 130),
                    );
                });

                ui.add_space(6.0);
                ui.columns(2, |cols| {
                    Self::explorer_card(
                        &mut cols[0],
                        self.tr("链身份", "Chain Identity"),
                        self.tr(
                            "当前窗口消费的 explorer 观测源。",
                            "Current explorer observation source.",
                        ),
                        |ui| {
                            Self::render_explorer_detail_row(
                                ui,
                                "node_id",
                                overview.node_id.as_str(),
                                true,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "world_id",
                                overview.world_id.as_str(),
                                true,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "observed_at",
                                &overview.observed_at_unix_ms.to_string(),
                                false,
                            );
                        },
                    );
                    Self::explorer_card(
                        &mut cols[1],
                        self.tr("最新哈希", "Latest Hashes"),
                        self.tr(
                            "用于快速判断区块推进和执行对齐。",
                            "Quick read on block progress and execution alignment.",
                        ),
                        |ui| {
                            Self::render_explorer_detail_row(
                                ui,
                                "last_block",
                                overview.last_block_hash.as_deref().unwrap_or("n/a"),
                                true,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "last_exec",
                                overview
                                    .last_execution_block_hash
                                    .as_deref()
                                    .unwrap_or("n/a"),
                                true,
                            );
                        },
                    );
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "{} {}",
                            self.tr("已受理", "Accepted"),
                            overview.transfer_accepted
                        ),
                        self.explorer_lifecycle_color(
                            transfer_window::WebTransferLifecycleStatus::Accepted,
                        ),
                    );
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "{} {}",
                            self.tr("待确认", "Pending"),
                            overview.transfer_pending
                        ),
                        self.explorer_lifecycle_color(
                            transfer_window::WebTransferLifecycleStatus::Pending,
                        ),
                    );
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "{} {}",
                            self.tr("已确认", "Confirmed"),
                            overview.transfer_confirmed
                        ),
                        self.explorer_lifecycle_color(
                            transfer_window::WebTransferLifecycleStatus::Confirmed,
                        ),
                    );
                    Self::explorer_status_chip(
                        ui,
                        format!("{} {}", self.tr("失败", "Failed"), overview.transfer_failed),
                        self.explorer_lifecycle_color(
                            transfer_window::WebTransferLifecycleStatus::Failed,
                        ),
                    );
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "{} {}",
                            self.tr("超时", "Timeout"),
                            overview.transfer_timeout
                        ),
                        self.explorer_lifecycle_color(
                            transfer_window::WebTransferLifecycleStatus::Timeout,
                        ),
                    );
                });
            });
        } else {
            Self::render_explorer_empty_panel(
                ui,
                self.tr("概览数据未就绪", "Overview Not Ready"),
                self.tr(
                    "Explorer 已打开，但第一轮链健康概览还没返回。",
                    "Explorer is open, but the first chain-health overview has not returned yet.",
                ),
            );
        }
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        let title = self.tr("导航", "Navigation");
        let subtitle = self.tr(
            "按主链浏览器的工作流切视图：先概览，再列表，再详情。",
            "Navigate like a mainnet explorer: overview first, then lists, then detail.",
        );
        Self::explorer_card(ui, title, subtitle, |ui| {
            ui.horizontal_wrapped(|ui| {
                for tab in [
                    ExplorerTab::Blocks,
                    ExplorerTab::Txs,
                    ExplorerTab::Search,
                    ExplorerTab::Address,
                    ExplorerTab::Contracts,
                    ExplorerTab::Assets,
                    ExplorerTab::Mempool,
                ] {
                    let selected = self.explorer_panel_state.active_tab == tab;
                    let label = format!(
                        "{} · {}",
                        self.explorer_tab_text(tab),
                        self.explorer_tab_count(tab)
                    );
                    if ui.selectable_label(selected, label).clicked() {
                        self.explorer_panel_state.active_tab = tab;
                        match tab {
                            ExplorerTab::Blocks => {
                                self.explorer_panel_state.pending_blocks_refresh = true
                            }
                            ExplorerTab::Txs => {
                                self.explorer_panel_state.pending_txs_refresh = true
                            }
                            ExplorerTab::Search => {
                                self.explorer_panel_state.pending_search_refresh = true
                            }
                            ExplorerTab::Address
                            | ExplorerTab::Contracts
                            | ExplorerTab::Assets
                            | ExplorerTab::Mempool => self.schedule_explorer_p1_tab_refresh(tab),
                        }
                    }
                }
            });
        });
    }

    fn render_blocks_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Blocks", "Blocks"),
            self.tr(
                "最近区块列表和单区块检查板。",
                "Recent block feed with a single-block inspector.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    let prev_disabled = self.explorer_panel_state.blocks_cursor == 0;
                    if ui
                        .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
                        .clicked()
                    {
                        self.explorer_panel_state.blocks_cursor = self
                            .explorer_panel_state
                            .blocks_cursor
                            .saturating_sub(self.explorer_panel_state.blocks_limit);
                        self.explorer_panel_state.pending_blocks_refresh = true;
                    }
                    let next_disabled = self.explorer_panel_state.blocks_next_cursor.is_none();
                    if ui
                        .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
                        .clicked()
                    {
                        if let Some(next_cursor) = self.explorer_panel_state.blocks_next_cursor {
                            self.explorer_panel_state.blocks_cursor = next_cursor;
                            self.explorer_panel_state.pending_blocks_refresh = true;
                        }
                    }
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "cursor {} / limit {} / total {}",
                            self.explorer_panel_state.blocks_cursor,
                            self.explorer_panel_state.blocks_limit,
                            self.explorer_panel_state.blocks_total
                        ),
                        egui::Color32::from_rgb(112, 121, 130),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("height");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.block_height_input);
                    ui.label("hash");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.block_hash_input);
                    if ui.button(self.tr("查询区块", "Query Block")).clicked() {
                        let height = parse_positive_u64(
                            self.explorer_panel_state.block_height_input.as_str(),
                        );
                        let hash = self
                            .explorer_panel_state
                            .block_hash_input
                            .trim()
                            .to_string();
                        if height.is_none() && hash.is_empty() {
                            self.append_log(self.tr(
                                "区块查询失败：height 或 hash 至少填写一个",
                                "Block query failed: provide height or hash",
                            ));
                        } else {
                            self.explorer_panel_state.pending_block_height = height;
                            self.explorer_panel_state.pending_block_hash =
                                if hash.is_empty() { None } else { Some(hash) };
                            self.explorer_panel_state.pending_block_refresh = true;
                        }
                    }
                });
            },
        );

        ui.add_space(6.0);
        ui.columns(2, |cols| {
            Self::explorer_card(
                &mut cols[0],
                self.tr("区块流", "Block Feed"),
                self.tr(
                    "最近区块按提交顺序排列，适合快速扫高度、slot 和 hash。",
                    "Recent blocks in commit order for quick scans across height, slot, and hash.",
                ),
                |ui| {
                    let mut clicked_height = None;
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            for block in &self.explorer_panel_state.blocks {
                                let is_selected = self
                                    .explorer_panel_state
                                    .selected_block
                                    .as_ref()
                                    .is_some_and(|selected| selected.height == block.height);
                                if Self::render_block_row_card(ui, block, is_selected) {
                                    clicked_height = Some(block.height);
                                }
                                ui.add_space(4.0);
                            }
                            if self.explorer_panel_state.blocks.is_empty() {
                                Self::render_explorer_empty_panel(
                                    ui,
                                    self.tr("暂无区块记录", "No Blocks"),
                                    self.tr(
                                        "当前窗口还没有可展示的区块缓存。",
                                        "There are no cached blocks to show in this window yet.",
                                    ),
                                );
                            }
                        });
                    if let Some(height) = clicked_height {
                        self.explorer_panel_state.block_height_input = height.to_string();
                        self.explorer_panel_state.pending_block_height = Some(height);
                        self.explorer_panel_state.pending_block_hash = None;
                        self.explorer_panel_state.pending_block_refresh = true;
                    }
                },
            );

            Self::explorer_card(
                &mut cols[1],
                self.tr("区块检查板", "Block Inspector"),
                self.tr(
                    "读取当前选中区块的关键字段与关联交易。",
                    "Inspect the selected block and its related transactions.",
                ),
                |ui| {
                    if let Some(block) = self.explorer_panel_state.selected_block.as_ref() {
                        Self::render_explorer_detail_row(
                            ui,
                            "height",
                            &block.height.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "slot",
                            &block.slot.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "epoch",
                            &block.epoch.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "action_count",
                            &block.action_count.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "committed_at",
                            &block.committed_at_unix_ms.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "block_hash",
                            block.block_hash.as_str(),
                            true,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "action_root",
                            block.action_root.as_str(),
                            true,
                        );

                        if !block.tx_hashes.is_empty() {
                            ui.add_space(6.0);
                            ui.strong(self.tr("关联交易", "Related Transactions"));
                            let hashes = block.tx_hashes.clone();
                            ui.horizontal_wrapped(|ui| {
                                for hash in hashes {
                                    if ui.button(short_hash(hash.as_str())).clicked() {
                                        self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                                        self.explorer_panel_state.tx_hash_input = hash.clone();
                                        self.explorer_panel_state.pending_tx_hash = Some(hash);
                                        self.explorer_panel_state.pending_tx_action_id = None;
                                        self.explorer_panel_state.pending_tx_refresh = true;
                                    }
                                }
                            });
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("未选择区块", "No Block Selected"),
                            self.tr(
                                "从左侧区块流选一条记录，右侧会展开完整字段。",
                                "Pick a block from the left feed to inspect full fields here.",
                            ),
                        );
                    }
                },
            );
        });
    }

    fn render_txs_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Transactions", "Transactions"),
            self.tr(
                "主链浏览器式的交易列表、过滤器和详情检查板。",
                "Mainnet-style transaction feed with filters and an inspector.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    let ui_language = self.ui_language;
                    ui.label(self.tr("账户", "Account"));
                    ui.text_edit_singleline(&mut self.explorer_panel_state.account_filter);
                    ui.label(self.tr("状态", "Status"));
                    egui::ComboBox::from_id_salt("explorer_status_filter")
                        .selected_text(explorer_status_filter_text(
                            ui_language,
                            self.explorer_panel_state.status_filter,
                        ))
                        .show_ui(ui, |ui| {
                            for filter in [
                                ExplorerStatusFilter::All,
                                ExplorerStatusFilter::Accepted,
                                ExplorerStatusFilter::Pending,
                                ExplorerStatusFilter::Confirmed,
                                ExplorerStatusFilter::Failed,
                                ExplorerStatusFilter::Timeout,
                            ] {
                                ui.selectable_value(
                                    &mut self.explorer_panel_state.status_filter,
                                    filter,
                                    explorer_status_filter_text(ui_language, filter),
                                );
                            }
                        });
                    ui.label("action_id");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.action_filter_input);
                    if ui.button(self.tr("应用过滤", "Apply Filter")).clicked() {
                        self.explorer_panel_state.txs_cursor = 0;
                        self.explorer_panel_state.pending_txs_refresh = true;
                    }
                    if ui.button(self.tr("清空过滤", "Clear Filters")).clicked() {
                        self.explorer_panel_state.account_filter.clear();
                        self.explorer_panel_state.action_filter_input.clear();
                        self.explorer_panel_state.status_filter = ExplorerStatusFilter::All;
                        self.explorer_panel_state.txs_cursor = 0;
                        self.explorer_panel_state.pending_txs_refresh = true;
                    }
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let prev_disabled = self.explorer_panel_state.txs_cursor == 0;
                    if ui
                        .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
                        .clicked()
                    {
                        self.explorer_panel_state.txs_cursor = self
                            .explorer_panel_state
                            .txs_cursor
                            .saturating_sub(self.explorer_panel_state.txs_limit);
                        self.explorer_panel_state.pending_txs_refresh = true;
                    }
                    let next_disabled = self.explorer_panel_state.txs_next_cursor.is_none();
                    if ui
                        .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
                        .clicked()
                    {
                        if let Some(next_cursor) = self.explorer_panel_state.txs_next_cursor {
                            self.explorer_panel_state.txs_cursor = next_cursor;
                            self.explorer_panel_state.pending_txs_refresh = true;
                        }
                    }
                    Self::explorer_status_chip(
                        ui,
                        format!(
                            "cursor {} / limit {} / total {}",
                            self.explorer_panel_state.txs_cursor,
                            self.explorer_panel_state.txs_limit,
                            self.explorer_panel_state.txs_total
                        ),
                        egui::Color32::from_rgb(112, 121, 130),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("tx_hash");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.tx_hash_input);
                    ui.label("action_id");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.tx_action_input);
                    if ui.button(self.tr("查询交易", "Query Tx")).clicked() {
                        let tx_hash = self.explorer_panel_state.tx_hash_input.trim().to_string();
                        let action_id =
                            parse_positive_u64(self.explorer_panel_state.tx_action_input.as_str());
                        if tx_hash.is_empty() && action_id.is_none() {
                            self.append_log(self.tr(
                                "交易查询失败：tx_hash 或 action_id 至少填写一个",
                                "Tx query failed: provide tx_hash or action_id",
                            ));
                        } else {
                            self.explorer_panel_state.pending_tx_hash = if tx_hash.is_empty() {
                                None
                            } else {
                                Some(tx_hash)
                            };
                            self.explorer_panel_state.pending_tx_action_id = action_id;
                            self.explorer_panel_state.pending_tx_refresh = true;
                        }
                    }
                });
            },
        );

        ui.add_space(6.0);
        ui.columns(2, |cols| {
            Self::explorer_card(
                &mut cols[0],
                self.tr("交易流", "Transaction Feed"),
                self.tr(
                    "按状态、账户和 action_id 扫描当前交易集。",
                    "Scan the current transaction set by status, account, and action_id.",
                ),
                |ui| {
                    let mut clicked_hash = None;
                    egui::ScrollArea::vertical()
                        .max_height(460.0)
                        .show(ui, |ui| {
                            for tx in &self.explorer_panel_state.txs {
                                let is_selected = self
                                    .explorer_panel_state
                                    .selected_tx
                                    .as_ref()
                                    .is_some_and(|selected| selected.tx_hash == tx.tx_hash);
                                if Self::render_tx_row_card(
                                    ui,
                                    tx,
                                    is_selected,
                                    self.explorer_lifecycle_text(tx.status),
                                    self.explorer_lifecycle_color(tx.status),
                                ) {
                                    clicked_hash = Some(tx.tx_hash.clone());
                                }
                                ui.add_space(4.0);
                            }
                            if self.explorer_panel_state.txs.is_empty() {
                                Self::render_explorer_empty_panel(
                                    ui,
                                    self.tr("暂无交易记录", "No Transactions"),
                                    self.tr(
                                        "当前过滤条件下没有命中交易。",
                                        "No transactions matched the current filters.",
                                    ),
                                );
                            }
                        });
                    if let Some(tx_hash) = clicked_hash {
                        self.explorer_panel_state.tx_hash_input = tx_hash.clone();
                        self.explorer_panel_state.pending_tx_hash = Some(tx_hash);
                        self.explorer_panel_state.pending_tx_action_id = None;
                        self.explorer_panel_state.pending_tx_refresh = true;
                    }
                },
            );

            Self::explorer_card(
                &mut cols[1],
                self.tr("交易检查板", "Transaction Inspector"),
                self.tr(
                    "读取当前选中交易的状态、路径、区块归属和错误。",
                    "Inspect status, route, block ownership, and failure data for the selected transaction.",
                ),
                |ui| {
                    if let Some(tx) = self.explorer_panel_state.selected_tx.as_ref() {
                        Self::explorer_status_chip(
                            ui,
                            self.explorer_lifecycle_text(tx.status),
                            self.explorer_lifecycle_color(tx.status),
                        );
                        ui.add_space(6.0);
                        Self::render_explorer_detail_row(ui, "tx_hash", tx.tx_hash.as_str(), true);
                        Self::render_explorer_detail_row(
                            ui,
                            "action_id",
                            &tx.action_id.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "from",
                            tx.from_account_id.as_str(),
                            true,
                        );
                        Self::render_explorer_detail_row(ui, "to", tx.to_account_id.as_str(), true);
                        Self::render_explorer_detail_row(
                            ui,
                            "amount",
                            &tx.amount.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "nonce",
                            &tx.nonce.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "submitted_at",
                            &tx.submitted_at_unix_ms.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "updated_at",
                            &tx.updated_at_unix_ms.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "block_height",
                            &tx.block_height
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "n/a".to_string()),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "block_hash",
                            tx.block_hash.as_deref().unwrap_or("n/a"),
                            true,
                        );
                        if let Some(error) = tx.error.as_deref() {
                            ui.add_space(6.0);
                            Self::render_explorer_error_panel(
                                ui,
                                self.tr("执行错误", "Execution Error"),
                                format!(
                                    "{} ({})",
                                    error,
                                    tx.error_code.as_deref().unwrap_or("unknown")
                                ),
                            );
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("未选择交易", "No Transaction Selected"),
                            self.tr(
                                "从左侧交易流点击一条记录，右侧会展开完整上下文。",
                                "Pick a transaction from the left feed to inspect the full context here.",
                            ),
                        );
                    }
                },
            );
        });
    }

    fn render_search_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("统一搜索", "Unified Search"),
            self.tr(
                "支持 block/tx/action/account 快速命中和跳转。",
                "Supports fast hits and jumps across block/tx/action/account.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.small(self.tr(
                        "支持 height/block_hash/tx_hash/action_id/account_id",
                        "Supports height/block_hash/tx_hash/action_id/account_id",
                    ));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.text_edit_singleline(&mut self.explorer_panel_state.search_query);
                    if ui.button(self.tr("搜索", "Search")).clicked() {
                        if self.explorer_panel_state.search_query.trim().is_empty() {
                            self.append_log(
                                self.tr("搜索失败：请输入关键词", "Search failed: query is empty"),
                            );
                        } else {
                            self.explorer_panel_state.pending_search_refresh = true;
                        }
                    }
                    if ui.button(self.tr("清空", "Clear")).clicked() {
                        self.explorer_panel_state.search_query.clear();
                        self.explorer_panel_state.search_results.clear();
                    }
                });
            },
        );

        ui.add_space(6.0);
        Self::explorer_card(
            ui,
            self.tr("搜索结果", "Search Results"),
            self.tr(
                "点结果即可跳到对应 tab 和详情查询。",
                "Click a result to jump into the matching tab and detail query.",
            ),
            |ui| {
                let mut clicked: Option<(String, String)> = None;
                egui::ScrollArea::vertical()
                    .max_height(460.0)
                    .show(ui, |ui| {
                        for item in &self.explorer_panel_state.search_results {
                            let mut triggered = false;
                            ui.group(|ui| {
                                if ui
                                    .selectable_label(
                                        false,
                                        format!("[{}] {}", item.item_type, item.key),
                                    )
                                    .clicked()
                                {
                                    triggered = true;
                                }
                                ui.small(item.summary.as_str());
                            });
                            if triggered {
                                clicked = Some((item.item_type.clone(), item.key.clone()));
                            }
                            ui.add_space(4.0);
                        }
                        if self.explorer_panel_state.search_results.is_empty() {
                            Self::render_explorer_empty_panel(
                                ui,
                                self.tr("暂无搜索结果", "No Search Results"),
                                self.tr(
                                    "输入 height、hash、action_id 或 account_id 进行统一检索。",
                                    "Search by height, hash, action_id, or account_id.",
                                ),
                            );
                        }
                    });

                if let Some((item_type, key)) = clicked {
                    match item_type.as_str() {
                        "block" => {
                            self.explorer_panel_state.active_tab = ExplorerTab::Blocks;
                            self.explorer_panel_state.block_height_input = key.clone();
                            self.explorer_panel_state.block_hash_input = key.clone();
                            self.explorer_panel_state.pending_block_height =
                                parse_positive_u64(key.as_str());
                            self.explorer_panel_state.pending_block_hash = Some(key);
                            self.explorer_panel_state.pending_block_refresh = true;
                        }
                        "tx" => {
                            self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                            self.explorer_panel_state.tx_hash_input = key.clone();
                            self.explorer_panel_state.pending_tx_hash = Some(key);
                            self.explorer_panel_state.pending_tx_action_id = None;
                            self.explorer_panel_state.pending_tx_refresh = true;
                        }
                        _ => {
                            self.append_log(format!(
                                "{}: {}",
                                self.tr("未支持的搜索类型", "Unsupported search item type"),
                                item_type,
                            ));
                        }
                    }
                }
            },
        );
    }

    fn explorer_tab_count(&self, tab: ExplorerTab) -> usize {
        match tab {
            ExplorerTab::Blocks => self
                .explorer_panel_state
                .blocks_total
                .max(self.explorer_panel_state.blocks.len()),
            ExplorerTab::Txs => self
                .explorer_panel_state
                .txs_total
                .max(self.explorer_panel_state.txs.len()),
            ExplorerTab::Search => self.explorer_panel_state.search_results.len(),
            ExplorerTab::Address => self
                .explorer_panel_state
                .p1
                .address_response
                .as_ref()
                .map(|response| response.total.max(response.items.len()))
                .unwrap_or(0),
            ExplorerTab::Contracts => self
                .explorer_panel_state
                .p1
                .contracts_response
                .as_ref()
                .map(|response| response.total.max(response.items.len()))
                .unwrap_or(0),
            ExplorerTab::Assets => self
                .explorer_panel_state
                .p1
                .assets_response
                .as_ref()
                .map(|response| response.total.max(response.holders.len()))
                .unwrap_or(0),
            ExplorerTab::Mempool => self
                .explorer_panel_state
                .p1
                .mempool_response
                .as_ref()
                .map(|response| response.total.max(response.items.len()))
                .unwrap_or(0),
        }
    }

    pub(super) fn explorer_card<F>(ui: &mut egui::Ui, title: &str, subtitle: &str, add: F)
    where
        F: FnOnce(&mut egui::Ui),
    {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading(title);
                    if !subtitle.is_empty() {
                        ui.small(subtitle);
                    }
                });
                ui.add_space(4.0);
                add(ui);
            });
        });
    }

    pub(super) fn explorer_metric_card(
        ui: &mut egui::Ui,
        label: &str,
        value: String,
        detail: Option<String>,
        accent: egui::Color32,
    ) {
        ui.group(|ui| {
            ui.small(label);
            ui.label(egui::RichText::new(value).strong().color(accent).size(18.0));
            if let Some(detail) = detail {
                ui.small(detail);
            }
        });
    }

    pub(super) fn explorer_status_chip(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        color: egui::Color32,
    ) {
        ui.label(egui::RichText::new(label.into()).color(color).strong());
    }

    pub(super) fn render_explorer_detail_row(
        ui: &mut egui::Ui,
        label: &str,
        value: &str,
        monospace: bool,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.small(egui::RichText::new(label).strong());
            if monospace {
                ui.label(egui::RichText::new(value).monospace());
            } else {
                ui.label(value);
            }
        });
    }

    pub(super) fn render_explorer_empty_panel(ui: &mut egui::Ui, title: &str, body: &str) {
        ui.group(|ui| {
            ui.strong(title);
            ui.small(body);
        });
    }

    pub(super) fn render_explorer_error_panel(ui: &mut egui::Ui, title: &str, body: String) {
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(egui::Color32::from_rgb(196, 84, 84)),
            );
            ui.small(egui::RichText::new(body).color(egui::Color32::from_rgb(196, 84, 84)));
        });
    }

    pub(super) fn render_block_row_card(
        ui: &mut egui::Ui,
        block: &WebExplorerBlockItem,
        selected: bool,
    ) -> bool {
        let mut clicked = false;
        ui.group(|ui| {
            if ui
                .selectable_label(selected, format!("Block #{}", block.height))
                .clicked()
            {
                clicked = true;
            }
            ui.small(format!(
                "slot {} · epoch {} · txs {} · committed {}",
                block.slot,
                block.epoch,
                block.tx_hashes.len(),
                block.committed_at_unix_ms
            ));
            ui.label(egui::RichText::new(short_hash(block.block_hash.as_str())).monospace());
        });
        clicked
    }

    pub(super) fn render_tx_row_card(
        ui: &mut egui::Ui,
        tx: &WebExplorerTxItem,
        selected: bool,
        status_text: &str,
        status_color: egui::Color32,
    ) -> bool {
        let mut clicked = false;
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                Self::explorer_status_chip(ui, status_text, status_color);
                if ui
                    .selectable_label(
                        selected,
                        format!("#{} · {}", tx.action_id, short_hash(tx.tx_hash.as_str())),
                    )
                    .clicked()
                {
                    clicked = true;
                }
            });
            ui.small(format!(
                "{} -> {} · amount {} · nonce {}",
                tx.from_account_id, tx.to_account_id, tx.amount, tx.nonce
            ));
            ui.small(format!(
                "block {} · submitted {}",
                tx.block_height
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                tx.submitted_at_unix_ms
            ));
        });
        clicked
    }
}
