use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, TryLockError, Weak};
use std::thread::{self, JoinHandle};

use crate::node_runtime_core::RuntimeState;
use crate::{NodeConsensusSnapshot, NodeError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeConsensusProgressObserverError {
    pub code: Option<String>,
    pub message: String,
}

impl NodeConsensusProgressObserverError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
        }
    }

    pub fn coded(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.into()),
            message: message.into(),
        }
    }
}

impl fmt::Display for NodeConsensusProgressObserverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message.as_str())
    }
}

impl Deref for NodeConsensusProgressObserverError {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.message.as_str()
    }
}

impl From<String> for NodeConsensusProgressObserverError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for NodeConsensusProgressObserverError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

pub trait NodeConsensusProgressObserver: Send {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError>;

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        None
    }

    /// Bind the generation-scoped authority used to fence durable observer work.
    /// Observers without durable side effects may ignore it.
    fn bind_lifecycle_authority(&mut self, _authority: ObserverLifecycleAuthority) {}

    fn take_restart_handoff(&mut self) -> Option<ObserverRestartHandoff> {
        None
    }
}

#[derive(Clone)]
pub struct ObserverLifecycleAuthority {
    state: Arc<ObserverLifecycleAuthorityState>,
    generation: u64,
}

struct ObserverLifecycleAuthorityState {
    active_generation: AtomicU64,
    commit_lock: Mutex<()>,
}

pub struct ObserverLifecycleMutationGuard {
    authority: ObserverLifecycleAuthority,
}

#[derive(Debug)]
pub struct ObserverLifecyclePublicationError {
    code: &'static str,
    source: Option<io::Error>,
}

impl ObserverLifecyclePublicationError {
    pub fn code(&self) -> &'static str {
        self.code
    }

    fn new(code: &'static str) -> Self {
        Self { code, source: None }
    }

    fn io(code: &'static str, source: io::Error) -> Self {
        Self {
            code,
            source: Some(source),
        }
    }
}

impl fmt::Display for ObserverLifecyclePublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl Error for ObserverLifecyclePublicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

impl ObserverLifecycleAuthority {
    /// Starts durable work only while this generation is still active.
    ///
    /// This deliberately does not hold the commit lock while callers write or fsync a temporary
    /// file. Restart must be able to revoke a stalled predecessor without waiting for that I/O.
    pub fn acquire_durable_mutation(&self) -> Option<ObserverLifecycleMutationGuard> {
        (self.state.active_generation.load(Ordering::SeqCst) == self.generation).then_some(
            ObserverLifecycleMutationGuard {
                authority: self.clone(),
            },
        )
    }
}

impl ObserverLifecycleMutationGuard {
    /// Publishes `target` from a fully written and synced same-directory staging file.
    ///
    /// Callers cannot run arbitrary side effects after the authority check. The commit lock is
    /// non-blocking so a stalled publisher cannot make a successor wait; poisoned state fails
    /// closed. The only non-cancellable operation after the final generation check is the native
    /// same-directory replacement itself.
    pub fn publish_staged_file(
        &self,
        staged: &Path,
        target: &Path,
    ) -> Result<(), ObserverLifecyclePublicationError> {
        let Some(parent) = staged
            .parent()
            .filter(|parent| Some(*parent) == target.parent())
        else {
            return Err(ObserverLifecyclePublicationError::new(
                "observer_lifecycle_invalid_staging",
            ));
        };
        if staged == target {
            return Err(ObserverLifecyclePublicationError::new(
                "observer_lifecycle_invalid_staging",
            ));
        }
        let _commit_lock = match self.authority.state.commit_lock.try_lock() {
            Ok(lock) => lock,
            Err(TryLockError::WouldBlock) => {
                return Err(ObserverLifecyclePublicationError::new(
                    "observer_lifecycle_commit_busy",
                ));
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err(ObserverLifecyclePublicationError::new(
                    "observer_lifecycle_commit_poisoned",
                ));
            }
        };
        if self
            .authority
            .state
            .active_generation
            .load(Ordering::SeqCst)
            != self.authority.generation
        {
            return Err(ObserverLifecyclePublicationError::new(
                "observer_lifecycle_revoked",
            ));
        }
        platform_replace_staged_file(staged, target).map_err(|source| {
            ObserverLifecyclePublicationError::io("state_replace_failed", source)
        })?;
        sync_parent_dir(parent).map_err(|source| {
            ObserverLifecyclePublicationError::io("state_parent_fsync_failed", source)
        })
    }
}

