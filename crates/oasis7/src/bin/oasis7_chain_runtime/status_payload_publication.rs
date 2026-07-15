use std::path::Path;

use oasis7_node::NodeSnapshot;
use serde::Deserialize;

use super::super::publication_lifecycle::{
    LifecycleError, PublicationHeadBinding, PublicationLifecycleSnapshot,
    SEQUENCER_HEAD_PUBLICATION_GRACE_MS, has_complete_publication_quorum_at_height, load_snapshot,
};

use super::{
    ChainConsensusNetworkHeadStatus, ChainNodeObservabilityAlert, ChainNodeObservabilityStatus,
    ChainReadinessPolicyStatus, observability_status_for_alerts, observability_summary_for_alerts,
    push_local_chain_ahead_alert, push_observability_alert,
    sequencer_head_publication_pending_summary,
};

const MAX_PUBLICATION_EPISODE_RECORD_SCAN: usize = 255;

pub(super) fn push_publication_or_divergence_alert(
    alerts: &mut Vec<ChainNodeObservabilityAlert>,
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    policy: &ChainReadinessPolicyStatus,
    observed_at_unix_ms: i64,
) {
    if let Some(summary) = sequencer_head_publication_pending_summary(
        snapshot,
        network_head,
        policy,
        observed_at_unix_ms,
    ) {
        push_observability_alert(
            alerts,
            "warn",
            "sequencer_head_publication_pending",
            summary,
        );
        return;
    }

    let observed_network_height = network_head
        .height
        .or_else(|| network_head.peer_heads.iter().map(|peer| peer.height).max());
    push_local_chain_ahead_alert(alerts, snapshot, observed_network_height);
}

pub(super) fn enforce_retained_publication_proof(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    policy: &ChainReadinessPolicyStatus,
    execution_world_dir: &Path,
    execution_records_dir: Option<&Path>,
    observed_at_unix_ms: i64,
    observability: &mut ChainNodeObservabilityStatus,
) {
    if policy.tier != "public_testnet" || policy.role != "sequencer" {
        return;
    }
    if has_complete_publication_quorum_at_height(
        snapshot,
        network_head,
        snapshot.consensus.committed_height,
        true,
    ) {
        return;
    }

    let warning_index = observability
        .alerts
        .iter()
        .position(|alert| alert.code == "sequencer_head_publication_pending");
    if warning_index.is_none() && !is_publication_proof_candidate(snapshot, network_head, policy) {
        return;
    }

    let decision = execution_records_dir
        .ok_or_else(|| PublicationProofRejection::new(Reason::RecordMissing, "records_dir"))
        .and_then(|records_dir| {
            let durable_state = load_snapshot(execution_world_dir)
                .map_err(PublicationProofRejection::from_lifecycle_error)?;
            derive_publication_episode(
                snapshot,
                network_head,
                records_dir,
                durable_state.as_ref(),
                observed_at_unix_ms,
            )
        });
    match decision {
        Ok(proof) => {
            if let Some(index) = warning_index {
                observability.alerts[index].summary.push_str(
                    format!(
                        " episode_started_at_ms={} episode_age_ms={}",
                        proof.episode.timestamp_ms, proof.episode_age_ms
                    )
                    .as_str(),
                );
            }
        }
        Err(rejection) => reject_publication_proof(
            snapshot,
            network_head,
            warning_index,
            rejection,
            observability,
        ),
    }
}

fn derive_publication_episode(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    records_dir: &Path,
    durable_state: Option<&PublicationLifecycleSnapshot>,
    observed_at_unix_ms: i64,
) -> Result<PublicationEpisodeProof, PublicationProofRejection> {
    let local_height = snapshot.consensus.committed_height;
    let parent_height = local_height.checked_sub(1).ok_or_else(|| {
        PublicationProofRejection::new(Reason::ContinuityInvalid, "local_height_zero")
    })?;
    let local = load_record(records_dir, local_height)?;
    let parent = load_record(records_dir, parent_height)?;
    validate_record(&local, local_height, snapshot.world_id.as_str())?;
    validate_record(&parent, parent_height, snapshot.world_id.as_str())?;
    validate_edge(&local, &parent)?;

    if Some(local.timestamp_ms) != snapshot.consensus.last_committed_at_ms {
        return Err(PublicationProofRejection::new(
            Reason::TimestampMismatch,
            format!("height={local_height}"),
        ));
    }
    validate_boundary_bindings(snapshot, network_head, &local, &parent)?;

    if let Some(state) = durable_state {
        validate_state_world(state, snapshot.world_id.as_str())?;
        if let Some(episode) = state.episode.as_ref() {
            return derive_from_durable_episode(
                snapshot,
                records_dir,
                local,
                parent,
                episode,
                observed_at_unix_ms,
            );
        }
        if state
            .catch_up
            .as_ref()
            .is_some_and(|marker| catch_up_binds_parent(marker, &parent))
        {
            return validate_grace(local, observed_at_unix_ms);
        }
    }

    derive_from_retained_history(
        snapshot,
        records_dir,
        vec![local, parent],
        observed_at_unix_ms,
    )
}

