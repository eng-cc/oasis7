use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::channel::mpsc;

use crate::error::WorldError;
use crate::util::unix_now_ms_i64;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::MembershipDirectorySnapshot;

use super::Command;

pub(super) fn decode_world_head(bytes: &[u8]) -> Result<WorldHeadAnnounce, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

pub(super) fn decode_membership_directory(
    bytes: &[u8],
) -> Result<MembershipDirectorySnapshot, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

pub(super) fn now_ms() -> i64 {
    unix_now_ms_i64()
}

pub(super) fn try_send_command(
    command_tx: &mpsc::Sender<Command>,
    command: Command,
) -> Result<(), WorldError> {
    let mut sender = command_tx.clone();
    sender
        .try_send(command)
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: if err.is_full() {
                "libp2p command queue saturated".to_string()
            } else {
                "libp2p command queue disconnected".to_string()
            },
        })
}

pub(super) fn push_bounded_clone<T: Clone>(
    values: &Arc<Mutex<Vec<T>>>,
    value: T,
    max_len: usize,
    lock_label: &str,
) {
    let mut guard = values.lock().expect(lock_label);
    push_bounded_vec(&mut guard, value, max_len);
}

pub(super) fn push_bounded_string_with_cooldown(
    values: &Arc<Mutex<Vec<String>>>,
    recent_values_at_ms: &mut HashMap<String, i64>,
    value: String,
    max_len: usize,
    lock_label: &str,
    now_ms: i64,
    cooldown_ms: i64,
) -> bool {
    push_bounded_string_with_keyed_cooldown(
        values,
        recent_values_at_ms,
        value.clone(),
        value,
        max_len,
        lock_label,
        now_ms,
        cooldown_ms,
    )
}

pub(super) fn push_bounded_string_with_keyed_cooldown(
    values: &Arc<Mutex<Vec<String>>>,
    recent_values_at_ms: &mut HashMap<String, i64>,
    key: String,
    value: String,
    max_len: usize,
    lock_label: &str,
    now_ms: i64,
    cooldown_ms: i64,
) -> bool {
    if cooldown_ms > 0 {
        recent_values_at_ms.retain(|_, last_ms| now_ms.saturating_sub(*last_ms) < cooldown_ms);
        if recent_values_at_ms
            .get(key.as_str())
            .is_some_and(|last_ms| now_ms.saturating_sub(*last_ms) < cooldown_ms)
        {
            return false;
        }
        recent_values_at_ms.insert(key, now_ms);
    }
    push_bounded_clone(values, value, max_len, lock_label);
    true
}

pub(super) fn push_bounded_vec<T>(values: &mut Vec<T>, value: T, max_len: usize) {
    let max_len = max_len.max(1);
    values.push(value);
    let overflow = values.len().saturating_sub(max_len);
    if overflow > 0 {
        values.drain(0..overflow);
    }
}

pub(super) fn should_republish(last_ms: i64, now_ms: i64, interval_ms: i64) -> bool {
    if interval_ms <= 0 {
        return false;
    }
    now_ms.saturating_sub(last_ms) >= interval_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_bounded_string_with_cooldown_suppresses_repeat_within_window() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let mut recent_values_at_ms = HashMap::new();

        assert!(push_bounded_string_with_cooldown(
            &values,
            &mut recent_values_at_ms,
            "libp2p connection established peer=peer-a".to_string(),
            8,
            "lock errors",
            1_000,
            5_000,
        ));
        assert!(!push_bounded_string_with_cooldown(
            &values,
            &mut recent_values_at_ms,
            "libp2p connection established peer=peer-a".to_string(),
            8,
            "lock errors",
            4_000,
            5_000,
        ));
        assert!(push_bounded_string_with_cooldown(
            &values,
            &mut recent_values_at_ms,
            "libp2p connection established peer=peer-a".to_string(),
            8,
            "lock errors",
            6_001,
            5_000,
        ));

        let guard = values.lock().expect("lock errors");
        assert_eq!(
            guard.as_slice(),
            &[
                "libp2p connection established peer=peer-a".to_string(),
                "libp2p connection established peer=peer-a".to_string(),
            ]
        );
    }

    #[test]
    fn push_bounded_string_with_cooldown_keeps_distinct_messages() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let mut recent_values_at_ms = HashMap::new();

        assert!(push_bounded_string_with_cooldown(
            &values,
            &mut recent_values_at_ms,
            "libp2p connection closed peer=peer-a num_established=1 active_path=/ip4/1.1.1.1/udp/4101/quic-v1".to_string(),
            8,
            "lock errors",
            1_000,
            5_000,
        ));
        assert!(push_bounded_string_with_cooldown(
            &values,
            &mut recent_values_at_ms,
            "libp2p connection closed peer=peer-a num_established=2 active_path=/ip4/1.1.1.1/udp/4101/quic-v1".to_string(),
            8,
            "lock errors",
            2_000,
            5_000,
        ));

        let guard = values.lock().expect("lock errors");
        assert_eq!(guard.len(), 2);
    }

    #[test]
    fn push_bounded_string_with_keyed_cooldown_suppresses_distinct_messages_for_same_key() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let mut recent_values_at_ms = HashMap::new();

        assert!(push_bounded_string_with_keyed_cooldown(
            &values,
            &mut recent_values_at_ms,
            "connection-closed:peer-a".to_string(),
            "libp2p connection closed peer=peer-a num_established=7 active_path=/ip4/1.1.1.1/tcp/4101".to_string(),
            8,
            "lock errors",
            1_000,
            5_000,
        ));
        assert!(!push_bounded_string_with_keyed_cooldown(
            &values,
            &mut recent_values_at_ms,
            "connection-closed:peer-a".to_string(),
            "libp2p connection closed peer=peer-a num_established=8 active_path=/ip4/1.1.1.1/tcp/4102".to_string(),
            8,
            "lock errors",
            2_000,
            5_000,
        ));

        let guard = values.lock().expect("lock errors");
        assert_eq!(guard.len(), 1);
    }
}