#[cfg(windows)]
fn platform_replace_staged_file(staged: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    // SAFETY: both paths are owned, NUL-terminated UTF-16 buffers that remain alive for the
    // duration of the call.
    let replaced = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) };
    if replaced == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn platform_replace_staged_file(staged: &Path, target: &Path) -> io::Result<()> {
    fs::rename(staged, target)
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Path) -> io::Result<()> {
    Ok(())
}

const CONSENSUS_PROGRESS_OBSERVER_BACKPRESSURE: &str =
    "consensus progress observer queue saturated";
const CONSENSUS_PROGRESS_OBSERVER_PENDING_LIMIT: usize = 2;

struct PendingConsensusProgress {
    sequence: u64,
    snapshot: NodeConsensusSnapshot,
    observed_at_ms: i64,
}

pub struct ObserverRestartHandoff {
    next_sequence: u64,
    pending: VecDeque<PendingConsensusProgress>,
    authority_state: Arc<ObserverLifecycleAuthorityState>,
}

struct RestartHandoffObserver {
    observer: Box<dyn NodeConsensusProgressObserver>,
    handoff: Option<ObserverRestartHandoff>,
}

impl NodeConsensusProgressObserver for RestartHandoffObserver {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        self.observer
            .observe_consensus_progress(snapshot, observed_at_ms)
    }

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        self.observer.recreate_for_restart()
    }

    fn bind_lifecycle_authority(&mut self, authority: ObserverLifecycleAuthority) {
        self.observer.bind_lifecycle_authority(authority);
    }

    fn take_restart_handoff(&mut self) -> Option<ObserverRestartHandoff> {
        self.handoff.take()
    }
}

#[derive(Default)]
struct ConsensusProgressObserverQueue {
    next_sequence: u64,
    pending: VecDeque<PendingConsensusProgress>,
    shutdown: bool,
}

impl ConsensusProgressObserverQueue {
    fn enqueue(&mut self, pending: PendingConsensusProgress) -> bool {
        if self.pending.len() < CONSENSUS_PROGRESS_OBSERVER_PENDING_LIMIT {
            self.pending.push_back(pending);
            return false;
        }

        let equal_head =
            pending.snapshot.committed_height == pending.snapshot.network_committed_height;
        if equal_head {
            if let Some(existing_equal_head) = self.pending.iter().position(|queued| {
                queued.snapshot.committed_height == queued.snapshot.network_committed_height
            }) {
                if existing_equal_head + 1 == self.pending.len() {
                    self.pending[existing_equal_head] = pending;
                } else if let Some(latest) = self.pending.back_mut() {
                    *latest = pending;
                }
            } else if let Some(latest) = self.pending.back_mut() {
                *latest = pending;
            }
            return true;
        }

        if let Some(equal_head) = self.pending.iter().position(|queued| {
            queued.snapshot.committed_height == queued.snapshot.network_committed_height
        }) {
            if equal_head == 0 {
                if let Some(latest) = self.pending.back_mut() {
                    *latest = pending;
                }
            } else {
                self.pending.pop_front();
                self.pending.push_back(pending);
            }
        } else if let Some(latest) = self.pending.back_mut() {
            *latest = pending;
        }
        true
    }
}

