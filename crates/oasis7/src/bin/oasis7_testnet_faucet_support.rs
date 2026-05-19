use super::*;

#[derive(Debug, Clone)]
pub(super) struct ClaimReservation {
    pub(super) previous_next_nonce: Option<u64>,
    pub(super) reserved_next_nonce: u64,
    pub(super) previous_account_claim_ms: Option<i64>,
    pub(super) previous_ip_claim_ms: Option<i64>,
}

pub(super) fn prune_faucet_state_trackers(state: &mut FaucetState, now_ms: i64, cooldown_ms: i64) {
    let cutoff_ms = now_ms.saturating_sub(cooldown_ms);
    prune_tracker_map(&mut state.last_account_claim_unix_ms, cutoff_ms);
    prune_tracker_map(&mut state.last_ip_claim_unix_ms, cutoff_ms);
}

pub(super) fn prune_tracker_map(map: &mut HashMap<String, i64>, cutoff_ms: i64) {
    map.retain(|_, claimed_at_ms| *claimed_at_ms >= cutoff_ms);
    while map.len() > MAX_TRACKED_FAUCET_CLAIMANTS {
        let oldest = map
            .iter()
            .min_by_key(|(_, claimed_at_ms)| *claimed_at_ms)
            .map(|(key, _)| key.clone());
        if let Some(oldest_key) = oldest {
            map.remove(oldest_key.as_str());
        } else {
            break;
        }
    }
}

pub(super) fn rollback_claim_reservation(
    state: &mut FaucetState,
    target_account_id: &str,
    remote_ip: &str,
    reserved_at_ms: i64,
    reservation: &ClaimReservation,
) {
    if state.next_nonce == Some(reservation.reserved_next_nonce) {
        state.next_nonce = reservation.previous_next_nonce;
    }
    rollback_tracker_entry(
        &mut state.last_account_claim_unix_ms,
        target_account_id,
        reserved_at_ms,
        reservation.previous_account_claim_ms,
    );
    rollback_tracker_entry(
        &mut state.last_ip_claim_unix_ms,
        remote_ip,
        reserved_at_ms,
        reservation.previous_ip_claim_ms,
    );
}

pub(super) fn faucet_claim_status_code(response: &FaucetClaimResponse) -> u16 {
    if response.ok {
        return 200;
    }
    match response.error_code.as_deref() {
        Some("bad_request") => 400,
        Some("cooldown_active") | Some("ip_cooldown_active") => 429,
        Some("insufficient_balance") => 503,
        Some("upstream_unavailable") | Some("nonce_replay") => 502,
        Some(_) | None => 502,
    }
}

fn rollback_tracker_entry(
    map: &mut HashMap<String, i64>,
    key: &str,
    reserved_at_ms: i64,
    previous_value: Option<i64>,
) {
    if map.get(key).copied() == Some(reserved_at_ms) {
        if let Some(previous_value) = previous_value {
            map.insert(key.to_string(), previous_value);
        } else {
            map.remove(key);
        }
    }
}