fn derive_from_durable_episode(
    snapshot: &NodeSnapshot,
    records_dir: &Path,
    local: PublicationExecutionRecord,
    parent: PublicationExecutionRecord,
    episode: &PublicationHeadBinding,
    observed_at_unix_ms: i64,
) -> Result<PublicationEpisodeProof, PublicationProofRejection> {
    if episode.height > local.height {
        return Err(PublicationProofRejection::new(
            Reason::StateBindingInvalid,
            format!("episode_height={}", episode.height),
        ));
    }
    let mut records = vec![local, parent];
    while records.last().expect("nonempty records").height > episode.height {
        if records.len() == MAX_PUBLICATION_EPISODE_RECORD_SCAN {
            return Err(PublicationProofRejection::new(
                Reason::ScanLimitExceeded,
                format!("records={}", records.len() + 1),
            ));
        }
        let oldest = records.last().expect("nonempty records");
        let previous_height = oldest.height.checked_sub(1).ok_or_else(|| {
            PublicationProofRejection::new(Reason::StateBindingInvalid, "episode_before_genesis")
        })?;
        let previous = load_record(records_dir, previous_height)?;
        validate_record(&previous, previous_height, snapshot.world_id.as_str())?;
        validate_edge(oldest, &previous)?;
        records.push(previous);
    }
    let retained_episode = records
        .iter()
        .find(|record| record.height == episode.height)
        .ok_or_else(|| {
            PublicationProofRejection::new(
                Reason::StateBindingInvalid,
                format!("episode_height={}", episode.height),
            )
        })?;
    if !binding_matches_record(episode, retained_episode) {
        return Err(PublicationProofRejection::new(
            Reason::StateBindingInvalid,
            format!("episode_height={}", episode.height),
        ));
    }
    validate_grace(retained_episode.clone(), observed_at_unix_ms)
}

fn derive_from_retained_history(
    snapshot: &NodeSnapshot,
    records_dir: &Path,
    mut records: Vec<PublicationExecutionRecord>,
    observed_at_unix_ms: i64,
) -> Result<PublicationEpisodeProof, PublicationProofRejection> {
    loop {
        let episode_index = records.len() - 2;
        let episode = records[episode_index].clone();
        if observed_at_unix_ms
            .saturating_sub(episode.timestamp_ms)
            .max(0)
            > SEQUENCER_HEAD_PUBLICATION_GRACE_MS
        {
            return Err(PublicationProofRejection::new(
                Reason::GraceExpired,
                format!("episode_started_at_ms={}", episode.timestamp_ms),
            ));
        }
        if records.len() == MAX_PUBLICATION_EPISODE_RECORD_SCAN {
            let oldest_height = records.last().expect("nonempty records").height;
            if oldest_height > 0 && record_path(records_dir, oldest_height - 1).exists() {
                return Err(PublicationProofRejection::new(
                    Reason::ScanLimitExceeded,
                    format!("records={}", records.len() + 1),
                ));
            }
            return validate_grace(episode, observed_at_unix_ms);
        }

        let oldest = records.last().expect("nonempty records");
        let Some(previous_height) = oldest.height.checked_sub(1) else {
            return validate_grace(episode, observed_at_unix_ms);
        };
        let previous = load_record(records_dir, previous_height)?;
        validate_record(&previous, previous_height, snapshot.world_id.as_str())?;
        validate_edge(oldest, &previous)?;
        records.push(previous);
    }
}

