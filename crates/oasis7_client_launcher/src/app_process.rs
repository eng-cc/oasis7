use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;

use serde::de::DeserializeOwned;

use super::*;

const CONTROL_PLANE_BOOT_TIMEOUT_MS: u64 = 4_000;
const CONTROL_PLANE_BOOT_POLL_INTERVAL_MS: u64 = 80;
const CONTROL_PLANE_HTTP_TIMEOUT_MS: u64 = 1_500;

impl ClientLauncherApp {
    pub(super) fn current_game_url(&self) -> String {
        self.web_game_url
            .clone()
            .unwrap_or_else(|| build_game_url(&self.config))
    }

    pub(super) fn is_feedback_available(&self) -> bool {
        self.config.chain_enabled && matches!(self.chain_runtime_status, ChainRuntimeStatus::Ready)
    }

    pub(super) fn maybe_auto_start_chain(&mut self) {
        self.ensure_control_plane_service();
        if self.chain_auto_start_attempted {
            return;
        }
        if !self.config.chain_enabled {
            self.chain_runtime_status = ChainRuntimeStatus::Disabled;
            self.chain_auto_start_attempted = true;
            return;
        }
        if !should_request_auto_chain_start(
            self.chain_auto_start_attempted,
            self.config.chain_enabled,
            self.web_request_inflight_for(WebRequestDomain::ControlAction),
            self.control_plane_snapshot_received,
        ) {
            return;
        }
        self.chain_auto_start_attempted = true;
        self.start_chain_process();
    }

    pub(super) fn update_chain_runtime_status(&mut self) {}

