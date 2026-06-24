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
