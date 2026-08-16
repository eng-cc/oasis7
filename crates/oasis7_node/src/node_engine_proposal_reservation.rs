use super::*;

impl PosNodeEngine {
    pub(super) fn pending_action_reservation_bytes(&self) -> usize {
        if self.pending_consensus_action_reservation_owned {
            return self.pending_consensus_action_reservation_bytes;
        }

        // Unit fixtures from before proposal-byte accounting assigned `pending`
        // directly after reserving the exact payload total. Keep that test-only
        // compatibility path narrow; production ownership is always explicit.
        #[cfg(test)]
        {
            let pending_bytes = self
                .pending
                .as_ref()
                .map(|proposal| action_payload_bytes(proposal.committed_actions.iter()))
                .unwrap_or(0);
            if pending_bytes > 0
                && self
                    .pending_consensus_action_queue_bytes
                    .load(std::sync::atomic::Ordering::Acquire)
                    == pending_bytes
            {
                return pending_bytes;
            }
        }
        0
    }

    pub(super) fn clear_pending_action_reservation(&mut self) -> Result<(), NodeError> {
        let reservation_bytes = self.pending_action_reservation_bytes();
        if reservation_bytes > 0 {
            let current = self
                .pending_consensus_action_queue_bytes
                .load(std::sync::atomic::Ordering::Acquire);
            if current < reservation_bytes {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "pending proposal reservation underflow: current={} owned={}",
                        current, reservation_bytes
                    ),
                });
            }
            release_action_payload_bytes(
                &self.pending_consensus_action_queue_bytes,
                reservation_bytes,
            );
        }
        self.pending_consensus_action_reservation_bytes = 0;
        self.pending_consensus_action_reservation_owned = false;
        Ok(())
    }

    pub(super) fn set_pending_action_reservation(&mut self, reservation_bytes: usize) {
        self.pending_consensus_action_reservation_bytes = reservation_bytes;
        self.pending_consensus_action_reservation_owned = reservation_bytes > 0;
    }

    pub(super) fn clear_pending_state_reservations(&mut self) -> Result<(), NodeError> {
        let proposal_bytes = self.pending_action_reservation_bytes();
        let queued_bytes = action_payload_bytes(self.pending_consensus_actions.values());
        let owned_bytes =
            proposal_bytes
                .checked_add(queued_bytes)
                .ok_or_else(|| NodeError::Consensus {
                    reason: "pending state reservation byte count overflow".to_string(),
                })?;
        if owned_bytes > 0 {
            let current = self
                .pending_consensus_action_queue_bytes
                .load(std::sync::atomic::Ordering::Acquire);
            if current < owned_bytes {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "pending state reservation underflow: current={} owned={}",
                        current, owned_bytes
                    ),
                });
            }
            release_action_payload_bytes(&self.pending_consensus_action_queue_bytes, owned_bytes);
        }
        self.pending_consensus_action_reservation_bytes = 0;
        self.pending_consensus_action_reservation_owned = false;
        self.pending_consensus_actions.clear();
        Ok(())
    }

    pub(super) fn validate_inbound_proposal_actions(
        &self,
        actions: &[NodeConsensusAction],
    ) -> Result<usize, NodeError> {
        if actions.len() > self.max_pending_consensus_actions {
            return Err(NodeError::Consensus {
                reason: format!(
                    "inbound consensus proposal action count exceeds limit: count={} limit={}",
                    actions.len(),
                    self.max_pending_consensus_actions
                ),
            });
        }
        actions.iter().try_fold(0usize, |total, action| {
            if action.payload_cbor.len() > self.max_consensus_action_payload_bytes {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "inbound consensus proposal action payload too large: bytes={} limit={}",
                        action.payload_cbor.len(),
                        self.max_consensus_action_payload_bytes
                    ),
                });
            }
            action.validate().map_err(node_consensus_error)?;
            total
                .checked_add(action.payload_cbor.len())
                .ok_or_else(|| NodeError::Consensus {
                    reason: "inbound consensus proposal payload byte count overflow".to_string(),
                })
        })
    }

    pub(super) fn adjust_pending_proposal_reservation(
        &mut self,
        incoming_bytes: usize,
    ) -> Result<(), NodeError> {
        let existing_bytes = self.pending_action_reservation_bytes();
        let mut current = self
            .pending_consensus_action_queue_bytes
            .load(std::sync::atomic::Ordering::Acquire);
        loop {
            if current < existing_bytes || current > self.max_pending_consensus_action_queue_bytes {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "pending proposal reservation invalid: current={} existing={} limit={}",
                        current, existing_bytes, self.max_pending_consensus_action_queue_bytes
                    ),
                });
            }
            let projected = current
                .checked_sub(existing_bytes)
                .and_then(|available| available.checked_add(incoming_bytes))
                .ok_or_else(|| NodeError::Consensus {
                    reason: "pending proposal reservation byte count overflow".to_string(),
                })?;
            if projected > self.max_pending_consensus_action_queue_bytes {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "pending proposal reservation exceeds byte budget: current={} existing={} incoming={} limit={}",
                        current,
                        existing_bytes,
                        incoming_bytes,
                        self.max_pending_consensus_action_queue_bytes
                    ),
                });
            }
            match self.pending_consensus_action_queue_bytes.compare_exchange(
                current,
                projected,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        Ok(())
    }
}
