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
            pending_mempool_refresh: false,
        }
    }
}

fn explorer_mempool_status_filter_text(
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
            self.explorer_panel_state.p1.mempool_cursor = response.cursor;
            self.explorer_panel_state.p1.mempool_limit = response.limit;
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
        ui.label(self.tr("地址查询", "Address"));
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
        });

        if let Some(response) = self.explorer_panel_state.p1.address_response.clone() {
            ui.small(format!(
                "observed_at={} | cursor={} limit={} total={}",
                response.observed_at_unix_ms, response.cursor, response.limit, response.total,
            ));

            if response.ok {
                ui.small(format!(
                    "account={} | liquid={} | vested={} | last_nonce={} | next_nonce_hint={}",
                    response.account_id.as_deref().unwrap_or("n/a"),
                    response.liquid_balance,
                    response.vested_balance,
                    response
                        .last_transfer_nonce
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "n/a".to_string()),
                    response.next_nonce_hint,
                ));

                ui.horizontal_wrapped(|ui| {
                    let prev_disabled = response.cursor == 0;
                    if ui
                        .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
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
                        .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
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
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for tx in &response.items {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "[{}]",
                                        self.explorer_lifecycle_text(tx.status)
                                    ))
                                    .color(self.explorer_lifecycle_color(tx.status)),
                                );
                                let line = format!(
                                    "{} | {} -> {} | amount={} | {}",
                                    short_hash(tx.tx_hash.as_str()),
                                    tx.from_account_id,
                                    tx.to_account_id,
                                    tx.amount,
                                    tx.submitted_at_unix_ms,
                                );
                                if ui.selectable_label(false, line).clicked() {
                                    clicked_hash = Some(tx.tx_hash.clone());
                                }
                            });
                        }
                        if response.items.is_empty() {
                            ui.small(self.tr("暂无地址交易", "No address txs"));
                        }
                    });
                if let Some(tx_hash) = clicked_hash {
                    self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                    self.explorer_panel_state.tx_hash_input = tx_hash.clone();
                    self.explorer_panel_state.pending_tx_hash = Some(tx_hash);
                    self.explorer_panel_state.pending_tx_action_id = None;
                    self.explorer_panel_state.pending_tx_refresh = true;
                }
            } else {
                ui.small(
                    egui::RichText::new(format!(
                        "{} ({})",
                        response.error.as_deref().unwrap_or("n/a"),
                        response.error_code.as_deref().unwrap_or("unknown")
                    ))
                    .color(egui::Color32::from_rgb(196, 84, 84)),
                );
            }
        } else {
            ui.small(self.tr("暂无地址数据", "No address data"));
        }
    }

    pub(super) fn render_contracts_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(self.tr("系统合约", "Contracts"));
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
        });

        if let Some(response) = self.explorer_panel_state.p1.contracts_response.clone() {
            ui.small(format!(
                "observed_at={} | cursor={} limit={} total={}",
                response.observed_at_unix_ms, response.cursor, response.limit, response.total,
            ));
            if response.ok {
                ui.horizontal_wrapped(|ui| {
                    let prev_disabled = response.cursor == 0;
                    if ui
                        .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
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
                        .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
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
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for contract in &response.items {
                            let line = format!(
                                "{} [{}] {} -> {} | {}",
                                contract.contract_id,
                                contract.status,
                                contract.creator_agent_id,
                                contract.counterparty_agent_id,
                                contract.summary,
                            );
                            if ui.selectable_label(false, line).clicked() {
                                selected_contract_id = Some(contract.contract_id.clone());
                            }
                        }
                        if response.items.is_empty() {
                            ui.small(self.tr("暂无合约记录", "No contracts"));
                        }
                    });
                if let Some(contract_id) = selected_contract_id {
                    self.explorer_panel_state.p1.contract_id_input = contract_id;
                    self.explorer_panel_state.p1.pending_contract_refresh = true;
                }
            }
        }

        ui.separator();
        ui.label(self.tr("合约详情", "Contract Detail"));
        if let Some(response) = self.explorer_panel_state.p1.contract_response.as_ref() {
            ui.small(format!(
                "contract_id={} | observed_at={}",
                response.contract_id.as_deref().unwrap_or("n/a"),
                response.observed_at_unix_ms,
            ));
            if response.ok {
                if let Some(contract) = response.contract.as_ref() {
                    if let Ok(pretty) = serde_json::to_string_pretty(contract) {
                        ui.code(pretty);
                    }
                }
                if !response.recent_txs.is_empty() {
                    ui.small(self.tr("近期交易", "Recent txs"));
                    ui.horizontal_wrapped(|ui| {
                        for tx in &response.recent_txs {
                            if ui.button(short_hash(tx.tx_hash.as_str())).clicked() {
                                self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                                self.explorer_panel_state.tx_hash_input = tx.tx_hash.clone();
                                self.explorer_panel_state.pending_tx_hash =
                                    Some(tx.tx_hash.clone());
                                self.explorer_panel_state.pending_tx_action_id = None;
                                self.explorer_panel_state.pending_tx_refresh = true;
                            }
                        }
                    });
                }
            } else {
                ui.small(
                    egui::RichText::new(format!(
                        "{} ({})",
                        response.error.as_deref().unwrap_or("n/a"),
                        response.error_code.as_deref().unwrap_or("unknown")
                    ))
                    .color(egui::Color32::from_rgb(196, 84, 84)),
                );
            }
        } else {
            ui.small(self.tr("未选择合约", "No contract selected"));
        }
    }

    pub(super) fn render_assets_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(self.tr("资产总览", "Assets"));
        ui.horizontal_wrapped(|ui| {
            ui.label(self.tr("账户过滤", "Account Filter"));
            ui.text_edit_singleline(&mut self.explorer_panel_state.p1.assets_account_filter);
            if ui.button(self.tr("查询资产", "Query Assets")).clicked() {
                self.explorer_panel_state.p1.assets_cursor = 0;
                self.explorer_panel_state.p1.pending_assets_refresh = true;
            }
            if ui.button(self.tr("清空过滤", "Clear Filter")).clicked() {
                self.explorer_panel_state.p1.assets_account_filter.clear();
                self.explorer_panel_state.p1.assets_cursor = 0;
                self.explorer_panel_state.p1.pending_assets_refresh = true;
            }
        });

        if let Some(response) = self.explorer_panel_state.p1.assets_response.clone() {
            ui.small(format!(
                "{} decimals={} | total_supply={} circulating={} issued={} burned={}",
                response.token_symbol,
                response.token_decimals,
                response.total_supply,
                response.circulating_supply,
                response.total_issued,
                response.total_burned,
            ));
            ui.small(format!(
                "nft_supported={} | nft_collections={}",
                response.nft_supported,
                response.nft_collections.len(),
            ));
            ui.small(format!(
                "cursor={} limit={} total={}",
                response.cursor, response.limit, response.total,
            ));

            ui.horizontal_wrapped(|ui| {
                let prev_disabled = response.cursor == 0;
                if ui
                    .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
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
                    .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
                    .clicked()
                {
                    if let Some(next_cursor) = response.next_cursor {
                        self.explorer_panel_state.p1.assets_cursor = next_cursor;
                        self.explorer_panel_state.p1.pending_assets_refresh = true;
                    }
                }
            });

            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for holder in &response.holders {
                        ui.small(format!(
                            "{} | liquid={} vested={} total={} | nonce={} -> next={}",
                            holder.account_id,
                            holder.liquid_balance,
                            holder.vested_balance,
                            holder.total_balance,
                            holder
                                .last_transfer_nonce
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "n/a".to_string()),
                            holder.next_nonce_hint,
                        ));
                    }
                    if response.holders.is_empty() {
                        ui.small(self.tr("暂无持仓记录", "No holders"));
                    }
                });
        } else {
            ui.small(self.tr("暂无资产数据", "No assets data"));
        }
    }

    pub(super) fn render_mempool_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(self.tr("内存池", "Mempool"));
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
        });

        if let Some(response) = self.explorer_panel_state.p1.mempool_response.clone() {
            ui.small(format!(
                "status={} | accepted={} pending={} | cursor={} limit={} total={}",
                response.status_filter,
                response.accepted_count,
                response.pending_count,
                response.cursor,
                response.limit,
                response.total,
            ));

            ui.horizontal_wrapped(|ui| {
                let prev_disabled = response.cursor == 0;
                if ui
                    .add_enabled(!prev_disabled, egui::Button::new(self.tr("上一页", "Prev")))
                    .clicked()
                {
                    self.explorer_panel_state.p1.mempool_cursor = self
                        .explorer_panel_state
                        .p1
                        .mempool_cursor
                        .saturating_sub(self.explorer_panel_state.p1.mempool_limit);
                    self.explorer_panel_state.p1.pending_mempool_refresh = true;
                }
                let next_disabled = response.next_cursor.is_none();
                if ui
                    .add_enabled(!next_disabled, egui::Button::new(self.tr("下一页", "Next")))
                    .clicked()
                {
                    if let Some(next_cursor) = response.next_cursor {
                        self.explorer_panel_state.p1.mempool_cursor = next_cursor;
                        self.explorer_panel_state.p1.pending_mempool_refresh = true;
                    }
                }
            });

            let mut clicked_hash = None;
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for tx in &response.items {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "[{}]",
                                    self.explorer_lifecycle_text(tx.status)
                                ))
                                .color(self.explorer_lifecycle_color(tx.status)),
                            );
                            let line = format!(
                                "{} | {} -> {} | amount={} | {}",
                                short_hash(tx.tx_hash.as_str()),
                                tx.from_account_id,
                                tx.to_account_id,
                                tx.amount,
                                tx.submitted_at_unix_ms,
                            );
                            if ui.selectable_label(false, line).clicked() {
                                clicked_hash = Some(tx.tx_hash.clone());
                            }
                        });
                    }
                    if response.items.is_empty() {
                        ui.small(self.tr("暂无待处理交易", "No pending txs"));
                    }
                });
            if let Some(tx_hash) = clicked_hash {
                self.explorer_panel_state.active_tab = ExplorerTab::Txs;
                self.explorer_panel_state.tx_hash_input = tx_hash.clone();
                self.explorer_panel_state.pending_tx_hash = Some(tx_hash);
                self.explorer_panel_state.pending_tx_action_id = None;
                self.explorer_panel_state.pending_tx_refresh = true;
            }
        } else {
            ui.small(self.tr("暂无内存池数据", "No mempool data"));
        }
    }
}
