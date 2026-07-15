use std::sync::{Arc, Condvar, Mutex, TryLockError, Weak};
use std::thread::{self, JoinHandle};

use crate::node_runtime_core::RuntimeState;
use crate::{NodeConsensusSnapshot, NodeError};

pub trait NodeConsensusProgressObserver: Send {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        observed_at_ms: i64,
    ) -> Result<(), String>;

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        None
    }
}

const CONSENSUS_PROGRESS_OBSERVER_BACKPRESSURE: &str =
    "consensus progress observer queue saturated";

struct PendingConsensusProgress {
    sequence: u64,
    snapshot: NodeConsensusSnapshot,
    observed_at_ms: i64,
}

#[derive(Default)]
struct ConsensusProgressObserverQueue {
    next_sequence: u64,
    pending: Option<PendingConsensusProgress>,
    shutdown: bool,
}

pub(super) struct ConsensusProgressObserverDispatcher {
    queue: Arc<(Mutex<ConsensusProgressObserverQueue>, Condvar)>,
    state: Weak<Mutex<RuntimeState>>,
    generation: u64,
    restart_observer: Option<Box<dyn NodeConsensusProgressObserver>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub(super) struct ConsensusProgressObserverSubmitter {
    queue: Arc<(Mutex<ConsensusProgressObserverQueue>, Condvar)>,
    state: Weak<Mutex<RuntimeState>>,
    generation: u64,
}

impl ConsensusProgressObserverDispatcher {
    pub(super) fn spawn(
        node_id: &str,
        state: Arc<Mutex<RuntimeState>>,
        generation: u64,
        mut observer: Box<dyn NodeConsensusProgressObserver>,
    ) -> std::io::Result<Self> {
        let restart_observer = observer.recreate_for_restart();
        let queue = Arc::new((
            Mutex::new(ConsensusProgressObserverQueue::default()),
            Condvar::new(),
        ));
        let worker_queue = Arc::clone(&queue);
        let worker_state = Arc::downgrade(&state);
        let worker = thread::Builder::new()
            .name(format!("aw-node-observer-{node_id}"))
            .spawn(move || {
                loop {
                    let pending = {
                        let (queue_lock, signal) = &*worker_queue;
                        let mut queue = queue_lock
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        while queue.pending.is_none() && !queue.shutdown {
                            queue = signal
                                .wait(queue)
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                        }
                        if queue.shutdown {
                            break;
                        }
                        queue.pending.take().expect("pending observer snapshot")
                    };

                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        observer
                            .observe_consensus_progress(&pending.snapshot, pending.observed_at_ms)
                    }))
                    .unwrap_or_else(|_| Err("consensus progress observer panicked".to_string()));
                    let has_newer_snapshot = {
                        let (queue_lock, _) = &*worker_queue;
                        let queue = queue_lock
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        if queue.shutdown {
                            break;
                        }
                        queue
                            .pending
                            .as_ref()
                            .is_some_and(|next| next.sequence > pending.sequence)
                    };
                    let Some(worker_state) = worker_state.upgrade() else {
                        break;
                    };
                    let mut current = worker_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if current.generation != generation {
                        break;
                    }
                    match result {
                        Ok(()) if !has_newer_snapshot => {
                            current.consensus_progress_observer_error = None;
                        }
                        Ok(()) => {}
                        Err(error) => {
                            current.consensus_progress_observer_error = Some(error);
                        }
                    }
                }
            })?;
        Ok(Self {
            queue,
            state: Arc::downgrade(&state),
            generation,
            restart_observer,
            worker: Some(worker),
        })
    }

    pub(super) fn submitter(&self) -> ConsensusProgressObserverSubmitter {
        ConsensusProgressObserverSubmitter {
            queue: Arc::clone(&self.queue),
            state: Weak::clone(&self.state),
            generation: self.generation,
        }
    }

    pub(super) fn shutdown(&mut self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        let (queue_lock, signal) = &*self.queue;
        let mut queue = queue_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.shutdown = true;
        queue.pending = None;
        drop(queue);
        signal.notify_all();

        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        } else {
            // External observer code can block forever; detaching keeps stop bounded.
            let _ = self.worker.take();
        }
        self.restart_observer.take()
    }
}

impl ConsensusProgressObserverSubmitter {
    pub(super) fn submit(&self, snapshot: NodeConsensusSnapshot, observed_at_ms: i64) {
        let (queue_lock, signal) = &*self.queue;
        let mut queue = match queue_lock.try_lock() {
            Ok(queue) => queue,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                self.record_backpressure();
                return;
            }
        };
        if queue.shutdown {
            return;
        }
        queue.next_sequence = queue.next_sequence.saturating_add(1);
        let sequence = queue.next_sequence;
        let coalesced = queue
            .pending
            .replace(PendingConsensusProgress {
                sequence,
                snapshot,
                observed_at_ms,
            })
            .is_some();
        drop(queue);
        if coalesced {
            self.record_backpressure();
        }
        signal.notify_one();
    }

    fn record_backpressure(&self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let mut current = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if current.generation != self.generation {
            return;
        }
        current.consensus_progress_observer_error =
            Some(CONSENSUS_PROGRESS_OBSERVER_BACKPRESSURE.to_string());
    }
}

impl Drop for ConsensusProgressObserverDispatcher {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub(super) fn publish_runtime_progress_snapshot(
    state: &Arc<Mutex<RuntimeState>>,
    observer: Option<&ConsensusProgressObserverSubmitter>,
    snapshot: NodeConsensusSnapshot,
    observed_at_ms: i64,
) -> Result<(), NodeError> {
    {
        let mut current = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        current.consensus = snapshot.clone();
        current.last_tick_unix_ms = Some(observed_at_ms);
    }
    if let Some(observer) = observer {
        observer.submit(snapshot, observed_at_ms);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::*;

    struct PermanentlyBlockingObserver {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }

    impl NodeConsensusProgressObserver for PermanentlyBlockingObserver {
        fn observe_consensus_progress(
            &mut self,
            _snapshot: &NodeConsensusSnapshot,
            _observed_at_ms: i64,
        ) -> Result<(), String> {
            let _ = self.entered.send(());
            let _ = self.release.recv();
            Ok(())
        }
    }

    #[test]
    fn dropping_dispatcher_never_joins_a_blocked_observer() {
        let state = Arc::new(Mutex::new(RuntimeState::default()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let dispatcher = ConsensusProgressObserverDispatcher::spawn(
            "blocked-shutdown",
            state,
            0,
            Box::new(PermanentlyBlockingObserver {
                entered: entered_tx,
                release: release_rx,
            }),
        )
        .expect("spawn observer dispatcher");
        dispatcher
            .submitter()
            .submit(NodeConsensusSnapshot::default(), 1);
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("observer entered blocking call");

        let started = Instant::now();
        drop(dispatcher);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "dispatcher drop waited for a blocked observer"
        );
        let _ = release_tx.send(());
    }
}
