use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PROVIDER_PUBLICATION_QUEUE_CAPACITY: usize = 64;
const PROVIDER_PUBLICATION_MAX_ATTEMPTS: usize = 3;
const PROVIDER_PUBLICATION_RETRY_BACKOFF: Duration = Duration::from_millis(10);

struct PublicationJob {
    key: String,
    work: Box<dyn FnMut() -> Result<(), String> + Send>,
}

/// A fixed, bounded executor for non-critical DHT provider publication.
#[derive(Clone)]
pub(super) struct ProviderPublicationQueue {
    sender: SyncSender<PublicationJob>,
    pending_keys: Arc<Mutex<HashSet<String>>>,
    counters: Arc<PublicationCounters>,
}

#[derive(Default)]
struct PublicationCounters {
    coalesced: AtomicU64,
    dropped: AtomicU64,
    failed: AtomicU64,
    completed: AtomicU64,
    retries: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderPublicationEnqueueResult {
    Enqueued,
    Coalesced,
    Saturated,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProviderPublicationQueueSnapshot {
    pub depth: usize,
    pub coalesced: u64,
    pub dropped: u64,
    pub failed: u64,
    pub completed: u64,
    pub retries: u64,
}

impl ProviderPublicationQueue {
    pub(super) fn new() -> Self {
        let (sender, receiver) =
            mpsc::sync_channel::<PublicationJob>(PROVIDER_PUBLICATION_QUEUE_CAPACITY);
        let pending_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
        let worker_keys = Arc::clone(&pending_keys);
        let counters = Arc::new(PublicationCounters::default());
        let worker_counters = Arc::clone(&counters);
        let _ = thread::Builder::new()
            .name("replication-provider-publication".to_string())
            .spawn(move || {
                while let Ok(mut job) = receiver.recv() {
                    let mut attempt = 1;
                    let result = loop {
                        let result =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (job.work)()));
                        match result {
                            Ok(Err(reason)) if attempt < PROVIDER_PUBLICATION_MAX_ATTEMPTS => {
                                worker_counters.retries.fetch_add(1, Ordering::Relaxed);
                                eprintln!(
                                    concat!(
                                        "provider publication worker retry ",
                                        "key={} attempt={} reason={}"
                                    ),
                                    job.key,
                                    attempt + 1,
                                    reason
                                );
                                thread::sleep(PROVIDER_PUBLICATION_RETRY_BACKOFF * attempt as u32);
                                attempt += 1;
                            }
                            terminal => break terminal,
                        }
                    };
                    worker_keys
                        .lock()
                        .expect("provider publication keys")
                        .remove(job.key.as_str());
                    match result {
                        Ok(Ok(())) => {
                            let completed =
                                worker_counters.completed.fetch_add(1, Ordering::Relaxed) + 1;
                            eprintln!(
                                concat!(
                                    "provider publication worker success ",
                                    "key={} completed={} attempts={}"
                                ),
                                job.key, completed, attempt
                            );
                        }
                        Ok(Err(reason)) => {
                            let failed = worker_counters.failed.fetch_add(1, Ordering::Relaxed) + 1;
                            eprintln!(
                                concat!(
                                    "provider publication worker failure ",
                                    "key={} failed={} reason={}"
                                ),
                                job.key, failed, reason
                            );
                        }
                        Err(_) => {
                            let failed = worker_counters.failed.fetch_add(1, Ordering::Relaxed) + 1;
                            eprintln!(
                                "provider publication worker panic key={} failed={failed}",
                                job.key
                            );
                        }
                    }
                }
            });
        Self {
            sender,
            pending_keys,
            counters,
        }
    }

