use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use oasis7::consensus_action_payload::{
    decode_consensus_action_payload, ConsensusActionPayloadBody,
};
use oasis7::runtime::Action;
use oasis7_node::NodeCommittedActionBatch;
use serde::{Deserialize, Serialize};

use super::super::transfer_submit_api::{ChainTransferSubmitRequest, TransferLifecycleStatus};
use super::explorer_p0_api_support::build_tx_hash;
use super::{
    ExplorerBlockItem, ExplorerBlockResponse, ExplorerBlocksResponse, ExplorerSearchHit,
    ExplorerSearchResponse, ExplorerTxItem, ExplorerTxResponse, ExplorerTxsResponse,
    EXPLORER_ERROR_NOT_FOUND, EXPLORER_INDEX_FILE, EXPLORER_INDEX_VERSION,
    EXPLORER_MAX_SEARCH_RESULTS, EXPLORER_MAX_TRACKED_BLOCKS, EXPLORER_MAX_TRACKED_TXS,
};

pub(super) static EXPLORER_STORE: OnceLock<Mutex<ExplorerStore>> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ExplorerStoreSnapshot {
    version: u32,
    #[serde(default)]
    blocks: Vec<ExplorerBlockItem>,
    #[serde(default)]
    txs: Vec<ExplorerTxItem>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ExplorerStore {
    pub(super) persistence_path: Option<PathBuf>,
    pub(super) loaded: bool,
    pub(super) blocks_by_height: BTreeMap<u64, ExplorerBlockItem>,
    pub(super) txs_by_hash: BTreeMap<String, ExplorerTxItem>,
    pub(super) tx_hash_by_action_id: BTreeMap<u64, String>,
}

impl ExplorerStore {
    pub(super) fn configure_persistence_path(&mut self, execution_world_dir: &Path) {
        let path = execution_world_dir.join(EXPLORER_INDEX_FILE);
        if let Some(existing) = self.persistence_path.as_ref() {
            if existing == &path {
                return;
            }
            return;
        }
        self.persistence_path = Some(path.clone());
        self.loaded = false;
        self.blocks_by_height.clear();
        self.txs_by_hash.clear();
        self.tx_hash_by_action_id.clear();
    }

    pub(super) fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }

        self.blocks_by_height.clear();
        self.txs_by_hash.clear();
        self.tx_hash_by_action_id.clear();

        let Some(path) = self.persistence_path.as_ref() else {
            self.loaded = true;
            return;
        };
        if !path.exists() {
            self.loaded = true;
            return;
        }

        let Ok(bytes) = std::fs::read(path) else {
            self.loaded = true;
            return;
        };
        let Ok(snapshot) = serde_json::from_slice::<ExplorerStoreSnapshot>(bytes.as_slice()) else {
            self.loaded = true;
            return;
        };
        if snapshot.version != EXPLORER_INDEX_VERSION {
            self.loaded = true;
            return;
        }

        for block in snapshot.blocks {
            self.blocks_by_height.insert(block.height, block);
        }
        for tx in snapshot.txs {
            self.tx_hash_by_action_id
                .insert(tx.action_id, tx.tx_hash.clone());
            self.txs_by_hash.insert(tx.tx_hash.clone(), tx);
        }
        self.prune();
        self.loaded = true;
    }

    fn persist(&self) -> Result<(), String> {
        let Some(path) = self.persistence_path.as_ref() else {
            return Ok(());
        };
        let snapshot = ExplorerStoreSnapshot {
            version: EXPLORER_INDEX_VERSION,
            blocks: self
                .blocks_by_height
                .values()
                .cloned()
                .collect::<Vec<ExplorerBlockItem>>(),
            txs: self
                .txs_by_hash
                .values()
                .cloned()
                .collect::<Vec<ExplorerTxItem>>(),
        };
        let body = serde_json::to_vec_pretty(&snapshot)
            .map_err(|err| format!("encode explorer index snapshot failed: {err}"))?;
        let Some(parent) = path.parent() else {
            return Err(format!(
                "explorer index path has no parent: {}",
                path.display()
            ));
        };
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "create explorer index parent directory failed (path={}): {err}",
                parent.display()
            )
        })?;

        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(tmp_path.as_path(), body.as_slice()).map_err(|err| {
            format!(
                "write explorer index temp file failed (path={}): {err}",
                tmp_path.display()
            )
        })?;
        std::fs::rename(tmp_path.as_path(), path.as_path()).map_err(|err| {
            format!(
                "rename explorer index file failed (path={}): {err}",
                path.display()
            )
        })?;
        Ok(())
    }

    pub(super) fn record_transfer_accepted(
        &mut self,
        action_id: u64,
        request: &ChainTransferSubmitRequest,
        now_ms: i64,
    ) {
        self.ensure_loaded();
        let tx_hash = self
            .tx_hash_by_action_id
            .get(&action_id)
            .cloned()
            .unwrap_or_else(|| {
                build_tx_hash(
                    action_id,
                    request.from_account_id.as_str(),
                    request.to_account_id.as_str(),
                    request.amount,
                    request.nonce,
                )
            });
        self.tx_hash_by_action_id.insert(action_id, tx_hash.clone());
        let mut item = self
            .txs_by_hash
            .remove(tx_hash.as_str())
            .unwrap_or_else(|| ExplorerTxItem {
                tx_hash: tx_hash.clone(),
                action_id,
                from_account_id: request.from_account_id.clone(),
                to_account_id: request.to_account_id.clone(),
                amount: request.amount,
                nonce: request.nonce,
                status: TransferLifecycleStatus::Accepted,
                submitted_at_unix_ms: now_ms,
                updated_at_unix_ms: now_ms,
                block_height: None,
                block_hash: None,
                error_code: None,
                error: None,
            });
        item.status = TransferLifecycleStatus::Accepted;
        item.updated_at_unix_ms = now_ms;
        if item.submitted_at_unix_ms <= 0 {
            item.submitted_at_unix_ms = now_ms;
        }
        self.txs_by_hash.insert(tx_hash, item);
        self.prune();
        let _ = self.persist();
    }

    pub(super) fn ingest_batches(&mut self, batches: &[NodeCommittedActionBatch]) {
        self.ensure_loaded();
        if batches.is_empty() {
            return;
        }

        for batch in batches {
            let mut block_item = self
                .blocks_by_height
                .get(&batch.height)
                .cloned()
                .unwrap_or_else(|| ExplorerBlockItem {
                    height: batch.height,
                    slot: batch.slot,
                    epoch: batch.epoch,
                    block_hash: batch.block_hash.clone(),
                    action_root: batch.action_root.clone(),
                    action_count: batch.actions.len(),
                    committed_at_unix_ms: batch.committed_at_unix_ms,
                    tx_hashes: Vec::new(),
                });
            block_item.slot = batch.slot;
            block_item.epoch = batch.epoch;
            block_item.block_hash = batch.block_hash.clone();
            block_item.action_root = batch.action_root.clone();
            block_item.action_count = batch.actions.len();
            block_item.committed_at_unix_ms = batch.committed_at_unix_ms;

            let mut known_hashes = block_item
                .tx_hashes
                .iter()
                .cloned()
                .collect::<BTreeSet<String>>();
            for action in &batch.actions {
                let decoded = match decode_consensus_action_payload(action.payload_cbor.as_slice())
                {
                    Ok(decoded) => decoded,
                    Err(_) => continue,
                };
                let ConsensusActionPayloadBody::RuntimeAction {
                    action: runtime_action,
                } = decoded
                else {
                    continue;
                };
                let Action::TransferMainToken {
                    from_account_id,
                    to_account_id,
                    amount,
                    nonce,
                } = runtime_action
                else {
                    continue;
                };

                let tx_hash = self
                    .tx_hash_by_action_id
                    .get(&action.action_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        build_tx_hash(
                            action.action_id,
                            from_account_id.as_str(),
                            to_account_id.as_str(),
                            amount,
                            nonce,
                        )
                    });
                self.tx_hash_by_action_id
                    .insert(action.action_id, tx_hash.clone());
                if known_hashes.insert(tx_hash.clone()) {
                    block_item.tx_hashes.push(tx_hash.clone());
                }

                let mut tx_item = self
                    .txs_by_hash
                    .remove(tx_hash.as_str())
                    .unwrap_or_else(|| ExplorerTxItem {
                        tx_hash: tx_hash.clone(),
                        action_id: action.action_id,
                        from_account_id: from_account_id.clone(),
                        to_account_id: to_account_id.clone(),
                        amount,
                        nonce,
                        status: TransferLifecycleStatus::Confirmed,
                        submitted_at_unix_ms: batch.committed_at_unix_ms,
                        updated_at_unix_ms: batch.committed_at_unix_ms,
                        block_height: Some(batch.height),
                        block_hash: Some(batch.block_hash.clone()),
                        error_code: None,
                        error: None,
                    });
                tx_item.from_account_id = from_account_id;
                tx_item.to_account_id = to_account_id;
                tx_item.amount = amount;
                tx_item.nonce = nonce;
                tx_item.status = TransferLifecycleStatus::Confirmed;
                tx_item.updated_at_unix_ms = batch.committed_at_unix_ms;
                if tx_item.submitted_at_unix_ms <= 0 {
                    tx_item.submitted_at_unix_ms = batch.committed_at_unix_ms;
                }
                tx_item.block_height = Some(batch.height);
                tx_item.block_hash = Some(batch.block_hash.clone());
                tx_item.error_code = None;
                tx_item.error = None;
                self.txs_by_hash.insert(tx_hash, tx_item);
            }

            self.blocks_by_height.insert(batch.height, block_item);
        }

        self.prune();
        let _ = self.persist();
    }

    pub(super) fn refresh_lifecycle_by_time(&mut self, now_ms: i64) {
        self.ensure_loaded();
        const TRANSFER_PENDING_AFTER_MS: i64 = 800;
        const TRANSFER_TIMEOUT_MS: i64 = 30_000;

        for tx in self.txs_by_hash.values_mut() {
            match tx.status {
                TransferLifecycleStatus::Accepted => {
                    if now_ms.saturating_sub(tx.submitted_at_unix_ms) >= TRANSFER_TIMEOUT_MS {
                        tx.status = TransferLifecycleStatus::Timeout;
                        tx.updated_at_unix_ms = now_ms;
                    } else if now_ms.saturating_sub(tx.submitted_at_unix_ms)
                        >= TRANSFER_PENDING_AFTER_MS
                    {
                        tx.status = TransferLifecycleStatus::Pending;
                        tx.updated_at_unix_ms = now_ms;
                    }
                }
                TransferLifecycleStatus::Pending => {
                    if now_ms.saturating_sub(tx.submitted_at_unix_ms) >= TRANSFER_TIMEOUT_MS {
                        tx.status = TransferLifecycleStatus::Timeout;
                        tx.updated_at_unix_ms = now_ms;
                    }
                }
                TransferLifecycleStatus::Confirmed
                | TransferLifecycleStatus::Failed
                | TransferLifecycleStatus::Timeout => {}
            }
        }
    }

    pub(super) fn query_blocks(&self, limit: usize, cursor: usize) -> ExplorerBlocksResponse {
        let mut blocks = self
            .blocks_by_height
            .values()
            .cloned()
            .collect::<Vec<ExplorerBlockItem>>();
        blocks.sort_by(|left, right| {
            right
                .height
                .cmp(&left.height)
                .then_with(|| right.block_hash.cmp(&left.block_hash))
        });
        let page = build_page_response(blocks, limit, cursor);
        ExplorerBlocksResponse {
            ok: page.ok,
            observed_at_unix_ms: page.observed_at_unix_ms,
            limit: page.limit,
            cursor: page.cursor,
            total: page.total,
            next_cursor: page.next_cursor,
            items: page.items,
            error_code: page.error_code,
            error: page.error,
        }
    }

    pub(super) fn query_block(
        &self,
        height: Option<u64>,
        hash: Option<&str>,
    ) -> ExplorerBlockResponse {
        let block = if let Some(height) = height {
            self.blocks_by_height.get(&height).cloned()
        } else if let Some(hash) = hash {
            self.blocks_by_height
                .values()
                .find(|item| item.block_hash == hash)
                .cloned()
        } else {
            None
        };
        match block {
            Some(block) => ExplorerBlockResponse {
                ok: true,
                observed_at_unix_ms: super::super::now_unix_ms(),
                height,
                block_hash: hash.map(ToOwned::to_owned),
                block: Some(block),
                error_code: None,
                error: None,
            },
            None => ExplorerBlockResponse {
                ok: false,
                observed_at_unix_ms: super::super::now_unix_ms(),
                height,
                block_hash: hash.map(ToOwned::to_owned),
                block: None,
                error_code: Some(EXPLORER_ERROR_NOT_FOUND.to_string()),
                error: Some("block not found".to_string()),
            },
        }
    }

    pub(super) fn query_txs(
        &self,
        account_filter: Option<&str>,
        status_filter: Option<TransferLifecycleStatus>,
        action_filter: Option<u64>,
        limit: usize,
        cursor: usize,
    ) -> ExplorerTxsResponse {
        let mut txs = self
            .txs_by_hash
            .values()
            .filter(|item| {
                if let Some(action_filter) = action_filter {
                    return item.action_id == action_filter;
                }
                if let Some(account_filter) = account_filter {
                    if item.from_account_id != account_filter
                        && item.to_account_id != account_filter
                    {
                        return false;
                    }
                }
                if let Some(status_filter) = status_filter {
                    if item.status != status_filter {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect::<Vec<ExplorerTxItem>>();
        txs.sort_by(|left, right| {
            right
                .submitted_at_unix_ms
                .cmp(&left.submitted_at_unix_ms)
                .then_with(|| right.tx_hash.cmp(&left.tx_hash))
        });

        let page = build_page_response(txs, limit, cursor);
        ExplorerTxsResponse {
            ok: page.ok,
            observed_at_unix_ms: page.observed_at_unix_ms,
            account_filter: account_filter.map(ToOwned::to_owned),
            status_filter,
            action_filter,
            limit: page.limit,
            cursor: page.cursor,
            total: page.total,
            next_cursor: page.next_cursor,
            items: page.items,
            error_code: page.error_code,
            error: page.error,
        }
    }

    pub(super) fn query_tx(
        &self,
        tx_hash: Option<&str>,
        action_id: Option<u64>,
    ) -> ExplorerTxResponse {
        let resolved_tx_hash = tx_hash.map(ToOwned::to_owned).or_else(|| {
            action_id.and_then(|action_id| self.tx_hash_by_action_id.get(&action_id).cloned())
        });

        let tx = resolved_tx_hash
            .as_ref()
            .and_then(|tx_hash| self.txs_by_hash.get(tx_hash.as_str()).cloned());
        match tx {
            Some(tx) => ExplorerTxResponse {
                ok: true,
                observed_at_unix_ms: super::super::now_unix_ms(),
                tx_hash: resolved_tx_hash,
                action_id,
                tx: Some(tx),
                error_code: None,
                error: None,
            },
            None => ExplorerTxResponse {
                ok: false,
                observed_at_unix_ms: super::super::now_unix_ms(),
                tx_hash: resolved_tx_hash,
                action_id,
                tx: None,
                error_code: Some(EXPLORER_ERROR_NOT_FOUND.to_string()),
                error: Some("tx not found".to_string()),
            },
        }
    }

    pub(super) fn query_search(&self, q: &str) -> ExplorerSearchResponse {
        let query = q.trim();
        let query_lower = query.to_ascii_lowercase();
        let mut dedup = BTreeSet::<String>::new();
        let mut items = Vec::<ExplorerSearchHit>::new();

        if let Ok(height) = query.parse::<u64>() {
            if let Some(block) = self.blocks_by_height.get(&height) {
                let dedup_key = format!("block:{}", block.height);
                if dedup.insert(dedup_key) {
                    items.push(ExplorerSearchHit {
                        item_type: "block".to_string(),
                        key: block.height.to_string(),
                        summary: format!("height={} hash={}", block.height, block.block_hash),
                    });
                }
            }
        }

        if let Ok(action_id) = query.parse::<u64>() {
            if let Some(tx_hash) = self.tx_hash_by_action_id.get(&action_id) {
                if let Some(tx) = self.txs_by_hash.get(tx_hash.as_str()) {
                    let dedup_key = format!("tx:{}", tx.tx_hash);
                    if dedup.insert(dedup_key) {
                        items.push(ExplorerSearchHit {
                            item_type: "tx".to_string(),
                            key: tx.tx_hash.clone(),
                            summary: format!(
                                "action_id={} status={:?} {}->{}",
                                tx.action_id, tx.status, tx.from_account_id, tx.to_account_id
                            ),
                        });
                    }
                }
            }
        }

        for block in self.blocks_by_height.values() {
            if block.block_hash.eq_ignore_ascii_case(query)
                || block
                    .block_hash
                    .to_ascii_lowercase()
                    .contains(query_lower.as_str())
            {
                let dedup_key = format!("block:{}", block.height);
                if dedup.insert(dedup_key) {
                    items.push(ExplorerSearchHit {
                        item_type: "block".to_string(),
                        key: block.block_hash.clone(),
                        summary: format!("height={} txs={}", block.height, block.tx_hashes.len()),
                    });
                }
            }
        }

        for tx in self.txs_by_hash.values() {
            let tx_hash_lc = tx.tx_hash.to_ascii_lowercase();
            let from_lc = tx.from_account_id.to_ascii_lowercase();
            let to_lc = tx.to_account_id.to_ascii_lowercase();
            let matches = tx_hash_lc.contains(query_lower.as_str())
                || from_lc.contains(query_lower.as_str())
                || to_lc.contains(query_lower.as_str());
            if !matches {
                continue;
            }
            let dedup_key = format!("tx:{}", tx.tx_hash);
            if dedup.insert(dedup_key) {
                items.push(ExplorerSearchHit {
                    item_type: "tx".to_string(),
                    key: tx.tx_hash.clone(),
                    summary: format!(
                        "action_id={} status={:?} {}->{}",
                        tx.action_id, tx.status, tx.from_account_id, tx.to_account_id
                    ),
                });
            }
            if items.len() >= EXPLORER_MAX_SEARCH_RESULTS {
                break;
            }
        }

        items.truncate(EXPLORER_MAX_SEARCH_RESULTS);
        ExplorerSearchResponse {
            ok: true,
            observed_at_unix_ms: super::super::now_unix_ms(),
            q: query.to_string(),
            total: items.len(),
            items,
            error_code: None,
            error: None,
        }
    }

    fn prune(&mut self) {
        if self.blocks_by_height.len() > EXPLORER_MAX_TRACKED_BLOCKS {
            let overflow = self.blocks_by_height.len() - EXPLORER_MAX_TRACKED_BLOCKS;
            let remove_heights = self
                .blocks_by_height
                .keys()
                .copied()
                .take(overflow)
                .collect::<Vec<u64>>();
            for height in remove_heights {
                self.blocks_by_height.remove(&height);
            }
        }

        if self.txs_by_hash.len() > EXPLORER_MAX_TRACKED_TXS {
            let overflow = self.txs_by_hash.len() - EXPLORER_MAX_TRACKED_TXS;
            let mut order = self
                .txs_by_hash
                .values()
                .map(|item| {
                    (
                        item.submitted_at_unix_ms,
                        item.action_id,
                        item.tx_hash.clone(),
                    )
                })
                .collect::<Vec<(i64, u64, String)>>();
            order.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
            for (_, action_id, tx_hash) in order.into_iter().take(overflow) {
                self.txs_by_hash.remove(tx_hash.as_str());
                self.tx_hash_by_action_id.remove(&action_id);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct PagedResponse<T> {
    ok: bool,
    observed_at_unix_ms: i64,
    limit: usize,
    cursor: usize,
    total: usize,
    next_cursor: Option<usize>,
    items: Vec<T>,
    error_code: Option<String>,
    error: Option<String>,
}

fn build_page_response<T>(mut items: Vec<T>, limit: usize, cursor: usize) -> PagedResponse<T> {
    let total = items.len();
    let bounded_cursor = cursor.min(total);
    let drained = items
        .drain(..)
        .skip(bounded_cursor)
        .take(limit)
        .collect::<Vec<T>>();
    let next_cursor = if bounded_cursor + drained.len() < total {
        Some(bounded_cursor + drained.len())
    } else {
        None
    };
    PagedResponse {
        ok: true,
        observed_at_unix_ms: super::super::now_unix_ms(),
        limit,
        cursor: bounded_cursor,
        total,
        next_cursor,
        items: drained,
        error_code: None,
        error: None,
    }
}
