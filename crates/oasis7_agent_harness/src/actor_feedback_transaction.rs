use std::sync::{Arc, Condvar, Mutex};

use oasis7_agent_api::FeedbackEnvelopeV1;

enum FeedbackDeliveryState {
    Pending,
    Commit(FeedbackEnvelopeV1),
    Abort,
}

pub(super) struct FeedbackDeliveryGate {
    state: Mutex<FeedbackDeliveryState>,
    ready: Condvar,
}

impl FeedbackDeliveryGate {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(FeedbackDeliveryState::Pending),
            ready: Condvar::new(),
        }
    }

    pub(super) fn commit(&self, feedback: FeedbackEnvelopeV1) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state = FeedbackDeliveryState::Commit(feedback);
        self.ready.notify_one();
    }

    pub(super) fn abort(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state = FeedbackDeliveryState::Abort;
        self.ready.notify_one();
    }

    pub(super) fn wait(&self) -> Option<FeedbackEnvelopeV1> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            match &*state {
                FeedbackDeliveryState::Pending => {
                    state = self
                        .ready
                        .wait(state)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                FeedbackDeliveryState::Commit(_) => {
                    let FeedbackDeliveryState::Commit(feedback) =
                        std::mem::replace(&mut *state, FeedbackDeliveryState::Abort)
                    else {
                        unreachable!();
                    };
                    return Some(feedback);
                }
                FeedbackDeliveryState::Abort => return None,
            }
        }
    }
}

pub(super) struct AbortOnDrop {
    gate: Arc<FeedbackDeliveryGate>,
    committed: bool,
}

impl AbortOnDrop {
    pub(super) fn new(gate: Arc<FeedbackDeliveryGate>) -> Self {
        Self {
            gate,
            committed: false,
        }
    }

    pub(super) fn commit(mut self, feedback: FeedbackEnvelopeV1) {
        self.gate.commit(feedback);
        self.committed = true;
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if !self.committed {
            self.gate.abort();
        }
    }
}
