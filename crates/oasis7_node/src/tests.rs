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

include!("tests_consensus_signatures.rs");
include!("tests_clock_and_replication.rs");
include!("tests_replication_gossip.rs");
include!("tests_storage_replication.rs");
mod non_sequencer_followers;
mod replication_state_sync;
mod restart_reconcile;
#[path = "tests_hello_throttle.rs"]
mod tests_hello_throttle;
