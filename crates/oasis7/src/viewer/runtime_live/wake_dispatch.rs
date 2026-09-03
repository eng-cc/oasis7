use super::*;
use crate::runtime::{CognitionWakeDispositionV1, ContinuationStatusV1};

pub(super) fn ensure_viewer_runtime_binding(
    world: &mut RuntimeWorld,
    config: &ViewerRuntimeLiveServerConfig,
) -> Result<(), ViewerRuntimeLiveServerError> {
    #[cfg(test)]
    let binding_override = config.test_cognition_runtime_binding.as_ref().map(
        |(branch_id, finality_epoch, finality_block_hash, finality_status, reorg_epoch)| {
            (
                branch_id.as_str(),
                *finality_epoch,
                finality_block_hash.clone(),
                finality_status.as_str(),
                *reorg_epoch,
            )
        },
    );
    #[cfg(not(test))]
    let binding_override = None;
    let world_id = config.world_id.as_str();
    match world.cognition().get("runtime_binding") {
        None | Some(serde_json::Value::Null) => {
            let (branch_id, finality_epoch, finality_block_hash, finality_status, reorg_epoch) =
                binding_override.unwrap_or(("main", 0, None, "pending", 0));
            world
                .bind_cognition_runtime(
                    world_id,
                    branch_id,
                    finality_epoch,
                    finality_block_hash,
                    finality_status,
                    reorg_epoch,
                )
                .map_err(ViewerRuntimeLiveServerError::Runtime)
        }
        Some(_) => {
            let binding = world
                .current_cognition_runtime_binding()
                .map_err(ViewerRuntimeLiveServerError::Runtime)?;
            if binding.world_id != world_id {
                return Err(ViewerRuntimeLiveServerError::Init(format!(
                    "Viewer Runtime binding world_id mismatch: expected {world_id}, got {}",
                    binding.world_id
                )));
            }
            Ok(())
        }
    }
}

impl ViewerRuntimeLiveServer {
    pub(super) fn handoff_runtime_wake_for_agent(
        &mut self,
        agent_id: &str,
        status: ContinuationStatusV1,
        reason: &str,
    ) -> Result<bool, String> {
        let Some(wake_id) = self
            .llm_sidecar
            .pending_runtime_wake_id_for_agent(agent_id)
            .map(str::to_string)
        else {
            return Ok(false);
        };
        self.world
            .consume_cognition_wake(&wake_id, |_wake| {
                Ok(CognitionWakeDispositionV1::Terminal {
                    status,
                    reason: reason.to_string(),
                })
            })
            .map_err(|error| format!("Runtime cognition wake handoff failed: {error:?}"))?;
        self.llm_sidecar.clear_runtime_wake(&wake_id);
        Ok(true)
    }

    pub(super) fn sync_runtime_wake_projection(
        &mut self,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        self.llm_sidecar
            .sync_runtime_wakes(&self.world)
            .map_err(ViewerRuntimeLiveServerError::Init)
    }
}
