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