    pub(super) fn enqueue(
        &self,
        key: String,
        work: impl FnMut() -> Result<(), String> + Send + 'static,
    ) -> ProviderPublicationEnqueueResult {
        {
            let mut pending = self.pending_keys.lock().expect("provider publication keys");
            if pending.contains(&key) {
                self.counters.coalesced.fetch_add(1, Ordering::Relaxed);
                return ProviderPublicationEnqueueResult::Coalesced;
            }
            if pending.len() >= PROVIDER_PUBLICATION_QUEUE_CAPACITY {
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                return ProviderPublicationEnqueueResult::Saturated;
            }
            pending.insert(key.clone());
        }
        match self.sender.try_send(PublicationJob {
            key: key.clone(),
            work: Box::new(work),
        }) {
            Ok(()) => ProviderPublicationEnqueueResult::Enqueued,
            Err(TrySendError::Full(_)) => {
                self.pending_keys
                    .lock()
                    .expect("provider publication keys")
                    .remove(key.as_str());
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                ProviderPublicationEnqueueResult::Saturated
            }
            Err(TrySendError::Disconnected(_)) => {
                self.pending_keys
                    .lock()
                    .expect("provider publication keys")
                    .remove(key.as_str());
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                ProviderPublicationEnqueueResult::Disconnected
            }
        }
    }

    #[cfg(test)]
    pub(super) fn pending_jobs(&self) -> usize {
        self.pending_keys
            .lock()
            .expect("provider publication keys")
            .len()
    }

    pub(super) fn snapshot(&self) -> ProviderPublicationQueueSnapshot {
        ProviderPublicationQueueSnapshot {
            depth: self
                .pending_keys
                .lock()
                .expect("provider publication keys")
                .len(),
            coalesced: self.counters.coalesced.load(Ordering::Relaxed),
            dropped: self.counters.dropped.load(Ordering::Relaxed),
            failed: self.counters.failed.load(Ordering::Relaxed),
            completed: self.counters.completed.load(Ordering::Relaxed),
            retries: self.counters.retries.load(Ordering::Relaxed),
        }
    }

    #[cfg(test)]
    fn disconnected_for_test() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        Self {
            sender,
            pending_keys: Arc::new(Mutex::new(HashSet::new())),
            counters: Arc::new(PublicationCounters::default()),
        }
    }
}

