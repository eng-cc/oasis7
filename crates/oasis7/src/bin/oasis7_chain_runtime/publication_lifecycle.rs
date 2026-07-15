use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7_node::{NodeConsensusProgressObserver, NodeConsensusSnapshot, NodeRole, NodeSnapshot};
use serde::{Deserialize, Serialize};

use super::status_payload::{
    ChainConsensusNetworkHeadStatus, build_network_head_status, readiness_policy,
};

pub(super) const SEQUENCER_HEAD_PUBLICATION_GRACE_MS: i64 = 30_000;
pub(super) const PUBLICATION_LAG_STATE_FILE: &str = "sequencer-publication-lag-state.json";
const PUBLICATION_LAG_STATE_SCHEMA_VERSION: u32 = 1;
const MAX_PUBLICATION_EPISODE_RECORD_SCAN: usize = 255;

pub(super) struct PublicationLifecycleObserver {
    node_id: String,
    player_id: String,
    world_id: String,
    role: NodeRole,
    replication_enabled: bool,
    manifest: Option<LoadedNetworkTierManifest>,
    execution_world_dir: PathBuf,
    execution_records_dir: PathBuf,
}

impl PublicationLifecycleObserver {
    pub(super) fn new(
        node_id: String,
        player_id: String,
        world_id: String,
        role: NodeRole,
        replication_enabled: bool,
        manifest: Option<LoadedNetworkTierManifest>,
        execution_world_dir: PathBuf,
        execution_records_dir: PathBuf,
    ) -> Self {
        Self {
            node_id,
            player_id,
            world_id,
            role,
            replication_enabled,
            manifest,
            execution_world_dir,
            execution_records_dir,
        }
    }
}

