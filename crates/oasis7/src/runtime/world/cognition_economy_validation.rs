use super::*;

impl CognitionEconomyStateV1 {
    pub fn validate(&self) -> Result<(), CognitionEconomyError> {
        if self.schema_version != COGNITION_ECONOMY_SCHEMA_VERSION
            || self.head_seq != self.journal.len() as u64
        {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_economy_head_invalid",
            ));
        }
        let expected_head = economy_digest(
            COGNITION_ECONOMY_JOURNAL_DOMAIN,
            &(self.head_seq, &self.journal),
        );
        if self.head_digest != expected_head {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_economy_head_digest_mismatch",
            ));
        }
        for (account_id, resources) in &self.balances {
            if !bounded_identity(account_id) {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_balance_identity_invalid",
                ));
            }
            for resource in resources.keys() {
                if !bounded_identity(resource) {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_balance_identity_invalid",
                    ));
                }
            }
        }
        let mut previous_digest = String::new();
        for (index, event) in self.journal.iter().enumerate() {
            if event.schema_version != COGNITION_ECONOMY_EVENT_SCHEMA_VERSION
                || event.journal_seq != index as u64 + 1
                || event.parent_event_digest != previous_digest
                || !valid_digest(&event.event_digest)
                || event.event_digest != event.recompute_digest()
                || !bounded_identity(&event.lease_id)
                || !bounded_identity(&event.operation_key)
                || !bounded_identity(&event.idempotency_key)
                || !bounded_identity(&event.resource)
                || ((event.event_kind != "reserve") && !valid_digest(&event.receipt_id))
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_journal_invalid",
                ));
            }
            let Some(lease) = self.leases.get(&event.lease_id) else {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_journal_lease_missing",
                ));
            };
            if event.idempotency_key != lease.idempotency_key
                || event.resource != lease.quote.resource
                || event.reserved_amount != lease.reserved_amount
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_journal_identity_invalid",
                ));
            }
            let (expected_status, expected_operation, expected_receipt) =
                match event.event_kind.as_str() {
                    "reserve" => (
                        CognitionLeaseStatusV1::Reserved,
                        operation_key(&lease.lease_id, "reserve"),
                        None,
                    ),
                    "settle" => (
                        CognitionLeaseStatusV1::Settled,
                        operation_key(&lease.lease_id, "settle"),
                        Some(event.receipt_id.as_str()),
                    ),
                    "release" => (
                        CognitionLeaseStatusV1::Released,
                        operation_key(&lease.lease_id, "release"),
                        Some(event.receipt_id.as_str()),
                    ),
                    "refund" => (
                        CognitionLeaseStatusV1::Refunded,
                        operation_key(&lease.lease_id, "refund"),
                        Some(event.receipt_id.as_str()),
                    ),
                    _ => {
                        return Err(CognitionEconomyError::InvalidState(
                            "cognition_economy_journal_kind_invalid",
                        ));
                    }
                };
            if event.operation_key != expected_operation
                || expected_receipt != Some(event.receipt_id.as_str()) && event.receipt_id != ""
                || event.status != expected_status
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_journal_transition_invalid",
                ));
            }
            if let Some(receipt_id) = expected_receipt {
                let Some(receipt) = self.receipts.get(receipt_id) else {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_journal_receipt_missing",
                    ));
                };
                if receipt.status != expected_status
                    || receipt.consumed_amount != event.consumed_amount
                    || receipt.refunded_amount != event.refunded_amount
                    || !receipt_matches_lease(receipt, lease)
                {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_journal_receipt_invalid",
                    ));
                }
            } else if event.consumed_amount != 0
                || event.refunded_amount != 0
                || !event.receipt_id.is_empty()
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_journal_reserve_invalid",
                ));
            }
            previous_digest.clone_from(&event.event_digest);
        }
        for (lease_id, lease) in &self.leases {
            lease.validate()?;
            let Some(record) = self.idempotency.get(&lease.idempotency_key) else {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_lease_index_invalid",
                ));
            };
            if lease_id != &lease.lease_id
                || record.lease_id != lease.lease_id
                || record.request_fingerprint != lease_fingerprint(lease)
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_lease_index_invalid",
                ));
            }
            if lease.status != CognitionLeaseStatusV1::Reserved {
                let Some(receipt_id) = lease.receipt_id.as_deref() else {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_lease_receipt_missing",
                    ));
                };
                if self.receipts.get(receipt_id).is_none_or(|receipt| {
                    receipt.status != lease.status || !receipt_matches_lease(receipt, lease)
                }) {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_lease_receipt_invalid",
                    ));
                }
            }
        }
        for (idempotency_key, record) in &self.idempotency {
            let Some(lease) = self.leases.get(&record.lease_id) else {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_idempotency_index_invalid",
                ));
            };
            if !bounded_identity(idempotency_key)
                || !valid_digest(&record.request_fingerprint)
                || idempotency_key != &lease.idempotency_key
                || record.request_fingerprint != lease_fingerprint(lease)
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_idempotency_index_invalid",
                ));
            }
        }
        for (receipt_id, receipt) in &self.receipts {
            receipt.validate()?;
            let Some(lease) = self.leases.get(&receipt.lease_id) else {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_receipt_index_invalid",
                ));
            };
            let operation = operation_key_for_status(&receipt.lease_id, receipt.status).ok_or(
                CognitionEconomyError::InvalidState("cognition_economy_receipt_status_invalid"),
            )?;
            let expected_receipt_id = economy_digest(
                COGNITION_ECONOMY_RECEIPT_ID_DOMAIN,
                &(receipt.lease_id.as_str(), operation.as_str()),
            );
            if receipt_id != &receipt.receipt_id
                || receipt.receipt_id != expected_receipt_id
                || !receipt_matches_lease(receipt, lease)
                || self.operations.get(&operation).is_none_or(|record| {
                    record.receipt_id != receipt.receipt_id || record.lease_id != receipt.lease_id
                })
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_receipt_index_invalid",
                ));
            }
        }
        for (operation_key, operation) in &self.operations {
            let Some(receipt) = self.receipts.get(&operation.receipt_id) else {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_operation_index_invalid",
                ));
            };
            let expected_digest = if receipt.status == CognitionLeaseStatusV1::Settled {
                economy_digest(
                    COGNITION_ECONOMY_OPERATION_DOMAIN,
                    &(operation_key.as_str(), receipt.consumed_amount),
                )
            } else {
                economy_digest(COGNITION_ECONOMY_OPERATION_DOMAIN, operation_key)
            };
            if operation_key != &operation.operation_key
                || !valid_digest(operation_key)
                || !valid_digest(&operation.operation_digest)
                || operation.operation_digest != expected_digest
                || operation.lease_id != receipt.lease_id
                || operation.receipt_id != receipt.receipt_id
                || operation_key
                    != &operation_key_for_status(&receipt.lease_id, receipt.status).unwrap()
            {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_operation_index_invalid",
                ));
            }
        }
        let mut expected_reserved: BTreeMap<(String, String), u64> = BTreeMap::new();
        for lease in self.leases.values() {
            if lease.status == CognitionLeaseStatusV1::Reserved {
                let key = (lease.account_id.clone(), lease.quote.resource.clone());
                let value = expected_reserved.entry(key).or_default();
                *value = value.checked_add(lease.reserved_amount).ok_or(
                    CognitionEconomyError::InvalidState("cognition_economy_reserved_overflow"),
                )?;
            }
        }
        for (account_id, resources) in &self.balances {
            for (resource, balance) in resources {
                if expected_reserved
                    .get(&(account_id.clone(), resource.clone()))
                    .copied()
                    .unwrap_or_default()
                    != balance.reserved
                {
                    return Err(CognitionEconomyError::InvalidState(
                        "cognition_economy_balance_reservation_mismatch",
                    ));
                }
            }
        }
        for ((account_id, resource), reserved) in expected_reserved {
            if self.reserved_balance(account_id.as_str(), resource.as_str()) != reserved {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_economy_balance_reservation_missing",
                ));
            }
        }
        Ok(())
    }
}

