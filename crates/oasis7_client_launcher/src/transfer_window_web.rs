use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebTransferDraftIssue {
    FromAccountRequired,
    ToAccountRequired,
    SameAccount,
    AmountInvalid,
    NonceInvalid,
}

fn validate_web_transfer_draft(draft: &TransferDraft) -> Vec<WebTransferDraftIssue> {
    let mut issues = Vec::new();
    if draft.from_account_id.trim().is_empty() {
        issues.push(WebTransferDraftIssue::FromAccountRequired);
    }
    if draft.to_account_id.trim().is_empty() {
        issues.push(WebTransferDraftIssue::ToAccountRequired);
    }
    if !draft.from_account_id.trim().is_empty()
        && !draft.to_account_id.trim().is_empty()
        && draft.from_account_id.trim() == draft.to_account_id.trim()
    {
        issues.push(WebTransferDraftIssue::SameAccount);
    }
    if parse_positive_u64(draft.amount.as_str()).is_none() {
        issues.push(WebTransferDraftIssue::AmountInvalid);
    }
    if parse_positive_u64(draft.nonce.as_str()).is_none() {
        issues.push(WebTransferDraftIssue::NonceInvalid);
    }
    issues
}

fn parse_positive_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

impl ClientLauncherApp {
    fn web_transfer_issue_text(&self, issue: WebTransferDraftIssue) -> &'static str {
        match (issue, self.ui_language) {
            (WebTransferDraftIssue::FromAccountRequired, UiLanguage::ZhCn) => "转出账户不能为空",
            (WebTransferDraftIssue::FromAccountRequired, UiLanguage::EnUs) => {
                "From account cannot be empty"
            }
            (WebTransferDraftIssue::ToAccountRequired, UiLanguage::ZhCn) => "转入账户不能为空",
            (WebTransferDraftIssue::ToAccountRequired, UiLanguage::EnUs) => {
                "To account cannot be empty"
            }
            (WebTransferDraftIssue::SameAccount, UiLanguage::ZhCn) => "转出与转入账户不能相同",
            (WebTransferDraftIssue::SameAccount, UiLanguage::EnUs) => {
                "From and to accounts cannot be the same"
            }
            (WebTransferDraftIssue::AmountInvalid, UiLanguage::ZhCn) => "金额必须是大于 0 的整数",
            (WebTransferDraftIssue::AmountInvalid, UiLanguage::EnUs) => {
                "Amount must be a positive integer"
            }
            (WebTransferDraftIssue::NonceInvalid, UiLanguage::ZhCn) => "Nonce 必须是大于 0 的整数",
            (WebTransferDraftIssue::NonceInvalid, UiLanguage::EnUs) => {
                "Nonce must be a positive integer"
            }
        }
    }

    pub(super) fn submit_transfer(&mut self) {
        let issues = validate_web_transfer_draft(&self.transfer_draft);
        if !issues.is_empty() {
            for issue in issues {
                self.append_log(format!(
                    "transfer validation failed: {}",
                    self.web_transfer_issue_text(issue)
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

        let amount = parse_positive_u64(self.transfer_draft.amount.as_str())
            .expect("validated amount should be positive");
        let nonce = parse_positive_u64(self.transfer_draft.nonce.as_str())
            .expect("validated nonce should be positive");
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

        let issues = validate_web_transfer_draft(&self.transfer_draft);
        let submit_enabled = issues.is_empty()
            && self.is_feedback_available()
            && !self.web_request_inflight_for(WebRequestDomain::TransferSubmit);

        let title = self.tr("链上转账", "On-Chain Transfer").to_string();
        let mut window_open = self.transfer_window_open;
        egui::Window::new(title)
            .open(&mut window_open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("转出账户", "From Account"));
                    ui.text_edit_singleline(&mut self.transfer_draft.from_account_id);
                    ui.label(self.tr("转入账户", "To Account"));
                    ui.text_edit_singleline(&mut self.transfer_draft.to_account_id);
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.tr("金额", "Amount"));
                    ui.text_edit_singleline(&mut self.transfer_draft.amount);
                    ui.label(self.tr("Nonce", "Nonce"));
                    ui.text_edit_singleline(&mut self.transfer_draft.nonce);
                    if ui
                        .add_enabled(
                            submit_enabled,
                            egui::Button::new(self.tr("提交转账", "Submit Transfer")),
                        )
                        .clicked()
                    {
                        self.submit_transfer();
                    }
                });

                if self.any_transfer_request_inflight() {
                    ui.small(
                        egui::RichText::new(
                            self.tr("请求处理中，请稍候…", "Request in flight, please wait..."),
                        )
                        .color(egui::Color32::from_rgb(201, 146, 44)),
                    );
                }
                if !self.is_feedback_available() {
                    ui.small(
                        egui::RichText::new(self.tr(
                            "区块链未就绪时不可提交转账",
                            "Transfer submit is unavailable until blockchain is ready",
                        ))
                        .color(egui::Color32::from_rgb(196, 84, 84)),
                    );
                }
                if !issues.is_empty() {
                    ui.small(
                        egui::RichText::new(self.tr(
                            "提交前请完善必填项：",
                            "Please complete required fields before submit:",
                        ))
                        .color(egui::Color32::from_rgb(196, 84, 84)),
                    );
                    for issue in issues {
                        ui.small(
                            egui::RichText::new(format!(
                                "- {}",
                                self.web_transfer_issue_text(issue)
                            ))
                            .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                }
                match &self.transfer_submit_state {
                    TransferSubmitState::Success(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(62, 152, 92)),
                        );
                    }
                    TransferSubmitState::Failed(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                    TransferSubmitState::None => {}
                }
            });

        self.transfer_window_open = window_open;
    }
}
