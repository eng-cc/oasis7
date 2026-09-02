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
            self.ensure_product_validation_delivery(&receipt, emitted)?;
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

    fn ensure_product_validation_delivery(
        &mut self,
        receipt: &ProductValidationReceiptV1,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<(), WorldError> {
        let existing_events = self.product_validation_recovery_events(receipt);
        if !existing_events.is_empty() {
            let has_delivery = existing_events
                .iter()
                .any(|event| Self::product_validation_delivery_matches(event, receipt));
            for event in &existing_events {
                // A journaled event may have been persisted by the next
                // output's pre-call checkpoint before the event loop routed it
                // to subscribers. Requeue that exact event without appending
                // another journal entry or invoking the validator again.
                Self::queue_product_validation_event(emitted, event);
            }
            if has_delivery {
                return Ok(());
            }
        }
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ProductValidated {
                requester_agent_id: receipt.requester_agent_id.clone(),
                module_id: receipt.module_id.clone(),
                stack: receipt.stack.clone(),
                stack_limit: receipt.decision.stack_limit,
                tradable: receipt.decision.tradable,
                quality_levels: receipt.decision.quality_levels.clone(),
                notes: receipt.decision.notes.clone(),
            }),
            None,
        )?;
        if let Some(event) = self.journal.events.last() {
            Self::queue_product_validation_event(emitted, event);
        }
        Ok(())
    }

    fn product_validation_recovery_events(
        &self,
        receipt: &ProductValidationReceiptV1,
    ) -> Vec<WorldEvent> {
        let Some(receipt_position) = self.journal.events.iter().rposition(|event| {
            matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::ProductValidationRecorded {
                    receipt: recorded,
                }) if recorded == receipt
            )
        }) else {
            return Vec::new();
        };
        let mut events = vec![self.journal.events[receipt_position].clone()];
        if let Some(event) = self
            .journal
            .events
            .iter()
            .skip(receipt_position.saturating_add(1))
            .take_while(|event| {
                !matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ProductValidationRecorded {
                        receipt: next,
                    }) if next.job_id == receipt.job_id
                        && next.validation_index == receipt.validation_index
                )
            })
            .find(|event| Self::product_validation_delivery_matches(event, receipt))
        {
            events.push(event.clone());
        }
        events
    }

    fn queue_product_validation_event(emitted: &mut Vec<WorldEvent>, event: &WorldEvent) {
        if !emitted.iter().any(|queued| queued.id == event.id) {
            emitted.push(event.clone());
        }
    }

    fn product_validation_delivery_matches(
        event: &WorldEvent,
        receipt: &ProductValidationReceiptV1,
    ) -> bool {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ProductValidated {
                requester_agent_id,
                module_id,
                stack,
                stack_limit,
                tradable,
                quality_levels,
                notes,
            }) if requester_agent_id == &receipt.requester_agent_id
                && module_id == &receipt.module_id
                && stack == &receipt.stack
                && *stack_limit == receipt.decision.stack_limit
                && *tradable == receipt.decision.tradable
                && quality_levels == &receipt.decision.quality_levels
                && notes == &receipt.decision.notes
        )
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
        emitted: &mut Vec<WorldEvent>,
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
        if let Some(event) = self.journal.events.last() {
            Self::queue_product_validation_event(emitted, event);
        }
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