pub(super) struct ConsensusProgressObserverDispatcher {
    queue: Arc<(Mutex<ConsensusProgressObserverQueue>, Condvar)>,
    state: Weak<Mutex<RuntimeState>>,
    generation: u64,
    restart_observer: Option<Box<dyn NodeConsensusProgressObserver>>,
    authority: ObserverLifecycleAuthority,
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
        let handoff = observer.take_restart_handoff();
        let authority_state = handoff.as_ref().map_or_else(
            || {
                Arc::new(ObserverLifecycleAuthorityState {
                    active_generation: AtomicU64::new(generation),
                    commit_lock: Mutex::new(()),
                })
            },
            |handoff| Arc::clone(&handoff.authority_state),
        );
        authority_state
            .active_generation
            .store(generation, Ordering::SeqCst);
        let authority = ObserverLifecycleAuthority {
            state: authority_state,
            generation,
        };
        observer.bind_lifecycle_authority(authority.clone());
        let restart_observer = observer.recreate_for_restart();
        // The dispatcher owns observer generation identity.  Publishing the new identity before
        // its worker starts fences a detached predecessor without requiring callers to infer or
        // synchronize observer lifecycle state themselves.
        lock_state_generation(&state, generation);
        let (next_sequence, pending) = handoff.map_or_else(
            || (0, VecDeque::new()),
            |handoff| (handoff.next_sequence, handoff.pending),
        );
        let queue = Arc::new((
            Mutex::new(ConsensusProgressObserverQueue {
                next_sequence,
                pending,
                ..ConsensusProgressObserverQueue::default()
            }),
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
                        while queue.pending.is_empty() && !queue.shutdown {
                            queue = signal
                                .wait(queue)
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                        }
                        if queue.shutdown {
                            break;
                        }
                        queue
                            .pending
                            .pop_front()
                            .expect("pending observer snapshot")
                    };

                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        observer
                            .observe_consensus_progress(&pending.snapshot, pending.observed_at_ms)
                    }))
                    .unwrap_or_else(|_| {
                        Err(NodeConsensusProgressObserverError::new(
                            "consensus progress observer panicked",
                        ))
                    });
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
                            .back()
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
            authority,
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
        let next_sequence = queue.next_sequence;
        let pending = std::mem::take(&mut queue.pending);
        drop(queue);
        if self
            .authority
            .state
            .active_generation
            .load(Ordering::SeqCst)
            == self.generation
        {
            self.authority
                .state
                .active_generation
                .store(u64::MAX, Ordering::SeqCst);
        }
        signal.notify_all();

        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        } else {
            // External observer code can block forever; detaching keeps stop bounded.
            let _ = self.worker.take();
        }
        self.restart_observer.take().map(|observer| {
            Box::new(RestartHandoffObserver {
                observer,
                handoff: Some(ObserverRestartHandoff {
                    next_sequence,
                    pending,
                    authority_state: Arc::clone(&self.authority.state),
                }),
            }) as Box<dyn NodeConsensusProgressObserver>
        })
    }
}

fn lock_state_generation(state: &Arc<Mutex<RuntimeState>>, generation: u64) {
    let mut current = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    current.generation = generation;
}

impl ConsensusProgressObserverSubmitter {
    #[cfg(test)]
    pub(super) fn current_sequence_for_test(&self) -> u64 {
        let (queue_lock, _) = &*self.queue;
        queue_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .next_sequence
    }

