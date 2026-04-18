use super::*;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerAddressResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) account_id: Option<String>,
    pub(super) liquid_balance: u64,
    pub(super) vested_balance: u64,
    pub(super) last_transfer_nonce: Option<u64>,
    pub(super) next_nonce_hint: u64,
    pub(super) limit: usize,
    pub(super) cursor: usize,
    pub(super) total: usize,
    pub(super) next_cursor: Option<usize>,
    #[serde(default)]
    pub(super) items: Vec<WebExplorerTxItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerContractListItem {
    pub(super) contract_id: String,
    pub(super) contract_type: String,
    pub(super) status: String,
    pub(super) creator_agent_id: String,
    pub(super) counterparty_agent_id: String,
    pub(super) settlement_kind: String,
    pub(super) settlement_amount: i64,
    pub(super) expires_at: u64,
    pub(super) summary: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerContractsResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) limit: usize,
    pub(super) cursor: usize,
    pub(super) total: usize,
    pub(super) next_cursor: Option<usize>,
    #[serde(default)]
    pub(super) items: Vec<WebExplorerContractListItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerContractResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) contract_id: Option<String>,
    pub(super) contract: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) recent_txs: Vec<WebExplorerTxItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerAssetHolderItem {
    pub(super) account_id: String,
    pub(super) liquid_balance: u64,
    pub(super) vested_balance: u64,
    pub(super) total_balance: u64,
    pub(super) last_transfer_nonce: Option<u64>,
    pub(super) next_nonce_hint: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerAssetsResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) token_symbol: String,
    pub(super) token_decimals: u8,
    pub(super) total_supply: u64,
    pub(super) circulating_supply: u64,
    pub(super) total_issued: u64,
    pub(super) total_burned: u64,
    pub(super) account_filter: Option<String>,
    pub(super) limit: usize,
    pub(super) cursor: usize,
    pub(super) total: usize,
    pub(super) next_cursor: Option<usize>,
    #[serde(default)]
    pub(super) holders: Vec<WebExplorerAssetHolderItem>,
    pub(super) nft_supported: bool,
    #[serde(default)]
    pub(super) nft_collections: Vec<String>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebExplorerMempoolResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) status_filter: String,
    pub(super) accepted_count: usize,
    pub(super) pending_count: usize,
    pub(super) limit: usize,
    pub(super) cursor: usize,
    pub(super) total: usize,
    pub(super) next_cursor: Option<usize>,
    #[serde(default)]
    pub(super) items: Vec<WebExplorerTxItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExplorerMempoolStatusFilter {
    All,
    Accepted,
    Pending,
}

impl Default for ExplorerMempoolStatusFilter {
    fn default() -> Self {
        Self::All
    }
}

