use super::explorer_window_p1::{explorer_mempool_status_filter_text, ExplorerMempoolStatusFilter};
use super::*;

impl ClientLauncherApp {
    pub(super) fn render_mempool_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Mempool", "Mempool"),
            self.tr(
                "未最终确认交易的主列表和本地检查板。",
                "Primary list of not-yet-finalized transactions with a local inspector.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    let ui_language = self.ui_language;
                    egui::ComboBox::from_id_salt("explorer_mempool_status_filter")
                        .selected_text(explorer_mempool_status_filter_text(
                            ui_language,
                            self.explorer_panel_state.p1.mempool_status_filter,
                        ))
                        .show_ui(ui, |ui| {
                            for filter in [
                                ExplorerMempoolStatusFilter::All,
                                ExplorerMempoolStatusFilter::Accepted,
                                ExplorerMempoolStatusFilter::Pending,
                            ] {
                                ui.selectable_value(
                                    &mut self.explorer_panel_state.p1.mempool_status_filter,
                                    filter,
                                    explorer_mempool_status_filter_text(ui_language, filter),
                                );
                            }
                        });
                    if ui.button(self.tr("查询内存池", "Query Mempool")).clicked() {
                        self.explorer_panel_state.p1.mempool_cursor = 0;
                        self.explorer_panel_state.p1.pending_mempool_refresh = true;
                    }
                    if ui.button(self.tr("清空过滤", "Clear Filter")).clicked() {
                        self.explorer_panel_state.p1.mempool_status_filter =
                            ExplorerMempoolStatusFilter::All;
                        self.explorer_panel_state.p1.mempool_cursor = 0;
                        self.explorer_panel_state.p1.pending_mempool_refresh = true;
                    }
                    if let Some(response) = self.explorer_panel_state.p1.mempool_response.as_ref() {
                        Self::explorer_status_chip(
                            ui,
                            format!(
                                "accepted {} / pending {} / total {}",
                                response.accepted_count, response.pending_count, response.total
                            ),
                            egui::Color32::from_rgb(112, 121, 130),
                        );
                    }
                });
            },
        );

        ui.add_space(6.0);
        ui.columns(2, |cols| {
            Self::explorer_card(
                &mut cols[0],
                self.tr("内存池交易流", "Mempool Feed"),
                self.tr(
                    "点选记录可在右侧本地 inspector 里看字段，也可进一步打开全局 Txs inspector。",
                    "Select a record to inspect it on the right, or open the global Txs inspector.",
                ),
                |ui| {
                    if self.explorer_panel_state.p1.mempool_response.is_some() {
                        let mut selected_tx = None;
                        ui.horizontal_wrapped(|ui| {
                            let prev_disabled =
                                self.explorer_panel_state.p1.mempool_response.as_ref().is_some_and(
                                    |response| response.cursor == 0,
                                );
                            if ui
                                .add_enabled(
                                    !prev_disabled,
                                    egui::Button::new(self.tr("上一页", "Prev")),
                                )
                                .clicked()
                            {
                                self.explorer_panel_state.p1.mempool_cursor = self
                                    .explorer_panel_state
                                    .p1
                                    .mempool_cursor
                                    .saturating_sub(self.explorer_panel_state.p1.mempool_limit);
                                self.explorer_panel_state.p1.pending_mempool_refresh = true;
                            }
                            let next_disabled = self
                                .explorer_panel_state
                                .p1
                                .mempool_response
                                .as_ref()
                                .and_then(|response| response.next_cursor)
                                .is_none();
                            if ui
                                .add_enabled(
                                    !next_disabled,
                                    egui::Button::new(self.tr("下一页", "Next")),
                                )
                                .clicked()
                            {
                                if let Some(next_cursor) = self
                                    .explorer_panel_state
                                    .p1
                                    .mempool_response
                                    .as_ref()
                                    .and_then(|response| response.next_cursor)
                                {
                                    self.explorer_panel_state.p1.mempool_cursor = next_cursor;
                                    self.explorer_panel_state.p1.pending_mempool_refresh = true;
                                }
                            }
                        });

                        {
                            let response = self
                                .explorer_panel_state
                                .p1
                                .mempool_response
                                .as_ref()
                                .expect("checked above");
                            egui::ScrollArea::vertical()
                                .max_height(420.0)
                                .show(ui, |ui| {
                                    for tx in &response.items {
                                        let is_selected = self
                                            .explorer_panel_state
                                            .p1
                                            .selected_mempool_tx
                                            .as_ref()
                                            .is_some_and(|selected| selected.tx_hash == tx.tx_hash);
                                        if Self::render_tx_row_card(
                                            ui,
                                            tx,
                                            is_selected,
                                            self.explorer_lifecycle_text(tx.status),
                                            self.explorer_lifecycle_color(tx.status),
                                        ) {
                                            selected_tx = Some(tx.clone());
                                        }
                                        ui.add_space(4.0);
                                    }
                                    if response.items.is_empty() {
                                        Self::render_explorer_empty_panel(
                                            ui,
                                            self.tr("暂无待处理交易", "No Pending Transactions"),
                                            self.tr(
                                                "当前过滤条件没有命中任何 mempool 交易。",
                                                "No mempool transactions matched the current filter.",
                                            ),
                                        );
                                    }
                                });
                        }
                        if let Some(tx) = selected_tx {
                            self.explorer_panel_state.p1.selected_mempool_tx = Some(tx);
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("暂无内存池数据", "No Mempool Data"),
                            self.tr(
                                "打开内存池视图后可查看 accepted/pending 交易。",
                                "Open the mempool view to inspect accepted and pending transactions.",
                            ),
                        );
                    }
                },
            );

            Self::explorer_card(
                &mut cols[1],
                self.tr("内存池检查板", "Mempool Inspector"),
                self.tr(
                    "本地查看选中 mempool 交易，必要时再跳进全局 Txs inspector。",
                    "Inspect a selected mempool transaction locally, then jump into the global Txs inspector if needed.",
                ),
                |ui| {
                    if let Some(tx) = self.explorer_panel_state.p1.selected_mempool_tx.as_ref() {
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
                        Self::render_explorer_detail_row(
                            ui,
                            "to",
                            tx.to_account_id.as_str(),
                            true,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "amount",
                            &tx.amount.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "submitted_at",
                            &tx.submitted_at_unix_ms.to_string(),
                            false,
                        );
                        if ui
                            .button(self.tr("打开全局交易检查板", "Open Global Tx Inspector"))
                            .clicked()
                        {
                            self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                            self.explorer_panel_state.tx_hash_input = tx.tx_hash.clone();
                            self.explorer_panel_state.pending_tx_hash = Some(tx.tx_hash.clone());
                            self.explorer_panel_state.pending_tx_action_id = None;
                            self.explorer_panel_state.pending_tx_refresh = true;
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("未选择内存池交易", "No Mempool Transaction Selected"),
                            self.tr(
                                "从左侧点一条 mempool 交易，右侧会展开字段。",
                                "Pick a mempool transaction from the left feed to inspect its fields here.",
                            ),
                        );
                    }
                },
            );
        });
    }
}
