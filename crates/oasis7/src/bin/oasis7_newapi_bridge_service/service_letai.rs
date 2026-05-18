use serde_json::Value;

use super::super::credit_adapter::{
    LetaiEnsureProjectTokenRequest, LetaiOpenApiAdapter, LetaiProjectTokenResult,
    LetaiUserTopupRequest, LetaiUserUpsertRequest,
};
use super::super::model::{
    BridgeBindingStatus, BridgeLedgerEntry, BridgeLedgerState, LetaiProjectBinding,
};
use super::{BridgeService, BridgeServiceError};

impl BridgeService {
    pub(super) fn ensure_inference_binding_ready(
        &self,
        bridge_user_id: &str,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        let adapter = match self.letai_adapter() {
            Ok(adapter) => adapter,
            Err(err) if err.code == "letai_openapi_not_configured" => return Ok(()),
            Err(err) => return Err(err),
        };
        let snapshot = self.snapshot();
        let binding = snapshot
            .bindings
            .iter()
            .find(|binding| {
                binding.bridge_user_id == bridge_user_id
                    && binding.status == BridgeBindingStatus::Active
            })
            .cloned()
            .ok_or_else(|| {
                BridgeServiceError::not_found(
                    "binding_not_found",
                    format!("active bridge binding `{bridge_user_id}` does not exist"),
                )
            })?;
        let project_binding = snapshot
            .project_bindings
            .iter()
            .find(|project| project.bridge_user_id == bridge_user_id)
            .cloned()
            .ok_or_else(|| {
                BridgeServiceError::not_found(
                    "project_binding_not_found",
                    format!("LetAI project binding `{bridge_user_id}` does not exist"),
                )
            })?;
        if binding.platform_user_id.is_some()
            && project_binding.platform_project_id.is_some()
            && project_binding.token_key.is_some()
        {
            return Ok(());
        }

        let upsert_result = adapter
            .upsert_user(&LetaiUserUpsertRequest {
                external_user_id: binding.letai_external_user_id.clone(),
                external_user_name: binding.letai_external_user_name.clone(),
                email: binding.email.clone(),
                metadata: binding.metadata.clone(),
            })
            .map_err(|err| BridgeServiceError::bad_gateway(err.code, err.message))?;
        self.persist_binding_user_ready(
            bridge_user_id,
            upsert_result.platform_user_id.as_str(),
            now_unix_ms,
        )?;

        let parent_channel_id = project_binding
            .parent_channel_id
            .clone()
            .or_else(|| adapter.parent_channel_id().map(ToOwned::to_owned));
        let ensure_project_result = adapter
            .ensure_project_token(
                upsert_result.platform_user_id.as_str(),
                &LetaiEnsureProjectTokenRequest {
                    external_project_id: project_binding.letai_external_project_id.clone(),
                    external_project_name: project_binding.project_name.clone(),
                    parent_channel_id,
                    metadata: project_binding.metadata.clone(),
                },
            )
            .map_err(|err| BridgeServiceError::bad_gateway(err.code, err.message))?;
        self.persist_binding_project_ready(
            bridge_user_id,
            upsert_result.platform_user_id.as_str(),
            &ensure_project_result,
            now_unix_ms,
        )?;
        Ok(())
    }

    pub(super) fn process_credit_ready_entries(
        &self,
        now_unix_ms: i64,
    ) -> Result<usize, BridgeServiceError> {
        let ready_entries = self
            .snapshot()
            .ledger
            .into_iter()
            .filter(|entry| {
                matches!(
                    entry.state,
                    BridgeLedgerState::Confirmed
                        | BridgeLedgerState::Resolved
                        | BridgeLedgerState::ProvisioningUser
                        | BridgeLedgerState::ProvisioningProject
                        | BridgeLedgerState::Crediting
                        | BridgeLedgerState::Credited
                ) || (entry.state == BridgeLedgerState::Failed
                    && entry.credit_attempt_count < self.max_credit_attempts())
            })
            .collect::<Vec<_>>();
        if ready_entries.is_empty() {
            return Ok(0);
        }
        let adapter = self.letai_adapter()?;
        let mut reconciled = 0usize;
        for entry in ready_entries {
            if self.try_reconcile_letai(&adapter, entry.bridge_deposit_id.as_str(), now_unix_ms)? {
                reconciled += 1;
            }
        }
        Ok(reconciled)
    }

