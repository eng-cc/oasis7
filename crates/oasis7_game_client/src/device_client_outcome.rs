use super::*;

impl Client {
    /// Settle only the original context with an explicit public terminal reason.
    /// An existing action always retains this operation's identity.
    pub fn outcome(&self, operation: &str, reason: &str) -> Result<IntentResponse<Value>, String> {
        if ![
            "executor_aborted",
            "provider_failed",
            "transport_exhausted",
            "context_expired",
        ]
        .contains(&reason)
        {
            return Err("unsupported context outcome".into());
        }
        if self
            .cached_intent(ClientStateKind::Action, operation)?
            .is_some()
        {
            return Err("context has an original action; recover that result instead".into());
        }
        let request = if let Some(request) =
            self.cached_intent(ClientStateKind::Outcome, operation)?
        {
            let WorldServicePayloadV1::Scheduler(signed) = &request.signed_payload else {
                return Err("invalid cached outcome".into());
            };
            let SchedulerOperationV1::TaskProviderFailure {
                reason: original, ..
            } = &signed.request.operation
            else {
                return Err("invalid cached outcome".into());
            };
            if original != reason {
                return Err("context already has a different original outcome".into());
            }
            request
        } else {
            let intent = if reason == "context_expired" {
                let prefix = self
                    .cached_intent(ClientStateKind::Prefix, operation)?
                    .ok_or("original context prefix unavailable")?;
                let WorldServicePayloadV1::Scheduler(signed) = prefix.signed_payload else {
                    return Err("invalid original prefix".into());
                };
                let SchedulerOperationV1::TaskProviderPrefix { context } = signed.request.operation
                else {
                    return Err("invalid original prefix".into());
                };
                let view = self.public_view()?;
                let binding = view
                    .projection()
                    .runtime_binding
                    .as_ref()
                    .ok_or("current public Runtime binding unavailable")?;
                let base = oasis7_agent_api::world_cognition::RuntimeCognitionBaseBindingV1 {
                    world_id: binding.world_id.clone(),
                    branch_id: binding.branch_id.clone(),
                    finality_epoch: binding.finality_epoch,
                    finality_block_hash: binding
                        .finality_block_hash
                        .as_ref()
                        .map(ToString::to_string),
                    finality_status: binding.finality_status.clone(),
                    base_tick: binding.base_tick,
                    base_world_hash: binding.base_world_hash.to_string(),
                    reorg_epoch: binding.reorg_epoch,
                    runtime_manifest_hash: binding.runtime_manifest_hash.to_string(),
                };
                public_outcome_intent(&context, base, reason)?
            } else {
                outcome_intent(&self.captured(operation)?, reason)?
            };
            let signed = authority::sign_read_request("scheduler", intent, &self.key_hex)?;
            self.cache_intent(
                ClientStateKind::Outcome,
                operation,
                WorldServicePayloadV1::Scheduler(signed),
            )?
        };
        self.submit_and_poll(&request)
    }
}