impl ExplorerMempoolStatusFilter {
    pub(super) fn query_value(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Accepted => Some("accepted"),
            Self::Pending => Some("pending"),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ExplorerP1State {
    pub(super) address_account_input: String,
    pub(super) address_cursor: usize,
    pub(super) address_limit: usize,
    pub(super) address_response: Option<WebExplorerAddressResponse>,
    pub(super) pending_address_refresh: bool,

    pub(super) contracts_cursor: usize,
    pub(super) contracts_limit: usize,
    pub(super) contracts_response: Option<WebExplorerContractsResponse>,
    pub(super) pending_contracts_refresh: bool,

    pub(super) contract_id_input: String,
    pub(super) contract_response: Option<WebExplorerContractResponse>,
    pub(super) pending_contract_refresh: bool,

    pub(super) assets_account_filter: String,
    pub(super) assets_cursor: usize,
    pub(super) assets_limit: usize,
    pub(super) assets_response: Option<WebExplorerAssetsResponse>,
    pub(super) pending_assets_refresh: bool,

    pub(super) mempool_status_filter: ExplorerMempoolStatusFilter,
    pub(super) mempool_cursor: usize,
    pub(super) mempool_limit: usize,
    pub(super) mempool_response: Option<WebExplorerMempoolResponse>,
    pub(super) selected_mempool_tx: Option<WebExplorerTxItem>,
    pub(super) pending_mempool_refresh: bool,
}

impl Default for ExplorerP1State {
    fn default() -> Self {
        Self {
            address_account_input: String::new(),
            address_cursor: 0,
            address_limit: EXPLORER_DEFAULT_LIMIT,
            address_response: None,
            pending_address_refresh: false,
            contracts_cursor: 0,
            contracts_limit: EXPLORER_DEFAULT_LIMIT,
            contracts_response: None,
            pending_contracts_refresh: false,
            contract_id_input: String::new(),
            contract_response: None,
            pending_contract_refresh: false,
            assets_account_filter: String::new(),
            assets_cursor: 0,
            assets_limit: EXPLORER_DEFAULT_LIMIT,
            assets_response: None,
            pending_assets_refresh: false,
            mempool_status_filter: ExplorerMempoolStatusFilter::default(),
            mempool_cursor: 0,
            mempool_limit: EXPLORER_DEFAULT_LIMIT,
            mempool_response: None,
            selected_mempool_tx: None,
            pending_mempool_refresh: false,
        }
    }
}

pub(super) fn explorer_mempool_status_filter_text(
    ui_language: UiLanguage,
    filter: ExplorerMempoolStatusFilter,
) -> &'static str {
    match (filter, ui_language) {
        (ExplorerMempoolStatusFilter::All, UiLanguage::ZhCn) => "全部",
        (ExplorerMempoolStatusFilter::All, UiLanguage::EnUs) => "All",
        (ExplorerMempoolStatusFilter::Accepted, UiLanguage::ZhCn) => "已受理",
        (ExplorerMempoolStatusFilter::Accepted, UiLanguage::EnUs) => "Accepted",
        (ExplorerMempoolStatusFilter::Pending, UiLanguage::ZhCn) => "待确认",
        (ExplorerMempoolStatusFilter::Pending, UiLanguage::EnUs) => "Pending",
    }
}

impl ClientLauncherApp {
    pub(super) fn schedule_explorer_p1_tab_refresh(&mut self, tab: ExplorerTab) {
        match tab {
            ExplorerTab::Address => self.explorer_panel_state.p1.pending_address_refresh = true,
            ExplorerTab::Contracts => self.explorer_panel_state.p1.pending_contracts_refresh = true,
            ExplorerTab::Assets => self.explorer_panel_state.p1.pending_assets_refresh = true,
            ExplorerTab::Mempool => self.explorer_panel_state.p1.pending_mempool_refresh = true,
            ExplorerTab::Blocks | ExplorerTab::Txs | ExplorerTab::Search => {}
        }
    }

    pub(super) fn maybe_request_explorer_p1_data(&mut self) -> bool {
        if self.explorer_panel_state.p1.pending_address_refresh {
            self.explorer_panel_state.p1.pending_address_refresh = false;
            let account_id = self
                .explorer_panel_state
                .p1
                .address_account_input
                .trim()
                .to_string();
            if account_id.is_empty() {
                self.append_log(self.tr(
                    "地址查询失败：请输入 account_id",
                    "Address query failed: account_id is required",
                ));
                return false;
            }
            self.request_web_chain_explorer_address(
                account_id,
                self.explorer_panel_state.p1.address_cursor,
                self.explorer_panel_state.p1.address_limit,
            );
            return true;
        }

        if self.explorer_panel_state.p1.pending_contracts_refresh {
            self.explorer_panel_state.p1.pending_contracts_refresh = false;
            self.request_web_chain_explorer_contracts(
                self.explorer_panel_state.p1.contracts_cursor,
                self.explorer_panel_state.p1.contracts_limit,
            );
            return true;
        }

        if self.explorer_panel_state.p1.pending_contract_refresh {
            self.explorer_panel_state.p1.pending_contract_refresh = false;
            let contract_id = self
                .explorer_panel_state
                .p1
                .contract_id_input
                .trim()
                .to_string();
            if contract_id.is_empty() {
                self.append_log(self.tr(
                    "合约查询失败：请输入 contract_id",
                    "Contract query failed: contract_id is required",
                ));
                return false;
            }
            self.request_web_chain_explorer_contract(contract_id);
            return true;
        }

        if self.explorer_panel_state.p1.pending_assets_refresh {
            self.explorer_panel_state.p1.pending_assets_refresh = false;
            self.request_web_chain_explorer_assets(
                self.explorer_panel_state.p1.assets_account_filter.clone(),
                self.explorer_panel_state.p1.assets_cursor,
                self.explorer_panel_state.p1.assets_limit,
            );
            return true;
        }

        if self.explorer_panel_state.p1.pending_mempool_refresh {
            self.explorer_panel_state.p1.pending_mempool_refresh = false;
            self.request_web_chain_explorer_mempool(
                self.explorer_panel_state
                    .p1
                    .mempool_status_filter
                    .query_value()
                    .map(str::to_string),
                self.explorer_panel_state.p1.mempool_cursor,
                self.explorer_panel_state.p1.mempool_limit,
            );
            return true;
        }

        false
    }

