use std::collections::HashSet;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

const PROVIDER_PUBLICATION_QUEUE_CAPACITY: usize = 64;

struct PublicationJob {
    key: String,
    work: Box<dyn FnOnce() + Send>,
}

/// A fixed, bounded executor for non-critical DHT provider publication.
#[derive(Clone)]
pub(super) struct ProviderPublicationQueue {
    sender: SyncSender<PublicationJob>,
    pending_keys: Arc<Mutex<HashSet<String>>>,
}

impl ProviderPublicationQueue {
    pub(super) fn new() -> Self {
        let (sender, receiver) =
            mpsc::sync_channel::<PublicationJob>(PROVIDER_PUBLICATION_QUEUE_CAPACITY);
        let pending_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
        let worker_keys = Arc::clone(&pending_keys);
        let _ = thread::Builder::new()
            .name("replication-provider-publication".to_string())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    (job.work)();
                    worker_keys
                        .lock()
                        .expect("provider publication keys")
                        .remove(job.key.as_str());
                }
            });
        Self {
            sender,
            pending_keys,
        }
    }

    pub(super) fn enqueue(&self, key: String, work: impl FnOnce() + Send + 'static) -> bool {
        {
            let mut pending = self.pending_keys.lock().expect("provider publication keys");
            if !pending.insert(key.clone()) {
                return false;
            }
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
}

#[cfg(test)]
mod tests {
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
        }));
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should run the first job");
        for _ in 0..128 {
            assert!(!queue.enqueue("commit-a".to_string(), || {}));
        }
        assert_eq!(
            queue.pending_jobs(),
            1,
            "duplicates must not create queued jobs"
        );
        release_tx.send(()).expect("release worker");
    }
}
