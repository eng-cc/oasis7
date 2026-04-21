use super::*;

impl ClientLauncherApp {
    pub(super) fn tr<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        match self.ui_language {
            UiLanguage::ZhCn => zh,
            UiLanguage::EnUs => en,
        }
    }

    pub(super) fn chain_user_mode_label(&self, mode: &str) -> &'static str {
        match (mode, self.ui_language) {
            ("auto_join", UiLanguage::ZhCn) => "自动加入",
            ("auto_join", UiLanguage::EnUs) => "Auto Join",
            ("private_safe", UiLanguage::ZhCn) => "私有安全",
            ("private_safe", UiLanguage::EnUs) => "Private Safe",
            ("public_entry", UiLanguage::ZhCn) => "公网入口",
            ("public_entry", UiLanguage::EnUs) => "Public Entry",
            (_, UiLanguage::ZhCn) => "未知模式",
            (_, UiLanguage::EnUs) => "Unknown Mode",
        }
    }

    pub(super) fn chain_user_mode_option_label(&self, mode: &str) -> &'static str {
        match (mode, self.ui_language) {
            ("auto_join", UiLanguage::ZhCn) => "自动加入（系统探测默认值）",
            ("auto_join", UiLanguage::EnUs) => "Auto Join (system default)",
            ("private_safe", UiLanguage::ZhCn) => "私有安全（禁止公网入口）",
            ("private_safe", UiLanguage::EnUs) => "Private Safe (no public ingress)",
            ("public_entry", UiLanguage::ZhCn) => "公网入口（承担公网职责）",
            ("public_entry", UiLanguage::EnUs) => "Public Entry (serve public ingress)",
            (_, UiLanguage::ZhCn) => "未知模式",
            (_, UiLanguage::EnUs) => "Unknown Mode",
        }
    }

    fn p2p_probe_bool_text(&self, value: bool) -> &'static str {
        match (value, self.ui_language) {
            (true, UiLanguage::ZhCn) => "是",
            (true, UiLanguage::EnUs) => "yes",
            (false, UiLanguage::ZhCn) => "否",
            (false, UiLanguage::EnUs) => "no",
        }
    }

    fn observability_status_text(&self, status: &str) -> &'static str {
        match (status, self.ui_language) {
            ("ok", UiLanguage::ZhCn) => "正常",
            ("ok", UiLanguage::EnUs) => "OK",
            ("warn", UiLanguage::ZhCn) => "告警",
            ("warn", UiLanguage::EnUs) => "Warn",
            ("critical", UiLanguage::ZhCn) => "严重",
            ("critical", UiLanguage::EnUs) => "Critical",
            (_, UiLanguage::ZhCn) => "未知",
            (_, UiLanguage::EnUs) => "Unknown",
        }
    }

    fn observability_status_color(&self, status: &str) -> egui::Color32 {
        match status {
            "ok" => egui::Color32::from_rgb(54, 132, 74),
            "warn" => egui::Color32::from_rgb(184, 122, 36),
            "critical" => egui::Color32::from_rgb(188, 60, 60),
            _ => egui::Color32::from_rgb(110, 110, 110),
        }
    }

    fn peer_health_status_text(&self, status: &str) -> &'static str {
        match (status, self.ui_language) {
            ("active", UiLanguage::ZhCn) => "活跃",
            ("active", UiLanguage::EnUs) => "Active",
            ("candidate", UiLanguage::ZhCn) => "候选",
            ("candidate", UiLanguage::EnUs) => "Candidate",
            ("suspect", UiLanguage::ZhCn) => "可疑",
            ("suspect", UiLanguage::EnUs) => "Suspect",
            ("blocked", UiLanguage::ZhCn) => "阻断",
            ("blocked", UiLanguage::EnUs) => "Blocked",
            (_, UiLanguage::ZhCn) => "未知",
            (_, UiLanguage::EnUs) => "Unknown",
        }
    }

    fn peer_health_status_color(&self, status: &str) -> egui::Color32 {
        match status {
            "active" => egui::Color32::from_rgb(54, 132, 74),
            "candidate" => egui::Color32::from_rgb(96, 122, 168),
            "suspect" => egui::Color32::from_rgb(184, 122, 36),
            "blocked" => egui::Color32::from_rgb(188, 60, 60),
            _ => egui::Color32::from_rgb(110, 110, 110),
        }
    }

    pub(super) fn render_chain_p2p_summary(&mut self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            return;
        }

        let status = self.chain_p2p_status.clone();
        ui.group(|ui| {
            ui.label(self.tr("P2P 加入模式", "P2P Join Mode"));

            if let Some(status) = status {
                ui.horizontal_wrapped(|ui| {
                    ui.small(format!(
                        "{}: {}",
                        self.tr("请求", "Requested"),
                        self.chain_user_mode_label(status.requested_user_mode.as_str())
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("推荐", "Recommended"),
                        self.chain_user_mode_label(status.recommended_user_mode.as_str())
                    ));
                    ui.separator();
                    let applied_mode = status
                        .applied_effective_user_mode
                        .as_deref()
                        .unwrap_or(status.effective_user_mode.as_str());
                    ui.small(format!(
                        "{}: {}",
                        self.tr("运行中", "Applied"),
                        self.chain_user_mode_label(applied_mode)
                    ));
                });

                ui.horizontal_wrapped(|ui| {
                    ui.small(format!(
                        "{}: {}",
                        self.tr("Reachability", "Reachability"),
                        status
                            .detected_reachability
                            .as_deref()
                            .unwrap_or(self.tr("未探测", "unknown"))
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("打洞", "Hole Punch"),
                        status.hole_punch_viability
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: relay={} probe={}",
                        self.tr("证据", "Signals"),
                        self.p2p_probe_bool_text(status.relay_available),
                        self.p2p_probe_bool_text(status.probe_stable)
                    ));
                });

                ui.small(format!(
                    "{}: {}/{}",
                    self.tr("底层角色映射", "Underlying Role Mapping"),
                    status.deployment_mode,
                    status.node_role_claim
                ));

                if !status.rationale.is_empty() {
                    ui.small(format!(
                        "{}: {}",
                        self.tr("检测依据", "Detection Rationale"),
                        status.rationale.join(" | ")
                    ));
                }

                if status.requires_explicit_public_entry_confirmation {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(188, 60, 60),
                        self.tr(
                            "系统检测当前节点可承担公网入口，但默认仍保持私有安全，直到你显式确认。",
                            "The node looks eligible for public entry, but the launcher keeps Private Safe until you explicitly confirm.",
                        ),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(self.tr(
                                "接受公网入口职责",
                                "Accept Public Entry Responsibility",
                            ))
                            .clicked()
                        {
                            self.config.chain_p2p_user_mode = "auto_join".to_string();
                            self.config.chain_p2p_accept_public_entry = true;
                            self.config_dirty = true;
                            self.append_log(self.tr(
                                "已确认公网入口职责；重启区块链后会按自动推荐应用。",
                                "Public entry responsibility confirmed; restart blockchain to apply the automatic recommendation.",
                            ));
                        }
                        if ui
                            .button(self.tr(
                                "保持自动但拒绝公网入口",
                                "Keep Auto Join but Reject Public Entry",
                            ))
                            .clicked()
                        {
                            self.config.chain_p2p_user_mode = "auto_join".to_string();
                            self.config.chain_p2p_accept_public_entry = false;
                            self.config_dirty = true;
                            self.append_log(self.tr(
                                "已拒绝公网入口职责；系统会继续保持非入口模式。",
                                "Public entry responsibility rejected; the launcher will keep a non-public-entry mode.",
                            ));
                        }
                    });
                    ui.small(self.tr(
                        "提示：这是启动器层的显式确认门；未重启前，当前运行态不会被立即切换。",
                        "This is a launcher-level confirmation gate; the running mode will not change until blockchain is restarted.",
                    ));
                }
            } else {
                ui.small(format!(
                    "{}: {}",
                    self.tr("当前选择", "Current Selection"),
                    self.chain_user_mode_option_label(self.config.chain_p2p_user_mode.as_str())
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("公网入口确认", "Public Entry Confirmation"),
                    self.p2p_probe_bool_text(self.config.chain_p2p_accept_public_entry)
                ));
                ui.small(self.tr(
                    "启动区块链后，这里会显示自动检测得到的推荐模式、运行态和证据摘要。",
                    "After blockchain starts, this card will show the recommended mode, applied runtime mode, and detection evidence.",
                ));
            }
        });
    }

    pub(super) fn render_chain_observability_summary(&mut self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            return;
        }

        let status = self.chain_observability_status.clone();
        let replication = self.chain_replication_status.clone();
        ui.group(|ui| {
            ui.label(self.tr("节点观测", "Node Observability"));

            if let Some(status) = status {
                ui.horizontal_wrapped(|ui| {
                    ui.small(format!(
                        "{}: ",
                        self.tr("状态", "Status"),
                    ));
                    ui.colored_label(
                        self.observability_status_color(status.status.as_str()),
                        self.observability_status_text(status.status.as_str()),
                    );
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("已连接 Peer", "Connected Peers"),
                        status.connected_peer_count
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("Peer Heads", "Peer Heads"),
                        status.known_peer_heads
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("网络落后高度", "Network Lag"),
                        status.network_height_lag
                    ));
                });

                ui.horizontal_wrapped(|ui| {
                    ui.small(format!(
                        "{}: active={} candidate={} suspect={} blocked={}",
                        self.tr("Peer 健康", "Peer Health"),
                        status.active_peer_count,
                        status.candidate_peer_count,
                        status.suspect_peer_count,
                        status.blocked_peer_count
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("带问题 Peer", "Peers With Issues"),
                        status.peer_with_issues_count
                    ));
                    ui.separator();
                    ui.small(format!(
                        "{}: {}",
                        self.tr("复制近期错误", "Recent Replication Errors"),
                        status.recent_replication_error_count
                    ));
                });

                ui.small(format!(
                    "{}: {}",
                    self.tr("摘要", "Summary"),
                    status.summary
                ));

                if !status.alerts.is_empty() {
                    let alert_lines = status
                        .alerts
                        .iter()
                        .take(3)
                        .map(|alert| {
                            format!(
                                "[{}] {}",
                                self.observability_status_text(alert.severity.as_str()),
                                alert.summary
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" | ");
                    ui.small(format!(
                        "{}: {}",
                        self.tr("活动告警", "Active Alerts"),
                        alert_lines
                    ));
                }

                if let Some(replication) = replication {
                    let connected_healths = replication
                        .connected_peers
                        .iter()
                        .map(|peer_id| {
                            let health = replication
                                .peer_healths
                                .iter()
                                .find(|health| health.peer_id == *peer_id);
                            (peer_id, health)
                        })
                        .collect::<Vec<_>>();

                    ui.separator();
                    if !replication.local_peer_id.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.small(format!("{}:", self.tr("本地 Peer", "Local Peer")));
                            ui.label(
                                egui::RichText::new(replication.local_peer_id.as_str()).monospace(),
                            );
                        });
                    }

                    ui.small(self.tr("已连接 Peer 明细", "Connected Peer Details"));
                    if connected_healths.is_empty() {
                        ui.small(self.tr(
                            "当前没有已连接 peer；如已发现候选 peer，可继续看上面的 Peer 健康统计和告警。",
                            "No peers are currently connected. Use the peer health counts and alerts above to inspect discovered candidates.",
                        ));
                    } else {
                        for (peer_id, health) in connected_healths {
                            ui.group(|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(peer_id.as_str()).monospace());
                                    if let Some(health) = health {
                                        ui.colored_label(
                                            self.peer_health_status_color(health.status.as_str()),
                                            self.peer_health_status_text(health.status.as_str()),
                                        );
                                        if let Some(path_kind) = health.active_path_kind.as_deref()
                                        {
                                            ui.small(format!(
                                                "{}: {}",
                                                self.tr("路径", "Path"),
                                                path_kind
                                            ));
                                        }
                                        if !health.issues.is_empty() {
                                            ui.small(format!(
                                                "{}: {}",
                                                self.tr("问题", "Issues"),
                                                health.issues.join(", ")
                                            ));
                                        }
                                    } else {
                                        ui.small(self.tr(
                                            "未附带 health 快照",
                                            "No health snapshot attached",
                                        ));
                                    }
                                });
                            });
                        }
                    }
                }
            } else {
                ui.small(self.tr(
                    "启动区块链后，这里会显示节点健康、Peer 数、网络滞后和当前告警。",
                    "After blockchain starts, this card will show node health, peer counts, network lag, and active alerts.",
                ));
            }
        });
    }

    pub(super) fn glossary_term_text(&self, term: GlossaryTerm) -> &'static str {
        match term {
            GlossaryTerm::Nonce => "nonce",
            GlossaryTerm::Slot => "slot",
            GlossaryTerm::Mempool => "mempool",
            GlossaryTerm::ActionId => "action_id",
        }
    }

    pub(super) fn glossary_term_definition(&self, term: GlossaryTerm) -> &'static str {
        match (term, self.ui_language) {
            (GlossaryTerm::Nonce, UiLanguage::ZhCn) => {
                "每个账户的递增序号，用于防重放；通常使用 next_nonce_hint。"
            }
            (GlossaryTerm::Nonce, UiLanguage::EnUs) => {
                "Per-account increasing sequence to prevent replay; usually use next_nonce_hint."
            }
            (GlossaryTerm::Slot, UiLanguage::ZhCn) => {
                "链出块时间片编号；多个 tick 组成一个 slot，用于排序区块时间。"
            }
            (GlossaryTerm::Slot, UiLanguage::EnUs) => {
                "Block time window index; multiple ticks form one slot for chain ordering."
            }
            (GlossaryTerm::Mempool, UiLanguage::ZhCn) => {
                "待打包交易池，包含 accepted/pending 状态的交易。"
            }
            (GlossaryTerm::Mempool, UiLanguage::EnUs) => {
                "Queue of transactions waiting to be packed, including accepted/pending states."
            }
            (GlossaryTerm::ActionId, UiLanguage::ZhCn) => {
                "链内动作编号，可用于精确追踪单笔转账状态与查询。"
            }
            (GlossaryTerm::ActionId, UiLanguage::EnUs) => {
                "On-chain action identifier for tracking one transfer lifecycle and queries."
            }
        }
    }

    pub(super) fn render_glossary_term_chip(&self, ui: &mut egui::Ui, term: GlossaryTerm) {
        ui.label(
            egui::RichText::new(self.glossary_term_text(term))
                .underline()
                .color(egui::Color32::from_rgb(74, 116, 168)),
        )
        .on_hover_text(self.glossary_term_definition(term));
    }

    pub(super) fn append_log<S: Into<String>>(&mut self, line: S) {
        self.logs.push_back(line.into());
        while self.logs.len() > MAX_LOG_LINES {
            self.logs.pop_front();
        }
    }

    pub(super) fn web_request_inflight_for(&self, domain: WebRequestDomain) -> bool {
        match domain {
            WebRequestDomain::StatePoll => self.web_request_inflight.state_poll,
            WebRequestDomain::ControlAction => self.web_request_inflight.control_action,
            #[cfg(target_arch = "wasm32")]
            WebRequestDomain::FeedbackSubmit => self.web_request_inflight.feedback_submit,
            WebRequestDomain::TransferSubmit => self.web_request_inflight.transfer_submit,
            WebRequestDomain::TransferQuery => self.web_request_inflight.transfer_query,
            WebRequestDomain::ExplorerQuery => self.web_request_inflight.explorer_query,
        }
    }

    pub(super) fn set_web_request_inflight(&mut self, domain: WebRequestDomain, inflight: bool) {
        match domain {
            WebRequestDomain::StatePoll => self.web_request_inflight.state_poll = inflight,
            WebRequestDomain::ControlAction => self.web_request_inflight.control_action = inflight,
            #[cfg(target_arch = "wasm32")]
            WebRequestDomain::FeedbackSubmit => {
                self.web_request_inflight.feedback_submit = inflight;
            }
            WebRequestDomain::TransferSubmit => {
                self.web_request_inflight.transfer_submit = inflight;
            }
            WebRequestDomain::TransferQuery => self.web_request_inflight.transfer_query = inflight,
            WebRequestDomain::ExplorerQuery => self.web_request_inflight.explorer_query = inflight,
        }
    }

    #[cfg(test)]
    pub(super) fn any_web_request_inflight(&self) -> bool {
        self.web_request_inflight.any()
    }

    pub(super) fn any_transfer_request_inflight(&self) -> bool {
        self.web_request_inflight.transfer_any()
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn apply_web_feedback_submit_result(
        &mut self,
        result: Result<WebFeedbackSubmitResponse, String>,
    ) {
        match result {
            Ok(response) => {
                if response.ok {
                    let feedback_id = response.feedback_id.unwrap_or_else(|| "n/a".to_string());
                    let event_id = response.event_id.unwrap_or_else(|| "n/a".to_string());
                    let message = format!(
                        "{}: feedback_id={feedback_id}, event_id={event_id}",
                        self.tr(
                            "反馈已提交到分布式网络",
                            "Feedback submitted to distributed network"
                        )
                    );
                    self.append_log(message.clone());
                    self.feedback_submit_state = FeedbackSubmitState::Success(message);
                } else {
                    let error_text = response
                        .error
                        .unwrap_or_else(|| self.tr("未知错误", "Unknown error").to_string());
                    let message = format!(
                        "{}: {error_text}",
                        self.tr("反馈提交被拒绝", "Feedback submit rejected")
                    );
                    self.append_log(message.clone());
                    self.feedback_submit_state = FeedbackSubmitState::Failed(message);
                }
            }
            Err(err) => {
                let message = format!(
                    "{}: {err}",
                    self.tr("反馈提交失败", "Feedback submit failed")
                );
                self.append_log(message.clone());
                self.feedback_submit_state = FeedbackSubmitState::Failed(message);
            }
        }
    }

    pub(super) fn feedback_unavailable_hint(&self) -> Option<String> {
        if self.is_feedback_available() {
            return None;
        }
        let message = match (&self.chain_runtime_status, self.ui_language) {
            (ChainRuntimeStatus::Disabled, UiLanguage::ZhCn) => {
                "反馈/转账/浏览器功能已禁用：区块链功能关闭".to_string()
            }
            (ChainRuntimeStatus::Disabled, UiLanguage::EnUs) => {
                "Feedback/Transfer/Explorer are disabled because blockchain is disabled".to_string()
            }
            (ChainRuntimeStatus::NotStarted, UiLanguage::ZhCn) => {
                "反馈/转账/浏览器功能暂不可用：区块链未启动".to_string()
            }
            (ChainRuntimeStatus::NotStarted, UiLanguage::EnUs) => {
                "Feedback/Transfer/Explorer are unavailable because blockchain is not started"
                    .to_string()
            }
            (ChainRuntimeStatus::Starting, UiLanguage::ZhCn) => {
                "反馈/转账/浏览器功能暂不可用：区块链启动中".to_string()
            }
            (ChainRuntimeStatus::Starting, UiLanguage::EnUs) => {
                "Feedback/Transfer/Explorer are unavailable while blockchain is starting"
                    .to_string()
            }
            (ChainRuntimeStatus::StaleExecutionWorld(detail), UiLanguage::ZhCn) => {
                format!("反馈/转账/浏览器功能暂不可用：检测到旧执行世界冲突（{detail}）")
            }
            (ChainRuntimeStatus::StaleExecutionWorld(detail), UiLanguage::EnUs) => {
                format!(
                    "Feedback/Transfer/Explorer are unavailable: stale execution world detected ({detail})"
                )
            }
            (ChainRuntimeStatus::Unreachable(detail), UiLanguage::ZhCn) => {
                format!("反馈/转账/浏览器功能暂不可用：区块链不可达（{detail}）")
            }
            (ChainRuntimeStatus::Unreachable(detail), UiLanguage::EnUs) => {
                format!(
                    "Feedback/Transfer/Explorer are unavailable: blockchain unreachable ({detail})"
                )
            }
            (ChainRuntimeStatus::ConfigError(detail), UiLanguage::ZhCn) => {
                format!("反馈/转账/浏览器功能暂不可用：区块链配置错误（{detail}）")
            }
            (ChainRuntimeStatus::ConfigError(detail), UiLanguage::EnUs) => {
                format!("Feedback/Transfer/Explorer are unavailable: blockchain config error ({detail})")
            }
            (ChainRuntimeStatus::Ready, UiLanguage::ZhCn) => {
                "反馈/转账/浏览器功能暂不可用：区块链功能关闭".to_string()
            }
            (ChainRuntimeStatus::Ready, UiLanguage::EnUs) => {
                "Feedback/Transfer/Explorer are unavailable: blockchain is disabled".to_string()
            }
        };
        Some(message)
    }
}