    pub(super) fn reset_explorer_active_tab_state(&mut self) {
        match self.explorer_panel_state.active_tab {
            ExplorerTab::Blocks => {
                self.explorer_panel_state.block_height_input.clear();
                self.explorer_panel_state.block_hash_input.clear();
                self.explorer_panel_state.blocks_cursor = 0;
                self.explorer_panel_state.pending_blocks_refresh = true;
                self.explorer_panel_state.selected_block = None;
            }
            ExplorerTab::Txs => {
                self.explorer_panel_state.account_filter.clear();
                self.explorer_panel_state.action_filter_input.clear();
                self.explorer_panel_state.status_filter = ExplorerStatusFilter::All;
                self.explorer_panel_state.txs_cursor = 0;
                self.explorer_panel_state.pending_txs_refresh = true;
                self.explorer_panel_state.selected_tx = None;
                self.explorer_panel_state.tx_hash_input.clear();
                self.explorer_panel_state.tx_action_input.clear();
            }
            ExplorerTab::Search => {
                self.explorer_panel_state.search_query.clear();
                self.explorer_panel_state.search_results.clear();
                self.explorer_panel_state.pending_search_refresh = false;
            }
            ExplorerTab::Address => {
                self.explorer_panel_state.p1.address_account_input.clear();
                self.explorer_panel_state.p1.address_cursor = 0;
                self.explorer_panel_state.p1.address_response = None;
                self.explorer_panel_state.p1.pending_address_refresh = false;
            }
            ExplorerTab::Contracts => {
                self.explorer_panel_state.p1.contract_id_input.clear();
                self.explorer_panel_state.p1.contract_response = None;
                self.explorer_panel_state.p1.contracts_cursor = 0;
                self.explorer_panel_state.p1.pending_contracts_refresh = true;
            }
            ExplorerTab::Assets => {
                self.explorer_panel_state.p1.assets_account_filter.clear();
                self.explorer_panel_state.p1.assets_cursor = 0;
                self.explorer_panel_state.p1.pending_assets_refresh = true;
            }
            ExplorerTab::Mempool => {
                self.explorer_panel_state.p1.mempool_status_filter =
                    ExplorerMempoolStatusFilter::All;
                self.explorer_panel_state.p1.mempool_cursor = 0;
                self.explorer_panel_state.p1.selected_mempool_tx = None;
                self.explorer_panel_state.p1.pending_mempool_refresh = true;
            }
        }
    }

    pub(super) fn apply_explorer_address_response(&mut self, response: WebExplorerAddressResponse) {
        if response.ok {
            self.explorer_panel_state.p1.address_cursor = response.cursor;
            self.explorer_panel_state.p1.address_limit = response.limit;
            if let Some(account_id) = response.account_id.as_ref() {
                self.explorer_panel_state.p1.address_account_input = account_id.clone();
            }
            self.explorer_panel_state.p1.address_response = Some(response);
        } else {
            self.explorer_panel_state.p1.address_response = Some(response.clone());
            self.log_explorer_error(
                self.tr("地址查询失败", "Address query failed"),
                response.error_code,
                response.error,
            );
        }
    }

    pub(super) fn apply_explorer_contracts_response(
        &mut self,
        response: WebExplorerContractsResponse,
    ) {
        if response.ok {
            self.explorer_panel_state.p1.contracts_cursor = response.cursor;
            self.explorer_panel_state.p1.contracts_limit = response.limit;
            self.explorer_panel_state.p1.contracts_response = Some(response);
        } else {
            self.log_explorer_error(
                self.tr("合约列表查询失败", "Contracts query failed"),
                response.error_code,
                response.error,
            );
        }
    }

    pub(super) fn apply_explorer_contract_response(
        &mut self,
        response: WebExplorerContractResponse,
    ) {
        if response.ok {
            if let Some(contract_id) = response.contract_id.as_ref() {
                self.explorer_panel_state.p1.contract_id_input = contract_id.clone();
            }
            self.explorer_panel_state.p1.contract_response = Some(response);
        } else {
            self.explorer_panel_state.p1.contract_response = Some(response.clone());
            self.log_explorer_error(
                self.tr("合约详情查询失败", "Contract detail query failed"),
                response.error_code,
                response.error,
            );
        }
    }