fn operation_key_for_status(lease_id: &str, status: CognitionLeaseStatusV1) -> Option<String> {
    let operation = match status {
        CognitionLeaseStatusV1::Reserved => return None,
        CognitionLeaseStatusV1::Settled => "settle",
        CognitionLeaseStatusV1::Released => "release",
        CognitionLeaseStatusV1::Refunded => "refund",
    };
    Some(operation_key(lease_id, operation))
}

fn lease_fingerprint(lease: &CognitionLeaseV1) -> String {
    CognitionLeaseRequestV1::new(
        lease.idempotency_key.clone(),
        lease.account_id.clone(),
        lease.agent_id.clone(),
        lease.agent_session_id.clone(),
        lease.agent_turn_id.clone(),
        lease.decision_request_id.clone(),
        lease.request_digest.clone(),
        lease.quote.clone(),
    )
    .fingerprint_digest()
}

fn receipt_matches_lease(receipt: &CognitionReceiptV1, lease: &CognitionLeaseV1) -> bool {
    receipt.lease_id == lease.lease_id
        && receipt.idempotency_key == lease.idempotency_key
        && receipt.account_id == lease.account_id
        && receipt.agent_id == lease.agent_id
        && receipt.agent_session_id == lease.agent_session_id
        && receipt.agent_turn_id == lease.agent_turn_id
        && receipt.decision_request_id == lease.decision_request_id
        && receipt.request_digest == lease.request_digest
        && receipt.quote == lease.quote
        && receipt.reserved_amount == lease.reserved_amount
}
