//! Signed, read-only native factory selection bound to one captured private task.
use super::private_goal::verify_owner_signature_strict;
use super::task_context::{TaskContextGetV1, TaskPublicContextV1, verify_get};
use super::{CommitRef, SignedReadRequest};
use oasis7_agent_api::game::TaskContextIdentityV1;
use oasis7_agent_api::{
    world_cognition::RuntimeCognitionBaseBindingV1, world_quote::NativeBuiltinFactoryQuote,
};
use serde::{Deserialize, Serialize};

pub const TASK_FACTORY_QUOTE_PATH: &str = "/v1/world/private-task/factory-quote";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskFactoryQuoteRequestV1 {
    pub schema_revision: u32,
    pub context_proof: SignedReadRequest<TaskContextGetV1>,
    pub expected_base_binding: RuntimeCognitionBaseBindingV1,
    pub factory_id: String,
    pub factory_kind: String,
    pub site_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskFactoryQuoteV1 {
    pub schema_revision: u32,
    pub context: TaskContextIdentityV1,
    pub read_commit: CommitRef,
    pub current_public_context: TaskPublicContextV1,
    pub quote: NativeBuiltinFactoryQuote,
}

pub fn verify_factory_quote_request(
    original: &SignedReadRequest<TaskFactoryQuoteRequestV1>,
) -> Result<(), String> {
    let request = &original.request;
    if request.schema_revision != 1
        || request.site_id.trim().is_empty()
        || request.factory_id.trim().is_empty()
        || request.factory_kind.trim().is_empty()
        || original.subject_public_key != request.context_proof.subject_public_key
    {
        return Err("invalid task factory quote request".into());
    }
    verify_get(&request.context_proof)?;
    request
        .expected_base_binding
        .validate()
        .map_err(|e| e.to_string())?;
    let fence = &request.context_proof.request.context.fence;
    let base = &request.expected_base_binding;
    if base.world_id != fence.world_id
        || base.branch_id != fence.branch_id
        || base.reorg_epoch != fence.reorg_epoch.0
    {
        return Err("quote base differs from task history".into());
    }
    verify_owner_signature_strict(
        TASK_FACTORY_QUOTE_PATH,
        request,
        &original.subject_public_key,
        &original.signature_hex,
    )
}
impl TaskFactoryQuoteV1 {
    pub fn validate_against(
        &self,
        original: &SignedReadRequest<TaskFactoryQuoteRequestV1>,
    ) -> Result<(), String> {
        verify_factory_quote_request(original)?;
        let request = &original.request;
        if self.schema_revision != 1
            || self.context != request.context_proof.request.context
            || self.read_commit != self.current_public_context.read_commit
            || self.current_public_context.captured_base_binding != request.expected_base_binding
            || self.quote.quoted_at_tick != request.expected_base_binding.base_tick
        {
            return Err("task factory quote binding mismatch".into());
        }
        self.current_public_context.validate_for(&self.context)?;
        self.quote.validate_selection(
            &self.context.fence.agent_id,
            &request.site_id,
            &request.factory_kind,
            &request.factory_id,
        )
    }
}