    pub(super) fn apply_explorer_assets_response(&mut self, response: WebExplorerAssetsResponse) {
        if response.ok {
            self.explorer_panel_state.p1.assets_cursor = response.cursor;
            self.explorer_panel_state.p1.assets_limit = response.limit;
            self.explorer_panel_state.p1.assets_response = Some(response);
        } else {
            self.log_explorer_error(
                self.tr("资产查询失败", "Assets query failed"),
                response.error_code,
                response.error,
            );
        }
    }

    pub(super) fn apply_explorer_mempool_response(&mut self, response: WebExplorerMempoolResponse) {
        if response.ok {
            let selected_hash = self
                .explorer_panel_state
                .p1
                .selected_mempool_tx
                .as_ref()
                .map(|tx| tx.tx_hash.clone());
            self.explorer_panel_state.p1.mempool_cursor = response.cursor;
            self.explorer_panel_state.p1.mempool_limit = response.limit;
            self.explorer_panel_state.p1.selected_mempool_tx = selected_hash
                .and_then(|hash| response.items.iter().find(|tx| tx.tx_hash == hash).cloned());
            self.explorer_panel_state.p1.mempool_response = Some(response);
        } else {
            self.log_explorer_error(
                self.tr("内存池查询失败", "Mempool query failed"),
                response.error_code,
                response.error,
            );
        }
    }

