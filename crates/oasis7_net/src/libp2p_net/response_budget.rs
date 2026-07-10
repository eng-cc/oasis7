use std::collections::HashMap;

use libp2p::PeerId;
use oasis7_proto::distributed::DistributedErrorCode;

use crate::error::WorldError;
use crate::util::to_canonical_cbor;

use super::error_mapping::error_response_from_world_error;

const FETCH_BLOB_PROTOCOL: &str = "/aw/node/replication/fetch-blob/1.0.0";
const FETCH_BLOB_RESPONSE_WINDOW_MS: i64 = 60_000;
const FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW: usize = 8 * 1024 * 1024;

#[derive(Default)]
pub(super) struct FetchBlobResponseBudget {
    by_peer: HashMap<PeerId, ResponseWindow>,
}

#[derive(Clone, Copy)]
struct ResponseWindow {
    started_at_ms: i64,
    bytes: usize,
}

impl FetchBlobResponseBudget {
    pub(super) fn admit(
        &mut self,
        peer: PeerId,
        protocol: &str,
        response_bytes: usize,
        now_ms: i64,
    ) -> Result<(), WorldError> {
        if protocol != FETCH_BLOB_PROTOCOL {
            return Ok(());
        }

        let window = self.by_peer.entry(peer).or_insert(ResponseWindow {
            started_at_ms: now_ms,
            bytes: 0,
        });
        if now_ms.saturating_sub(window.started_at_ms) >= FETCH_BLOB_RESPONSE_WINDOW_MS {
            *window = ResponseWindow {
                started_at_ms: now_ms,
                bytes: 0,
            };
        }

        if response_bytes > FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW.saturating_sub(window.bytes) {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrRateLimited,
                message: format!(
                    "fetch-blob response budget exhausted for peer={peer}; retry after window reset"
                ),
                retryable: false,
            });
        }

        window.bytes = window.bytes.saturating_add(response_bytes);
        Ok(())
    }
}

pub(super) fn budgeted_response_bytes(
    budget: &mut FetchBlobResponseBudget,
    peer: PeerId,
    protocol: &str,
    reply: Result<Vec<u8>, WorldError>,
    now_ms: i64,
) -> Vec<u8> {
    let response_bytes = reply.unwrap_or_else(|err| {
        to_canonical_cbor(&error_response_from_world_error(&err)).unwrap_or_default()
    });
    budget
        .admit(peer, protocol, response_bytes.len(), now_ms)
        .map(|()| response_bytes)
        .unwrap_or_else(|err| {
            to_canonical_cbor(&error_response_from_world_error(&err)).unwrap_or_default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_blob_budget_is_per_peer_and_resets_after_its_window() {
        let mut budget = FetchBlobResponseBudget::default();
        let first_peer = PeerId::random();
        let second_peer = PeerId::random();
        let first_window = 1_000;

        budget
            .admit(
                first_peer,
                FETCH_BLOB_PROTOCOL,
                FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW,
                first_window,
            )
            .expect("first peer fits exactly once");
        let err = budget
            .admit(first_peer, FETCH_BLOB_PROTOCOL, 1, first_window)
            .expect_err("same peer is limited inside its window");
        assert!(matches!(
            err,
            WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrRateLimited,
                retryable: false,
                ..
            }
        ));

        budget
            .admit(second_peer, FETCH_BLOB_PROTOCOL, 1, first_window)
            .expect("another peer has an independent budget");
        budget
            .admit(
                first_peer,
                FETCH_BLOB_PROTOCOL,
                1,
                first_window + FETCH_BLOB_RESPONSE_WINDOW_MS,
            )
            .expect("expired window resets budget");
    }

    #[test]
    fn fetch_blob_budget_does_not_apply_to_other_protocols() {
        let mut budget = FetchBlobResponseBudget::default();
        budget
            .admit(
                PeerId::random(),
                "/aw/node/replication/fetch-commit/1.0.0",
                FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW.saturating_add(1),
                1_000,
            )
            .expect("other protocols are unaffected");
    }
}