    fn try_reconcile_letai(
        &self,
        adapter: &LetaiOpenApiAdapter,
        bridge_deposit_id: &str,
        now_unix_ms: i64,
    ) -> Result<bool, BridgeServiceError> {
        let Some(ledger_entry) =
            self.transition_entry_for_processing(bridge_deposit_id, now_unix_ms)?
        else {
            return Ok(false);
        };

        let snapshot = self.snapshot();
        let binding = snapshot
            .bindings
            .iter()
            .find(|binding| {
                binding.bridge_user_id == ledger_entry.bridge_user_id
                    && binding.status == BridgeBindingStatus::Active
            })
            .cloned();
        let Some(binding) = binding else {
            let message = format!(
                "active bridge binding `{}` does not exist",
                ledger_entry.bridge_user_id
            );
            self.record_manual_review(
                bridge_deposit_id,
                "binding_not_found",
                message.as_str(),
                now_unix_ms,
            )?;
            return Ok(false);
        };
        let project_binding = snapshot
            .project_bindings
            .iter()
            .find(|project| project.bridge_user_id == ledger_entry.bridge_user_id)
            .cloned();
        let Some(project_binding) = project_binding else {
            let message = format!(
                "missing LetAI project binding for {}",
                ledger_entry.bridge_user_id
            );
            self.record_manual_review(
                bridge_deposit_id,
                "project_binding_not_found",
                message.as_str(),
                now_unix_ms,
            )?;
            return Ok(false);
        };

        let upsert_result = match adapter.upsert_user(&LetaiUserUpsertRequest {
            external_user_id: binding.letai_external_user_id.clone(),
            external_user_name: binding.letai_external_user_name.clone(),
            email: binding.email.clone(),
            metadata: binding.metadata.clone(),
        }) {
            Ok(result) => result,
            Err(err) => {
                self.record_retryable_failure(
                    bridge_deposit_id,
                    err.code,
                    err.message.as_str(),
                    now_unix_ms,
                )?;
                return Ok(false);
            }
        };
        self.persist_user_ready(
            bridge_deposit_id,
            binding.bridge_user_id.as_str(),
            upsert_result.platform_user_id.as_str(),
            &upsert_result.snapshot,
            now_unix_ms,
        )?;

        let parent_channel_id = project_binding
            .parent_channel_id
            .clone()
            .or_else(|| adapter.parent_channel_id().map(ToOwned::to_owned));
        let ensure_project_result = match adapter.ensure_project_token(
            upsert_result.platform_user_id.as_str(),
            &LetaiEnsureProjectTokenRequest {
                external_project_id: project_binding.letai_external_project_id.clone(),
                external_project_name: project_binding.project_name.clone(),
                parent_channel_id,
                metadata: project_binding.metadata.clone(),
            },
        ) {
            Ok(result) => result,
            Err(err) => {
                self.record_retryable_failure(
                    bridge_deposit_id,
                    err.code,
                    err.message.as_str(),
                    now_unix_ms,
                )?;
                return Ok(false);
            }
        };
        self.persist_project_ready(
            bridge_deposit_id,
            binding.bridge_user_id.as_str(),
            upsert_result.platform_user_id.as_str(),
            &ensure_project_result,
            now_unix_ms,
        )?;

        let external_order_id = ledger_entry.external_order_id.clone().unwrap_or_else(|| {
            BridgeService::build_external_order_id(ledger_entry.bridge_deposit_id.as_str())
        });
        let quota = ledger_entry.total_credit_units;
        if ledger_entry.state != BridgeLedgerState::Credited {
            let topup_request = LetaiUserTopupRequest {
                external_order_id: external_order_id.clone(),
                quota,
                amount: Some(ledger_entry.amount_oc.to_string()),
                currency: Some("OC".to_string()),
            };
            let topup_receipt =
                match adapter.topup_user(upsert_result.platform_user_id.as_str(), &topup_request) {
                    Ok(receipt) => receipt,
                    Err(err) => {
                        self.record_retryable_failure(
                            bridge_deposit_id,
                            err.code,
                            err.message.as_str(),
                            now_unix_ms,
                        )?;
                        return Ok(false);
                    }
                };
            self.persist_topup_receipt(
                bridge_deposit_id,
                upsert_result.platform_user_id.as_str(),
                ensure_project_result.platform_project_id.as_str(),
                ensure_project_result.token_key.as_str(),
                external_order_id.as_str(),
                quota,
                &topup_receipt,
                now_unix_ms,
            )?;
        }

        let user_snapshot =
            match adapter.fetch_user_summary(upsert_result.platform_user_id.as_str()) {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    self.record_manual_review(
                        bridge_deposit_id,
                        err.code,
                        err.message.as_str(),
                        now_unix_ms,
                    )?;
                    return Ok(false);
                }
            };
        let project_snapshot = match adapter
            .fetch_project_token_summary(ensure_project_result.platform_project_id.as_str())
        {
            Ok(snapshot) => snapshot,
            Err(err) => {
                self.record_manual_review(
                    bridge_deposit_id,
                    err.code,
                    err.message.as_str(),
                    now_unix_ms,
                )?;
                return Ok(false);
            }
        };
        let topup_log_snapshot = match adapter.fetch_project_logs(
            ensure_project_result.platform_project_id.as_str(),
            external_order_id.as_str(),
        ) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                self.record_manual_review(
                    bridge_deposit_id,
                    err.code,
                    err.message.as_str(),
                    now_unix_ms,
                )?;
                return Ok(false);
            }
        };

        if !project_summary_matches(
            &project_snapshot,
            ensure_project_result.platform_project_id.as_str(),
            ensure_project_result.token_key.as_str(),
        ) {
            self.persist_verification_failure(
                bridge_deposit_id,
                "letai_project_summary_mismatch",
                "LetAI project token summary does not contain the expected project/token pair",
                Some(user_snapshot),
                Some(project_snapshot),
                Some(topup_log_snapshot),
                now_unix_ms,
            )?;
            return Ok(false);
        }
        if !topup_log_contains(&topup_log_snapshot, external_order_id.as_str(), quota) {
            self.persist_verification_failure(
                bridge_deposit_id,
                "letai_topup_log_mismatch",
                "LetAI user logs do not contain the expected external_order_id/quota record",
                Some(user_snapshot),
                Some(project_snapshot),
                Some(topup_log_snapshot),
                now_unix_ms,
            )?;
            return Ok(false);
        }

        self.persist_reconciled(
            bridge_deposit_id,
            upsert_result.platform_user_id.as_str(),
            ensure_project_result.platform_project_id.as_str(),
            ensure_project_result.token_key.as_str(),
            external_order_id.as_str(),
            quota,
            user_snapshot,
            project_snapshot,
            topup_log_snapshot,
            now_unix_ms,
        )?;
        Ok(true)
    }

    fn transition_entry_for_processing(
        &self,
        bridge_deposit_id: &str,
        now_unix_ms: i64,
    ) -> Result<Option<BridgeLedgerEntry>, BridgeServiceError> {
        let bridge_deposit_id = bridge_deposit_id.to_string();
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            if !matches!(
                entry.state,
                BridgeLedgerState::Confirmed
                    | BridgeLedgerState::Failed
                    | BridgeLedgerState::Resolved
                    | BridgeLedgerState::ProvisioningUser
                    | BridgeLedgerState::ProvisioningProject
                    | BridgeLedgerState::Crediting
                    | BridgeLedgerState::Credited
            ) {
                return Ok(None);
            }
            let next_state = match entry.state {
                BridgeLedgerState::Confirmed
                | BridgeLedgerState::Failed
                | BridgeLedgerState::Resolved => BridgeLedgerState::ProvisioningUser,
                _ => entry.state.clone(),
            };
            if entry.state != next_state {
                entry.state = next_state;
                entry.updated_at_unix_ms = now_unix_ms;
            }
            Ok(Some(entry.clone()))
        })
    }

    fn persist_user_ready(
        &self,
        bridge_deposit_id: &str,
        bridge_user_id: &str,
        platform_user_id: &str,
        user_snapshot: &Value,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(binding) = state
                .bindings
                .iter_mut()
                .find(|binding| binding.bridge_user_id == bridge_user_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "binding_not_found",
                    format!("bridge binding `{bridge_user_id}` does not exist"),
                ));
            };
            binding.platform_user_id = Some(platform_user_id.to_string());
            binding.updated_at_unix_ms = now_unix_ms;
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.platform_user_id = Some(platform_user_id.to_string());
            entry.user_snapshot = Some(user_snapshot.clone());
            entry.state = BridgeLedgerState::ProvisioningProject;
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_binding_user_ready(
        &self,
        bridge_user_id: &str,
        platform_user_id: &str,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(binding) = state
                .bindings
                .iter_mut()
                .find(|binding| binding.bridge_user_id == bridge_user_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "binding_not_found",
                    format!("bridge binding `{bridge_user_id}` does not exist"),
                ));
            };
            binding.platform_user_id = Some(platform_user_id.to_string());
            binding.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_project_ready(
        &self,
        bridge_deposit_id: &str,
        bridge_user_id: &str,
        platform_user_id: &str,
        result: &LetaiProjectTokenResult,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(project_binding) = state
                .project_bindings
                .iter_mut()
                .find(|project| project.bridge_user_id == bridge_user_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "project_binding_not_found",
                    format!("LetAI project binding `{bridge_user_id}` does not exist"),
                ));
            };
            project_binding.platform_project_id = Some(result.platform_project_id.clone());
            project_binding.token_key = Some(result.token_key.clone());
            project_binding.token_status = result.token_status.clone();
            if project_binding.parent_channel_id.is_none() {
                project_binding.parent_channel_id = self.letai_parent_channel_id();
            }
            project_binding.updated_at_unix_ms = now_unix_ms;

            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.platform_user_id = Some(platform_user_id.to_string());
            entry.platform_project_id = Some(result.platform_project_id.clone());
            entry.token_key = Some(result.token_key.clone());
            entry.project_snapshot = Some(result.snapshot.clone());
            entry.state = BridgeLedgerState::Crediting;
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_binding_project_ready(
        &self,
        bridge_user_id: &str,
        platform_user_id: &str,
        result: &LetaiProjectTokenResult,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(binding) = state
                .bindings
                .iter_mut()
                .find(|binding| binding.bridge_user_id == bridge_user_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "binding_not_found",
                    format!("bridge binding `{bridge_user_id}` does not exist"),
                ));
            };
            binding.platform_user_id = Some(platform_user_id.to_string());
            binding.updated_at_unix_ms = now_unix_ms;

            let Some(project_binding) = state
                .project_bindings
                .iter_mut()
                .find(|project| project.bridge_user_id == bridge_user_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "project_binding_not_found",
                    format!("LetAI project binding `{bridge_user_id}` does not exist"),
                ));
            };
            project_binding.platform_project_id = Some(result.platform_project_id.clone());
            project_binding.token_key = Some(result.token_key.clone());
            project_binding.token_status = result.token_status.clone();
            if project_binding.parent_channel_id.is_none() {
                project_binding.parent_channel_id = self.letai_parent_channel_id();
            }
            project_binding.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_topup_receipt(
        &self,
        bridge_deposit_id: &str,
        platform_user_id: &str,
        platform_project_id: &str,
        token_key: &str,
        external_order_id: &str,
        quota: u64,
        receipt: &Value,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.platform_user_id = Some(platform_user_id.to_string());
            entry.platform_project_id = Some(platform_project_id.to_string());
            entry.token_key = Some(token_key.to_string());
            entry.external_order_id = Some(external_order_id.to_string());
            entry.quota = Some(quota);
            entry.amount_audit = Some(entry.amount_oc.to_string());
            entry.currency = Some("OC".to_string());
            entry.topup_receipt = Some(receipt.clone());
            entry.state = BridgeLedgerState::Credited;
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_reconciled(
        &self,
        bridge_deposit_id: &str,
        platform_user_id: &str,
        platform_project_id: &str,
        token_key: &str,
        external_order_id: &str,
        quota: u64,
        user_snapshot: Value,
        project_snapshot: Value,
        topup_log_snapshot: Value,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.platform_user_id = Some(platform_user_id.to_string());
            entry.platform_project_id = Some(platform_project_id.to_string());
            entry.token_key = Some(token_key.to_string());
            entry.external_order_id = Some(external_order_id.to_string());
            entry.quota = Some(quota);
            entry.user_snapshot = Some(user_snapshot);
            entry.project_snapshot = Some(project_snapshot);
            entry.topup_log_snapshot = Some(topup_log_snapshot);
            entry.state = BridgeLedgerState::Reconciled;
            entry.last_error_code = None;
            entry.last_error = None;
            entry.review_reason = None;
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn record_retryable_failure(
        &self,
        bridge_deposit_id: &str,
        error_code: &str,
        error_message: &str,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        let max_attempts = self.max_credit_attempts();
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.credit_attempt_count = entry.credit_attempt_count.saturating_add(1);
            entry.last_error_code = Some(error_code.to_string());
            entry.last_error = Some(error_message.to_string());
            entry.updated_at_unix_ms = now_unix_ms;
            if entry.credit_attempt_count >= max_attempts {
                entry.state = BridgeLedgerState::ManualReview;
                entry.review_reason = Some(error_code.to_string());
            } else {
                entry.state = BridgeLedgerState::Failed;
            }
            Ok(())
        })
    }

    fn record_manual_review(
        &self,
        bridge_deposit_id: &str,
        error_code: &str,
        error_message: &str,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.state = BridgeLedgerState::ManualReview;
            entry.review_reason = Some(error_code.to_string());
            entry.last_error_code = Some(error_code.to_string());
            entry.last_error = Some(error_message.to_string());
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }

    fn persist_verification_failure(
        &self,
        bridge_deposit_id: &str,
        error_code: &str,
        error_message: &str,
        user_snapshot: Option<Value>,
        project_snapshot: Option<Value>,
        topup_log_snapshot: Option<Value>,
        now_unix_ms: i64,
    ) -> Result<(), BridgeServiceError> {
        self.store_mutate(|state| {
            let Some(entry) = state
                .ledger
                .iter_mut()
                .find(|entry| entry.bridge_deposit_id == bridge_deposit_id)
            else {
                return Err(BridgeServiceError::not_found(
                    "bridge_deposit_not_found",
                    format!("bridge deposit `{bridge_deposit_id}` does not exist"),
                ));
            };
            entry.state = BridgeLedgerState::ManualReview;
            entry.review_reason = Some(error_code.to_string());
            entry.last_error_code = Some(error_code.to_string());
            entry.last_error = Some(error_message.to_string());
            if let Some(snapshot) = user_snapshot {
                entry.user_snapshot = Some(snapshot);
            }
            if let Some(snapshot) = project_snapshot {
                entry.project_snapshot = Some(snapshot);
            }
            if let Some(snapshot) = topup_log_snapshot {
                entry.topup_log_snapshot = Some(snapshot);
            }
            entry.updated_at_unix_ms = now_unix_ms;
            Ok(())
        })
    }
}

