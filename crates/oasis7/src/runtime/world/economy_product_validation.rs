use super::super::{
    ActionId, DomainEvent, MaterialStack, ProductValidationAttemptV1, ProductValidationDecision,
    WorldError, WorldEventBody,
};
use super::World;

impl World {
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
