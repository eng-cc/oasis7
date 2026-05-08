use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ObservedChainTransfer {
    pub(super) tx_hash: String,
    pub(super) action_id: u64,
    pub(super) from_account_id: String,
    pub(super) to_account_id: String,
    pub(super) amount: u64,
    pub(super) submitted_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
    pub(super) block_height: Option<u64>,
}

#[derive(Debug, Clone)]
pub(super) struct ChainExplorerClient {
    base_url: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ChainExplorerOverviewResponse {
    ok: bool,
    committed_height: u64,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChainExplorerTxItem {
    tx_hash: String,
    action_id: u64,
    from_account_id: String,
    to_account_id: String,
    amount: u64,
    submitted_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    #[serde(default)]
    block_height: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ChainExplorerTxsResponse {
    ok: bool,
    #[serde(default)]
    items: Vec<ChainExplorerTxItem>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

impl ChainExplorerClient {
    pub(super) fn new(base_url: &str, timeout_ms: u64) -> Result<Self, String> {
        let base_url = normalize_base_url(base_url)?;
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1)))
            .build()
            .map_err(|err| format!("build chain explorer client failed: {err}"))?;
        Ok(Self { base_url, client })
    }

    pub(super) fn fetch_committed_height(&self) -> Result<u64, String> {
        let url = format!("{}/v1/chain/explorer/overview", self.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|err| format!("chain explorer overview request failed: {err}"))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|err| format!("read chain explorer overview body failed: {err}"))?;
        if !status.is_success() {
            return Err(format!(
                "chain explorer overview returned status {status}: {body}"
            ));
        }
        let payload: ChainExplorerOverviewResponse = serde_json::from_str(body.as_str())
            .map_err(|err| format!("decode chain explorer overview failed: {err}; body={body}"))?;
        if !payload.ok {
            return Err(format!(
                "chain explorer overview not ok{}: {}",
                format_error_code(payload.error_code.as_deref()),
                payload.error.unwrap_or_else(|| "unknown error".to_string())
            ));
        }
        Ok(payload.committed_height)
    }

    pub(super) fn fetch_confirmed_account_txs(
        &self,
        account_id: &str,
    ) -> Result<Vec<ObservedChainTransfer>, String> {
        let url = format!("{}/v1/chain/explorer/txs", self.base_url);
        let response = self
            .client
            .get(url)
            .query(&[
                ("account_id", account_id),
                ("status", "confirmed"),
                ("limit", "200"),
            ])
            .send()
            .map_err(|err| format!("chain explorer tx query failed: {err}"))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|err| format!("read chain explorer tx body failed: {err}"))?;
        if !status.is_success() {
            return Err(format!(
                "chain explorer tx query returned status {status}: {body}"
            ));
        }
        let payload: ChainExplorerTxsResponse = serde_json::from_str(body.as_str())
            .map_err(|err| format!("decode chain explorer tx query failed: {err}; body={body}"))?;
        if !payload.ok {
            return Err(format!(
                "chain explorer tx query not ok{}: {}",
                format_error_code(payload.error_code.as_deref()),
                payload.error.unwrap_or_else(|| "unknown error".to_string())
            ));
        }
        Ok(payload
            .items
            .into_iter()
            .map(|item| ObservedChainTransfer {
                tx_hash: item.tx_hash,
                action_id: item.action_id,
                from_account_id: item.from_account_id,
                to_account_id: item.to_account_id,
                amount: item.amount,
                submitted_at_unix_ms: item.submitted_at_unix_ms,
                updated_at_unix_ms: item.updated_at_unix_ms,
                block_height: item.block_height,
            })
            .collect())
    }
}

fn normalize_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("chain explorer base URL must not be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn format_error_code(code: Option<&str>) -> String {
    code.map(|code| format!(" ({code})")).unwrap_or_default()
}
