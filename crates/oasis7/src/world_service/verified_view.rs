use super::projection::WorldServiceProjection;
use oasis7_client_api::world_service::{
    EventCursor, ProjectionVersion, ReadWorldViewRequest, ReadWorldViewResponse,
};

/// Constructible only by the authenticated transport, after request-bound
/// service signature and structural validation. This is a service assertion,
/// not independent consensus-proof verification.
#[derive(Debug, Clone)]
pub struct VerifiedWorldView {
    response: ReadWorldViewResponse<WorldServiceProjection>,
}

impl VerifiedWorldView {
    pub(crate) fn new(
        response: ReadWorldViewResponse<WorldServiceProjection>,
        request: &ReadWorldViewRequest,
    ) -> Result<Self, String> {
        response.validate(request).map_err(|e| e.to_string())?;
        if response.view.state.time != response.logical_tick {
            return Err("projection tick differs from declared logical tick".into());
        }
        if let Some(binding) = &response.view.runtime_binding {
            binding.validate().map_err(|e| format!("{e:?}"))?;
            let expected = &response.version.commit.binding;
            if binding.world_id != expected.provider_world_id
                || binding.branch_id != expected.branch_id
                || binding.reorg_epoch != expected.reorg_generation
                || binding.base_tick != response.logical_tick
                || binding.runtime_manifest_hash.to_string() != expected.governing_manifest_ref
                || super::authority::request_digest(
                    "finality",
                    &(
                        binding.finality_epoch,
                        &binding.finality_status,
                        &binding.finality_block_hash,
                    ),
                )? != expected.finality_ref
            {
                return Err("projection runtime binding differs from committed view".into());
            }
        }
        let agent = request.scope_id.strip_prefix("agent:");
        if response
            .view
            .agent_context
            .as_ref()
            .is_some_and(|context| Some(context.agent_id.as_str()) != agent)
            || response
                .view
                .scheduler_wakes
                .iter()
                .any(|wake| Some(wake.agent_id.as_str()) != agent)
            || response
                .view
                .continuations
                .iter()
                .any(|entry| Some(entry.agent_id.as_str()) != agent)
            || response
                .view
                .cognition_leases
                .iter()
                .any(|lease| Some(lease.agent_id.as_str()) != agent)
        {
            return Err("Agent observation differs from authorized visibility scope".into());
        }
        Ok(Self { response })
    }
    pub fn version(&self) -> &ProjectionVersion {
        &self.response.version
    }
    pub fn continuation(&self) -> &EventCursor {
        &self.response.continuation
    }
    pub fn logical_tick(&self) -> u64 {
        self.response.logical_tick
    }
    pub fn projection(&self) -> &WorldServiceProjection {
        &self.response.view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_client_api::world_service::{
        CommitRef, ExecutionBinding, WORLD_SERVICE_CONTRACT_VERSION, WorldIdentity,
    };

    fn fixture() -> (
        ReadWorldViewRequest,
        ReadWorldViewResponse<WorldServiceProjection>,
    ) {
        let world = crate::runtime::World::new_production_hardened();
        let identity = WorldIdentity {
            world_id: "world".into(),
            genesis_digest: "genesis".into(),
        };
        let commit = CommitRef {
            world: identity.clone(),
            binding: ExecutionBinding {
                provider_world_id: "provider".into(),
                branch_id: "branch".into(),
                finality_ref: "finality".into(),
                reorg_generation: 0,
                governing_manifest_ref: "manifest".into(),
                authority_generation: 1,
                permission_generation: 1,
            },
            position: 7,
            execution_block_hash: "block".into(),
            state_root_ref: "root".into(),
        };
        let request = ReadWorldViewRequest {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            world: identity,
            scope_id: "public".into(),
            min_commit: Some(commit.clone()),
            fixed_commit: None,
            deadline_unix_ms: None,
        };
        let mut projection = WorldServiceProjection::from_world(&world, None).unwrap();
        projection.runtime_binding = None;
        let response = ReadWorldViewResponse {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            version: ProjectionVersion {
                commit: commit.clone(),
                projection_revision: "projection".into(),
                visibility_scope: "public".into(),
            },
            logical_tick: world.state().time,
            continuation: EventCursor {
                stream_id: "events".into(),
                scope_id: "public".into(),
                era: 0,
                sequence: 3,
                commit,
            },
            view: projection,
        };
        (request, response)
    }

    #[test]
    fn view_rejects_tick_cursor_world_and_generation_drift() {
        let (request, response) = fixture();
        assert!(VerifiedWorldView::new(response.clone(), &request).is_ok());
        let mut tick = response.clone();
        tick.view.state.time += 1;
        assert!(VerifiedWorldView::new(tick, &request).is_err());
        let mut cursor = response.clone();
        cursor.continuation.commit.position += 1;
        assert!(VerifiedWorldView::new(cursor, &request).is_err());
        let mut world = response.clone();
        world.version.commit.world.genesis_digest = "other-genesis".into();
        world.continuation.commit = world.version.commit.clone();
        assert!(VerifiedWorldView::new(world, &request).is_err());
        let mut generation = response;
        generation.version.commit.binding.permission_generation += 1;
        generation.continuation.commit = generation.version.commit.clone();
        assert!(VerifiedWorldView::new(generation, &request).is_err());
    }
}
