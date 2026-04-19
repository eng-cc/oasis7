use super::*;

impl ClientLauncherApp {
    pub(super) fn current_game_url(&self) -> String {
        self.web_game_url
            .clone()
            .unwrap_or_else(|| build_game_url(&self.config))
    }

    pub(super) fn is_feedback_available(&self) -> bool {
        matches!(self.chain_runtime_status, ChainRuntimeStatus::Ready)
    }

    pub(super) fn maybe_auto_start_chain(&mut self) {
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
                WebApiEvent::Feedback(result) => {
                    self.set_web_request_inflight(WebRequestDomain::FeedbackSubmit, false);
                    self.apply_web_feedback_submit_result(result);
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
        if self.web_request_inflight_for(WebRequestDomain::ControlAction) {
            self.append_log("skip chain start: previous web request still in flight".to_string());
            return;
        }
        self.request_web_chain_start();
    }

    pub(super) fn request_web_chain_transfer(&mut self, request: WebTransferSubmitRequest) {
        if self.web_request_inflight_for(WebRequestDomain::TransferSubmit) {
            self.append_log("skip transfer submit: previous transfer submit still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferSubmit, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Transfer(
                post_web_chain_transfer(request).await,
            ));
        });
    }

    pub(super) fn request_web_chain_transfer_accounts(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log(
                "skip transfer accounts query: previous transfer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::TransferQuery(
                fetch_web_transfer_accounts()
                    .await
                    .map(transfer_window::TransferQueryResponse::Accounts),
            ));
        });
    }

    pub(super) fn request_web_chain_transfer_history(
        &mut self,
        account_filter: String,
        action_filter: String,
    ) {
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log("skip transfer history query: previous transfer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::TransferQuery(
                fetch_web_transfer_history(account_filter, action_filter)
                    .await
                    .map(transfer_window::TransferQueryResponse::History),
            ));
        });
    }

    pub(super) fn request_web_chain_transfer_status(&mut self, action_id: u64) {
        if self.web_request_inflight_for(WebRequestDomain::TransferQuery) {
            self.append_log("skip transfer status query: previous transfer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::TransferQuery(
                fetch_web_transfer_status(action_id)
                    .await
                    .map(transfer_window::TransferQueryResponse::Status),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_overview(&mut self) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer overview query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_overview()
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Overview),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_blocks(&mut self, cursor: usize, limit: usize) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer blocks query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_blocks(cursor, limit)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Blocks),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_block(
        &mut self,
        block_height: Option<u64>,
        block_hash: Option<String>,
    ) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer block query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_block(block_height, block_hash)
                    .await
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
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer tx list query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_txs(account_filter, status_filter, action_filter, cursor, limit)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Txs),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_tx(
        &mut self,
        tx_hash: Option<String>,
        action_id: Option<u64>,
    ) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer tx query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_tx(tx_hash, action_id)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Tx),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_search(&mut self, query: String) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer search query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_search(query)
                    .await
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
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer address query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_address(account_id, cursor, limit)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Address),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_contracts(&mut self, cursor: usize, limit: usize) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer contracts query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_contracts(cursor, limit)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Contracts),
            ));
        });
    }

    pub(super) fn request_web_chain_explorer_contract(&mut self, contract_id: String) {
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log(
                "skip explorer contract query: previous explorer query still in flight",
            );
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_contract(contract_id)
                    .await
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
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer assets query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_assets(account_filter, cursor, limit)
                    .await
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
        if self.web_request_inflight_for(WebRequestDomain::ExplorerQuery) {
            self.append_log("skip explorer mempool query: previous explorer query still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::ExplorerQuery, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::ExplorerQuery(
                fetch_web_explorer_mempool(status_filter, cursor, limit)
                    .await
                    .map(explorer_window::ExplorerQueryResponse::Mempool),
            ));
        });
    }

    pub(super) fn request_web_chain_feedback(&mut self, request: WebFeedbackSubmitRequest) {
        if self.web_request_inflight_for(WebRequestDomain::FeedbackSubmit) {
            self.append_log("skip feedback submit: previous feedback submit still in flight");
            return;
        }
        self.set_web_request_inflight(WebRequestDomain::FeedbackSubmit, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Feedback(
                post_web_chain_feedback(request).await,
            ));
        });
    }

    fn request_web_state(&mut self) {
        self.set_web_request_inflight(WebRequestDomain::StatePoll, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::State(fetch_web_state().await));
        });
    }

    fn request_web_start(&mut self) {
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let config = self.config.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Action(post_web_start(config).await));
        });
    }

    fn request_web_stop(&mut self) {
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Action(post_web_stop().await));
        });
    }

    fn request_web_chain_start(&mut self) {
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        let config = self.config.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Action(post_web_chain_start(config).await));
        });
    }

    fn request_web_chain_stop(&mut self) {
        self.set_web_request_inflight(WebRequestDomain::ControlAction, true);
        self.last_web_poll_at = Some(Instant::now());
        let tx = self.web_api_tx.clone();
        spawn_local(async move {
            let _ = tx.send(WebApiEvent::Action(post_web_chain_stop().await));
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

async fn fetch_web_state() -> Result<WebStateSnapshot, String> {
    let response = Request::get("/api/state")
        .send()
        .await
        .map_err(|err| format!("GET /api/state failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/state failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebStateSnapshot>()
        .await
        .map_err(|err| format!("decode /api/state response failed: {err}"))
}

async fn post_web_start(config: LaunchConfig) -> Result<WebApiResponse, String> {
    let payload = serde_json::to_string(&config)
        .map_err(|err| format!("serialize /api/start payload failed: {err}"))?;
    let request = Request::post("/api/start")
        .header("content-type", "application/json")
        .body(payload)
        .map_err(|err| format!("build /api/start request failed: {err}"))?;
    let response = request
        .send()
        .await
        .map_err(|err| format!("POST /api/start failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/start failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebApiResponse>()
        .await
        .map_err(|err| format!("decode /api/start response failed: {err}"))
}

async fn post_web_stop() -> Result<WebApiResponse, String> {
    let response = Request::post("/api/stop")
        .send()
        .await
        .map_err(|err| format!("POST /api/stop failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/stop failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebApiResponse>()
        .await
        .map_err(|err| format!("decode /api/stop response failed: {err}"))
}

async fn post_web_chain_start(config: LaunchConfig) -> Result<WebApiResponse, String> {
    let payload = serde_json::to_string(&config)
        .map_err(|err| format!("serialize /api/chain/start payload failed: {err}"))?;
    let request = Request::post("/api/chain/start")
        .header("content-type", "application/json")
        .body(payload)
        .map_err(|err| format!("build /api/chain/start request failed: {err}"))?;
    let response = request
        .send()
        .await
        .map_err(|err| format!("POST /api/chain/start failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/chain/start failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebApiResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/start response failed: {err}"))
}

async fn post_web_chain_stop() -> Result<WebApiResponse, String> {
    let response = Request::post("/api/chain/stop")
        .send()
        .await
        .map_err(|err| format!("POST /api/chain/stop failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/chain/stop failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebApiResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/stop response failed: {err}"))
}

async fn post_web_chain_transfer(
    request_payload: WebTransferSubmitRequest,
) -> Result<WebTransferSubmitResponse, String> {
    let payload = serde_json::to_string(&request_payload)
        .map_err(|err| format!("serialize /api/chain/transfer payload failed: {err}"))?;
    let request = Request::post("/api/chain/transfer")
        .header("content-type", "application/json")
        .body(payload)
        .map_err(|err| format!("build /api/chain/transfer request failed: {err}"))?;
    let response = request
        .send()
        .await
        .map_err(|err| format!("POST /api/chain/transfer failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/chain/transfer failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebTransferSubmitResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/transfer response failed: {err}"))
}

async fn fetch_web_transfer_accounts(
) -> Result<transfer_window::WebTransferAccountsResponse, String> {
    let response = Request::get("/api/chain/transfer/accounts")
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/transfer/accounts failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/transfer/accounts failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<transfer_window::WebTransferAccountsResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/transfer/accounts response failed: {err}"))
}

async fn fetch_web_transfer_status(
    action_id: u64,
) -> Result<transfer_window::WebTransferStatusResponse, String> {
    let path = format!("/api/chain/transfer/status?action_id={action_id}");
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/transfer/status failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/transfer/status failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<transfer_window::WebTransferStatusResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/transfer/status response failed: {err}"))
}

async fn fetch_web_transfer_history(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/transfer/history failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/transfer/history failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<transfer_window::WebTransferHistoryResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/transfer/history response failed: {err}"))
}

async fn fetch_web_explorer_overview(
) -> Result<explorer_window::WebExplorerOverviewResponse, String> {
    let response = Request::get("/api/chain/explorer/overview")
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/overview failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/overview failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerOverviewResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/overview response failed: {err}"))
}

async fn fetch_web_explorer_blocks(
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerBlocksResponse, String> {
    let path = format!(
        "/api/chain/explorer/blocks?limit={}&cursor={cursor}",
        limit.clamp(1, 200)
    );
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/blocks failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/blocks failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerBlocksResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/blocks response failed: {err}"))
}

async fn fetch_web_explorer_block(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/block failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/block failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerBlockResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/block response failed: {err}"))
}

async fn fetch_web_explorer_txs(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/txs failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/txs failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerTxsResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/txs response failed: {err}"))
}

async fn fetch_web_explorer_tx(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/tx failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/tx failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerTxResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/tx response failed: {err}"))
}

async fn fetch_web_explorer_search(
    query: String,
) -> Result<explorer_window::WebExplorerSearchResponse, String> {
    let path = format!(
        "/api/chain/explorer/search?{}",
        encoded_query_pair("q", query.trim())
    );
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/search failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/search failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerSearchResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/search response failed: {err}"))
}

async fn fetch_web_explorer_address(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/address failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/address failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerAddressResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/address response failed: {err}"))
}

async fn fetch_web_explorer_contracts(
    cursor: usize,
    limit: usize,
) -> Result<explorer_window::WebExplorerContractsResponse, String> {
    let path = format!(
        "/api/chain/explorer/contracts?limit={}&cursor={cursor}",
        limit.clamp(1, 200)
    );
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/contracts failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/contracts failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerContractsResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/contracts response failed: {err}"))
}

async fn fetch_web_explorer_contract(
    contract_id: String,
) -> Result<explorer_window::WebExplorerContractResponse, String> {
    let contract_id = contract_id.trim().to_string();
    let path = format!(
        "/api/chain/explorer/contract?{}",
        encoded_query_pair("contract_id", contract_id.as_str())
    );
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/contract failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/contract failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerContractResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/contract response failed: {err}"))
}

async fn fetch_web_explorer_assets(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/assets failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/assets failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerAssetsResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/assets response failed: {err}"))
}

async fn fetch_web_explorer_mempool(
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
    let response = Request::get(path.as_str())
        .send()
        .await
        .map_err(|err| format!("GET /api/chain/explorer/mempool failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "GET /api/chain/explorer/mempool failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<explorer_window::WebExplorerMempoolResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/explorer/mempool response failed: {err}"))
}

async fn post_web_chain_feedback(
    request_payload: WebFeedbackSubmitRequest,
) -> Result<WebFeedbackSubmitResponse, String> {
    let payload = serde_json::to_string(&request_payload)
        .map_err(|err| format!("serialize /api/chain/feedback payload failed: {err}"))?;
    let request = Request::post("/api/chain/feedback")
        .header("content-type", "application/json")
        .body(payload)
        .map_err(|err| format!("build /api/chain/feedback request failed: {err}"))?;
    let response = request
        .send()
        .await
        .map_err(|err| format!("POST /api/chain/feedback failed: {err}"))?;
    if !response.ok() {
        return Err(format!(
            "POST /api/chain/feedback failed with HTTP {}",
            response.status()
        ));
    }
    response
        .json::<WebFeedbackSubmitResponse>()
        .await
        .map_err(|err| format!("decode /api/chain/feedback response failed: {err}"))
}
