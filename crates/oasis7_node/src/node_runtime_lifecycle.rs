use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::consensus_progress_observer::ConsensusProgressObserverDispatcher;
use crate::runtime_util::lock_state;
use crate::{NodeError, NodeRuntime};

impl NodeRuntime {
    pub(super) fn start_consensus_progress_observer_dispatcher(&mut self) -> Result<(), NodeError> {
        if self.consensus_progress_observer_dispatcher.is_some() {
            return Ok(());
        }
        let Some(observer) = self.consensus_progress_observer.take() else {
            return Ok(());
        };
        let generation = lock_state(&self.state).generation;
        #[cfg(test)]
        let fail_observer_spawn = std::mem::take(&mut self.fail_next_observer_worker_spawn);
        #[cfg(test)]
        let dispatcher = if fail_observer_spawn {
            ConsensusProgressObserverDispatcher::fail_spawn_for_test(observer)
        } else {
            ConsensusProgressObserverDispatcher::spawn(
                self.config.node_id.as_str(),
                Arc::clone(&self.state),
                generation,
                observer,
            )
        };
        #[cfg(not(test))]
        let dispatcher = ConsensusProgressObserverDispatcher::spawn(
            self.config.node_id.as_str(),
            Arc::clone(&self.state),
            generation,
            observer,
        );
        match dispatcher {
            Ok(dispatcher) => {
                self.consensus_progress_observer_dispatcher = Some(dispatcher);
                Ok(())
            }
            Err(error) => {
                let reason = error.to_string();
                self.consensus_progress_observer = Some(error.into_observer());
                self.running.store(false, Ordering::SeqCst);
                Err(NodeError::ThreadSpawnFailed { reason })
            }
        }
    }

    pub(super) fn restore_consensus_progress_observer_after_start_failure(&mut self) {
        if let Some(mut dispatcher) = self.consensus_progress_observer_dispatcher.take() {
            self.consensus_progress_observer = dispatcher.shutdown();
        }
    }

    #[cfg(test)]
    pub(super) fn fail_next_worker_spawn_for_test(&mut self) {
        self.fail_next_worker_spawn = true;
    }

    #[cfg(test)]
    pub(super) fn fail_next_observer_worker_spawn_for_test(&mut self) {
        self.fail_next_observer_worker_spawn = true;
    }

    pub fn stop(&mut self) -> Result<(), NodeError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(NodeError::NotRunning {
                node_id: self.config.node_id.clone(),
            });
        }
        let (_, committed_signal) = &*self.committed_action_batches;
        committed_signal.notify_all();
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        let join_result = if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| NodeError::ThreadJoinFailed {
                node_id: self.config.node_id.clone(),
            })
        } else {
            Ok(())
        };
        self.shutdown_consensus_progress_observer();
        // Release sockets and flags even when the main worker panicked.
        self.gossip_endpoint = None;
        self.running.store(false, Ordering::SeqCst);
        join_result
    }

    fn shutdown_consensus_progress_observer(&mut self) {
        if let Some(mut dispatcher) = self.consensus_progress_observer_dispatcher.take() {
            if self.consensus_progress_observer.is_none() {
                self.consensus_progress_observer = dispatcher.shutdown();
            } else {
                let _ = dispatcher.shutdown();
            }
        }
    }
}

impl Drop for NodeRuntime {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            if let Some(stop_tx) = self.stop_tx.take() {
                let _ = stop_tx.send(());
            }
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
        self.shutdown_consensus_progress_observer();
        self.running.store(false, Ordering::SeqCst);
    }
}
