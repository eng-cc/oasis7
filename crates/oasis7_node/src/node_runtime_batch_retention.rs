use std::sync::{Condvar, Mutex};

use crate::{NodeCommittedActionBatch, NodeConsensusAction};

pub(super) fn action_payload_bytes<'a>(
    actions: impl Iterator<Item = &'a NodeConsensusAction>,
) -> usize {
    actions.fold(0usize, |total, action| {
        total.saturating_add(action.payload_cbor.len())
    })
}

pub(super) fn push_committed_action_batch(
    state: &(Mutex<Vec<NodeCommittedActionBatch>>, Condvar),
    batch: NodeCommittedActionBatch,
    max_batches: usize,
    max_payload_bytes: usize,
) {
    let (committed_lock, committed_signal) = state;
    let mut committed = committed_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    retain_committed_action_batch(&mut committed, batch, max_batches, max_payload_bytes);
    committed_signal.notify_all();
}

pub(super) fn retain_committed_action_batch(
    retained: &mut Vec<NodeCommittedActionBatch>,
    batch: NodeCommittedActionBatch,
    max_batches: usize,
    max_payload_bytes: usize,
) {
    let max_batches = max_batches.max(1);
    let batch_payload_bytes = action_payload_bytes(batch.actions.iter());
    if batch_payload_bytes > max_payload_bytes {
        return;
    }

    let mut retained_payload_bytes =
        action_payload_bytes(retained.iter().flat_map(|entry| entry.actions.iter()));
    while retained.len() >= max_batches
        || retained_payload_bytes.saturating_add(batch_payload_bytes) > max_payload_bytes
    {
        if retained.is_empty() {
            return;
        }
        let removed = retained.remove(0);
        retained_payload_bytes =
            retained_payload_bytes.saturating_sub(action_payload_bytes(removed.actions.iter()));
    }
    retained.push(batch);
}
