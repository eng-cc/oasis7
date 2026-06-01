use oasis7_node::NodeSnapshot;

pub(super) fn consensus_participation_hold_reason(
    snapshot: &NodeSnapshot,
    network_height_lag: u64,
    replication_state_gap: u64,
    max_network_height_lag: u64,
) -> Option<String> {
    if !snapshot.replication_enabled {
        return None;
    }
    if let Some(height) = snapshot.consensus.replication_gap_sync_blocked_height {
        return Some(format!("replication_gap_sync_blocked_height={height}"));
    }
    if replication_state_gap > 0 {
        return Some(format!("replication_state_gap={replication_state_gap}"));
    }
    if network_height_lag > max_network_height_lag {
        return Some(format!(
            "network_height_lag={network_height_lag} allowed={}",
            max_network_height_lag
        ));
    }
    None
}

pub(super) fn state_sync_trusted_checkpoint_required_height(
    snapshot: &NodeSnapshot,
    replication_state_gap: u64,
    network_height_lag: u64,
) -> Option<u64> {
    if !snapshot.replication_enabled {
        return None;
    }
    let mut required_height = snapshot.consensus.replication_gap_sync_blocked_height;
    if replication_state_gap > 0 {
        required_height = Some(
            required_height
                .unwrap_or(0)
                .max(snapshot.consensus.committed_height),
        );
    }
    if network_height_lag > 0 {
        required_height = Some(
            required_height
                .unwrap_or(0)
                .max(snapshot.consensus.network_committed_height),
        );
    }
    required_height
}

pub(super) fn state_sync_fallback_reason(
    snapshot: &NodeSnapshot,
    replication_state_gap: u64,
    network_height_lag: u64,
) -> Option<String> {
    let required_height = state_sync_trusted_checkpoint_required_height(
        snapshot,
        replication_state_gap,
        network_height_lag,
    )?;
    let mut reasons = Vec::new();
    if let Some(height) = snapshot.consensus.replication_gap_sync_blocked_height {
        reasons.push(format!("replication_gap_sync_blocked_height={height}"));
    }
    if replication_state_gap > 0 {
        reasons.push(format!("replication_state_gap={replication_state_gap}"));
    }
    if network_height_lag > 0 {
        reasons.push(format!("network_height_lag={network_height_lag}"));
    }
    Some(format!(
        "trusted checkpoint and verified snapshot/state-sync required at height >= {required_height}: {}",
        reasons.join(",")
    ))
}