pub(super) fn ensure_project_binding(
    state: &mut super::super::model::PersistedBridgeState,
    bridge_user_id: &str,
    newapi_user_ref: &str,
    project_name: Option<&str>,
    project_metadata: Option<Value>,
    now_unix_ms: i64,
) -> LetaiProjectBinding {
    if let Some(existing) = state
        .project_bindings
        .iter_mut()
        .find(|project| project.bridge_user_id == bridge_user_id)
    {
        if let Some(project_name) = project_name {
            existing.project_name = project_name.to_string();
        }
        if project_metadata.is_some() {
            existing.metadata = project_metadata;
        }
        existing.updated_at_unix_ms = now_unix_ms;
        return existing.clone();
    }
    let project_binding = LetaiProjectBinding {
        bridge_user_id: bridge_user_id.to_string(),
        letai_external_project_id: BridgeService::build_letai_external_project_id(bridge_user_id),
        project_name: project_name
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| BridgeService::default_project_name(newapi_user_ref)),
        parent_channel_id: None,
        metadata: project_metadata,
        platform_project_id: None,
        token_key: None,
        token_status: None,
        created_at_unix_ms: now_unix_ms,
        updated_at_unix_ms: now_unix_ms,
    };
    state.project_bindings.push(project_binding);
    state
        .project_bindings
        .last()
        .cloned()
        .expect("project binding exists after insert")
}