impl NodeConsensusProgressObserver for PublicationLifecycleObserver {
    fn observe_consensus_progress(
        &mut self,
        consensus: &NodeConsensusSnapshot,
        observed_at_ms: i64,
    ) -> Result<(), String> {
        let snapshot = NodeSnapshot {
            node_id: self.node_id.clone(),
            player_id: self.player_id.clone(),
            world_id: self.world_id.clone(),
            role: self.role,
            replication_enabled: self.replication_enabled,
            running: true,
            tick_count: 0,
            last_tick_unix_ms: Some(observed_at_ms),
            consensus: consensus.clone(),
            last_error: None,
        };
        reconcile(
            &snapshot,
            self.manifest.as_ref(),
            self.execution_world_dir.as_path(),
            self.execution_records_dir.as_path(),
            observed_at_ms,
        )
        .map_err(|error| {
            format!(
                "publication lifecycle reconciliation failed: reason={} detail={}",
                error.reason, error.detail
            )
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PublicationHeadBinding {
    pub(super) height: u64,
    pub(super) node_block_hash: String,
    pub(super) execution_block_hash: String,
    pub(super) execution_state_root: String,
    pub(super) timestamp_ms: i64,
}

impl PublicationHeadBinding {
    pub(super) fn matches_record(&self, record: &PublicationExecutionRecord) -> bool {
        self.height == record.height
            && Some(self.node_block_hash.as_str()) == record.node_block_hash.as_deref()
            && self.execution_block_hash == record.execution_block_hash
            && self.execution_state_root == record.execution_state_root
            && self.timestamp_ms == record.timestamp_ms
    }

    fn from_record(record: &PublicationExecutionRecord) -> Result<Self, LifecycleError> {
        let node_block_hash = nonempty(record.node_block_hash.as_deref())
            .ok_or_else(|| LifecycleError::binding("record_block_hash"))?;
        if record.execution_block_hash.trim().is_empty()
            || record.execution_state_root.trim().is_empty()
        {
            return Err(LifecycleError::binding("record_execution_binding"));
        }
        Ok(Self {
            height: record.height,
            node_block_hash: node_block_hash.to_string(),
            execution_block_hash: record.execution_block_hash.clone(),
            execution_state_root: record.execution_state_root.clone(),
            timestamp_ms: record.timestamp_ms,
        })
    }

    fn from_snapshot(snapshot: &NodeSnapshot) -> Result<Self, LifecycleError> {
        Ok(Self {
            height: snapshot.consensus.committed_height,
            node_block_hash: nonempty(snapshot.consensus.last_block_hash.as_deref())
                .ok_or_else(|| LifecycleError::binding("snapshot_block_hash"))?
                .to_string(),
            execution_block_hash: nonempty(snapshot.consensus.last_execution_block_hash.as_deref())
                .ok_or_else(|| LifecycleError::binding("snapshot_execution_hash"))?
                .to_string(),
            execution_state_root: nonempty(snapshot.consensus.last_execution_state_root.as_deref())
                .ok_or_else(|| LifecycleError::binding("snapshot_state_root"))?
                .to_string(),
            timestamp_ms: snapshot
                .consensus
                .last_committed_at_ms
                .ok_or_else(|| LifecycleError::binding("snapshot_commit_timestamp"))?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PublicationLifecycleSnapshot {
    #[serde(default)]
    pub(super) schema_version: u32,
    #[serde(default)]
    pub(super) world_id: String,
    #[serde(default)]
    pub(super) episode: Option<PublicationHeadBinding>,
    #[serde(default)]
    pub(super) catch_up: Option<PublicationHeadBinding>,
}

impl PublicationLifecycleSnapshot {
    fn episode(world_id: &str, binding: PublicationHeadBinding) -> Self {
        Self {
            schema_version: PUBLICATION_LAG_STATE_SCHEMA_VERSION,
            world_id: world_id.to_string(),
            episode: Some(binding),
            catch_up: None,
        }
    }

    fn catch_up(world_id: &str, binding: PublicationHeadBinding) -> Self {
        Self {
            schema_version: PUBLICATION_LAG_STATE_SCHEMA_VERSION,
            world_id: world_id.to_string(),
            episode: None,
            catch_up: Some(binding),
        }
    }

    fn validate_shape(&self) -> Result<(), LifecycleError> {
        if self.schema_version != PUBLICATION_LAG_STATE_SCHEMA_VERSION
            || self.world_id.trim().is_empty()
            || self.episode.is_some() == self.catch_up.is_some()
        {
            return Err(LifecycleError::malformed("state_shape_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PublicationScope {
    EqualHead,
    OneBlockLag,
    Outside,
}

pub(super) fn classify_scope(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    tier: &str,
    role: &str,
) -> PublicationScope {
    if tier != "public_testnet" || role != "sequencer" {
        return PublicationScope::Outside;
    }
    let local_height = snapshot.consensus.committed_height;
    if exact_quorum_at_height(snapshot, network_head, local_height, true) {
        return PublicationScope::EqualHead;
    }
    if local_height
        .checked_sub(1)
        .is_some_and(|parent| exact_quorum_at_height(snapshot, network_head, parent, false))
    {
        return PublicationScope::OneBlockLag;
    }
    PublicationScope::Outside
}

pub(super) fn load_snapshot(
    execution_world_dir: &Path,
) -> Result<Option<PublicationLifecycleSnapshot>, LifecycleError> {
    let path = execution_world_dir.join(PUBLICATION_LAG_STATE_FILE);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(LifecycleError::malformed("state_read_failed")),
    };
    let state = serde_json::from_slice::<PublicationLifecycleSnapshot>(&bytes)
        .map_err(|_| LifecycleError::malformed("state_parse_failed"))?;
    state.validate_shape()?;
    Ok(Some(state))
}

pub(super) fn reconcile(
    snapshot: &NodeSnapshot,
    manifest: Option<&LoadedNetworkTierManifest>,
    execution_world_dir: &Path,
    execution_records_dir: &Path,
    observed_at_unix_ms: i64,
) -> Result<(), LifecycleError> {
    let policy = readiness_policy(snapshot, manifest);
    let network_head = build_network_head_status(snapshot, observed_at_unix_ms, manifest);
    match classify_scope(
        snapshot,
        &network_head,
        policy.tier.as_str(),
        policy.role.as_str(),
    ) {
        PublicationScope::Outside => Ok(()),
        PublicationScope::EqualHead => {
            reconcile_catch_up(snapshot, execution_world_dir, execution_records_dir)
        }
        PublicationScope::OneBlockLag => reconcile_episode(
            snapshot,
            &network_head,
            execution_world_dir,
            execution_records_dir,
        ),
    }
}

fn reconcile_catch_up(
    snapshot: &NodeSnapshot,
    execution_world_dir: &Path,
    execution_records_dir: &Path,
) -> Result<(), LifecycleError> {
    let next_binding = PublicationHeadBinding::from_snapshot(snapshot)?;
    if let Some(current) = load_snapshot(execution_world_dir)? {
        validate_world(&current, snapshot.world_id.as_str())?;
        let current_binding = current.episode.as_ref().or(current.catch_up.as_ref());
        if current_binding.is_some_and(|binding| binding.height > next_binding.height) {
            return Err(LifecycleError::binding("catch_up_height_rollback"));
        }
        if current
            .catch_up
            .as_ref()
            .is_some_and(|binding| bindings_equal(binding, &next_binding))
        {
            return Ok(());
        }
        if let Some(binding) = current_binding {
            let retained = load_record(execution_records_dir, binding.height)?;
            validate_record(&retained, binding.height, snapshot.world_id.as_str())?;
            if !binding.matches_record(&retained)
                || (binding.height == next_binding.height
                    && !bindings_equal(binding, &next_binding))
            {
                return Err(LifecycleError::binding("catch_up_binding_mismatch"));
            }
        }
    }
    save_snapshot(
        execution_world_dir,
        &PublicationLifecycleSnapshot::catch_up(snapshot.world_id.as_str(), next_binding),
    )
}

fn reconcile_episode(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    execution_world_dir: &Path,
    execution_records_dir: &Path,
) -> Result<(), LifecycleError> {
    let local_height = snapshot.consensus.committed_height;
    let parent_height = local_height
        .checked_sub(1)
        .ok_or_else(|| LifecycleError::binding("local_height_zero"))?;
    let local = load_record(execution_records_dir, local_height)?;
    let parent = load_record(execution_records_dir, parent_height)?;
    validate_record(&local, local_height, snapshot.world_id.as_str())?;
    validate_record(&parent, parent_height, snapshot.world_id.as_str())?;
    validate_edge(&local, &parent)?;
    if Some(local.timestamp_ms) != snapshot.consensus.last_committed_at_ms {
        return Err(LifecycleError::binding("local_timestamp_mismatch"));
    }
    validate_boundary_bindings(snapshot, network_head, &local, &parent)?;

    let current = load_snapshot(execution_world_dir)?;
    let episode = match current.as_ref() {
        Some(state) => {
            validate_world(state, snapshot.world_id.as_str())?;
            if let Some(episode) = state.episode.as_ref() {
                if episode.height > local_height {
                    return Err(LifecycleError::binding("episode_height_rollback"));
                }
                let retained = load_record(execution_records_dir, episode.height)?;
                if !episode.matches_record(&retained) {
                    return Err(LifecycleError::binding("episode_binding_mismatch"));
                }
                validate_contiguous_binding(
                    snapshot.world_id.as_str(),
                    execution_records_dir,
                    &local,
                    episode,
                )?;
                return Ok(());
            }
            let catch_up = state
                .catch_up
                .as_ref()
                .ok_or_else(|| LifecycleError::malformed("state_phase_missing"))?;
            if !catch_up.matches_record(&parent) {
                return Err(LifecycleError::binding("catch_up_parent_mismatch"));
            }
            PublicationHeadBinding::from_record(&local)?
        }
        None => derive_retained_episode(
            snapshot.world_id.as_str(),
            execution_records_dir,
            local,
            parent,
        )?,
    };
    save_snapshot(
        execution_world_dir,
        &PublicationLifecycleSnapshot::episode(snapshot.world_id.as_str(), episode),
    )
}

fn derive_retained_episode(
    world_id: &str,
    records_dir: &Path,
    local: PublicationExecutionRecord,
    parent: PublicationExecutionRecord,
) -> Result<PublicationHeadBinding, LifecycleError> {
    let mut records = vec![local, parent];
    loop {
        if records.len() == MAX_PUBLICATION_EPISODE_RECORD_SCAN {
            let oldest_height = records.last().expect("nonempty records").height;
            if oldest_height > 0 && record_path(records_dir, oldest_height - 1).exists() {
                return Err(LifecycleError::binding("scan_limit_exceeded"));
            }
            return PublicationHeadBinding::from_record(&records[records.len() - 2]);
        }
        let oldest = records.last().expect("nonempty records");
        let Some(previous_height) = oldest.height.checked_sub(1) else {
            return PublicationHeadBinding::from_record(&records[records.len() - 2]);
        };
        let previous = load_record(records_dir, previous_height)?;
        validate_record(&previous, previous_height, world_id)?;
        validate_edge(oldest, &previous)?;
        records.push(previous);
    }
}

fn exact_quorum_at_height(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    expected_height: u64,
    bind_local: bool,
) -> bool {
    let local_height = snapshot.consensus.committed_height;
    let complete_local_boundary = snapshot.consensus.latest_height == local_height
        && snapshot.consensus.network_committed_height == local_height
        && snapshot.consensus.replication_persisted_height == local_height
        && snapshot.consensus.last_execution_height == local_height;
    let complete_bindings = [
        snapshot.consensus.last_block_hash.as_deref(),
        snapshot.consensus.last_execution_block_hash.as_deref(),
        snapshot.consensus.last_execution_state_root.as_deref(),
        network_head.block_hash.as_deref(),
        network_head.execution_block_hash.as_deref(),
        network_head.execution_state_root.as_deref(),
    ]
    .into_iter()
    .all(|value| nonempty(value).is_some());
    let every_fresh_peer_binds = network_head
        .peer_heads
        .iter()
        .filter(|peer| peer.fresh)
        .all(|peer| {
            peer.height == expected_height
                && Some(peer.block_hash.as_str()) == network_head.block_hash.as_deref()
                && peer.execution_block_hash == network_head.execution_block_hash
                && peer.execution_state_root == network_head.execution_state_root
        });
    let equal_local_binding = !bind_local
        || (network_head.block_hash == snapshot.consensus.last_block_hash
            && network_head.execution_block_hash == snapshot.consensus.last_execution_block_hash
            && network_head.execution_state_root == snapshot.consensus.last_execution_state_root);
    complete_local_boundary
        && complete_bindings
        && network_head.source == "peer_quorum"
        && network_head.decision == "ready"
        && network_head.height == Some(expected_height)
        && network_head.conflicting_peer_count == 0
        && network_head.stake_quorum_met
        && (snapshot.consensus.required_stake == 0
            || network_head.observed_stake >= snapshot.consensus.required_stake)
        && network_head.required_peer_count > 0
        && network_head.fresh_peer_count >= network_head.required_peer_count
        && every_fresh_peer_binds
        && equal_local_binding
}

fn validate_boundary_bindings(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    local: &PublicationExecutionRecord,
    parent: &PublicationExecutionRecord,
) -> Result<(), LifecycleError> {
    let valid = local.node_block_hash == snapshot.consensus.last_block_hash
        && Some(local.execution_block_hash.as_str())
            == snapshot.consensus.last_execution_block_hash.as_deref()
        && Some(local.execution_state_root.as_str())
            == snapshot.consensus.last_execution_state_root.as_deref()
        && local.prev_node_block_hash == network_head.block_hash
        && parent.node_block_hash == network_head.block_hash
        && Some(parent.execution_block_hash.as_str())
            == network_head.execution_block_hash.as_deref()
        && Some(parent.execution_state_root.as_str())
            == network_head.execution_state_root.as_deref();
    if !valid {
        return Err(LifecycleError::binding("boundary_binding_mismatch"));
    }
    Ok(())
}

fn validate_contiguous_binding(
    world_id: &str,
    records_dir: &Path,
    local: &PublicationExecutionRecord,
    episode: &PublicationHeadBinding,
) -> Result<(), LifecycleError> {
    let mut child = local.clone();
    let mut scanned = 1_usize;
    while child.height > episode.height {
        if scanned == MAX_PUBLICATION_EPISODE_RECORD_SCAN {
            return Err(LifecycleError::binding("scan_limit_exceeded"));
        }
        let height = child
            .height
            .checked_sub(1)
            .ok_or_else(|| LifecycleError::binding("episode_before_genesis"))?;
        let parent = load_record(records_dir, height)?;
        validate_record(&parent, height, world_id)?;
        validate_edge(&child, &parent)?;
        child = parent;
        scanned += 1;
    }
    if !episode.matches_record(&child) {
        return Err(LifecycleError::binding("episode_binding_mismatch"));
    }
    Ok(())
}

fn save_snapshot(
    execution_world_dir: &Path,
    snapshot: &PublicationLifecycleSnapshot,
) -> Result<(), LifecycleError> {
    fs::create_dir_all(execution_world_dir)
        .map_err(|_| LifecycleError::persist("state_dir_create_failed"))?;
    let bytes = serde_json::to_vec_pretty(snapshot)
        .map_err(|_| LifecycleError::persist("state_serialize_failed"))?;
    let target = execution_world_dir.join(PUBLICATION_LAG_STATE_FILE);
    let (temp_path, mut temp_file) = create_unique_temp(execution_world_dir)?;
    let result = (|| {
        temp_file
            .write_all(bytes.as_slice())
            .map_err(|_| LifecycleError::persist("state_write_failed"))?;
        temp_file
            .sync_all()
            .map_err(|_| LifecycleError::persist("state_fsync_failed"))?;
        drop(temp_file);
        replace_file(temp_path.as_path(), target.as_path())?;
        sync_parent_dir(execution_world_dir)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp_path);
    }
    result
}

fn create_unique_temp(parent: &Path) -> Result<(PathBuf, File), LifecycleError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..16_u8 {
        let path = parent.join(format!(
            ".{PUBLICATION_LAG_STATE_FILE}.{}.{}.tmp",
            std::process::id(),
            nonce + u128::from(attempt)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(LifecycleError::persist("state_temp_create_failed")),
        }
    }
    Err(LifecycleError::persist("state_temp_collision"))
}

#[cfg(unix)]
fn replace_file(temp: &Path, target: &Path) -> Result<(), LifecycleError> {
    fs::rename(temp, target).map_err(|_| LifecycleError::persist("state_replace_failed"))
}

#[cfg(not(unix))]
fn replace_file(temp: &Path, target: &Path) -> Result<(), LifecycleError> {
    if target.exists() {
        fs::remove_file(target).map_err(|_| LifecycleError::persist("state_remove_failed"))?;
    }
    fs::rename(temp, target).map_err(|_| LifecycleError::persist("state_replace_failed"))
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Path) -> Result<(), LifecycleError> {
    File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|_| LifecycleError::persist("state_parent_fsync_failed"))
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Path) -> Result<(), LifecycleError> {
    Ok(())
}

fn validate_world(
    state: &PublicationLifecycleSnapshot,
    world_id: &str,
) -> Result<(), LifecycleError> {
    if state.world_id != world_id {
        return Err(LifecycleError::binding("world_mismatch"));
    }
    Ok(())
}

fn bindings_equal(left: &PublicationHeadBinding, right: &PublicationHeadBinding) -> bool {
    left.height == right.height
        && left.node_block_hash == right.node_block_hash
        && left.execution_block_hash == right.execution_block_hash
        && left.execution_state_root == right.execution_state_root
        && left.timestamp_ms == right.timestamp_ms
}

fn validate_record(
    record: &PublicationExecutionRecord,
    expected_height: u64,
    world_id: &str,
) -> Result<(), LifecycleError> {
    if record.height != expected_height || record.world_id != world_id {
        return Err(LifecycleError::binding("record_continuity_invalid"));
    }
    if nonempty(record.node_block_hash.as_deref()).is_none()
        || nonempty(record.prev_node_block_hash.as_deref()).is_none()
    {
        return Err(LifecycleError::binding("record_ancestry_invalid"));
    }
    Ok(())
}

fn validate_edge(
    child: &PublicationExecutionRecord,
    parent: &PublicationExecutionRecord,
) -> Result<(), LifecycleError> {
    if child.height.checked_sub(1) != Some(parent.height)
        || child.world_id != parent.world_id
        || child.prev_node_block_hash.as_deref() != parent.node_block_hash.as_deref()
        || child.timestamp_ms < parent.timestamp_ms
    {
        return Err(LifecycleError::binding("record_edge_invalid"));
    }
    Ok(())
}

fn load_record(
    records_dir: &Path,
    height: u64,
) -> Result<PublicationExecutionRecord, LifecycleError> {
    let bytes = fs::read(record_path(records_dir, height)).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            LifecycleError::binding("record_missing")
        } else {
            LifecycleError::malformed("record_read_failed")
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|_| LifecycleError::malformed("record_parse_failed"))
}

fn record_path(records_dir: &Path, height: u64) -> PathBuf {
    records_dir.join(format!("{height:020}.json"))
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

#[derive(Clone, Debug)]
pub(super) struct LifecycleError {
    pub(super) reason: &'static str,
    pub(super) detail: &'static str,
}

impl LifecycleError {
    fn malformed(detail: &'static str) -> Self {
        Self {
            reason: "state_malformed",
            detail,
        }
    }

    fn binding(detail: &'static str) -> Self {
        Self {
            reason: "state_binding_invalid",
            detail,
        }
    }

    fn persist(detail: &'static str) -> Self {
        Self {
            reason: "state_persist_failed",
            detail,
        }
    }
}

#[derive(Clone, Deserialize)]
pub(super) struct PublicationExecutionRecord {
    world_id: String,
    height: u64,
    node_block_hash: Option<String>,
    #[serde(default)]
    prev_node_block_hash: Option<String>,
    execution_block_hash: String,
    execution_state_root: String,
    timestamp_ms: i64,
}