// Shutdown is sender-driven: dropping the final queue handle disconnects the channel. The worker
// drains every already accepted job in FIFO order, then `recv` observes disconnect and exits.

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn duplicate_publications_coalesce_while_the_fixed_worker_is_busy() {
        let queue = ProviderPublicationQueue::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        assert_eq!(
            queue.enqueue("commit-a".to_string(), move || {
                started_tx.send(()).expect("notify worker start");
                release_rx.recv().expect("release worker");
                Ok(())
            }),
            ProviderPublicationEnqueueResult::Enqueued
        );
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should run the first job");
        for _ in 0..128 {
            assert_eq!(
                queue.enqueue("commit-a".to_string(), || Ok(())),
                ProviderPublicationEnqueueResult::Coalesced
            );
        }
        assert_eq!(
            queue.pending_jobs(),
            1,
            "duplicates must not create queued jobs"
        );
        let snapshot = queue.snapshot();
        assert_eq!(snapshot.coalesced, 128);
        assert_eq!(snapshot.dropped, 0);
        release_tx.send(()).expect("release worker");
    }

    #[test]
    fn publication_retries_until_actual_success_and_cleans_dedup_key() {
        let queue = ProviderPublicationQueue::new();
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker_attempts = Arc::clone(&attempts);
        assert_eq!(
            queue.enqueue("eventual-success".into(), move || {
                let attempt = worker_attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt < 3 {
                    Err(format!("transient failure {attempt}"))
                } else {
                    Ok(())
                }
            }),
            ProviderPublicationEnqueueResult::Enqueued
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().completed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert_eq!(queue.snapshot().completed, 1);
        assert_eq!(queue.snapshot().failed, 0);
        assert_eq!(queue.pending_jobs(), 0);
        assert_eq!(
            queue.enqueue("eventual-success".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Enqueued
        );
    }

    #[test]
    fn permanent_publication_failure_is_counted_after_bounded_retries() {
        let queue = ProviderPublicationQueue::new();
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker_attempts = Arc::clone(&attempts);
        assert_eq!(
            queue.enqueue("permanent-failure".into(), move || {
                worker_attempts.fetch_add(1, Ordering::Relaxed);
                Err("permanent DHT failure".into())
            }),
            ProviderPublicationEnqueueResult::Enqueued
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().failed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert_eq!(queue.snapshot().failed, 1);
        assert_eq!(queue.snapshot().completed, 0);
        assert_eq!(queue.pending_jobs(), 0);
    }

    #[test]
    fn disconnected_queue_reports_actual_loss_and_cleans_key() {
        let queue = ProviderPublicationQueue::disconnected_for_test();
        assert_eq!(
            queue.enqueue("disconnected".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Disconnected
        );
        assert_eq!(queue.pending_jobs(), 0);
        assert_eq!(queue.snapshot().dropped, 1);
        assert_eq!(queue.snapshot().coalesced, 0);
    }

    #[test]
    fn panic_cleans_key_and_worker_accepts_retry() {
        let queue = ProviderPublicationQueue::new();
        assert_eq!(
            queue.enqueue("panic-key".into(), || panic!("controlled panic")),
            ProviderPublicationEnqueueResult::Enqueued
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().failed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(queue.snapshot().failed, 1);
        assert_eq!(
            queue.enqueue("panic-key".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Enqueued
        );
    }

    #[test]
    fn exact_capacity_rejection_cleans_rejected_key_and_allows_republish() {
        let queue = ProviderPublicationQueue::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        assert_eq!(
            queue.enqueue("active".into(), move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            }),
            ProviderPublicationEnqueueResult::Enqueued
        );
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let completed = Arc::new(AtomicUsize::new(0));
        for index in 1..PROVIDER_PUBLICATION_QUEUE_CAPACITY {
            let completed = Arc::clone(&completed);
            assert_eq!(
                queue.enqueue(format!("queued-{index}"), move || {
                    completed.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }),
                ProviderPublicationEnqueueResult::Enqueued
            );
        }
        assert_eq!(queue.snapshot().depth, PROVIDER_PUBLICATION_QUEUE_CAPACITY);
        assert_eq!(
            queue.enqueue("rejected".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Saturated
        );
        assert_eq!(queue.snapshot().dropped, 1);
        release_tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while queue.snapshot().depth != 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(
            completed.load(Ordering::Relaxed),
            PROVIDER_PUBLICATION_QUEUE_CAPACITY - 1
        );
        assert_eq!(
            queue.enqueue("rejected".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Enqueued
        );
        assert_eq!(
            queue.enqueue("republish".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Enqueued
        );
    }

    #[test]
    fn returned_failure_is_counted_and_key_can_retry() {
        let queue = ProviderPublicationQueue::new();
        assert_eq!(
            queue.enqueue("failed-key".into(), || Err("controlled DHT failure".into())),
            ProviderPublicationEnqueueResult::Enqueued
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().failed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(queue.snapshot().failed, 1);
        assert_eq!(
            queue.enqueue("failed-key".into(), || Ok(())),
            ProviderPublicationEnqueueResult::Enqueued
        );
    }

    #[test]
    fn dropping_all_handles_drains_already_accepted_jobs() {
        let queue = ProviderPublicationQueue::new();
        let (completed_tx, completed_rx) = mpsc::channel();
        for index in 0..8 {
            let completed_tx = completed_tx.clone();
            assert_eq!(
                queue.enqueue(format!("drain-{index}"), move || {
                    completed_tx.send(index).unwrap();
                    Ok(())
                }),
                ProviderPublicationEnqueueResult::Enqueued
            );
        }
        drop(completed_tx);
        drop(queue);
        let completed: Vec<_> = completed_rx.iter().collect();
        assert_eq!(completed, (0..8).collect::<Vec<_>>());
    }
}