fn validate_grace(
    episode: PublicationExecutionRecord,
    observed_at_unix_ms: i64,
) -> Result<PublicationEpisodeProof, PublicationProofRejection> {
    let episode_age_ms = observed_at_unix_ms
        .saturating_sub(episode.timestamp_ms)
        .max(0);
    if episode_age_ms > SEQUENCER_HEAD_PUBLICATION_GRACE_MS {
        return Err(PublicationProofRejection::new(
            Reason::GraceExpired,
            format!("episode_age_ms={episode_age_ms}"),
        ));
    }
    Ok(PublicationEpisodeProof {
        episode: binding_from_record(&episode),
        episode_age_ms,
    })
}

fn validate_record(
    record: &PublicationExecutionRecord,
    expected_height: u64,
    expected_world_id: &str,
) -> Result<(), PublicationProofRejection> {
    if record.height != expected_height || record.world_id != expected_world_id {
        return Err(PublicationProofRejection::new(
            Reason::ContinuityInvalid,
            format!("expected_height={expected_height}"),
        ));
    }
    if nonempty_hash(record.node_block_hash.as_deref()).is_none()
        || nonempty_hash(record.prev_node_block_hash.as_deref()).is_none()
    {
        return Err(PublicationProofRejection::new(
            Reason::AncestryInvalid,
            format!("height={expected_height}"),
        ));
    }
    Ok(())
}

fn validate_edge(
    child: &PublicationExecutionRecord,
    parent: &PublicationExecutionRecord,
) -> Result<(), PublicationProofRejection> {
    if child.height.checked_sub(1) != Some(parent.height) || child.world_id != parent.world_id {
        return Err(PublicationProofRejection::new(
            Reason::ContinuityInvalid,
            format!("child_height={}", child.height),
        ));
    }
    if child.prev_node_block_hash.as_deref() != parent.node_block_hash.as_deref() {
        return Err(PublicationProofRejection::new(
            Reason::AncestryInvalid,
            format!("child_height={}", child.height),
        ));
    }
    if child.timestamp_ms < parent.timestamp_ms {
        return Err(PublicationProofRejection::new(
            Reason::ChronologyInvalid,
            format!("child_height={}", child.height),
        ));
    }
    Ok(())
}

fn validate_boundary_bindings(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    local: &PublicationExecutionRecord,
    parent: &PublicationExecutionRecord,
) -> Result<(), PublicationProofRejection> {
    let valid = local.node_block_hash.as_deref() == snapshot.consensus.last_block_hash.as_deref()
        && local.execution_block_hash
            == snapshot
                .consensus
                .last_execution_block_hash
                .as_deref()
                .unwrap_or_default()
        && local.execution_state_root
            == snapshot
                .consensus
                .last_execution_state_root
                .as_deref()
                .unwrap_or_default()
        && local.prev_node_block_hash.as_deref() == network_head.block_hash.as_deref()
        && parent.node_block_hash.as_deref() == network_head.block_hash.as_deref()
        && parent.execution_block_hash
            == network_head
                .execution_block_hash
                .as_deref()
                .unwrap_or_default()
        && parent.execution_state_root
            == network_head
                .execution_state_root
                .as_deref()
                .unwrap_or_default();
    if !valid {
        return Err(PublicationProofRejection::new(
            Reason::BindingInvalid,
            format!("local_height={}", local.height),
        ));
    }
    Ok(())
}

fn is_publication_proof_candidate(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    policy: &ChainReadinessPolicyStatus,
) -> bool {
    let local_height = snapshot.consensus.committed_height;
    let Some(parent_height) = local_height.checked_sub(1) else {
        return false;
    };
    policy.tier == "public_testnet"
        && policy.role == "sequencer"
        && has_complete_publication_quorum_at_height(snapshot, network_head, parent_height, false)
}

fn validate_state_world(
    state: &PublicationLifecycleSnapshot,
    world_id: &str,
) -> Result<(), PublicationProofRejection> {
    if state.world_id != world_id {
        return Err(PublicationProofRejection::new(
            Reason::StateBindingInvalid,
            "world_mismatch",
        ));
    }
    Ok(())
}

fn catch_up_binds_parent(
    catch_up: &PublicationHeadBinding,
    parent: &PublicationExecutionRecord,
) -> bool {
    binding_matches_record(catch_up, parent)
}

