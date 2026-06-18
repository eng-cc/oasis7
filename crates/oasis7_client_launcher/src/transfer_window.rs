use super::*;

const TRANSFER_STATUS_POLL_INTERVAL_MS: u64 = 1_000;
const TRANSFER_AMOUNT_PRESETS: [u64; 3] = [1, 10, 100];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum WebTransferLifecycleStatus {
    Accepted,
    Pending,
    Confirmed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebTransferAccountEntry {
    pub(super) account_id: String,
    pub(super) liquid_balance: u64,
    pub(super) vested_balance: u64,
    pub(super) next_nonce_hint: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebTransferAccountsResponse {
    pub(super) ok: bool,
    #[serde(default)]
    pub(super) accounts: Vec<WebTransferAccountEntry>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebTransferHistoryItem {
    pub(super) action_id: u64,
    pub(super) from_account_id: String,
    pub(super) to_account_id: String,
    pub(super) amount: u64,
    pub(super) nonce: u64,
    pub(super) status: WebTransferLifecycleStatus,
    pub(super) submitted_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebTransferHistoryResponse {
    pub(super) ok: bool,
    #[serde(default)]
    pub(super) items: Vec<WebTransferHistoryItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebTransferStatusResponse {
    pub(super) ok: bool,
    pub(super) action_id: u64,
    pub(super) status: Option<WebTransferHistoryItem>,
    pub(super) error_code: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum TransferQueryResponse {
    Accounts(WebTransferAccountsResponse),
    History(WebTransferHistoryResponse),
    Status(WebTransferStatusResponse),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferNonceMode {
    Auto,
    Manual,
}

impl Default for TransferNonceMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct TransferPanelState {
    pub(super) accounts: Vec<WebTransferAccountEntry>,
    pub(super) history: Vec<WebTransferHistoryItem>,
    pub(super) tracked_action_status: Option<WebTransferHistoryItem>,
    pub(super) tracked_action_id: Option<u64>,
    pub(super) history_account_filter: String,
    pub(super) history_action_filter: String,
    pub(super) auto_nonce_hint: Option<u64>,
    pub(super) nonce_mode: TransferNonceMode,
    pub(super) pending_accounts_refresh: bool,
    pub(super) pending_history_refresh: bool,
    pub(super) pending_status_refresh: bool,
    pub(super) last_status_poll_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferDraftIssue {
    FromAccountRequired,
    ToAccountRequired,
    SameAccount,
    AmountInvalid,
    NonceInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferTimelineState {
    Waiting,
    Active,
    Done,
    Failed,
}

fn parse_positive_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

pub(super) fn hosted_public_join_transfer_blocked(config: &LaunchConfig) -> bool {
    config.deployment_mode.trim() == "hosted_public_join"
}

fn hosted_public_join_transfer_block_message(app: &ClientLauncherApp) -> String {
    app.tr(
        "转账提交失败：hosted public join 当前仍要求 strong auth，不能继续使用 trusted-local signer bootstrap",
        "Transfer submit failed: hosted public join still requires strong auth and cannot reuse trusted-local signer bootstrap",
    )
    .to_string()
}

pub(super) fn transfer_amount_presets() -> &'static [u64] {
    &TRANSFER_AMOUNT_PRESETS
}

pub(super) fn recommend_default_from_account(
    accounts: &[WebTransferAccountEntry],
) -> Option<String> {
    accounts
        .iter()
        .max_by(|lhs, rhs| {
            lhs.liquid_balance
                .cmp(&rhs.liquid_balance)
                .then_with(|| rhs.account_id.cmp(&lhs.account_id))
        })
        .map(|account| account.account_id.clone())
}

pub(super) fn recommend_transfer_account_ids(
    accounts: &[WebTransferAccountEntry],
    from_account_id: &str,
    limit: usize,
) -> Vec<String> {
    let from_account_id = from_account_id.trim();
    let mut candidates: Vec<&WebTransferAccountEntry> = accounts
        .iter()
        .filter(|account| account.account_id.as_str() != from_account_id)
        .collect();
    candidates.sort_by(|lhs, rhs| {
        rhs.liquid_balance
            .cmp(&lhs.liquid_balance)
            .then_with(|| lhs.account_id.cmp(&rhs.account_id))
    });
    candidates
        .into_iter()
        .take(limit)
        .map(|account| account.account_id.clone())
        .collect()
}

pub(super) fn resolve_transfer_timeline(
    status: WebTransferLifecycleStatus,
) -> [TransferTimelineState; 3] {
    match status {
        WebTransferLifecycleStatus::Accepted => [
            TransferTimelineState::Active,
            TransferTimelineState::Waiting,
            TransferTimelineState::Waiting,
        ],
        WebTransferLifecycleStatus::Pending => [
            TransferTimelineState::Done,
            TransferTimelineState::Active,
            TransferTimelineState::Waiting,
        ],
        WebTransferLifecycleStatus::Confirmed => [
            TransferTimelineState::Done,
            TransferTimelineState::Done,
            TransferTimelineState::Done,
        ],
        WebTransferLifecycleStatus::Failed | WebTransferLifecycleStatus::Timeout => [
            TransferTimelineState::Done,
            TransferTimelineState::Done,
            TransferTimelineState::Failed,
        ],
    }
}

fn is_final_status(status: WebTransferLifecycleStatus) -> bool {
    matches!(
        status,
        WebTransferLifecycleStatus::Confirmed
            | WebTransferLifecycleStatus::Failed
            | WebTransferLifecycleStatus::Timeout
    )
}

fn validate_transfer_draft(
    draft: &TransferDraft,
    nonce_mode: TransferNonceMode,
    auto_nonce_hint: Option<u64>,
) -> Vec<TransferDraftIssue> {
    let mut issues = Vec::new();
    if draft.from_account_id.trim().is_empty() {
        issues.push(TransferDraftIssue::FromAccountRequired);
    }
    if draft.to_account_id.trim().is_empty() {
        issues.push(TransferDraftIssue::ToAccountRequired);
    }
    if !draft.from_account_id.trim().is_empty()
        && !draft.to_account_id.trim().is_empty()
        && draft.from_account_id.trim() == draft.to_account_id.trim()
    {
        issues.push(TransferDraftIssue::SameAccount);
    }
    if parse_positive_u64(draft.amount.as_str()).is_none() {
        issues.push(TransferDraftIssue::AmountInvalid);
    }
    match nonce_mode {
        TransferNonceMode::Auto => {
            if auto_nonce_hint.is_none() {
                issues.push(TransferDraftIssue::NonceInvalid);
            }
        }
        TransferNonceMode::Manual => {
            if parse_positive_u64(draft.nonce.as_str()).is_none() {
                issues.push(TransferDraftIssue::NonceInvalid);
            }
        }
    }
    issues
}

impl ClientLauncherApp {
    fn transfer_issue_text(&self, issue: TransferDraftIssue) -> &'static str {
        match (issue, self.ui_language) {
            (TransferDraftIssue::FromAccountRequired, UiLanguage::ZhCn) => "转出账户不能为空",
            (TransferDraftIssue::FromAccountRequired, UiLanguage::EnUs) => {
                "From account cannot be empty"
            }
            (TransferDraftIssue::ToAccountRequired, UiLanguage::ZhCn) => "转入账户不能为空",
            (TransferDraftIssue::ToAccountRequired, UiLanguage::EnUs) => {
                "To account cannot be empty"
            }
            (TransferDraftIssue::SameAccount, UiLanguage::ZhCn) => "转出与转入账户不能相同",
            (TransferDraftIssue::SameAccount, UiLanguage::EnUs) => {
                "From and to accounts cannot be the same"
            }
            (TransferDraftIssue::AmountInvalid, UiLanguage::ZhCn) => "金额必须是大于 0 的整数",
            (TransferDraftIssue::AmountInvalid, UiLanguage::EnUs) => {
                "Amount must be a positive integer"
            }
            (TransferDraftIssue::NonceInvalid, UiLanguage::ZhCn) => {
                "Nonce 无效（自动模式需可用 nonce 提示）"
            }
            (TransferDraftIssue::NonceInvalid, UiLanguage::EnUs) => {
                "Nonce is invalid (auto mode requires a nonce hint)"
            }
        }
    }

    fn transfer_status_text(&self, status: WebTransferLifecycleStatus) -> &'static str {
        match (status, self.ui_language) {
            (WebTransferLifecycleStatus::Accepted, UiLanguage::ZhCn) => "已受理",
            (WebTransferLifecycleStatus::Accepted, UiLanguage::EnUs) => "Accepted",
            (WebTransferLifecycleStatus::Pending, UiLanguage::ZhCn) => "待确认",
            (WebTransferLifecycleStatus::Pending, UiLanguage::EnUs) => "Pending",
            (WebTransferLifecycleStatus::Confirmed, UiLanguage::ZhCn) => "已确认",
            (WebTransferLifecycleStatus::Confirmed, UiLanguage::EnUs) => "Confirmed",
            (WebTransferLifecycleStatus::Failed, UiLanguage::ZhCn) => "失败",
            (WebTransferLifecycleStatus::Failed, UiLanguage::EnUs) => "Failed",
            (WebTransferLifecycleStatus::Timeout, UiLanguage::ZhCn) => "超时",
            (WebTransferLifecycleStatus::Timeout, UiLanguage::EnUs) => "Timeout",
        }
    }

    fn transfer_timeline_stage_label(&self, stage_index: usize) -> &'static str {
        match (stage_index, self.ui_language) {
            (0, UiLanguage::ZhCn) => "已受理",
            (0, UiLanguage::EnUs) => "Accepted",
            (1, UiLanguage::ZhCn) => "待确认",
            (1, UiLanguage::EnUs) => "Pending",
            (2, UiLanguage::ZhCn) => "最终",
            (2, UiLanguage::EnUs) => "Final",
            _ => "",
        }
    }

    fn transfer_timeline_marker(&self, state: TransferTimelineState) -> &'static str {
        match state {
            TransferTimelineState::Waiting => "[ ]",
            TransferTimelineState::Active => "[>]",
            TransferTimelineState::Done => "[x]",
            TransferTimelineState::Failed => "[!]",
        }
    }

    fn transfer_timeline_color(&self, state: TransferTimelineState) -> egui::Color32 {
        match state {
            TransferTimelineState::Waiting => egui::Color32::from_rgb(148, 148, 148),
            TransferTimelineState::Active => egui::Color32::from_rgb(74, 116, 168),
            TransferTimelineState::Done => egui::Color32::from_rgb(62, 152, 92),
            TransferTimelineState::Failed => egui::Color32::from_rgb(196, 84, 84),
        }
    }

    fn render_transfer_timeline(
        &self,
        ui: &mut egui::Ui,
        states: [TransferTimelineState; 3],
        final_status: Option<WebTransferLifecycleStatus>,
    ) {
        ui.horizontal_wrapped(|ui| {
            for (index, state) in states.iter().enumerate() {
                let mut stage_text = format!(
                    "{} {}",
                    self.transfer_timeline_marker(*state),
                    self.transfer_timeline_stage_label(index)
                );
                if index == 2 {
                    if let Some(status) = final_status {
                        if matches!(
                            status,
                            WebTransferLifecycleStatus::Confirmed
                                | WebTransferLifecycleStatus::Failed
                                | WebTransferLifecycleStatus::Timeout
                        ) {
                            stage_text =
                                format!("{stage_text} ({})", self.transfer_status_text(status));
                        }
                    }
                }
                ui.small(
                    egui::RichText::new(stage_text).color(self.transfer_timeline_color(*state)),
                );
                if index < 2 {
                    ui.small("->");
                }
            }
        });
    }

    fn transfer_account(&self, account_id: &str) -> Option<&WebTransferAccountEntry> {
        self.transfer_panel_state
            .accounts
            .iter()
            .find(|item| item.account_id == account_id)
    }

    fn refresh_auto_nonce_hint(&mut self) {
        self.transfer_panel_state.auto_nonce_hint = self
            .transfer_account(self.transfer_draft.from_account_id.trim())
            .map(|item| item.next_nonce_hint)
            .filter(|value| *value > 0);
        if self.transfer_panel_state.nonce_mode == TransferNonceMode::Auto {
            if let Some(nonce) = self.transfer_panel_state.auto_nonce_hint {
                self.transfer_draft.nonce = nonce.to_string();
            }
        }
    }

    pub(super) fn clear_transfer_history_filters(&mut self) {
        self.transfer_panel_state.history_account_filter.clear();
        self.transfer_panel_state.history_action_filter.clear();
        self.transfer_panel_state.pending_history_refresh = true;
    }

    fn maybe_request_transfer_panel_data(&mut self) {
        if self.any_transfer_request_inflight() {
            return;
        }

        if self.transfer_panel_state.pending_accounts_refresh {
            self.transfer_panel_state.pending_accounts_refresh = false;
            self.request_web_chain_transfer_accounts();
            return;
        }

        if self.transfer_panel_state.pending_history_refresh {
            self.transfer_panel_state.pending_history_refresh = false;
            self.request_web_chain_transfer_history(
                self.transfer_panel_state.history_account_filter.clone(),
                self.transfer_panel_state.history_action_filter.clone(),
            );
            return;
        }

        if self.transfer_panel_state.pending_status_refresh {
            if let Some(action_id) = self.transfer_panel_state.tracked_action_id {
                self.transfer_panel_state.pending_status_refresh = false;
                self.transfer_panel_state.last_status_poll_at = Some(Instant::now());
                self.request_web_chain_transfer_status(action_id);
            }
            return;
        }

        let Some(action_id) = self.transfer_panel_state.tracked_action_id else {
            return;
        };
        let status_final = self
            .transfer_panel_state
            .tracked_action_status
            .as_ref()
            .is_some_and(|item| is_final_status(item.status));
        if status_final {
            return;
        }
        let now = Instant::now();
        let should_poll = self
            .transfer_panel_state
            .last_status_poll_at
            .is_none_or(|last| {
                now.duration_since(last) >= Duration::from_millis(TRANSFER_STATUS_POLL_INTERVAL_MS)
            });
        if should_poll {
            self.transfer_panel_state.last_status_poll_at = Some(now);
            self.request_web_chain_transfer_status(action_id);
        }
    }

    pub(super) fn apply_web_transfer_submit_result(
        &mut self,
        result: Result<WebTransferSubmitResponse, String>,
    ) {
        match result {
            Ok(response) => {
                if response.ok {
                    let action_id_text = response
                        .action_id
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "n/a".to_string());
                    let submitted_at_text = response
                        .submitted_at_unix_ms
                        .map(|value| format!(", submitted_at={value}"))
                        .unwrap_or_default();
                    let lifecycle_text = response
                        .lifecycle_status
                        .map(|status| format!(", status={status:?}"))
                        .unwrap_or_default();
                    let message = format!(
                        "{}: action_id={action_id_text}{submitted_at_text}{lifecycle_text}",
                        self.tr("转账请求已提交", "Transfer request submitted")
                    );
                    self.append_log(message.clone());
                    self.transfer_submit_state = TransferSubmitState::Success(message);
                    self.transfer_panel_state.tracked_action_id = response.action_id;
                    self.transfer_panel_state.pending_status_refresh = true;
                    self.transfer_panel_state.pending_history_refresh = true;
                } else {
                    let error_text = response
                        .error
                        .unwrap_or_else(|| self.tr("未知错误", "Unknown error").to_string());
                    let error_code = response
                        .error_code
                        .map(|value| format!(" ({value})"))
                        .unwrap_or_default();
                    let message = format!(
                        "{}{}: {error_text}",
                        self.tr("转账提交被拒绝", "Transfer submit rejected"),
                        error_code
                    );
                    self.append_log(message.clone());
                    self.transfer_submit_state = TransferSubmitState::Failed(message);
                }
            }
            Err(err) => {
                let message = format!(
                    "{}: {err}",
                    self.tr("转账提交失败", "Transfer submit failed")
                );
                self.append_log(message.clone());
                self.transfer_submit_state = TransferSubmitState::Failed(message);
            }
        }
    }

    pub(super) fn apply_web_transfer_query_result(
        &mut self,
        result: Result<TransferQueryResponse, String>,
    ) {
        match result {
            Ok(TransferQueryResponse::Accounts(response)) => {
                if response.ok {
                    self.transfer_panel_state.accounts = response.accounts;
                    self.refresh_auto_nonce_hint();
                } else {
                    let error_code = response
                        .error_code
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default();
                    let error_text = response
                        .error
                        .unwrap_or_else(|| self.tr("未知错误", "Unknown error").to_string());
                    self.append_log(format!(
                        "{}{}: {error_text}",
                        self.tr("账户/余额查询失败", "Transfer accounts query failed"),
                        error_code
                    ));
                }
            }
            Ok(TransferQueryResponse::History(response)) => {
                if response.ok {
                    self.transfer_panel_state.history = response.items;
                } else {
                    let error_code = response
                        .error_code
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default();
                    let error_text = response
                        .error
                        .unwrap_or_else(|| self.tr("未知错误", "Unknown error").to_string());
                    self.append_log(format!(
                        "{}{}: {error_text}",
                        self.tr("转账历史查询失败", "Transfer history query failed"),
                        error_code
                    ));
                }
            }
            Ok(TransferQueryResponse::Status(response)) => {
                if response.ok {
                    self.transfer_panel_state.tracked_action_id = Some(response.action_id);
                    self.transfer_panel_state.tracked_action_status = response.status;
                } else {
                    let error_code = response
                        .error_code
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default();
                    let error_text = response
                        .error
                        .unwrap_or_else(|| self.tr("未知错误", "Unknown error").to_string());
                    self.append_log(format!(
                        "{}{}: {error_text}",
                        self.tr("转账状态查询失败", "Transfer status query failed"),
                        error_code
                    ));
                }
            }
            Err(err) => {
                self.append_log(format!(
                    "{}: {err}",
                    self.tr("转账查询失败", "Transfer query failed")
                ));
            }
        }
    }

    pub(super) fn submit_transfer(&mut self) {
        let issues = validate_transfer_draft(
            &self.transfer_draft,
            self.transfer_panel_state.nonce_mode,
            self.transfer_panel_state.auto_nonce_hint,
        );
        if !issues.is_empty() {
            for issue in issues {
                self.append_log(format!(
                    "transfer validation failed: {}",
                    self.transfer_issue_text(issue)
                ));
            }
            self.transfer_submit_state = TransferSubmitState::Failed(
                self.tr(
                    "转账提交失败：请先修复输入项",
                    "Transfer submit failed: fix required input fields first",
                )
                .to_string(),
            );
            return;
        }

        if hosted_public_join_transfer_blocked(&self.config) {
            let message = hosted_public_join_transfer_block_message(self);
            self.append_log(message.clone());
            self.transfer_submit_state = TransferSubmitState::Failed(message);
            return;
        }

        if !self.config.chain_enabled {
            let message = self
                .tr(
                    "转账提交失败：链运行时未启用",
                    "Transfer submit failed: chain runtime is disabled",
                )
                .to_string();
            self.append_log(message.clone());
            self.transfer_submit_state = TransferSubmitState::Failed(message);
            return;
        }

        if !self.is_feedback_available() {
            let message = self
                .tr(
                    "转账提交失败：区块链尚未就绪",
                    "Transfer submit failed: blockchain is not ready",
                )
                .to_string();
            self.append_log(message.clone());
            self.transfer_submit_state = TransferSubmitState::Failed(message);
            return;
        }

        if self.web_request_inflight_for(WebRequestDomain::TransferSubmit) {
            let message = self
                .tr(
                    "转账提交失败：已有请求处理中",
                    "Transfer submit failed: another request is in flight",
                )
                .to_string();
            self.append_log(message.clone());
            self.transfer_submit_state = TransferSubmitState::Failed(message);
            return;
        }

        let Some(amount) = parse_positive_u64(self.transfer_draft.amount.as_str()) else {
            self.transfer_submit_state = TransferSubmitState::Failed(
                self.tr(
                    "转账提交失败：金额非法",
                    "Transfer submit failed: invalid amount",
                )
                .to_string(),
            );
            return;
        };

        let nonce = match self.transfer_panel_state.nonce_mode {
            TransferNonceMode::Auto => self.transfer_panel_state.auto_nonce_hint,
            TransferNonceMode::Manual => parse_positive_u64(self.transfer_draft.nonce.as_str()),
        };
        let Some(nonce) = nonce else {
            self.transfer_submit_state = TransferSubmitState::Failed(
                self.tr(
                    "转账提交失败：Nonce 不可用",
                    "Transfer submit failed: nonce is unavailable",
                )
                .to_string(),
            );
            return;
        };

        self.transfer_draft.nonce = nonce.to_string();
        let Some(chain_identity) = self.chain_identity.as_ref() else {
            let message = self
                .tr(
                    "转账提交失败：未获取到运行中链身份，请等待链状态就绪",
                    "Transfer submit failed: live chain identity is unavailable; wait for chain status to become ready",
                )
                .to_string();
            self.append_log(message.clone());
            self.transfer_submit_state = TransferSubmitState::Failed(message);
            return;
        };
        let request = match crate::transfer_auth::build_signed_web_transfer_submit_request(
            self.transfer_draft.from_account_id.as_str(),
            self.transfer_draft.to_account_id.as_str(),
            amount,
            nonce,
            Some(chain_identity.chain_id.as_str()),
            Some(chain_identity.network_id.as_str()),
        ) {
            Ok(request) => request,
            Err(err) => {
                let message = format!(
                    "{}: {err}",
                    self.tr("转账签名失败", "Transfer signing failed")
                );
                self.append_log(message.clone());
                self.transfer_submit_state = TransferSubmitState::Failed(message);
                return;
            }
        };
        self.transfer_submit_state = TransferSubmitState::None;
        self.request_web_chain_transfer(request);
    }

    pub(super) fn show_transfer_window(&mut self, ctx: &egui::Context) {
        if !self.transfer_window_open {
            return;
        }

        if self.transfer_panel_state.accounts.is_empty() {
            self.transfer_panel_state.pending_accounts_refresh = true;
        }
        if self.transfer_panel_state.history.is_empty() {
            self.transfer_panel_state.pending_history_refresh = true;
        }
        self.maybe_request_transfer_panel_data();

        let issues = validate_transfer_draft(
            &self.transfer_draft,
            self.transfer_panel_state.nonce_mode,
            self.transfer_panel_state.auto_nonce_hint,
        );
        let strong_auth_barrier = hosted_public_join_transfer_blocked(&self.config);
        let submit_enabled = issues.is_empty()
            && !strong_auth_barrier
            && self.is_feedback_available()
            && !self.web_request_inflight_for(WebRequestDomain::TransferSubmit);

        let title = self.tr("链上转账", "On-Chain Transfer").to_string();
        let mut window_open = self.transfer_window_open;
        egui::Window::new(title)
            .open(&mut window_open)
            .resizable(true)
            .default_size(Self::modal_window_size(ctx, 920.0, 680.0))
            .show(ctx, |ui| {
                let header_chip = if submit_enabled {
                    (
                        self.tr("可提交", "Ready"),
                        egui::Color32::from_rgb(62, 152, 92),
                    )
                } else if self.any_transfer_request_inflight() {
                    (
                        self.tr("请求中", "In Flight"),
                        egui::Color32::from_rgb(201, 146, 44),
                    )
                } else {
                    (
                        self.tr("需检查", "Needs Check"),
                        egui::Color32::from_rgb(188, 60, 60),
                    )
                };
                Self::modal_header(
                    ui,
                    self.tr("链上转账", "On-Chain Transfer"),
                    self.tr(
                        "选择账户、金额和 nonce 后提交链上转账。",
                        "Choose accounts, amount, and nonce before submitting an on-chain transfer.",
                    ),
                    Some(header_chip),
                );
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        Self::modal_card(ui, |ui| {
                            ui.label(
                                egui::RichText::new(self.tr("提交转账", "Submit Transfer"))
                                    .strong()
                                    .size(14.0),
                            );
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(self.tr("转出账户", "From Account"));
                                let mut selected_from_account: Option<String> = None;
                                egui::ComboBox::from_id_salt("transfer_from_account_selector")
                                    .selected_text(
                                        if self.transfer_draft.from_account_id.trim().is_empty() {
                                            self.tr("请选择", "Select")
                                        } else {
                                            self.transfer_draft.from_account_id.as_str()
                                        },
                                    )
                                    .show_ui(ui, |ui| {
                                        for account in &self.transfer_panel_state.accounts {
                                            if ui
                                                .selectable_label(
                                                    self.transfer_draft.from_account_id
                                                        == account.account_id,
                                                    account.account_id.as_str(),
                                                )
                                                .clicked()
                                            {
                                                selected_from_account =
                                                    Some(account.account_id.clone());
                                            }
                                        }
                                    });
                                if let Some(account_id) = selected_from_account {
                                    self.transfer_draft.from_account_id = account_id;
                                    self.refresh_auto_nonce_hint();
                                }
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(
                                            &mut self.transfer_draft.from_account_id,
                                        )
                                        .desired_width(260.0),
                                    )
                                    .changed()
                                {
                                    self.refresh_auto_nonce_hint();
                                }
                            });

                            if self.transfer_draft.from_account_id.trim().is_empty() {
                                if let Some(recommended_from) = recommend_default_from_account(
                                    self.transfer_panel_state.accounts.as_slice(),
                                ) {
                                    let button_text = format!(
                                        "{}: {recommended_from}",
                                        self.tr("推荐转出", "Suggested Sender")
                                    );
                                    if Self::modal_secondary_button(ui, &button_text).clicked() {
                                        self.transfer_draft.from_account_id = recommended_from;
                                        self.refresh_auto_nonce_hint();
                                    }
                                }
                            } else if let Some(from_account) =
                                self.transfer_account(self.transfer_draft.from_account_id.trim())
                            {
                                Self::modal_status_chip(
                                    ui,
                                    &format!(
                                        "{}: {} / vested {}",
                                        self.tr("可用余额", "Liquid balance"),
                                        from_account.liquid_balance,
                                        from_account.vested_balance
                                    ),
                                    egui::Color32::from_rgb(74, 116, 168),
                                );
                            }

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(self.tr("转入账户", "To Account"));
                                egui::ComboBox::from_id_salt("transfer_to_account_selector")
                                    .selected_text(
                                        if self.transfer_draft.to_account_id.trim().is_empty() {
                                            self.tr("请选择", "Select")
                                        } else {
                                            self.transfer_draft.to_account_id.as_str()
                                        },
                                    )
                                    .show_ui(ui, |ui| {
                                        for account in &self.transfer_panel_state.accounts {
                                            if ui
                                                .selectable_label(
                                                    self.transfer_draft.to_account_id
                                                        == account.account_id,
                                                    account.account_id.as_str(),
                                                )
                                                .clicked()
                                            {
                                                self.transfer_draft.to_account_id =
                                                    account.account_id.clone();
                                            }
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut self.transfer_draft.to_account_id,
                                    )
                                    .desired_width(260.0),
                                );
                            });

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(self.tr("金额", "Amount"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.transfer_draft.amount)
                                        .desired_width(140.0),
                                );
                                ui.label(self.tr("快捷金额", "Quick Amount"));
                                for preset in transfer_amount_presets() {
                                    if Self::modal_secondary_button(ui, &preset.to_string())
                                        .clicked()
                                    {
                                        self.transfer_draft.amount = preset.to_string();
                                    }
                                }
                            });

                            let recommended_to_accounts = recommend_transfer_account_ids(
                                self.transfer_panel_state.accounts.as_slice(),
                                self.transfer_draft.from_account_id.as_str(),
                                3,
                            );
                            if !recommended_to_accounts.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(self.tr("推荐转入", "Suggested Receiver"));
                                    for account_id in &recommended_to_accounts {
                                        if Self::modal_secondary_button(ui, account_id.as_str())
                                            .clicked()
                                        {
                                            self.transfer_draft.to_account_id = account_id.clone();
                                        }
                                    }
                                });
                            }

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(self.tr("Nonce 模式", "Nonce Mode"));
                                let auto_text = self.tr("自动", "Auto");
                                let manual_text = self.tr("手动", "Manual");
                                ui.selectable_value(
                                    &mut self.transfer_panel_state.nonce_mode,
                                    TransferNonceMode::Auto,
                                    auto_text,
                                );
                                ui.selectable_value(
                                    &mut self.transfer_panel_state.nonce_mode,
                                    TransferNonceMode::Manual,
                                    manual_text,
                                );
                                match self.transfer_panel_state.nonce_mode {
                                    TransferNonceMode::Auto => {
                                        if let Some(hint) =
                                            self.transfer_panel_state.auto_nonce_hint
                                        {
                                            self.transfer_draft.nonce = hint.to_string();
                                            Self::modal_status_chip(
                                                ui,
                                                &format!(
                                                    "{}: {}",
                                                    self.tr("自动 nonce", "Auto nonce hint"),
                                                    hint
                                                ),
                                                egui::Color32::from_rgb(62, 152, 92),
                                            );
                                        } else {
                                            ui.small(self.tr(
                                                "当前账户暂无 nonce 提示，请先刷新账户或改为手动",
                                                "No nonce hint for current account, refresh accounts or switch to manual",
                                            ));
                                        }
                                    }
                                    TransferNonceMode::Manual => {
                                        ui.label("Nonce");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.transfer_draft.nonce,
                                            )
                                            .desired_width(120.0),
                                        );
                                    }
                                }
                            });

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.small(self.tr("术语", "Glossary"));
                                self.render_glossary_term_chip(ui, GlossaryTerm::Nonce);
                                self.render_glossary_term_chip(ui, GlossaryTerm::ActionId);
                            });
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add_enabled(
                                        submit_enabled,
                                        egui::Button::new(self.tr("提交转账", "Submit Transfer"))
                                            .fill(egui::Color32::from_rgb(39, 128, 75)),
                                    )
                                    .clicked()
                                {
                                    self.submit_transfer();
                                }
                                if Self::modal_secondary_button(
                                    ui,
                                    self.tr("刷新账户/历史", "Refresh Accounts/History"),
                                )
                                .clicked()
                                {
                                    self.transfer_panel_state.pending_accounts_refresh = true;
                                    self.transfer_panel_state.pending_history_refresh = true;
                                }
                            });
                            ui.small(self.tr(
                                "建议：先选转出/转入账户，再点快捷金额，保持自动 nonce 后提交。",
                                "Tip: choose sender/receiver, use quick amount presets, then submit with auto nonce.",
                            ));
                        });
                        ui.add_space(8.0);

                if self.any_transfer_request_inflight() {
                    Self::modal_banner(
                        ui,
                            self.tr("请求处理中，请稍候…", "Request in flight, please wait..."),
                        egui::Color32::from_rgb(201, 146, 44),
                    );
                }
                if !self.is_feedback_available() {
                    Self::modal_banner(
                        ui,
                        self.tr(
                            "区块链未就绪时不可提交转账",
                            "Transfer submit is unavailable until blockchain is ready",
                        ),
                        egui::Color32::from_rgb(196, 84, 84),
                    );
                }
                if strong_auth_barrier {
                    Self::modal_banner(
                        ui,
                        &hosted_public_join_transfer_block_message(self),
                        egui::Color32::from_rgb(196, 84, 84),
                    );
                }
                if !issues.is_empty() {
                    Self::modal_card(ui, |ui| {
                        ui.label(
                            egui::RichText::new(self.tr(
                            "提交前请完善必填项：",
                            "Please complete required fields before submit:",
                        ))
                        .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                        for issue in &issues {
                            ui.small(
                                egui::RichText::new(format!(
                                    "- {}",
                                    self.transfer_issue_text(*issue)
                                ))
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
                match &self.transfer_submit_state {
                    TransferSubmitState::Success(message) => {
                        Self::modal_banner(
                            ui,
                            message.as_str(),
                            egui::Color32::from_rgb(62, 152, 92),
                        );
                    }
                    TransferSubmitState::Failed(message) => {
                        Self::modal_banner(
                            ui,
                            message.as_str(),
                            egui::Color32::from_rgb(196, 84, 84),
                        );
                    }
                    TransferSubmitState::None => {}
                }

                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                ui.label(egui::RichText::new(self.tr("状态追踪", "Transfer Status")).strong().size(14.0));
                if let Some(status) = self.transfer_panel_state.tracked_action_status.as_ref() {
                    ui.small(format!(
                        "action_id={} | from={} -> to={} | amount={} | nonce={} | submitted_at={} | updated_at={}",
                        status.action_id,
                        status.from_account_id,
                        status.to_account_id,
                        status.amount,
                        status.nonce,
                        status.submitted_at_unix_ms,
                        status.updated_at_unix_ms,
                    ));
                    self.render_transfer_timeline(
                        ui,
                        resolve_transfer_timeline(status.status),
                        Some(status.status),
                    );
                    if let Some(error) = status.error.as_deref() {
                        let error_code = status.error_code.as_deref().unwrap_or("unknown");
                        ui.small(
                            egui::RichText::new(format!("error ({error_code}): {error}"))
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                } else if let Some(action_id) = self.transfer_panel_state.tracked_action_id {
                    ui.small(format!(
                        "action_id={} | {}",
                        action_id,
                        self.tr("等待状态更新", "Waiting for status update")
                    ));
                    self.render_transfer_timeline(
                        ui,
                        [
                            TransferTimelineState::Active,
                            TransferTimelineState::Waiting,
                            TransferTimelineState::Waiting,
                        ],
                        None,
                    );
                } else {
                    ui.small(self.tr(
                        "提交后会在这里展示 action 状态和确认进度。",
                        "After submit, action status and confirmation progress appear here.",
                    ));
                }
                });

                ui.add_space(8.0);
                Self::modal_card(ui, |ui| {
                ui.label(egui::RichText::new(self.tr("转账历史", "Transfer History")).strong().size(14.0));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("账户过滤", "Account Filter"));
                    ui.add(
                        egui::TextEdit::singleline(
                            &mut self.transfer_panel_state.history_account_filter,
                        )
                        .desired_width(180.0),
                    );
                    ui.label(self.tr("Action 查询", "Action Query"));
                    ui.add(
                        egui::TextEdit::singleline(
                            &mut self.transfer_panel_state.history_action_filter,
                        )
                        .desired_width(140.0),
                    );
                    if Self::modal_secondary_button(ui, self.tr("应用过滤", "Apply Filter")).clicked()
                    {
                        self.transfer_panel_state.pending_history_refresh = true;
                    }
                    if Self::modal_secondary_button(ui, self.tr("清空过滤", "Clear Filters")).clicked() {
                        self.clear_transfer_history_filters();
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for item in &self.transfer_panel_state.history {
                            let status_text = self.transfer_status_text(item.status);
                            ui.small(format!(
                                "#{:>4} | {} | {} -> {} | amount={} | nonce={} | submitted_at={}",
                                item.action_id,
                                status_text,
                                item.from_account_id,
                                item.to_account_id,
                                item.amount,
                                item.nonce,
                                item.submitted_at_unix_ms,
                            ));
                        }
                        if self.transfer_panel_state.history.is_empty() {
                            ui.small(self.tr("暂无转账历史", "No transfer history"));
                        }
                    });
                });
                    });
            });

        self.transfer_window_open = window_open;
    }
}
