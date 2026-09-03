use super::super::cognition_persistence_validation::strict_optional_finality_hash;
use super::World;
use crate::runtime::cognition_retention::CognitionRetentionStore;
use crate::runtime::cognition_wake::{AgentContinuation, CognitionContinuationProposalV1};
use crate::runtime::error::WorldError;
use serde_json::Value as JsonValue;

impl World {
    pub(super) fn bind_cognition_proposal_fields(
        &mut self,
        proposal: &mut CognitionContinuationProposalV1,
    ) -> Result<(), WorldError> {
        let manifest_hash = self.current_manifest_hash()?;
        if proposal.runtime_manifest_hash.is_empty() {
            proposal.runtime_manifest_hash = manifest_hash.clone();
        } else if proposal.runtime_manifest_hash != manifest_hash {
            return Err(super::cognition_validation_error(
                "runtime_manifest_mismatch",
            ));
        }
        let Some(binding) = self.cognition.get("runtime_binding") else {
            return Err(super::cognition_validation_error(
                "runtime_binding_required",
            ));
        };
        if proposal.branch_id.is_empty() {
            proposal.branch_id = binding["branch_id"].as_str().unwrap_or("main").to_string();
        }
        if proposal.finality_status.is_empty() {
            proposal.finality_status = binding["finality_status"]
                .as_str()
                .unwrap_or("pending")
                .to_string();
        }
        if proposal.finality_epoch == 0 {
            proposal.finality_epoch = binding["finality_epoch"].as_u64().unwrap_or_default();
        }
        if proposal.reorg_epoch == 0 {
            proposal.reorg_epoch = binding["reorg_epoch"].as_u64().unwrap_or_default();
        }
        if proposal.finality_block_hash.is_none() {
            proposal.finality_block_hash =
                strict_optional_finality_hash(binding.get("finality_block_hash"))?;
        }
        let block_hash = strict_optional_finality_hash(binding.get("finality_block_hash"))?;
        if binding["world_id"].as_str() != Some(proposal.world_id.as_str())
            || binding["branch_id"].as_str() != Some(proposal.branch_id.as_str())
            || binding["finality_epoch"].as_u64() != Some(proposal.finality_epoch)
            || block_hash != proposal.finality_block_hash
            || binding["finality_status"].as_str() != Some(proposal.finality_status.as_str())
            || binding["reorg_epoch"].as_u64() != Some(proposal.reorg_epoch)
        {
            return Err(super::cognition_validation_error(
                "foreign_continuation_proposal",
            ));
        }
        if !self.state.agents.is_empty() && !self.state.agents.contains_key(&proposal.agent_id) {
            return Err(super::cognition_validation_error(
                "continuation_agent_missing",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_cognition_proposal_binding(
        &self,
        proposal: &CognitionContinuationProposalV1,
    ) -> Result<(), WorldError> {
        if proposal.runtime_manifest_hash != self.current_manifest_hash()? {
            return Err(super::cognition_validation_error(
                "runtime_manifest_mismatch",
            ));
        }
        if let Some(binding) = self.cognition.get("runtime_binding") {
            let block_hash = strict_optional_finality_hash(binding.get("finality_block_hash"))?;
            if binding["world_id"].as_str() != Some(proposal.world_id.as_str())
                || binding["branch_id"].as_str() != Some(proposal.branch_id.as_str())
                || binding["finality_epoch"].as_u64() != Some(proposal.finality_epoch)
                || block_hash != proposal.finality_block_hash
                || binding["finality_status"].as_str() != Some(proposal.finality_status.as_str())
                || binding["reorg_epoch"].as_u64() != Some(proposal.reorg_epoch)
            {
                return Err(super::cognition_validation_error(
                    "foreign_continuation_proposal",
                ));
            }
        }
        if !self.state.agents.is_empty() && !self.state.agents.contains_key(&proposal.agent_id) {
            return Err(super::cognition_validation_error(
                "continuation_agent_missing",
            ));
        }
        Ok(())
    }

    pub(super) fn cognition_turn_is_registered(
        &self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
        request_digest: &str,
    ) -> bool {
        self.cognition
            .get("cognition_journal")
            .and_then(|journal| journal.get("events"))
            .and_then(JsonValue::as_array)
            .is_some_and(|events| {
                events.iter().any(|event| {
                    matches!(
                        event.get("kind").and_then(JsonValue::as_str),
                        Some("TurnStarted")
                            | Some("ContextCaptured")
                            | Some("RequestDispatched")
                            | Some("DecisionValidated")
                    ) && event.get("agent_id").and_then(JsonValue::as_str) == Some(agent_id)
                        && event.get("agent_session_id").and_then(JsonValue::as_str)
                            == Some(agent_session_id)
                        && event.get("agent_turn_id").and_then(JsonValue::as_str)
                            == Some(agent_turn_id)
                        && event.get("decision_request_id").and_then(JsonValue::as_str)
                            == Some(decision_request_id)
                        // An initial turn matches its own request digest. A
                        // continuation wake creates a new logical request
                        // digest because the Runtime continuation context is
                        // part of the next Harness request identity; its
                        // TurnStarted event therefore carries the stable
                        // origin digest for admission correlation.
                        && (event.get("request_digest").and_then(JsonValue::as_str)
                            == Some(request_digest)
                            || event
                                .get("origin_request_digest")
                                .and_then(JsonValue::as_str)
                                == Some(request_digest))
                })
            })
    }

    pub(in crate::runtime::world) fn cognition_continuations_typed(
        &self,
    ) -> Result<Vec<AgentContinuation>, WorldError> {
        let value = self
            .cognition
            .get("continuations")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new()));
        serde_json::from_value(value).map_err(WorldError::from)
    }

    pub(super) fn cognition_set_continuations(
        &mut self,
        continuations: &[AgentContinuation],
    ) -> Result<(), WorldError> {
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert(
            "continuations".to_string(),
            serde_json::to_value(continuations).map_err(WorldError::from)?,
        );
        self.cognition = JsonValue::Object(projection);
        Ok(())
    }

    pub(super) fn cognition_retention_store(&self) -> Result<CognitionRetentionStore, WorldError> {
        let Some(value) = self.cognition.get("retention_state") else {
            return Ok(CognitionRetentionStore::default());
        };
        serde_json::from_value(value.clone()).map_err(WorldError::from)
    }

    pub(super) fn cognition_set_retention_store(
        &mut self,
        store: &CognitionRetentionStore,
    ) -> Result<(), WorldError> {
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert(
            "retention_state".to_string(),
            serde_json::to_value(store).map_err(WorldError::from)?,
        );
        self.cognition = JsonValue::Object(projection);
        Ok(())
    }
}
