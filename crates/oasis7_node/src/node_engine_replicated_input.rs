use super::*;

impl PosNodeEngine {
    pub(super) fn discard_replicated_execution_inputs_through_height(
        &mut self,
        committed_height: u64,
    ) -> Result<(), NodeError> {
        let mut stale_action_ids = Vec::new();
        let mut stale_payload_bytes = 0usize;
        for (action_id, action) in &self.pending_consensus_actions {
            let decoded_input = decode_replicated_execution_input_action(action).map_err(
                |reason| NodeError::Consensus {
                    reason: format!(
                        "decode queued replicated execution input failed action_id={action_id}: {reason}"
                    ),
                },
            )?;
            if (action.action_id == REPLICATED_EXECUTION_INPUT_ACTION_ID
                || action.submitter_player_id == REPLICATED_EXECUTION_INPUT_SUBMITTER)
                && decoded_input.is_none()
            {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "reserved replicated execution input identity is malformed action_id={action_id} submitter={}",
                        action.submitter_player_id
                    ),
                });
            }
            let Some(input) = decoded_input else {
                continue;
            };
            if input.target_height == 0 {
                return Err(NodeError::Consensus {
                    reason: format!(
                        "queued replicated execution input is not height-bound at committed height {}",
                        committed_height
                    ),
                });
            }
            if input.target_height <= committed_height {
                stale_action_ids.push(*action_id);
                stale_payload_bytes = stale_payload_bytes
                    .checked_add(action.payload_cbor.len())
                    .ok_or_else(|| NodeError::Consensus {
                        reason: "queued replicated execution input payload byte count overflow"
                            .to_string(),
                    })?;
            }
        }
        for action_id in stale_action_ids {
            self.pending_consensus_actions.remove(&action_id);
        }
        if stale_payload_bytes > 0 {
            release_action_payload_bytes(
                &self.pending_consensus_action_queue_bytes,
                stale_payload_bytes,
            );
        }
        Ok(())
    }
}