pub(super) fn project_summary_matches(
    value: &Value,
    expected_project_id: &str,
    expected_token_key: &str,
) -> bool {
    match value {
        Value::Object(map) => {
            let project_id = map
                .get("platform_project_id")
                .or_else(|| map.get("project_id"))
                .or_else(|| map.get("id"))
                .and_then(Value::as_str);
            let token_key = map.get("token_key").and_then(Value::as_str);
            let project_match = project_id
                .map(|value| value == expected_project_id)
                .unwrap_or(false);
            let token_match = token_key
                .map(|value| value == expected_token_key)
                .unwrap_or(false);
            if project_match && (token_match || token_key.is_none()) {
                return true;
            }
            if token_match && project_id.is_none() {
                return true;
            }
            map.values().any(|child| {
                project_summary_matches(child, expected_project_id, expected_token_key)
            })
        }
        Value::Array(items) => items
            .iter()
            .any(|item| project_summary_matches(item, expected_project_id, expected_token_key)),
        _ => false,
    }
}

pub(super) fn topup_log_contains(
    value: &Value,
    expected_order_id: &str,
    expected_quota: u64,
) -> bool {
    match value {
        Value::Object(map) => {
            let order_match = map
                .get("external_order_id")
                .and_then(Value::as_str)
                .map(|value| value == expected_order_id)
                .unwrap_or(false);
            let quota_match = map
                .get("quota")
                .map(|value| quota_value_matches(value, expected_quota))
                .unwrap_or(false);
            if order_match && quota_match {
                return true;
            }
            map.values()
                .any(|child| topup_log_contains(child, expected_order_id, expected_quota))
        }
        Value::Array(items) => items
            .iter()
            .any(|item| topup_log_contains(item, expected_order_id, expected_quota)),
        _ => false,
    }
}

pub(super) fn quota_value_matches(value: &Value, expected_quota: u64) -> bool {
    match value {
        Value::Number(number) => number.as_u64() == Some(expected_quota),
        Value::String(string) => string.parse::<u64>().ok() == Some(expected_quota),
        _ => false,
    }
}
