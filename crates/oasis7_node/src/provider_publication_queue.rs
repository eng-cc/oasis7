use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

const PROVIDER_PUBLICATION_QUEUE_CAPACITY: usize = 64;

struct PublicationJob {
    key: String,
    work: Box<dyn FnOnce() -> Result<(), String> + Send>,
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
    dropped: AtomicU64,
    failed: AtomicU64,
    completed: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProviderPublicationQueueSnapshot {
    pub depth: usize,
    pub dropped: u64,
    pub failed: u64,
    pub completed: u64,
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
                while let Ok(job) = receiver.recv() {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job.work));
                    worker_keys
                        .lock()
                        .expect("provider publication keys")
                        .remove(job.key.as_str());
                    match result {
                        Ok(Ok(())) => {
                            worker_counters.completed.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Err(reason)) => {
                            let failed = worker_counters.failed.fetch_add(1, Ordering::Relaxed) + 1;
                            eprintln!("provider publication worker failure key={} failed={} reason={reason}", job.key, failed);
                        }
                        Err(_) => {
                            let failed = worker_counters.failed.fetch_add(1, Ordering::Relaxed) + 1;
                            eprintln!("provider publication worker panic key={} failed={failed}", job.key);
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
        work: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> bool {
        {
            let mut pending = self.pending_keys.lock().expect("provider publication keys");
            if pending.contains(&key) || pending.len() >= PROVIDER_PUBLICATION_QUEUE_CAPACITY {
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            pending.insert(key.clone());
        }
        match self.sender.try_send(PublicationJob {
            key: key.clone(),
            work: Box::new(work),
        }) {
            Ok(()) => true,
            Err(_) => {
                self.pending_keys
                    .lock()
                    .expect("provider publication keys")
                    .remove(key.as_str());
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                false
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
            dropped: self.counters.dropped.load(Ordering::Relaxed),
            failed: self.counters.failed.load(Ordering::Relaxed),
            completed: self.counters.completed.load(Ordering::Relaxed),
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
        assert!(queue.enqueue("commit-a".to_string(), move || {
            started_tx.send(()).expect("notify worker start");
            release_rx.recv().expect("release worker");
            Ok(())
        }));
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should run the first job");
        for _ in 0..128 {
            assert!(!queue.enqueue("commit-a".to_string(), || Ok(())));
        }
        assert_eq!(
            queue.pending_jobs(),
            1,
            "duplicates must not create queued jobs"
        );
        release_tx.send(()).expect("release worker");
    }

    #[test]
    fn panic_cleans_key_and_worker_accepts_retry() {
        let queue = ProviderPublicationQueue::new();
        assert!(queue.enqueue("panic-key".into(), || panic!("controlled panic")));
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().failed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(queue.snapshot().failed, 1);
        assert!(queue.enqueue("panic-key".into(), || Ok(())));
    }

    #[test]
    fn exact_capacity_rejection_cleans_rejected_key_and_allows_republish() {
        let queue = ProviderPublicationQueue::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        assert!(queue.enqueue("active".into(), move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        }));
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let completed = Arc::new(AtomicUsize::new(0));
        for index in 1..PROVIDER_PUBLICATION_QUEUE_CAPACITY {
            let completed = Arc::clone(&completed);
            assert!(queue.enqueue(format!("queued-{index}"), move || {
                completed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }));
        }
        assert_eq!(queue.snapshot().depth, PROVIDER_PUBLICATION_QUEUE_CAPACITY);
        assert!(!queue.enqueue("rejected".into(), || Ok(())));
        release_tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while queue.snapshot().depth != 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(
            completed.load(Ordering::Relaxed),
            PROVIDER_PUBLICATION_QUEUE_CAPACITY - 1
        );
        assert!(queue.enqueue("rejected".into(), || Ok(())));
        assert!(queue.enqueue("republish".into(), || Ok(())));
    }

    #[test]
    fn returned_failure_is_counted_and_key_can_retry() {
        let queue = ProviderPublicationQueue::new();
        assert!(queue.enqueue("failed-key".into(), || Err("controlled DHT failure".into())));
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while queue.snapshot().failed == 0 && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(queue.snapshot().failed, 1);
        assert!(queue.enqueue("failed-key".into(), || Ok(())));
    }

    #[test]
    fn dropping_all_handles_drains_already_accepted_jobs() {
        let queue = ProviderPublicationQueue::new();
        let (completed_tx, completed_rx) = mpsc::channel();
        for index in 0..8 {
            let completed_tx = completed_tx.clone();
            assert!(queue.enqueue(format!("drain-{index}"), move || {
                completed_tx.send(index).unwrap();
                Ok(())
            }));
        }
        drop(completed_tx);
        drop(queue);
        let completed: Vec<_> = completed_rx.iter().collect();
        assert_eq!(completed, (0..8).collect::<Vec<_>>());
    }
}
