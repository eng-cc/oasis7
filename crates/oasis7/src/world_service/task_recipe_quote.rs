//! Signed, read-only native recipe selection bound to one captured private task.
use super::private_goal_prepare::verify_owner_signature_strict;
use super::private_task_context::{TaskContextGetV1, TaskPublicContextV1, verify_get};
use super::{CommitRef, SignedReadRequest};
use crate::runtime::{NativeBuiltinRecipeQuote, RuntimeCognitionBaseBindingV1};
use oasis7_agent_api::game::TaskContextIdentityV1;
use serde::{Deserialize, Serialize};

pub const TASK_RECIPE_QUOTE_PATH: &str = "/v1/world/private-task/recipe-quote";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskRecipeQuoteRequestV1 {
    pub schema_revision: u32,
    pub context_proof: SignedReadRequest<TaskContextGetV1>,
    pub expected_base_binding: RuntimeCognitionBaseBindingV1,
    pub factory_id: String,
    pub recipe_id: String,
    pub batches: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskRecipeQuoteV1 {
    pub schema_revision: u32,
    pub context: TaskContextIdentityV1,
    pub read_commit: CommitRef,
    pub current_public_context: TaskPublicContextV1,
    pub quote: NativeBuiltinRecipeQuote,
}

pub fn verify_recipe_quote_request(
    original: &SignedReadRequest<TaskRecipeQuoteRequestV1>,
) -> Result<(), String> {
    let request = &original.request;
    if request.schema_revision != 1
        || request.batches == 0
        || request.factory_id.trim().is_empty()
        || request.recipe_id.trim().is_empty()
        || original.subject_public_key != request.context_proof.subject_public_key
    {
        return Err("invalid task recipe quote request".into());
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
        TASK_RECIPE_QUOTE_PATH,
        request,
        &original.subject_public_key,
        &original.signature_hex,
    )
}
impl TaskRecipeQuoteV1 {
    pub fn validate_against(
        &self,
        original: &SignedReadRequest<TaskRecipeQuoteRequestV1>,
    ) -> Result<(), String> {
        verify_recipe_quote_request(original)?;
        let request = &original.request;
        if self.schema_revision != 1
            || self.context != request.context_proof.request.context
            || self.read_commit != self.current_public_context.read_commit
            || self.current_public_context.captured_base_binding != request.expected_base_binding
            || self.quote.quoted_at_tick != request.expected_base_binding.base_tick
        {
            return Err("task recipe quote binding mismatch".into());
        }
        self.current_public_context.validate_for(&self.context)?;
        self.quote.validate_selection(
            &self.context.fence.agent_id,
            &request.factory_id,
            &request.recipe_id,
            request.batches,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_service::authority::sign_read_request;
    use crate::world_service::private_task_context::TASK_CONTEXT_READ_PATH;
    use oasis7_agent_api::game::{
        DecimalU64, GameExecutorBindingV1, GameGoalReferenceV1, GameTaskFenceV1, GameTaskStatusV1,
    };

    fn request() -> SignedReadRequest<TaskRecipeQuoteRequestV1> {
        let key = hex::encode([73; 32]);
        let public = sign_read_request("key", (), &key)
            .unwrap()
            .subject_public_key;
        let context = TaskContextIdentityV1 {
            schema_revision: 1,
            fence: GameTaskFenceV1 {
                world_id: "world".into(),
                genesis_digest: "genesis".into(),
                branch_id: "main".into(),
                reorg_epoch: DecimalU64(0),
                agent_id: "agent".into(),
                owner_binding: "owner".into(),
                owner_generation: DecimalU64(1),
                owner_public_key: public.clone(),
                task_id: "task".into(),
                task_revision: DecimalU64(3),
                task_status: GameTaskStatusV1::Active,
                latest_goal_revision: DecimalU64(1),
                active_goal_revision: DecimalU64(1),
                active_goal_reference: Some(GameGoalReferenceV1 {
                    goal_revision: DecimalU64(1),
                    goal_ref: "ab".repeat(32),
                    prepare_operation_id: "prepare".into(),
                    publish_operation_id: "publish".into(),
                    activation_committed_height: DecimalU64(4),
                }),
                executor_epoch: DecimalU64(1),
                executor_binding: Some(GameExecutorBindingV1 {
                    executor_id: "executor".into(),
                    executor_public_key: public,
                    delegation_generation: DecimalU64(1),
                    executor_epoch: DecimalU64(1),
                }),
            },
            agent_session_id: "session".into(),
            agent_turn_id: "turn".into(),
            decision_request_id: "decision".into(),
            retry_seq: 1,
            transport_attempt: 1,
            request_commitment: format!("blake3:{}", "11".repeat(32)),
            context_commitment: format!("blake3:{}", "22".repeat(32)),
            provider_invocation_commitment: format!("blake3:{}", "33".repeat(32)),
            valid_until_tick: DecimalU64(16),
        };
        let context_proof = sign_read_request(
            TASK_CONTEXT_READ_PATH,
            TaskContextGetV1 {
                schema_revision: 1,
                context,
                prepare_operation_id: "prepare-context".into(),
            },
            &key,
        )
        .unwrap();
        sign_read_request(
            TASK_RECIPE_QUOTE_PATH,
            TaskRecipeQuoteRequestV1 {
                schema_revision: 1,
                context_proof,
                expected_base_binding: RuntimeCognitionBaseBindingV1 {
                    world_id: "world".into(),
                    branch_id: "main".into(),
                    finality_epoch: 0,
                    finality_block_hash: None,
                    finality_status: "pending".into(),
                    base_tick: 0,
                    base_world_hash: format!("blake3:{}", "55".repeat(32)),
                    reorg_epoch: 0,
                    runtime_manifest_hash: format!("blake3:{}", "66".repeat(32)),
                },
                factory_id: "factory".into(),
                recipe_id: "recipe.smelter.iron_ingot".into(),
                batches: 1,
            },
            &key,
        )
        .unwrap()
    }
    #[test]
    fn task_recipe_quote_requires_matching_strict_nested_signatures_and_history() {
        let original = request();
        assert!(verify_recipe_quote_request(&original).is_ok());
        let outsider = sign_read_request(
            TASK_RECIPE_QUOTE_PATH,
            original.request.clone(),
            &hex::encode([74; 32]),
        )
        .unwrap();
        assert!(verify_recipe_quote_request(&outsider).is_err());
        let mut changed = original.request.clone();
        changed.context_proof.request.prepare_operation_id = "changed".into();
        let changed =
            sign_read_request(TASK_RECIPE_QUOTE_PATH, changed, &hex::encode([73; 32])).unwrap();
        assert!(verify_recipe_quote_request(&changed).is_err());
        let mut changed = original.request;
        changed.expected_base_binding.branch_id = "different".into();
        let changed =
            sign_read_request(TASK_RECIPE_QUOTE_PATH, changed, &hex::encode([73; 32])).unwrap();
        assert!(verify_recipe_quote_request(&changed).is_err());
    }
    #[test]
    fn task_recipe_quote_wire_rejects_caller_plan_authorization_and_duplicate_fields() {
        let original = request();
        for field in ["plan", "authorization", "action"] {
            let mut value = serde_json::to_value(&original).unwrap();
            value["request"][field] = serde_json::json!({});
            assert!(
                oasis7_agent_api::game::parse_bounded_json::<
                    SignedReadRequest<TaskRecipeQuoteRequestV1>,
                >(&serde_json::to_vec(&value).unwrap(), 4096)
                .is_err()
            );
        }
        let encoded = serde_json::to_string(&original)
            .unwrap()
            .replace("\"batches\":1", "\"batches\":1,\"batches\":2");
        assert!(
            oasis7_agent_api::game::parse_bounded_json::<SignedReadRequest<TaskRecipeQuoteRequestV1>>(
                encoded.as_bytes(),
                4096
            )
            .is_err()
        );
    }
}
