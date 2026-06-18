use super::*;

impl ClientLauncherApp {
    pub(super) fn render_search_tab(&mut self, ui: &mut egui::Ui) {
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
                    self.apply_explorer_search_result(item_type.as_str(), key);
                }
            },
        );
    }

    pub(super) fn explorer_tab_count(&self, tab: ExplorerTab) -> usize {
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
        Self::modal_card(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(14.0));
                    if !subtitle.is_empty() {
                        ui.small(
                            egui::RichText::new(subtitle)
                                .color(egui::Color32::from_rgb(92, 105, 118)),
                        );
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
        Self::modal_card(ui, |ui| {
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
        Self::modal_status_chip(ui, &label.into(), color);
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
        Self::modal_card(ui, |ui| {
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
            if tx.asset_id.is_some() || tx.memo.is_some() {
                ui.small(format!(
                    "asset {} · memo {}",
                    tx.asset_id.as_deref().unwrap_or("n/a"),
                    tx.memo.as_deref().unwrap_or("n/a"),
                ));
            }
            if tx.chain_id.is_some()
                || tx.network_id.is_some()
                || tx.tx_type.is_some()
                || tx.tx_version.is_some()
            {
                ui.small(format!(
                    "chain {} · network {} · type {} · v{}",
                    tx.chain_id.as_deref().unwrap_or("n/a"),
                    tx.network_id.as_deref().unwrap_or("n/a"),
                    tx.tx_type.as_deref().unwrap_or("n/a"),
                    tx.tx_version.unwrap_or_default()
                ));
            }
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
