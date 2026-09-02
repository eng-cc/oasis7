use super::super::{
    ActionId, DomainEvent, MaterialStack, ProductValidationAttemptV1, ProductValidationDecision,
    ProductValidationReceiptV1, WorldError, WorldEvent, WorldEventBody,
};
use super::World;

impl World {
    pub(super) fn product_validation_receipt_for_output(
        &self,
        job_id: ActionId,
        validation_index: Option<u32>,
        requester_agent_id: &str,
        stack: &MaterialStack,
    ) -> Result<Option<ProductValidationReceiptV1>, WorldError> {
        let Some(receipt) = self
            .state
            .product_validation_receipts
            .get(&job_id)
            .and_then(|receipts| {
                receipts
                    .iter()
                    .find(|receipt| receipt.validation_index == validation_index)
            })
            .cloned()
        else {
            return Ok(None);
        };
        if receipt.requester_agent_id != requester_agent_id || receipt.stack != *stack {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "product validation receipt conflicts with job: job_id={job_id} index={validation_index:?}"
                ),
            });
        }
        Ok(Some(receipt))
    }

    pub(super) fn append_missing_product_validation_blocker(
        &mut self,
        job_id: ActionId,
        requester_agent_id: &str,
        factory_id: &str,
        recipe_id: &str,
        stack: &MaterialStack,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<(), WorldError> {
        if self
            .state
            .factory_production_failure_dispositions
            .contains_key(&job_id)
        {
            return Ok(());
        }
        self.append_event(
            WorldEventBody::Domain(DomainEvent::FactoryProductionBlocked {
                action_id: job_id,
                requester_agent_id: requester_agent_id.to_string(),
                factory_id: factory_id.to_string(),
                recipe_id: recipe_id.to_string(),
                blocker_kind: "product_validation".to_string(),
                blocker_detail: format!(
                    "product validation rejected for {} before production settlement",
                    stack.kind
                ),
            }),
            None,
        )?;
        if let Some(event) = self.journal.events.last() {
            emitted.push(event.clone());
        }
        Ok(())
    }

    pub(super) fn resume_product_validation_receipt(
        &mut self,
        job_id: ActionId,
        validation_index: Option<u32>,
        requester_agent_id: &str,
        factory_id: &str,
        recipe_id: &str,
        stack: &MaterialStack,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<Option<bool>, WorldError> {
        let Some(receipt) = self.product_validation_receipt_for_output(
            job_id,
            validation_index,
            requester_agent_id,
            stack,
        )?
        else {
            return Ok(None);
        };
        self.product_validation_attempt_for_output(
            job_id,
            validation_index,
            requester_agent_id,
            stack,
            Some(receipt.module_id.as_str()),
        )?;
        if Self::product_validation_decision_matches_stack(&receipt.decision, stack) {
            return Ok(Some(false));
        }
        self.append_missing_product_validation_blocker(
            job_id,
            requester_agent_id,
            factory_id,
            recipe_id,
            stack,
            emitted,
        )?;
        Ok(Some(true))
    }

    pub(super) fn product_validation_decision_matches_stack(
        decision: &ProductValidationDecision,
        stack: &MaterialStack,
    ) -> bool {
        decision.accepted
            && decision.product_id == stack.kind
            && stack.amount > 0
            && stack.amount <= decision.stack_limit as i64
    }

    pub(super) fn product_validation_attempt_for_output(
        &self,
        job_id: ActionId,
        validation_index: Option<u32>,
        requester_agent_id: &str,
        stack: &MaterialStack,
        receipt_module_id: Option<&str>,
    ) -> Result<Option<ProductValidationAttemptV1>, WorldError> {
        let attempt = self
            .state
            .product_validation_attempts
            .get(&job_id)
            .and_then(|attempts| {
                attempts
                    .iter()
                    .find(|attempt| attempt.validation_index == validation_index)
            })
            .cloned();
        if let Some(attempt) = &attempt {
            if attempt.requester_agent_id != requester_agent_id
                || receipt_module_id.is_some_and(|module_id| attempt.module_id != module_id)
                || attempt.stack != *stack
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "product validation attempt conflicts with job or receipt: job_id={job_id} index={validation_index:?}"
                    ),
                });
            }
        }
        Ok(attempt)
    }

    pub(super) fn record_product_validation_attempt(
        &mut self,
        job_id: ActionId,
        validation_index: Option<u32>,
        requester_agent_id: &str,
        module_id: &str,
        stack: &MaterialStack,
    ) -> Result<(), WorldError> {
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ProductValidationAttemptStarted {
                attempt: ProductValidationAttemptV1 {
                    job_id,
                    validation_index,
                    requester_agent_id: requester_agent_id.to_string(),
                    module_id: module_id.to_string(),
                    stack: stack.clone(),
                },
            }),
            None,
        )?;
        Ok(())
    }

    pub(super) fn fail_closed_product_validation(
        module_id: &str,
        stack: &MaterialStack,
        reason: impl Into<String>,
    ) -> (ProductValidationDecision, String) {
        let detail = format!(
            "product validation module failed and output was rejected: module_id={module_id} reason={}",
            reason.into()
        );
        (
            ProductValidationDecision::rejected(
                stack.kind.clone(),
                0,
                false,
                Vec::new(),
                vec![detail.clone()],
            ),
            detail,
        )
    }
}