    pub(super) fn trigger_state_refresh(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::StatePoll) {
            self.append_log("skip state refresh: previous web request still in flight".to_string());
            return;
        }
        self.request_web_state();
    }

    pub(super) fn poll_process(&mut self) {
        self.poll_control_plane_process();

        while let Ok(event) = self.web_api_rx.try_recv() {
            self.last_web_poll_at = Some(Instant::now());
            match event {
                WebApiEvent::State(result) => {
                    self.set_web_request_inflight(WebRequestDomain::StatePoll, false);
                    match result {
                        Ok(snapshot) => self.apply_web_snapshot(snapshot),
                        Err(err) => {
                            self.status = LauncherStatus::QueryFailed;
                            self.append_log(format!("web state refresh failed: {err}"));
                        }
                    }
                }
                WebApiEvent::Action(result) => {
                    self.set_web_request_inflight(WebRequestDomain::ControlAction, false);
                    match result {
                        Ok(response) => {
                            if !response.ok {
                                if let Some(error) = response.error {
                                    self.append_log(format!("web action failed: {error}"));
                                } else {
                                    self.append_log("web action failed".to_string());
                                }
                            }
                            self.apply_web_snapshot(response.state);
                        }
                        Err(err) => {
                            self.status = LauncherStatus::QueryFailed;
                            self.append_log(format!("web action request failed: {err}"));
                        }
                    }
                }
                WebApiEvent::Transfer(result) => {
                    self.set_web_request_inflight(WebRequestDomain::TransferSubmit, false);
                    self.apply_web_transfer_submit_result(result);
                }
                WebApiEvent::TransferQuery(result) => {
                    self.set_web_request_inflight(WebRequestDomain::TransferQuery, false);
                    self.apply_web_transfer_query_result(result);
                }
                WebApiEvent::ExplorerQuery(result) => {
                    self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, false);
                    self.apply_web_explorer_query_result(result);
                }
            }
        }

        let now = Instant::now();
        let should_poll = self.last_web_poll_at.is_none_or(|last| {
            now.duration_since(last) >= Duration::from_millis(WEB_POLL_INTERVAL_MS)
        });
        if should_poll && !self.web_request_inflight_for(WebRequestDomain::StatePoll) {
            self.request_web_state();
        }
    }

    pub(super) fn poll_chain_process(&mut self) {}

    pub(super) fn stop_process(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::ControlAction) {
            self.append_log("skip stop: previous web request still in flight".to_string());
            return;
        }
        self.request_web_stop();
    }

    pub(super) fn start_process(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::ControlAction) {
            self.append_log("skip start: previous web request still in flight".to_string());
            return;
        }

        let config_issues = collect_required_config_issues(&self.config);
        if !config_issues.is_empty() {
            self.status = LauncherStatus::InvalidArgs;
            let message = self
                .tr(
                    "游戏启动前校验失败：请先修复必填配置项",
                    "game preflight validation failed: fix required configuration issues first",
                )
                .to_string();
            self.append_log(message);
            for issue in config_issues {
                self.append_log(format!("- {}", issue.text(self.ui_language)));
            }
            return;
        }

        self.request_web_start();
    }

    pub(super) fn stop_chain_process(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::ControlAction) {
            self.append_log("skip chain stop: previous web request still in flight".to_string());
            return;
        }
        self.request_web_chain_stop();
    }

    pub(super) fn start_chain_process(&mut self) {
        if !self.config.chain_enabled {
            self.chain_runtime_status = ChainRuntimeStatus::Disabled;
            self.append_log("chain runtime start skipped: chain runtime disabled");
            return;
        }

        if self.web_request_inflight_for(WebRequestDomain::ControlAction) {
            self.append_log("skip chain start: previous web request still in flight".to_string());
            return;
        }

        let config_issues = collect_chain_required_config_issues(&self.config);
        if !config_issues.is_empty() {
            let mut details = Vec::new();
            for issue in config_issues {
                let detail = issue.text(self.ui_language).to_string();
                details.push(detail.clone());
                self.append_log(format!("- {detail}"));
            }
            self.chain_runtime_status = ChainRuntimeStatus::ConfigError(details.join("; "));
            self.append_log("chain runtime preflight validation failed");
            return;
        }

        self.request_web_chain_start();
    }

    fn ensure_control_plane_service(&mut self) {
        if !self.control_manage_service {
            return;
        }
        if self.running.is_some() {
            return;
        }

        let web_launcher_bin = platform_ops::resolve_web_launcher_binary_path()
            .to_string_lossy()
            .to_string();
        if web_launcher_bin.trim().is_empty() {
            self.status = LauncherStatus::QueryFailed;
            self.append_log("control plane bootstrap failed: web launcher binary is empty");
            return;
        }

        let launcher_bin = if self.config.launcher_bin.trim().is_empty() {
            resolve_launcher_binary_path().to_string_lossy().to_string()
        } else {
            self.config.launcher_bin.trim().to_string()
        };
        let chain_runtime_bin = if self.config.chain_runtime_bin.trim().is_empty() {
            resolve_chain_runtime_binary_path()
                .to_string_lossy()
                .to_string()
        } else {
            self.config.chain_runtime_bin.trim().to_string()
        };

        let mut args = vec![
            "--listen-bind".to_string(),
            self.control_listen_bind.clone(),
            "--launcher-bin".to_string(),
            launcher_bin,
            "--chain-runtime-bin".to_string(),
            chain_runtime_bin,
        ];
        if let Some((_, static_dir)) = read_named_env_value(&["OASIS7_WEB_LAUNCHER_STATIC_DIR"]) {
            args.push("--console-static-dir".to_string());
            args.push(static_dir);
        }

        match spawn_child_process(web_launcher_bin.as_str(), args.as_slice(), "control") {
            Ok(process) => {
                self.append_log(format!(
                    "control plane started (bind={}, bin={web_launcher_bin})",
                    self.control_listen_bind
                ));
                self.running = Some(process);
                self.wait_control_plane_ready();
            }
            Err(err) => {
                self.status = LauncherStatus::QueryFailed;
                self.append_log(format!("control plane start failed: {err}"));
            }
        }
    }

    fn wait_control_plane_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_millis(CONTROL_PLANE_BOOT_TIMEOUT_MS);
        while Instant::now() < deadline {
            self.poll_control_plane_process();
            if self.running.is_none() {
                return;
            }
            if check_web_health(self.control_api_base.as_str()).is_ok() {
                self.append_log(format!("control plane ready at {}", self.control_api_base));
                return;
            }
            thread::sleep(Duration::from_millis(CONTROL_PLANE_BOOT_POLL_INTERVAL_MS));
        }
        self.append_log(format!(
            "control plane health check timeout at {}",
            self.control_api_base
        ));
    }

    fn poll_control_plane_process(&mut self) {
        let mut running = match self.running.take() {
            Some(process) => process,
            None => return,
        };

        loop {
            match running.log_rx.try_recv() {
                Ok(line) => self.append_log(line),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        match running.child.try_wait() {
            Ok(Some(status)) => {
                self.status = LauncherStatus::QueryFailed;
                self.append_log(format!("control plane exited: {status}"));
                self.running = None;
            }
            Ok(None) => {
                self.running = Some(running);
            }
            Err(err) => {
                self.status = LauncherStatus::QueryFailed;
                self.append_log(format!("query control plane status failed: {err}"));
                self.running = None;
            }
        }
    }

    fn request_web_state(&mut self) {
        self.ensure_control_plane_service();
        self.set_web_request_inflight(WebRequestDomain::StatePoll, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::State(fetch_web_state_blocking(
                base_url.as_str(),
            )));
        });
    }

    fn request_web_start(&mut self) {
        self.ensure_control_plane_service();
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        let config = self.config.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::Action(post_web_start_blocking(
                base_url.as_str(),
                config,
            )));
        });
    }

    fn request_web_stop(&mut self) {
        self.ensure_control_plane_service();
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::Action(post_web_stop_blocking(
                base_url.as_str(),
            )));
        });
    }

    fn request_web_chain_start(&mut self) {
        self.ensure_control_plane_service();
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        let config = self.config.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::Action(post_web_chain_start_blocking(
                base_url.as_str(),
                config,
            )));
        });
    }

    fn request_web_chain_stop(&mut self) {
        self.ensure_control_plane_service();
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::Action(post_web_chain_stop_blocking(
                base_url.as_str(),
            )));
        });
    }

    pub(super) fn request_web_chain_transfer(&mut self, request: WebTransferSubmitRequest) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::TransferSubmit) {
            self.append_log("skip transfer submit: previous transfer submit still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferSubmit, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::Transfer(post_web_chain_transfer_blocking(
                base_url.as_str(),
                request,
            )));
        });
    }

    pub(super) fn request_web_chain_transfer_accounts(&mut self) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log(
                "skip transfer accounts query: previous transfer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::TransferQuery(
                get_web_chain_transfer_accounts_blocking(base_url.as_str())
                    .map(transfer_window::TransferQueryResponse::Accounts),
            ));
        });
    }

    pub(super) fn request_web_chain_transfer_history(
        &mut self,
        account_filter: String,
        action_filter: String,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log("skip transfer history query: previous transfer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::TransferQuery(
                get_web_chain_transfer_history_blocking(
                    base_url.as_str(),
                    account_filter,
                    action_filter,
                )
                .map(transfer_window::TransferQueryResponse::History),
            ));
        });
    }

    pub(super) fn request_web_chain_transfer_status(&mut self, action_id: u64) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log("skip transfer status query: previous transfer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::TransferQuery(
                get_web_chain_transfer_status_blocking(base_url.as_str(), action_id)
                    .map(transfer_window::TransferQueryResponse::Status),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_overview(&mut self) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer overview query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_overview_blocking(base_url.as_str())
                    .map(explorer_window::ExplorerQueryResponse::Overview),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_blocks(&mut self, cursor: usize, limit: usize) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer blocks query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_blocks_blocking(base_url.as_str(), cursor, limit)
                    .map(explorer_window::ExplorerQueryResponse::Blocks),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_block(
        &mut self,
        block_height: Option<u64>,
        block_hash: Option<String>,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer block query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_block_blocking(base_url.as_str(), block_height, block_hash)
                    .map(explorer_window::ExplorerQueryResponse::Block),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_txs(
        &mut self,
        account_filter: String,
        status_filter: Option<String>,
        action_filter: String,
        cursor: usize,
        limit: usize,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer tx list query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_txs_blocking(
                    base_url.as_str(),
                    account_filter,
                    status_filter,
                    action_filter,
                    cursor,
                    limit,
                )
                .map(explorer_window::ExplorerQueryResponse::Txs),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_tx(
        &mut self,
        tx_hash: Option<String>,
        action_id: Option<u64>,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer tx query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_tx_blocking(base_url.as_str(), tx_hash, action_id)
                    .map(explorer_window::ExplorerQueryResponse::Tx),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_search(&mut self, query: String) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer search query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_search_blocking(base_url.as_str(), query)
                    .map(explorer_window::ExplorerQueryResponse::Search),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_address(
        &mut self,
        account_id: String,
        cursor: usize,
        limit: usize,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer address query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_address_blocking(
                    base_url.as_str(),
                    account_id,
                    cursor,
                    limit,
                )
                .map(explorer_window::ExplorerQueryResponse::Address),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_contracts(&mut self, cursor: usize, limit: usize) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer contracts query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_contracts_blocking(base_url.as_str(), cursor, limit)
                    .map(explorer_window::ExplorerQueryResponse::Contracts),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_contract(&mut self, contract_id: String) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer contract query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_contract_blocking(base_url.as_str(), contract_id)
                    .map(explorer_window::ExplorerQueryResponse::Contract),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_assets(
        &mut self,
        account_filter: String,
        cursor: usize,
        limit: usize,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer assets query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_assets_blocking(
                    base_url.as_str(),
                    account_filter,
                    cursor,
                    limit,
                )
                .map(explorer_window::ExplorerQueryResponse::Assets),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_mempool(
        &mut self,
        status_filter: Option<String>,
        cursor: usize,
        limit: usize,
    ) {
        self.ensure_control_plane_service();
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer mempool query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let base_url = self.control_api_base.clone();
        thread::spawn(move || {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                get_web_chain_explorer_mempool_blocking(
                    base_url.as_str(),
                    status_filter,
                    cursor,
                    limit,
                )
                .map(explorer_window::ExplorerQueryResponse::Mempool),
            ));
        });
    }

    pub(super) fn apply_web_snapshot(&mut self, snapshot: WebStateSnapshot) {
        self.status =
            launcher_status_from_web(snapshot.status.as_str(), snapshot.detail.as_deref());
        self.chain_runtime_status = chain_runtime_status_from_web(
            snapshot.chain_status.as_str(),
            snapshot.chain_detail.as_deref(),
        );
        self.chain_p2p_status = snapshot.chain_p2p_status;
        self.chain_observability_status = snapshot.chain_observability_status;
        self.chain_replication_status = snapshot.chain_replication_status;
        self.chain_recovery = snapshot.chain_recovery;
        self.control_plane_snapshot_received = true;
        self.web_game_url = Some(snapshot.game_url);
        if self.config_dirty {
            if self.config == snapshot.config {
                self.config_dirty = false;
            }
        } else {
            self.config = snapshot.config;
        }
        self.logs = snapshot.logs.into_iter().collect();
        while self.logs.len() > MAX_LOG_LINES {
            self.logs.pop_front();
        }

        if matches!(
            self.chain_runtime_status,
            ChainRuntimeStatus::Starting | ChainRuntimeStatus::Ready
        ) {
            self.chain_auto_start_attempted = true;
        }
    }
}

fn fetch_web_state_blocking(base_url: &str) -> Result<WebStateSnapshot, String> {
    http_json_request(base_url, "GET", "/api/state", None)
}

fn post_web_start_blocking(base_url: &str, config: LaunchConfig) -> Result<WebApiResponse, String> {
    let payload = serde_json::to_vec(&config)
        .map_err(|err| format!("serialize /api/start payload failed: {err}"))?;
    http_json_request(base_url, "POST", "/api/start", Some(payload.as_slice()))
}

fn post_web_stop_blocking(base_url: &str) -> Result<WebApiResponse, String> {
    http_json_request(base_url, "POST", "/api/stop", None)
}

fn post_web_chain_start_blocking(
    base_url: &str,
    config: LaunchConfig,
) -> Result<WebApiResponse, String> {
    let payload = serde_json::to_vec(&config)
        .map_err(|err| format!("serialize /api/chain/start payload failed: {err}"))?;
    http_json_request(
        base_url,
        "POST",
        "/api/chain/start",
        Some(payload.as_slice()),
    )
}

fn post_web_chain_stop_blocking(base_url: &str) -> Result<WebApiResponse, String> {
    http_json_request(base_url, "POST", "/api/chain/stop", None)
}

fn post_web_chain_transfer_blocking(
    base_url: &str,
    request: WebTransferSubmitRequest,
) -> Result<WebTransferSubmitResponse, String> {
    let payload = serde_json::to_vec(&request)
        .map_err(|err| format!("serialize /api/chain/transfer payload failed: {err}"))?;
    http_json_request(
        base_url,
        "POST",
        "/api/chain/transfer",
        Some(payload.as_slice()),
    )
}

fn get_web_chain_transfer_accounts_blocking(
    base_url: &str,
) -> Result<transfer_window::WebTransferAccountsResponse, String> {
    http_json_request(base_url, "GET", "/api/chain/transfer/accounts", None)
}

fn get_web_chain_transfer_status_blocking(
    base_url: &str,
    action_id: u64,
) -> Result<transfer_window::WebTransferStatusResponse, String> {
    let path = format!("/api/chain/transfer/status?action_id={action_id}");
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_transfer_history_blocking(
    base_url: &str,
    account_filter: String,
    action_filter: String,
) -> Result<transfer_window::WebTransferHistoryResponse, String> {
    let mut query = vec!["limit=50".to_string()];
    let account_filter = account_filter.trim();
    if !account_filter.is_empty() {
        query.push(encoded_query_pair("account_id", account_filter));
    }
    let action_filter = action_filter.trim();
    if !action_filter.is_empty() {
        query.push(encoded_query_pair("action_id", action_filter));
    }
    let path = format!("/api/chain/transfer/history?{}", query.join("&"));
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_overview_blocking(
    base_url: &str,
) -> Result<explorer_window::WebExplorerOverviewResponse, String> {
    http_json_request(base_url, "GET", "/api/chain/explorer/overview", None)
}

fn get_web_chain_explorer_blocks_blocking(
    base_url: &str,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerBlocksResponse, String> {
    let path = format!(
        "/api/chain/explorer/blocks?limit={}&cursor={cursor}",
        limit.clamp(1, 200)
    );
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_block_blocking(
    base_url: &str,
    block_height: Option<u64>,
    block_hash: Option<String>,
) -> Result<explorer_window::WebExplorerBlockResponse, String> {
    let mut query = Vec::new();
    if let Some(block_height) = block_height {
        query.push(format!("height={block_height}"));
    }
    if let Some(block_hash) = block_hash
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
    {
        query.push(encoded_query_pair("hash", block_hash.as_str()));
    }
    let path = if query.is_empty() {
        "/api/chain/explorer/block".to_string()
    } else {
        format!("/api/chain/explorer/block?{}", query.join("&"))
    };
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_txs_blocking(
    base_url: &str,
    account_filter: String,
    status_filter: Option<String>,
    action_filter: String,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerTxsResponse, String> {
    let mut query = vec![
        format!("limit={}", limit.clamp(1, 200)),
        format!("cursor={cursor}"),
    ];
    let account_filter = account_filter.trim();
    if !account_filter.is_empty() {
        query.push(encoded_query_pair("account_id", account_filter));
    }
    if let Some(status_filter) = status_filter {
        let status_filter = status_filter.trim().to_string();
        if !status_filter.is_empty() {
            query.push(encoded_query_pair("status", status_filter.as_str()));
        }
    }
    let action_filter = action_filter.trim();
    if !action_filter.is_empty() {
        query.push(encoded_query_pair("action_id", action_filter));
    }
    let path = format!("/api/chain/explorer/txs?{}", query.join("&"));
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_tx_blocking(
    base_url: &str,
    tx_hash: Option<String>,
    action_id: Option<u64>,
) -> Result<explorer_window::WebExplorerTxResponse, String> {
    let mut query = Vec::new();
    if let Some(tx_hash) = tx_hash
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
    {
        query.push(encoded_query_pair("tx_hash", tx_hash.as_str()));
    }
    if let Some(action_id) = action_id {
        query.push(format!("action_id={action_id}"));
    }
    let path = if query.is_empty() {
        "/api/chain/explorer/tx".to_string()
    } else {
        format!("/api/chain/explorer/tx?{}", query.join("&"))
    };
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_search_blocking(
    base_url: &str,
    query: String,
) -> Result<explorer_window::WebExplorerSearchResponse, String> {
    let query = query.trim().to_string();
    let path = format!(
        "/api/chain/explorer/search?{}",
        encoded_query_pair("q", query.as_str())
    );
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_address_blocking(
    base_url: &str,
    account_id: String,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerAddressResponse, String> {
    let account_id = account_id.trim().to_string();
    let path = format!(
        "/api/chain/explorer/address?{}&limit={}&cursor={cursor}",
        encoded_query_pair("account_id", account_id.as_str()),
        limit.clamp(1, 200)
    );
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_contracts_blocking(
    base_url: &str,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerContractsResponse, String> {
    let path = format!(
        "/api/chain/explorer/contracts?limit={}&cursor={cursor}",
        limit.clamp(1, 200)
    );
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_contract_blocking(
    base_url: &str,
    contract_id: String,
) -> Result<explorer_window::WebExplorerContractResponse, String> {
    let contract_id = contract_id.trim().to_string();
    let path = format!(
        "/api/chain/explorer/contract?{}",
        encoded_query_pair("contract_id", contract_id.as_str())
    );
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_assets_blocking(
    base_url: &str,
    account_filter: String,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerAssetsResponse, String> {
    let mut query = vec![
        format!("limit={}", limit.clamp(1, 200)),
        format!("cursor={cursor}"),
    ];
    let account_filter = account_filter.trim();
    if !account_filter.is_empty() {
        query.push(encoded_query_pair("account_id", account_filter));
    }
    let path = format!("/api/chain/explorer/assets?{}", query.join("&"));
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn get_web_chain_explorer_mempool_blocking(
    base_url: &str,
    status_filter: Option<String>,
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerMempoolResponse, String> {
    let mut query = vec![
        format!("limit={}", limit.clamp(1, 200)),
        format!("cursor={cursor}"),
    ];
    if let Some(status_filter) = status_filter {
        let status_filter = status_filter.trim().to_string();
        if !status_filter.is_empty() {
            query.push(encoded_query_pair("status", status_filter.as_str()));
        }
    }
    let path = format!("/api/chain/explorer/mempool?{}", query.join("&"));
    http_json_request(base_url, "GET", path.as_str(), None)
}

fn check_web_health(base_url: &str) -> Result<(), String> {
    let (status_code, _body) = http_request(base_url, "GET", "/healthz", None)?;
    if (200..=299).contains(&status_code) {
        Ok(())
    } else {
        Err(format!("GET /healthz failed with HTTP {status_code}"))
    }
}

fn http_json_request<T: DeserializeOwned>(
    base_url: &str,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<T, String> {
    let (status_code, response_body) = http_request(base_url, method, path, body)?;
    if !(200..=299).contains(&status_code) {
        let body_text = String::from_utf8_lossy(response_body.as_slice());
        return Err(format!(
            "{method} {path} failed with HTTP {status_code}: {body_text}"
        ));
    }
    serde_json::from_slice(response_body.as_slice())
        .map_err(|err| format!("decode {method} {path} response failed: {err}"))
}

fn http_request(
    base_url: &str,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, Vec<u8>), String> {
    let (host, port) = parse_http_base_url(base_url)?;
    let connect_host = normalize_host_for_connect(host.as_str());
    let socket_host = host_for_url(connect_host.as_str());
    let mut stream = TcpStream::connect(format!("{socket_host}:{port}"))
        .map_err(|err| format!("connect control plane failed: {err}"))?;

    let timeout = Some(Duration::from_millis(CONTROL_PLANE_HTTP_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let payload = body.unwrap_or(&[]);
    let host_header = host_for_url(host.as_str());
    let mut request_head = String::new();
    request_head.push_str(&format!("{method} {path} HTTP/1.1\r\n"));
    request_head.push_str(&format!("Host: {host_header}:{port}\r\n"));
    request_head.push_str("Connection: close\r\n");
    if !payload.is_empty() {
        request_head.push_str("Content-Type: application/json\r\n");
        request_head.push_str(&format!("Content-Length: {}\r\n", payload.len()));
    }
    request_head.push_str("\r\n");

    stream
        .write_all(request_head.as_bytes())
        .map_err(|err| format!("write request header failed: {err}"))?;
    if !payload.is_empty() {
        stream
            .write_all(payload)
            .map_err(|err| format!("write request body failed: {err}"))?;
    }
    stream
        .flush()
        .map_err(|err| format!("flush request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|err| format!("read response failed: {err}"))?;
    parse_http_response(response_bytes.as_slice())
}

fn parse_http_response(bytes: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let Some(boundary) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("invalid HTTP response: missing header terminator".to_string());
    };
    let header = std::str::from_utf8(&bytes[..boundary])
        .map_err(|_| "invalid HTTP response: header is not UTF-8".to_string())?;
    let body = bytes[(boundary + 4)..].to_vec();

    let Some(status_line) = header.lines().next() else {
        return Err("invalid HTTP response: missing status line".to_string());
    };
    let Some(status_code) = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
    else {
        return Err(format!("invalid HTTP response status line: {status_line}"));
    };

    Ok((status_code, body))
}

fn parse_http_base_url(base_url: &str) -> Result<(String, u16), String> {
    let mut raw = base_url.trim();
    if let Some(stripped) = raw.strip_prefix("http://") {
        raw = stripped;
    }
    raw = raw.trim_end_matches('/');
    let authority = raw
        .split('/')
        .next()
        .ok_or_else(|| format!("invalid control plane URL: {base_url}"))?
        .trim();
    if authority.is_empty() {
        return Err(format!("invalid control plane URL: {base_url}"));
    }

    if authority.starts_with('[') || authority.contains(':') {
        parse_host_port(authority, OASIS7_CLIENT_LAUNCHER_CONTROL_URL_ENV)
    } else {
        Ok((authority.to_string(), 80))
    }
}