    pub(super) fn submit(&self, snapshot: NodeConsensusSnapshot, observed_at_ms: i64) {
        let (queue_lock, signal) = &*self.queue;
        // This lock covers only bounded queue bookkeeping. Waiting for a concurrent worker
        // dequeue must not discard a semantic snapshot during startup; observer and state work
        // remain outside this critical section.
        let mut queue = queue_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if queue.shutdown {
            return;
        }
        queue.next_sequence = queue.next_sequence.saturating_add(1);
        let sequence = queue.next_sequence;
        let coalesced = queue.enqueue(PendingConsensusProgress {
            sequence,
            snapshot,
            observed_at_ms,
        });
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
            Some(NodeConsensusProgressObserverError::coded(
                "consensus_progress_observer_backpressure",
                CONSENSUS_PROGRESS_OBSERVER_BACKPRESSURE,
            ));
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
        ) -> Result<(), NodeConsensusProgressObserverError> {
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
            Arc::clone(&state),
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

    #[test]
    fn submit_waits_for_queue_bookkeeping_instead_of_losing_a_snapshot() {
        let state = Arc::new(Mutex::new(RuntimeState::default()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let dispatcher = ConsensusProgressObserverDispatcher::spawn(
            "queue-lock-contention",
            Arc::clone(&state),
            0,
            Box::new(PermanentlyBlockingObserver {
                entered: entered_tx,
                release: release_rx,
            }),
        )
        .expect("spawn observer dispatcher");
        let submitter = dispatcher.submitter();
        let (started_tx, started_rx) = mpsc::channel();
        let (submitted_tx, submitted_rx) = mpsc::channel();
        let queue_guard = dispatcher.queue.0.lock().expect("lock observer queue");
        let submit_thread = thread::spawn(move || {
            started_tx.send(()).expect("signal submit start");
            submitter.submit(NodeConsensusSnapshot::default(), 1);
            submitted_tx.send(()).expect("signal submit completion");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("submit thread started");
        assert!(
            submitted_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "submit unexpectedly bypassed queue bookkeeping while the queue lock was held"
        );
        drop(queue_guard);
        submitted_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("submit completed after queue lock release");
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("observer received the snapshot after queue lock release");
        submit_thread.join().expect("join submit thread");
        drop(dispatcher);
    }

    #[test]
    fn revoked_durable_mutation_cannot_enter_final_commit() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-revoked-observer-publication-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create revoked publication fixture");
        let staged = root.join("state.tmp");
        let target = root.join("state.json");
        fs::write(&staged, b"stale").expect("write staged stale publication");
        fs::write(&target, b"current").expect("write current publication");
        let state = Arc::new(ObserverLifecycleAuthorityState {
            active_generation: AtomicU64::new(0),
            commit_lock: Mutex::new(()),
        });
        let authority = ObserverLifecycleAuthority {
            state: Arc::clone(&state),
            generation: 0,
        };
        let mutation = authority
            .acquire_durable_mutation()
            .expect("active generation starts durable work");

        state.active_generation.store(1, Ordering::SeqCst);
        let error = mutation
            .publish_staged_file(&staged, &target)
            .expect_err("revoked generation must not publish staged state");
        assert_eq!(error.code(), "observer_lifecycle_revoked");
        assert_eq!(
            fs::read(&target).expect("read current publication"),
            b"current",
            "a generation revoked during temporary-file work reached final commit"
        );
        fs::remove_dir_all(root).expect("remove revoked publication fixture");
    }

    #[test]
    fn poisoned_publication_state_fails_closed_without_replacing_target() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-poisoned-observer-publication-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create poisoned publication fixture");
        let staged = root.join("state.tmp");
        let target = root.join("state.json");
        fs::write(&staged, b"replacement").expect("write staged replacement");
        fs::write(&target, b"last-good").expect("write last-good publication");
        let state = Arc::new(ObserverLifecycleAuthorityState {
            active_generation: AtomicU64::new(0),
            commit_lock: Mutex::new(()),
        });
        let poison_state = Arc::clone(&state);
        let _ = thread::spawn(move || {
            let _commit = poison_state
                .commit_lock
                .lock()
                .expect("lock publication state before poison");
            panic!("poison publication state");
        })
        .join();
        let authority = ObserverLifecycleAuthority {
            state,
            generation: 0,
        };
        let mutation = authority
            .acquire_durable_mutation()
            .expect("active generation starts durable work");

        let error = mutation
            .publish_staged_file(&staged, &target)
            .expect_err("poisoned publication state must fail closed");
        assert_eq!(error.code(), "observer_lifecycle_commit_poisoned");
        assert_eq!(
            fs::read(&target).expect("read last-good publication"),
            b"last-good",
            "poisoned publication state replaced the last-good target"
        );
        fs::remove_dir_all(root).expect("remove poisoned publication fixture");
    }

    #[test]
    fn pending_queue_is_bounded_ordered_and_preserves_equal_head_checkpoints() {
        let cases = [
            (
                "coalesces_lagging_snapshots_to_the_latest",
                &[(2, false), (3, false), (4, false)][..],
                &[(2, false), (4, false)][..],
            ),
            (
                "keeps_an_equal_head_checkpoint_before_the_latest_lag",
                &[(2, false), (3, true), (4, false)][..],
                &[(3, true), (4, false)][..],
            ),
            (
                "never_reorders_a_new_equal_head_behind_a_queued_lag",
                &[(2, true), (3, false), (4, true)][..],
                &[(2, true), (4, true)][..],
            ),
        ];

        for (name, inputs, expected) in cases {
            let mut queue = ConsensusProgressObserverQueue::default();
            for (sequence, equal_head) in inputs {
                let committed_height: u64 = *sequence;
                let network_committed_height = if *equal_head {
                    committed_height
                } else {
                    committed_height.saturating_sub(1)
                };
                queue.enqueue(PendingConsensusProgress {
                    sequence: *sequence,
                    snapshot: NodeConsensusSnapshot {
                        committed_height,
                        network_committed_height,
                        ..NodeConsensusSnapshot::default()
                    },
                    observed_at_ms: *sequence as i64,
                });

                assert!(
                    queue.pending.len() <= CONSENSUS_PROGRESS_OBSERVER_PENDING_LIMIT,
                    "{name}: queue exceeded its fixed pending bound"
                );
                assert!(
                    queue
                        .pending
                        .iter()
                        .zip(queue.pending.iter().skip(1))
                        .all(|(earlier, later)| earlier.sequence < later.sequence),
                    "{name}: queue reordered retained snapshots"
                );
            }

            let retained = queue
                .pending
                .iter()
                .map(|pending| {
                    (
                        pending.sequence,
                        pending.snapshot.committed_height
                            == pending.snapshot.network_committed_height,
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(retained, expected, "{name}: unexpected retained snapshots");
        }
    }
}
