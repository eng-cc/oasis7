use std::sync::atomic::Ordering;

use crate::{NodeError, NodeRuntime};

impl NodeRuntime {
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
