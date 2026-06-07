use std::collections::BTreeSet;

use oasis7_node::NodeSnapshot;

use super::{
    ChainLivenessStatus, ChainNodeObservabilityAlert, ChainP2pStatus, ChainReadinessPolicyStatus,
    ChainReplicationTransportStability, TRANSPORT_STABILITY_MIN_SCORE,
};

pub(crate) fn push_observability_alert(
    alerts: &mut Vec<ChainNodeObservabilityAlert>,
    severity: &str,
    code: &str,
    summary: String,
) {
    alerts.push(ChainNodeObservabilityAlert {
        severity: severity.to_string(),
        code: code.to_string(),
        summary,
    });
}

pub(crate) fn observability_status_for_alerts(alerts: &[ChainNodeObservabilityAlert]) -> String {
    if alerts.iter().any(|alert| alert.severity == "critical") {
        "critical".to_string()
    } else if alerts.iter().any(|alert| alert.severity == "warn") {
        "warn".to_string()
    } else {
        "ok".to_string()
    }
}

pub(crate) fn observability_summary_for_alerts(alerts: &[ChainNodeObservabilityAlert]) -> String {
    match alerts {
        [] => "no active node alerts".to_string(),
        [only] => only.summary.clone(),
        _ => format!("{}; +{} more alerts", alerts[0].summary, alerts.len() - 1),
    }
}

pub(crate) fn classify_transport_stability(
    replication: &super::super::ChainReplicationDebugStatus,
) -> ChainReplicationTransportStability {
    let mut connection_closed_count = 0usize;
    let mut insufficient_peers_count = 0usize;
    let mut timeout_count = 0usize;
    let mut protocol_error_count = 0usize;
    for error in &replication.recent_errors {
        if !replication_error_is_blocking(replication, error) {
            continue;
        }
        let lower = error.to_ascii_lowercase();
        if lower.contains("connectionclosed") || lower.contains("connection closed") {
            connection_closed_count += 1;
        }
        if lower.contains("insufficientpeers") || lower.contains("insufficient peers") {
            insufficient_peers_count += 1;
        }
        if lower.contains("timeout") {
            timeout_count += 1;
        }
        if lower.contains("protocol")
            || lower.contains("unsupported")
            || lower.contains("mismatch")
            || lower.contains("unavailable")
        {
            protocol_error_count += 1;
        }
    }
    let active_peer_ids = replication
        .peer_healths
        .iter()
        .filter(|health| health.status == "active")
        .map(|health| health.peer_id.as_str())
        .collect::<BTreeSet<_>>();
    let protocol_cooldown_peer_ids = replication
        .protocol_retry_cooldown_peers
        .values()
        .flat_map(|peers| peers.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let transport_cooldown_peer_ids = replication
        .transport_retry_cooldown_peers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let has_stable_active_peer = active_peer_ids.iter().any(|peer_id| {
        !protocol_cooldown_peer_ids.contains(peer_id)
            && !transport_cooldown_peer_ids.contains(peer_id)
    });
    let transport_cooldown_penalty_peers = if has_stable_active_peer {
        0
    } else {
        replication.transport_retry_cooldown_peers.len()
    };
    let protocol_cooldown_penalty_peers = if has_stable_active_peer {
        0
    } else {
        replication
            .protocol_retry_cooldown_peers
            .values()
            .map(Vec::len)
            .sum::<usize>()
    };
    let penalty = connection_closed_count
        .saturating_mul(5)
        .saturating_add(insufficient_peers_count.saturating_mul(10))
        .saturating_add(timeout_count.saturating_mul(10))
        .saturating_add(protocol_error_count.saturating_mul(15))
        .saturating_add(transport_cooldown_penalty_peers.saturating_mul(15))
        .saturating_add(protocol_cooldown_penalty_peers.saturating_mul(20));
    let score = 100u8.saturating_sub(penalty.min(100) as u8);
    ChainReplicationTransportStability {
        stable: score >= TRANSPORT_STABILITY_MIN_SCORE,
        score,
        recent_error_count: replication.recent_errors.len(),
        blocking_error_count: blocking_replication_error_count(replication),
        connection_closed_count,
        insufficient_peers_count,
        timeout_count,
        protocol_error_count,
    }
}

pub(crate) fn reachability_policy_ok(
    snapshot: &NodeSnapshot,
    p2p: &ChainP2pStatus,
    active_peer_count: usize,
    policy: &ChainReadinessPolicyStatus,
) -> bool {
    if !snapshot.replication_enabled {
        return true;
    }
    if snapshot.role.as_str() == "observer" {
        return active_peer_count > 0 || p2p.relay_available;
    }
    let has_public_direct = p2p.detected_reachability.as_deref() == Some("public")
        || p2p.autonat_status == "public"
        || p2p.public_port_reachability == "reachable"
        || !p2p.confirmed_external_direct_addrs.is_empty()
        || p2p.observed_public_addr.is_some();
    if policy.tier == "mainnet" && policy.relay_policy == "public_direct_or_governed_relay" {
        return has_public_direct || (p2p.relay_available && active_peer_count >= 2);
    }
    has_public_direct || p2p.relay_available
}

pub(crate) fn build_liveness_status(snapshot: &NodeSnapshot) -> ChainLivenessStatus {
    let status = if snapshot.running && snapshot.last_error.is_none() {
        "ok"
    } else {
        "critical"
    };
    ChainLivenessStatus {
        status: status.to_string(),
        running: snapshot.running,
        runtime_last_error: snapshot.last_error.clone(),
    }
}

fn blocking_replication_error_count(
    replication: &super::super::ChainReplicationDebugStatus,
) -> usize {
    replication
        .recent_errors
        .iter()
        .filter(|error| replication_error_is_blocking(replication, error))
        .count()
}

fn replication_error_is_blocking(
    replication: &super::super::ChainReplicationDebugStatus,
    error: &str,
) -> bool {
    if replication_error_is_diagnostic(error) {
        return false;
    }
    let lower = error.to_ascii_lowercase();
    let active_peer_available = replication
        .peer_healths
        .iter()
        .any(|health| health.status == "active")
        || !replication.connected_peers.is_empty();
    if active_peer_available
        && (lower.contains("connection closed")
            || lower.contains("connectionclosed")
            || lower.contains("outgoing connection error")
            || lower.contains("connection refused")
            || lower.contains("redundant connections pruned"))
    {
        return false;
    }
    true
}

fn replication_error_is_diagnostic(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("libp2p autonat")
        || lower.contains("autonat")
        || lower.contains("libp2p connection established")
        || lower.contains("libp2p routing updated")
        || lower.contains("libp2p transport active")
        || lower.contains("libp2p redundant connections pruned")
        || lower.contains("peer record request failed")
        || lower.contains("missingpeerrecord")
        || lower.contains("missing_peer_record")
        || lower.contains("insufficientactivediscoverysources")
        || lower.contains("insufficient_active_discovery_sources")
        || lower.contains("single_source_discovery")
        || lower.contains("quarantine suppresses")
        || lower.contains("dial condition")
}