fn reject_publication_proof(
    snapshot: &NodeSnapshot,
    network_head: &ChainConsensusNetworkHeadStatus,
    warning_index: Option<usize>,
    rejection: PublicationProofRejection,
    observability: &mut ChainNodeObservabilityStatus,
) {
    if let Some(index) = warning_index {
        observability.alerts.remove(index);
    }
    if !observability
        .alerts
        .iter()
        .any(|alert| alert.code == "local_chain_ahead_of_network_head")
    {
        push_local_chain_ahead_alert(&mut observability.alerts, snapshot, network_head.height);
    }
    push_observability_alert(
        &mut observability.alerts,
        "critical",
        "sequencer_head_publication_proof_rejected",
        rejection.summary(),
    );
    observability.status = observability_status_for_alerts(observability.alerts.as_slice());
    observability.summary = observability_summary_for_alerts(observability.alerts.as_slice());
    observability.ready = false;
}

fn load_record(
    records_dir: &Path,
    height: u64,
) -> Result<PublicationExecutionRecord, PublicationProofRejection> {
    let path = record_path(records_dir, height);
    let bytes = std::fs::read(&path).map_err(|error| {
        let reason = if error.kind() == std::io::ErrorKind::NotFound {
            Reason::RecordMissing
        } else {
            Reason::RecordMalformed
        };
        PublicationProofRejection::new(reason, format!("height={height}"))
    })?;
    serde_json::from_slice::<PublicationExecutionRecord>(&bytes).map_err(|_| {
        PublicationProofRejection::new(Reason::RecordMalformed, format!("height={height}"))
    })
}

fn nonempty_hash(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn record_path(records_dir: &Path, height: u64) -> std::path::PathBuf {
    records_dir.join(format!("{height:020}.json"))
}

struct PublicationEpisodeProof {
    episode: PublicationHeadBinding,
    episode_age_ms: i64,
}

struct PublicationProofRejection {
    reason: Reason,
    detail: String,
}

impl PublicationProofRejection {
    fn new(reason: Reason, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: detail.into(),
        }
    }

    fn summary(&self) -> String {
        format!(
            "sequencer publication proof rejected: reason={} detail={}",
            self.reason.as_str(),
            self.detail
        )
    }

    fn from_lifecycle_error(error: LifecycleError) -> Self {
        let reason = match error.reason {
            "state_malformed" => Reason::StateMalformed,
            _ => Reason::StateBindingInvalid,
        };
        Self::new(reason, error.detail)
    }
}

#[derive(Clone, Copy)]
enum Reason {
    RecordMissing,
    RecordMalformed,
    TimestampMismatch,
    ChronologyInvalid,
    AncestryInvalid,
    ContinuityInvalid,
    BindingInvalid,
    ScanLimitExceeded,
    GraceExpired,
    StateMalformed,
    StateBindingInvalid,
}

impl Reason {
    fn as_str(self) -> &'static str {
        match self {
            Self::RecordMissing => "record_missing",
            Self::RecordMalformed => "record_malformed",
            Self::TimestampMismatch => "timestamp_mismatch",
            Self::ChronologyInvalid => "chronology_invalid",
            Self::AncestryInvalid => "ancestry_invalid",
            Self::ContinuityInvalid => "continuity_invalid",
            Self::BindingInvalid => "binding_invalid",
            Self::ScanLimitExceeded => "scan_limit_exceeded",
            Self::GraceExpired => "grace_expired",
            Self::StateMalformed => "state_malformed",
            Self::StateBindingInvalid => "state_binding_invalid",
        }
    }
}

fn binding_from_record(record: &PublicationExecutionRecord) -> PublicationHeadBinding {
    PublicationHeadBinding {
        height: record.height,
        node_block_hash: record.node_block_hash.clone().unwrap_or_default(),
        execution_block_hash: record.execution_block_hash.clone(),
        execution_state_root: record.execution_state_root.clone(),
        timestamp_ms: record.timestamp_ms,
    }
}

fn binding_matches_record(
    binding: &PublicationHeadBinding,
    record: &PublicationExecutionRecord,
) -> bool {
    binding.height == record.height
        && Some(binding.node_block_hash.as_str()) == record.node_block_hash.as_deref()
        && binding.execution_block_hash == record.execution_block_hash
        && binding.execution_state_root == record.execution_state_root
        && binding.timestamp_ms == record.timestamp_ms
}

#[derive(Clone, Deserialize)]
struct PublicationExecutionRecord {
    world_id: String,
    height: u64,
    node_block_hash: Option<String>,
    #[serde(default)]
    prev_node_block_hash: Option<String>,
    execution_block_hash: String,
    execution_state_root: String,
    timestamp_ms: i64,
}
