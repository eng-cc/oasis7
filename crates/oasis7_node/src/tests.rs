fn run_test_with_timeout(
    name: &'static str,
    timeout: std::time::Duration,
    test: impl FnOnce() + Send + 'static,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => {}
        Ok(Err(payload)) => std::panic::resume_unwind(payload),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            panic!("{name} exceeded timeout of {timeout:?}");
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            panic!("{name} worker exited without reporting a result");
        }
    }
}

#[test]
fn fetch_blob_response_slice_honors_offset_and_limit() {
    assert_eq!(
        super::slice_fetch_blob_response(b"abcdefgh".to_vec(), Some(2), Some(3)),
        (b"cde".to_vec(), Some(false))
    );
    assert_eq!(
        super::slice_fetch_blob_response(b"abcdefgh".to_vec(), Some(6), Some(99)),
        (b"gh".to_vec(), Some(true))
    );
    assert_eq!(
        super::slice_fetch_blob_response(b"abcdefgh".to_vec(), Some(99), Some(3)),
        (Vec::<u8>::new(), Some(true))
    );
}

#[test]
fn fetch_blob_range_rejects_oversized_response_before_loading() {
    let err = super::replication_fetch_handler_support::validated_fetch_blob_range(
        Some(0),
        Some(super::replication_fetch_handler_support::FETCH_BLOB_MAX_RESPONSE_BYTES + 1),
    )
    .expect_err("oversized fetch-blob range must be rejected before disk read");

    assert!(err.contains("fetch-blob requested response bytes"));
}

#[test]
fn fetch_blob_range_admits_legacy_v1_two_mib_request() {
    const LEGACY_V1_FETCH_BLOB_BYTES: u64 = 2 * 1024 * 1024;
    let request = super::replication::FetchBlobRequest {
        content_hash: "legacy-v1-two-mib".to_string(),
        offset_bytes: Some(0),
        limit_bytes: Some(LEGACY_V1_FETCH_BLOB_BYTES),
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let (offset, limit) = super::replication_fetch_handler_support::validated_fetch_blob_range(
        request.offset_bytes,
        request.limit_bytes,
    )
    .expect("legacy v1 2 MiB fetch-blob request must remain admitted");

    assert_eq!(offset, 0);
    assert_eq!(limit, LEGACY_V1_FETCH_BLOB_BYTES as usize);
}

fn test_fetch_blob_response(
    found: bool,
    blob: Option<Vec<u8>>,
) -> super::replication::FetchBlobResponse {
    super::replication::FetchBlobResponse {
        found,
        range_offset_bytes: None,
        range_complete: None,
        blob,
    }
}

include!("tests_consensus_signatures.rs");
include!("tests_clock_and_replication.rs");
include!("tests_replication_gossip.rs");
include!("tests_replication_rebroadcast.rs");
include!("tests_storage_replication.rs");
mod non_sequencer_followers;
mod replication_state_sync;
mod restart_reconcile;
#[path = "tests_fetch_blob_chunking.rs"]
mod tests_fetch_blob_chunking;
#[path = "tests_hello_throttle.rs"]
mod tests_hello_throttle;