    pub(super) fn render_address_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Address", "Address"),
            self.tr(
                "地址页优先展示账户快照，再看关联交易流。",
                "Address view is summary-first: account snapshot first, related transactions second.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("account_id");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.p1.address_account_input);
                    if ui.button(self.tr("查询地址", "Query Address")).clicked() {
                        self.explorer_panel_state.p1.address_cursor = 0;
                        self.explorer_panel_state.p1.pending_address_refresh = true;
                    }
                    if ui.button(self.tr("清空", "Clear")).clicked() {
                        self.explorer_panel_state.p1.address_account_input.clear();
                        self.explorer_panel_state.p1.address_cursor = 0;
                        self.explorer_panel_state.p1.address_response = None;
                    }
                    if let Some(response) = self.explorer_panel_state.p1.address_response.as_ref() {
                        Self::explorer_status_chip(
                            ui,
                            format!(
                                "cursor {} / limit {} / total {}",
                                response.cursor, response.limit, response.total
                            ),
                            egui::Color32::from_rgb(112, 121, 130),
                        );
                    }
                });
            },
        );

        ui.add_space(6.0);
        if let Some(response) = self.explorer_panel_state.p1.address_response.clone() {
            if response.ok {
                ui.columns(4, |cols| {
                    Self::explorer_metric_card(
                        &mut cols[0],
                        self.tr("账户", "Account"),
                        response
                            .account_id
                            .clone()
                            .unwrap_or_else(|| "n/a".to_string()),
                        Some(self.tr("query target", "query target").to_string()),
                        egui::Color32::from_rgb(74, 116, 168),
                    );
                    Self::explorer_metric_card(
                        &mut cols[1],
                        self.tr("流动余额", "Liquid"),
                        response.liquid_balance.to_string(),
                        None,
                        egui::Color32::from_rgb(62, 152, 92),
                    );
                    Self::explorer_metric_card(
                        &mut cols[2],
                        self.tr("冻结余额", "Vested"),
                        response.vested_balance.to_string(),
                        None,
                        egui::Color32::from_rgb(201, 146, 44),
                    );
                    Self::explorer_metric_card(
                        &mut cols[3],
                        self.tr("下一 nonce", "Next Nonce"),
                        response.next_nonce_hint.to_string(),
                        Some(
                            response
                                .last_transfer_nonce
                                .map(|value| format!("last {}", value))
                                .unwrap_or_else(|| "last n/a".to_string()),
                        ),
                        egui::Color32::from_rgb(81, 104, 132),
                    );
                });

                ui.add_space(6.0);
                ui.columns(2, |cols| {
                    Self::explorer_card(
                        &mut cols[0],
                        self.tr("地址交易流", "Address Transactions"),
                        self.tr(
                            "按该账户命中的交易集，可直接跳转全局 Txs inspector。",
                            "Transactions matching this account. Jump into the global Txs inspector when needed.",
                        ),
                        |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let prev_disabled = response.cursor == 0;
                                if ui
                                    .add_enabled(
                                        !prev_disabled,
                                        egui::Button::new(self.tr("上一页", "Prev")),
                                    )
                                    .clicked()
                                {
                                    self.explorer_panel_state.p1.address_cursor = self
                                        .explorer_panel_state
                                        .p1
                                        .address_cursor
                                        .saturating_sub(self.explorer_panel_state.p1.address_limit);
                                    self.explorer_panel_state.p1.pending_address_refresh = true;
                                }
                                let next_disabled = response.next_cursor.is_none();
                                if ui
                                    .add_enabled(
                                        !next_disabled,
                                        egui::Button::new(self.tr("下一页", "Next")),
                                    )
                                    .clicked()
                                {
                                    if let Some(next_cursor) = response.next_cursor {
                                        self.explorer_panel_state.p1.address_cursor = next_cursor;
                                        self.explorer_panel_state.p1.pending_address_refresh = true;
                                    }
                                }
                            });

                            let mut clicked_hash = None;
                            egui::ScrollArea::vertical()
                                .max_height(420.0)
                                .show(ui, |ui| {
                                    for tx in &response.items {
                                        if Self::render_tx_row_card(
                                            ui,
                                            tx,
                                            false,
                                            self.explorer_lifecycle_text(tx.status),
                                            self.explorer_lifecycle_color(tx.status),
                                        ) {
                                            clicked_hash = Some(tx.tx_hash.clone());
                                        }
                                        ui.add_space(4.0);
                                    }
                                    if response.items.is_empty() {
                                        Self::render_explorer_empty_panel(
                                            ui,
                                            self.tr("暂无地址交易", "No Address Transactions"),
                                            self.tr(
                                                "该地址当前没有命中任何 explorer 交易记录。",
                                                "This address currently has no matching explorer transaction records.",
                                            ),
                                        );
                                    }
                                });
                            if let Some(tx_hash) = clicked_hash {
                                self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                                self.explorer_panel_state.tx_hash_input = tx_hash.clone();
                                self.explorer_panel_state.pending_tx_hash = Some(tx_hash);
                                self.explorer_panel_state.pending_tx_action_id = None;
                                self.explorer_panel_state.pending_tx_refresh = true;
                            }
                        },
                    );

                    Self::explorer_card(
                        &mut cols[1],
                        self.tr("地址检查板", "Address Inspector"),
                        self.tr(
                            "用于核对余额、nonce 和本次查询元信息。",
                            "Inspect balances, nonce hints, and query metadata.",
                        ),
                        |ui| {
                            Self::render_explorer_detail_row(
                                ui,
                                "account_id",
                                response.account_id.as_deref().unwrap_or("n/a"),
                                true,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "liquid_balance",
                                &response.liquid_balance.to_string(),
                                false,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "vested_balance",
                                &response.vested_balance.to_string(),
                                false,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "last_transfer_nonce",
                                &response
                                    .last_transfer_nonce
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "n/a".to_string()),
                                false,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "next_nonce_hint",
                                &response.next_nonce_hint.to_string(),
                                false,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "observed_at",
                                &response.observed_at_unix_ms.to_string(),
                                false,
                            );
                        },
                    );
                });
            } else {
                Self::render_explorer_error_panel(
                    ui,
                    self.tr("地址查询失败", "Address Query Failed"),
                    format!(
                        "{} ({})",
                        response.error.as_deref().unwrap_or("n/a"),
                        response.error_code.as_deref().unwrap_or("unknown")
                    ),
                );
            }
        } else {
            Self::render_explorer_empty_panel(
                ui,
                self.tr("暂无地址数据", "No Address Data"),
                self.tr(
                    "输入一个 account_id 后即可读取余额、nonce 和关联交易。",
                    "Enter an account_id to inspect balances, nonce hints, and related transactions.",
                ),
            );
        }
    }

    pub(super) fn render_contracts_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Contracts", "Contracts"),
            self.tr(
                "系统合约目录 + 单合约详情检查板。",
                "System contract directory plus a single-contract inspector.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(self.tr("刷新合约目录", "Refresh Contracts"))
                        .clicked()
                    {
                        self.explorer_panel_state.p1.pending_contracts_refresh = true;
                    }
                    ui.label("contract_id");
                    ui.text_edit_singleline(&mut self.explorer_panel_state.p1.contract_id_input);
                    if ui.button(self.tr("查询合约", "Query Contract")).clicked() {
                        self.explorer_panel_state.p1.pending_contract_refresh = true;
                    }
                    if ui.button(self.tr("清空", "Clear")).clicked() {
                        self.explorer_panel_state.p1.contract_id_input.clear();
                        self.explorer_panel_state.p1.contract_response = None;
                    }
                    if let Some(response) = self.explorer_panel_state.p1.contracts_response.as_ref()
                    {
                        Self::explorer_status_chip(
                            ui,
                            format!(
                                "cursor {} / limit {} / total {}",
                                response.cursor, response.limit, response.total
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
                self.tr("合约目录", "Contract Directory"),
                self.tr(
                    "浏览系统合约条目，再点进单合约详情。",
                    "Browse the system contract catalog, then drill into a single contract.",
                ),
                |ui| {
                    if let Some(response) = self.explorer_panel_state.p1.contracts_response.clone() {
                        if response.ok {
                            ui.horizontal_wrapped(|ui| {
                                let prev_disabled = response.cursor == 0;
                                if ui
                                    .add_enabled(
                                        !prev_disabled,
                                        egui::Button::new(self.tr("上一页", "Prev")),
                                    )
                                    .clicked()
                                {
                                    self.explorer_panel_state.p1.contracts_cursor = self
                                        .explorer_panel_state
                                        .p1
                                        .contracts_cursor
                                        .saturating_sub(self.explorer_panel_state.p1.contracts_limit);
                                    self.explorer_panel_state.p1.pending_contracts_refresh = true;
                                }
                                let next_disabled = response.next_cursor.is_none();
                                if ui
                                    .add_enabled(
                                        !next_disabled,
                                        egui::Button::new(self.tr("下一页", "Next")),
                                    )
                                    .clicked()
                                {
                                    if let Some(next_cursor) = response.next_cursor {
                                        self.explorer_panel_state.p1.contracts_cursor = next_cursor;
                                        self.explorer_panel_state.p1.pending_contracts_refresh = true;
                                    }
                                }
                            });

                            let mut selected_contract_id = None;
                            egui::ScrollArea::vertical()
                                .max_height(430.0)
                                .show(ui, |ui| {
                                    for contract in &response.items {
                                        ui.group(|ui| {
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    format!(
                                                        "{} · {}",
                                                        contract.contract_id, contract.contract_type
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                selected_contract_id =
                                                    Some(contract.contract_id.clone());
                                            }
                                            ui.small(format!(
                                                "{} -> {} · {}",
                                                contract.creator_agent_id,
                                                contract.counterparty_agent_id,
                                                contract.summary
                                            ));
                                            Self::explorer_status_chip(
                                                ui,
                                                contract.status.as_str(),
                                                egui::Color32::from_rgb(74, 116, 168),
                                            );
                                        });
                                        ui.add_space(4.0);
                                    }
                                    if response.items.is_empty() {
                                        Self::render_explorer_empty_panel(
                                            ui,
                                            self.tr("暂无合约记录", "No Contracts"),
                                            self.tr(
                                                "当前目录没有返回任何系统合约。",
                                                "The directory returned no system contracts.",
                                            ),
                                        );
                                    }
                                });
                            if let Some(contract_id) = selected_contract_id {
                                self.explorer_panel_state.p1.contract_id_input = contract_id;
                                self.explorer_panel_state.p1.pending_contract_refresh = true;
                            }
                        } else {
                            Self::render_explorer_error_panel(
                                ui,
                                self.tr("合约目录查询失败", "Contract Directory Failed"),
                                format!(
                                    "{} ({})",
                                    response.error.as_deref().unwrap_or("n/a"),
                                    response.error_code.as_deref().unwrap_or("unknown")
                                ),
                            );
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("目录未加载", "Directory Not Loaded"),
                            self.tr(
                                "先刷新合约目录，再选择一个 contract_id。",
                                "Refresh the contract directory first, then choose a contract_id.",
                            ),
                        );
                    }
                },
            );

            Self::explorer_card(
                &mut cols[1],
                self.tr("合约检查板", "Contract Inspector"),
                self.tr(
                    "检查单合约 JSON 快照以及近期交易。",
                    "Inspect a single contract JSON snapshot and its recent transactions.",
                ),
                |ui| {
                    if let Some(response) = self.explorer_panel_state.p1.contract_response.as_ref() {
                        if response.ok {
                            Self::render_explorer_detail_row(
                                ui,
                                "contract_id",
                                response.contract_id.as_deref().unwrap_or("n/a"),
                                true,
                            );
                            Self::render_explorer_detail_row(
                                ui,
                                "observed_at",
                                &response.observed_at_unix_ms.to_string(),
                                false,
                            );
                            if let Some(contract) = response.contract.as_ref() {
                                if let Ok(pretty) = serde_json::to_string_pretty(contract) {
                                    egui::ScrollArea::vertical()
                                        .max_height(260.0)
                                        .show(ui, |ui| {
                                            ui.code(pretty);
                                        });
                                }
                            }
                            if !response.recent_txs.is_empty() {
                                ui.add_space(6.0);
                                ui.strong(self.tr("近期交易", "Recent Transactions"));
                                ui.horizontal_wrapped(|ui| {
                                    for tx in &response.recent_txs {
                                        if ui.button(short_hash(tx.tx_hash.as_str())).clicked() {
                                            self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                                            self.explorer_panel_state.tx_hash_input =
                                                tx.tx_hash.clone();
                                            self.explorer_panel_state.pending_tx_hash =
                                                Some(tx.tx_hash.clone());
                                            self.explorer_panel_state.pending_tx_action_id = None;
                                            self.explorer_panel_state.pending_tx_refresh = true;
                                        }
                                    }
                                });
                            }
                        } else {
                            Self::render_explorer_error_panel(
                                ui,
                                self.tr("合约详情查询失败", "Contract Query Failed"),
                                format!(
                                    "{} ({})",
                                    response.error.as_deref().unwrap_or("n/a"),
                                    response.error_code.as_deref().unwrap_or("unknown")
                                ),
                            );
                        }
                    } else {
                        Self::render_explorer_empty_panel(
                            ui,
                            self.tr("未选择合约", "No Contract Selected"),
                            self.tr(
                                "从左侧目录挑一个 contract_id，右侧会展开 JSON 明细。",
                                "Pick a contract_id from the left directory to inspect JSON detail here.",
                            ),
                        );
                    }
                },
            );
        });
    }

    pub(super) fn render_assets_tab(&mut self, ui: &mut egui::Ui) {
        Self::explorer_card(
            ui,
            self.tr("Assets", "Assets"),
            self.tr(
                "资产页以供应与持仓概览为核心，而不是逐行余额日志。",
                "Assets view is supply-and-holders first, not a balance log dump.",
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("账户过滤", "Account Filter"));
                    ui.text_edit_singleline(
                        &mut self.explorer_panel_state.p1.assets_account_filter,
                    );
                    if ui.button(self.tr("查询资产", "Query Assets")).clicked() {
                        self.explorer_panel_state.p1.assets_cursor = 0;
                        self.explorer_panel_state.p1.pending_assets_refresh = true;
                    }
                    if ui.button(self.tr("清空过滤", "Clear Filter")).clicked() {
                        self.explorer_panel_state.p1.assets_account_filter.clear();
                        self.explorer_panel_state.p1.assets_cursor = 0;
                        self.explorer_panel_state.p1.pending_assets_refresh = true;
                    }
                    if let Some(response) = self.explorer_panel_state.p1.assets_response.as_ref() {
                        Self::explorer_status_chip(
                            ui,
                            format!(
                                "cursor {} / limit {} / total {}",
                                response.cursor, response.limit, response.total
                            ),
                            egui::Color32::from_rgb(112, 121, 130),
                        );
                    }
                });
            },
        );

        ui.add_space(6.0);
        if let Some(response) = self.explorer_panel_state.p1.assets_response.clone() {
            ui.columns(5, |cols| {
                Self::explorer_metric_card(
                    &mut cols[0],
                    self.tr("Token", "Token"),
                    response.token_symbol.clone(),
                    Some(format!("decimals {}", response.token_decimals)),
                    egui::Color32::from_rgb(74, 116, 168),
                );
                Self::explorer_metric_card(
                    &mut cols[1],
                    self.tr("总供应", "Total Supply"),
                    response.total_supply.to_string(),
                    None,
                    egui::Color32::from_rgb(62, 152, 92),
                );
                Self::explorer_metric_card(
                    &mut cols[2],
                    self.tr("流通量", "Circulating"),
                    response.circulating_supply.to_string(),
                    None,
                    egui::Color32::from_rgb(81, 104, 132),
                );
                Self::explorer_metric_card(
                    &mut cols[3],
                    self.tr("已发行", "Issued"),
                    response.total_issued.to_string(),
                    None,
                    egui::Color32::from_rgb(201, 146, 44),
                );
                Self::explorer_metric_card(
                    &mut cols[4],
                    self.tr("已销毁", "Burned"),
                    response.total_burned.to_string(),
                    Some(format!(
                        "nft {}",
                        if response.nft_supported { "on" } else { "off" }
                    )),
                    egui::Color32::from_rgb(188, 60, 60),
                );
            });

            ui.add_space(6.0);
            ui.columns(2, |cols| {
                Self::explorer_card(
                    &mut cols[0],
                    self.tr("持仓排行", "Holder Book"),
                    self.tr(
                        "按账户读取持仓，适合快速核对供应分布。",
                        "Read holder balances account by account to verify supply distribution.",
                    ),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let prev_disabled = response.cursor == 0;
                            if ui
                                .add_enabled(
                                    !prev_disabled,
                                    egui::Button::new(self.tr("上一页", "Prev")),
                                )
                                .clicked()
                            {
                                self.explorer_panel_state.p1.assets_cursor = self
                                    .explorer_panel_state
                                    .p1
                                    .assets_cursor
                                    .saturating_sub(self.explorer_panel_state.p1.assets_limit);
                                self.explorer_panel_state.p1.pending_assets_refresh = true;
                            }
                            let next_disabled = response.next_cursor.is_none();
                            if ui
                                .add_enabled(
                                    !next_disabled,
                                    egui::Button::new(self.tr("下一页", "Next")),
                                )
                                .clicked()
                            {
                                if let Some(next_cursor) = response.next_cursor {
                                    self.explorer_panel_state.p1.assets_cursor = next_cursor;
                                    self.explorer_panel_state.p1.pending_assets_refresh = true;
                                }
                            }
                        });

                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                for holder in &response.holders {
                                    ui.group(|ui| {
                                        ui.label(
                                            egui::RichText::new(holder.account_id.as_str())
                                                .monospace(),
                                        );
                                        ui.small(format!(
                                            "liquid {} · vested {} · total {}",
                                            holder.liquid_balance,
                                            holder.vested_balance,
                                            holder.total_balance
                                        ));
                                        ui.small(format!(
                                            "last nonce {} · next {}",
                                            holder
                                                .last_transfer_nonce
                                                .map(|value| value.to_string())
                                                .unwrap_or_else(|| "n/a".to_string()),
                                            holder.next_nonce_hint
                                        ));
                                    });
                                    ui.add_space(4.0);
                                }
                                if response.holders.is_empty() {
                                    Self::render_explorer_empty_panel(
                                        ui,
                                        self.tr("暂无持仓记录", "No Holders"),
                                        self.tr(
                                            "当前过滤条件没有命中任何持仓账户。",
                                            "No holders matched the current filter.",
                                        ),
                                    );
                                }
                            });
                    },
                );

                Self::explorer_card(
                    &mut cols[1],
                    self.tr("资产检查板", "Asset Inspector"),
                    self.tr(
                        "检查供应元数据、NFT 能力和当前过滤条件。",
                        "Inspect supply metadata, NFT capability, and the active filter.",
                    ),
                    |ui| {
                        Self::render_explorer_detail_row(
                            ui,
                            "token_symbol",
                            response.token_symbol.as_str(),
                            true,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "token_decimals",
                            &response.token_decimals.to_string(),
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "account_filter",
                            response.account_filter.as_deref().unwrap_or("n/a"),
                            true,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "nft_supported",
                            if response.nft_supported {
                                "true"
                            } else {
                                "false"
                            },
                            false,
                        );
                        Self::render_explorer_detail_row(
                            ui,
                            "nft_collections",
                            &response.nft_collections.len().to_string(),
                            false,
                        );
                    },
                );
            });
        } else {
            Self::render_explorer_empty_panel(
                ui,
                self.tr("暂无资产数据", "No Asset Data"),
                self.tr(
                    "打开资产页后可查看 token 供应和 holders 分布。",
                    "Open the Assets view to inspect token supply and holder distribution.",
                ),
            );
        }
    }
}
