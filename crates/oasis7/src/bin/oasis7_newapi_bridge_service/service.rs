use std::collections::BTreeMap;
use std::sync::Arc;

#[path = "service_letai.rs"]
mod service_letai;

use self::service_letai::ensure_project_binding;
use super::chain_client::{ChainExplorerClient, ObservedChainTransfer};
use super::credit_adapter::LetaiOpenApiAdapter;
use super::model::{
    BindBridgeUserRequest, BindBridgeUserResponse, BridgeBinding, BridgeBindingStatus,
    BridgeHealthResponse, BridgeLedgerEntry, BridgeLedgerState, BridgeReconcileResponse,
    CreateDepositRouteRequest, CreateDepositRouteResponse, DepositRoute, DepositRouteStatus,
    LetaiProjectBinding, OperatorReviewRequest, OperatorReviewResponse,
};
use super::store::{BridgeStateStore, StoreMutateError};

const ROUTE_TYPE_OPERATOR_ASSIGNED_ACCOUNT: &str = "operator_assigned_account";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BridgePricingRuleConfig {
    pub(super) pricing_version: String,
    pub(super) oc_amount: u64,
    pub(super) credit_units: u64,
    pub(super) bonus_units: u64,
}

#[derive(Debug, Clone)]
pub(super) struct BridgeServiceConfig {
    pub(super) route_ttl_seconds: u64,
    pub(super) deposit_account_prefix: String,
    pub(super) chain_base_url: Option<String>,
    pub(super) chain_timeout_ms: u64,
    pub(super) chain_confirmations_required: u64,
    pub(super) pricing_rules: Vec<BridgePricingRuleConfig>,
    pub(super) letai_base_url: Option<String>,
    pub(super) letai_platform_key: Option<String>,
    pub(super) letai_parent_channel_id: Option<String>,
    pub(super) letai_timeout_ms: u64,
    pub(super) max_credit_attempts: u32,
}

#[derive(Debug, Clone)]
pub(super) struct BridgeService {
    store: Arc<BridgeStateStore>,
    config: BridgeServiceConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BridgeServiceError {
    pub(super) status_code: u16,
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl BridgeServiceError {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_code: 400,
            code,
            message: message.into(),
        }
    }

    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_code: 404,
            code,
            message: message.into(),
        }
    }

    fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_code: 409,
            code,
            message: message.into(),
        }
    }

    fn bad_gateway(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_code: 502,
            code,
            message: message.into(),
        }
    }

    fn service_unavailable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_code: 503,
            code,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status_code: 500,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl BridgeService {
    pub(super) fn new(store: Arc<BridgeStateStore>, config: BridgeServiceConfig) -> Self {
        Self { store, config }
    }

    pub(super) fn health(&self, now_unix_ms: i64) -> BridgeHealthResponse {
        let snapshot = self.store.snapshot();
        let active_binding_count = snapshot
            .bindings
            .iter()
            .filter(|binding| binding.status == BridgeBindingStatus::Active)
            .count();
        let active_route_count = snapshot
            .routes
            .iter()
            .filter(|route| {
                route.status == DepositRouteStatus::Issued && route.expires_at_unix_ms > now_unix_ms
            })
            .count();
        BridgeHealthResponse {
            ok: true,
            service: "oasis7_newapi_bridge_service".to_string(),
            observed_at_unix_ms: now_unix_ms,
            binding_count: snapshot.bindings.len(),
            active_binding_count,
            project_binding_count: snapshot.project_bindings.len(),
            route_count: snapshot.routes.len(),
            active_route_count,
            ledger_count: snapshot.ledger.len(),
            pending_confirmation_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::PendingConfirmations)
                .count(),
            manual_review_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::ManualReview)
                .count(),
            failed_credit_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::Failed)
                .count(),
            reconciled_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::Reconciled)
                .count(),
        }
    }

    pub(super) fn bind_user(
        &self,
        request: BindBridgeUserRequest,
        now_unix_ms: i64,
    ) -> Result<BindBridgeUserResponse, BridgeServiceError> {
        let newapi_user_ref =
            normalize_required("newapi_user_ref", request.newapi_user_ref.as_str())?;
        let oasis_sender_account_id = normalize_required(
            "oasis_sender_account_id",
            request.oasis_sender_account_id.as_str(),
        )?;
        let external_user_name = normalize_optional(request.external_user_name.as_deref());
        let email = normalize_optional(request.email.as_deref());
        let project_name = normalize_optional(request.project_name.as_deref())
            .unwrap_or_else(|| Self::default_project_name(newapi_user_ref.as_str()));
        let letai_external_user_id = Self::build_letai_external_user_id(newapi_user_ref.as_str());
        let response = self.store
            .mutate(|state| {
                expire_routes(state.routes.as_mut_slice(), now_unix_ms);
                if let Some(existing_index) = state.bindings.iter().position(|binding| {
                    binding.status == BridgeBindingStatus::Active
                        && binding.newapi_user_ref == newapi_user_ref
                        && binding.oasis_sender_account_id == oasis_sender_account_id
                }) {
                    let bridge_user_id = {
                        let existing = &mut state.bindings[existing_index];
                        existing.letai_external_user_name = external_user_name
                            .clone()
                            .or(existing.letai_external_user_name.clone());
                        existing.email = email.clone().or(existing.email.clone());
                        if request.metadata.is_some() {
                            existing.metadata = request.metadata.clone();
                        }
                        existing.updated_at_unix_ms = now_unix_ms;
                        existing.bridge_user_id.clone()
                    };
                    let project_binding = ensure_project_binding(
                        state,
                        bridge_user_id.as_str(),
                        newapi_user_ref.as_str(),
                        Some(project_name.as_str()),
                        request.project_metadata.clone(),
                        now_unix_ms,
                    );
                    let existing = state
                        .bindings
                        .iter()
                        .find(|binding| binding.bridge_user_id == bridge_user_id)
                        .cloned()
                        .expect("binding exists after update");
                    return Ok(bind_response(&existing, &project_binding, true));
                }

                if let Some(existing) = state.bindings.iter().find(|binding| {
                    binding.status == BridgeBindingStatus::Active
                        && binding.newapi_user_ref == newapi_user_ref
                }) {
                    return Err(BridgeServiceError::conflict(
                        "binding_conflict",
                        format!(
                            "newapi_user_ref `{newapi_user_ref}` is already bound to {}",
                            existing.bridge_user_id
                        ),
                    ));
                }
                if let Some(existing) = state.bindings.iter().find(|binding| {
                    binding.status == BridgeBindingStatus::Active
                        && binding.oasis_sender_account_id == oasis_sender_account_id
                }) {
                    return Err(BridgeServiceError::conflict(
                        "binding_conflict",
                        format!(
                            "oasis_sender_account_id `{oasis_sender_account_id}` is already bound to {}",
                            existing.bridge_user_id
                        ),
                    ));
                }

                let bridge_user_id = format!("bridge-user-{:06}", state.next_binding_seq);
                state.next_binding_seq = state.next_binding_seq.saturating_add(1);
                let binding = BridgeBinding {
                    bridge_user_id: bridge_user_id.clone(),
                    newapi_user_ref: newapi_user_ref.clone(),
                    oasis_sender_account_id: oasis_sender_account_id.clone(),
                    letai_external_user_id,
                    letai_external_user_name: external_user_name,
                    email,
                    metadata: request.metadata.clone(),
                    platform_user_id: None,
                    status: BridgeBindingStatus::Active,
                    created_at_unix_ms: now_unix_ms,
                    updated_at_unix_ms: now_unix_ms,
                };
                state.bindings.push(binding);
                let project_binding = ensure_project_binding(
                    state,
                    bridge_user_id.as_str(),
                    newapi_user_ref.as_str(),
                    Some(project_name.as_str()),
                    request.project_metadata.clone(),
                    now_unix_ms,
                );
                let binding = state
                    .bindings
                    .iter()
                    .find(|binding| binding.bridge_user_id == bridge_user_id)
                    .cloned()
                    .expect("binding exists after insert");
                Ok(bind_response(&binding, &project_binding, false))
            })
            .map_err(|err| map_store_error(err, "persist bind bridge user failed"))?;
        self.ensure_inference_binding_ready(response.bridge_user_id.as_str(), now_unix_ms)?;
        Ok(response)
    }

    pub(super) fn create_deposit_route(
        &self,
        request: CreateDepositRouteRequest,
        now_unix_ms: i64,
    ) -> Result<CreateDepositRouteResponse, BridgeServiceError> {
        let bridge_user_id = normalize_required("bridge_user_id", request.bridge_user_id.as_str())?;
        let pricing_version = normalize_optional(request.pricing_version.as_deref());
        let topup_plan_id = normalize_optional(request.topup_plan_id.as_deref());
        if pricing_version.is_none() && topup_plan_id.is_none() {
            return Err(BridgeServiceError::bad_request(
                "missing_plan_context",
                "pricing_version or topup_plan_id is required",
            ));
        }

        self.store
            .mutate(|state| {
                expire_routes(state.routes.as_mut_slice(), now_unix_ms);
                let binding = state
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

                if let Some(existing) = state.routes.iter().find(|route| {
                    route.bridge_user_id == bridge_user_id
                        && route.status == DepositRouteStatus::Issued
                        && route.expires_at_unix_ms > now_unix_ms
                }) {
                    return Ok(route_response(existing, true));
                }

                let route_id = format!("route-{:06}", state.next_route_seq);
                let deposit_account_id = format!(
                    "{}{:06}",
                    self.config.deposit_account_prefix, state.next_route_seq
                );
                state.next_route_seq = state.next_route_seq.saturating_add(1);
                let expires_at_unix_ms =
                    compute_route_expiry_unix_ms(self.config.route_ttl_seconds, now_unix_ms)?;
                let route = DepositRoute {
                    route_id: route_id.clone(),
                    bridge_user_id: bridge_user_id.clone(),
                    beneficiary_ref: binding.newapi_user_ref,
                    deposit_account_id,
                    route_type: ROUTE_TYPE_OPERATOR_ASSIGNED_ACCOUNT.to_string(),
                    pricing_version: pricing_version.clone(),
                    topup_plan_id: topup_plan_id.clone(),
                    expires_at_unix_ms,
                    status: DepositRouteStatus::Issued,
                    created_at_unix_ms: now_unix_ms,
                    updated_at_unix_ms: now_unix_ms,
                };
                state.routes.push(route.clone());
                Ok(route_response(&route, false))
            })
            .map_err(|err| map_store_error(err, "persist deposit route failed"))
    }

    pub(super) fn reconcile_once(
        &self,
        now_unix_ms: i64,
    ) -> Result<BridgeReconcileResponse, BridgeServiceError> {
        self.store
            .mutate(|state| {
                expire_routes(state.routes.as_mut_slice(), now_unix_ms);
                Ok(())
            })
            .map_err(|err| map_store_error(err, "persist route expiry before reconcile failed"))?;

        let chain_client = self.chain_client()?;
        let committed_height = chain_client
            .fetch_committed_height()
            .map_err(|err| BridgeServiceError::bad_gateway("chain_explorer_unavailable", err))?;

        let routes = self
            .store
            .snapshot()
            .routes
            .into_iter()
            .filter(|route| route.status != DepositRouteStatus::Disabled)
            .collect::<Vec<_>>();

        let mut observed_new_deposit_count = 0usize;
        let mut updated_deposit_count = 0usize;
        for route in &routes {
            let transfers = chain_client
                .fetch_confirmed_account_txs(route.deposit_account_id.as_str())
                .map_err(|err| {
                    BridgeServiceError::bad_gateway("chain_explorer_unavailable", err)
                })?;
            for transfer in transfers
                .into_iter()
                .filter(|tx| tx.to_account_id == route.deposit_account_id)
            {
                let outcome =
                    self.observe_chain_transfer(route, &transfer, committed_height, now_unix_ms)?;
                observed_new_deposit_count += outcome.new_entries;
                updated_deposit_count += outcome.updated_entries;
            }
        }

        updated_deposit_count +=
            self.promote_pending_confirmations(committed_height, now_unix_ms)?;
        let reconciled_credit_count = self.process_credit_ready_entries(now_unix_ms)?;

        let snapshot = self.store.snapshot();
        Ok(BridgeReconcileResponse {
            ok: true,
            observed_at_unix_ms: now_unix_ms,
            latest_committed_height: Some(committed_height),
            scanned_route_count: routes.len(),
            observed_new_deposit_count,
            updated_deposit_count,
            reconciled_credit_count,
            manual_review_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::ManualReview)
                .count(),
            failed_credit_count: snapshot
                .ledger
                .iter()
                .filter(|entry| entry.state == BridgeLedgerState::Failed)
                .count(),
        })
    }

    pub(super) fn apply_operator_review(
        &self,
        bridge_deposit_id: &str,
        request: OperatorReviewRequest,
        now_unix_ms: i64,
    ) -> Result<OperatorReviewResponse, BridgeServiceError> {
        let bridge_deposit_id = normalize_required("bridge_deposit_id", bridge_deposit_id)?;
        let resolution = normalize_required("resolution", request.resolution.as_str())?;
        let operator_note = normalize_optional(request.operator_note.as_deref());
        self.store
            .mutate(|state| {
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
                let previous_state = entry.state.clone();
                if entry.state != BridgeLedgerState::ManualReview {
                    return Err(BridgeServiceError::conflict(
                        "invalid_review_state",
                        format!(
                            "bridge deposit `{bridge_deposit_id}` is in state {:?}, expected manual_review",
                            entry.state
                        ),
                    ));
                }

                let next_state = match resolution.as_str() {
                    "mark_resolved" | "resolve" => BridgeLedgerState::Resolved,
                    "close" => BridgeLedgerState::Closed,
                    other => {
                        return Err(BridgeServiceError::bad_request(
                            "invalid_resolution",
                            format!(
                                "unsupported resolution `{other}`; expected `mark_resolved`, `resolve`, or `close`"
                            ),
                        ))
                    }
                };
                entry.state = next_state.clone();
                entry.review_resolution = Some(resolution.clone());
                entry.operator_note = operator_note.clone();
                entry.updated_at_unix_ms = now_unix_ms;
                Ok(OperatorReviewResponse {
                    ok: true,
                    bridge_deposit_id,
                    previous_state,
                    state: next_state,
                    resolution,
                    operator_note,
                })
            })
            .map_err(|err| map_store_error(err, "persist operator review failed"))
    }

    pub(super) fn snapshot(&self) -> super::model::PersistedBridgeState {
        self.store.snapshot()
    }

    pub(super) fn store_mutate<T, F>(&self, op: F) -> Result<T, BridgeServiceError>
    where
        F: FnOnce(&mut super::model::PersistedBridgeState) -> Result<T, BridgeServiceError>,
    {
        self.store
            .mutate(op)
            .map_err(|err| map_store_error(err, "persist LetAI bridge state failed"))
    }

    #[cfg(test)]
    pub(super) fn store_mutate_test<T, F>(&self, op: F) -> Result<T, BridgeServiceError>
    where
        F: FnOnce(&mut super::model::PersistedBridgeState) -> Result<T, BridgeServiceError>,
    {
        self.store_mutate(op)
    }

    pub(super) fn max_credit_attempts(&self) -> u32 {
        self.config.max_credit_attempts.max(1)
    }

    pub(super) fn letai_parent_channel_id(&self) -> Option<String> {
        self.config.letai_parent_channel_id.clone()
    }

    fn chain_client(&self) -> Result<ChainExplorerClient, BridgeServiceError> {
        let Some(base_url) = self.config.chain_base_url.as_deref() else {
            return Err(BridgeServiceError::service_unavailable(
                "chain_explorer_not_configured",
                "bridge reconcile requires operator flag `--chain-base-url`",
            ));
        };
        ChainExplorerClient::new(base_url, self.config.chain_timeout_ms)
            .map_err(BridgeServiceError::internal)
    }

    fn letai_adapter(&self) -> Result<LetaiOpenApiAdapter, BridgeServiceError> {
        let Some(base_url) = self.config.letai_base_url.as_deref() else {
            return Err(BridgeServiceError::service_unavailable(
                "letai_openapi_not_configured",
                "bridge auto credit requires operator flags `--letai-base-url` and `--letai-platform-key`",
            ));
        };
        let Some(platform_key) = self.config.letai_platform_key.as_deref() else {
            return Err(BridgeServiceError::service_unavailable(
                "letai_openapi_not_configured",
                "bridge auto credit requires operator flags `--letai-base-url` and `--letai-platform-key`",
            ));
        };
        LetaiOpenApiAdapter::new(
            base_url,
            platform_key,
            self.config.letai_parent_channel_id.as_deref(),
            self.config.letai_timeout_ms,
        )
        .map_err(BridgeServiceError::internal)
    }

    fn observe_chain_transfer(
        &self,
        route: &DepositRoute,
        transfer: &ObservedChainTransfer,
        committed_height: u64,
        now_unix_ms: i64,
    ) -> Result<ObserveOutcome, BridgeServiceError> {
        let pricing_rules = self.pricing_rules_by_version();
        let required_confirmations = self.config.chain_confirmations_required.max(1);
        self.store
            .mutate(|state| {
                expire_routes(state.routes.as_mut_slice(), now_unix_ms);

                if let Some(existing) = state
                    .ledger
                    .iter_mut()
                    .find(|entry| ledger_matches_transfer(entry, route, transfer))
                {
                    let confirmations =
                        compute_confirmations(committed_height, transfer.block_height);
                    existing.chain_action_id = Some(transfer.action_id);
                    existing.block_height = transfer.block_height;
                    existing.confirmations = confirmations;
                    existing.updated_at_unix_ms = now_unix_ms;
                    if existing.state == BridgeLedgerState::PendingConfirmations
                        && confirmations >= existing.required_confirmations
                    {
                        existing.state = BridgeLedgerState::Confirmed;
                    }
                    return Ok(ObserveOutcome {
                        new_entries: 0,
                        updated_entries: 1,
                    });
                }

                let Some(route_position) = state
                    .routes
                    .iter()
                    .position(|candidate| candidate.route_id == route.route_id)
                else {
                    return Err(BridgeServiceError::not_found(
                        "route_not_found",
                        format!("deposit route `{}` does not exist", route.route_id),
                    ));
                };
                let route_state = state.routes[route_position].status.clone();
                let duplicate_route_deposit = state
                    .ledger
                    .iter()
                    .any(|entry| entry.route_id == route.route_id);
                let confirmations = compute_confirmations(committed_height, transfer.block_height);
                let pricing_evaluation = evaluate_pricing(
                    route.pricing_version.as_deref(),
                    route.topup_plan_id.as_deref(),
                    transfer.amount,
                    &pricing_rules,
                );
                let (
                    expected_amount_oc,
                    credit_units,
                    bonus_units,
                    total_credit_units,
                    state_name,
                    review_reason,
                ) = match pricing_evaluation {
                    PricingEvaluation::Matched {
                        expected_amount_oc,
                        credit_units,
                        bonus_units,
                        total_credit_units,
                    } => {
                        if matches!(route_state, DepositRouteStatus::Expired) {
                            (
                                Some(expected_amount_oc),
                                credit_units,
                                bonus_units,
                                total_credit_units,
                                BridgeLedgerState::ManualReview,
                                Some("expired_route_deposit".to_string()),
                            )
                        } else if duplicate_route_deposit {
                            (
                                Some(expected_amount_oc),
                                credit_units,
                                bonus_units,
                                total_credit_units,
                                BridgeLedgerState::ManualReview,
                                Some("duplicate_route_deposit".to_string()),
                            )
                        } else if transfer.block_height.is_none() {
                            (
                                Some(expected_amount_oc),
                                credit_units,
                                bonus_units,
                                total_credit_units,
                                BridgeLedgerState::ManualReview,
                                Some("missing_block_height".to_string()),
                            )
                        } else if confirmations >= required_confirmations {
                            (
                                Some(expected_amount_oc),
                                credit_units,
                                bonus_units,
                                total_credit_units,
                                BridgeLedgerState::Confirmed,
                                None,
                            )
                        } else {
                            (
                                Some(expected_amount_oc),
                                credit_units,
                                bonus_units,
                                total_credit_units,
                                BridgeLedgerState::PendingConfirmations,
                                None,
                            )
                        }
                    }
                    PricingEvaluation::ManualReview {
                        expected_amount_oc,
                        reason,
                    } => (
                        expected_amount_oc,
                        0,
                        0,
                        0,
                        BridgeLedgerState::ManualReview,
                        Some(reason.to_string()),
                    ),
                };

                state.routes[route_position].status = DepositRouteStatus::Settled;
                state.routes[route_position].updated_at_unix_ms = now_unix_ms;

                let bridge_deposit_id = format!("bridge-deposit-{:06}", state.next_deposit_seq);
                state.next_deposit_seq = state.next_deposit_seq.saturating_add(1);
                let idempotency_key = build_idempotency_key(
                    route.route_id.as_str(),
                    transfer.tx_hash.as_str(),
                    transfer.action_id,
                );
                state.ledger.push(BridgeLedgerEntry {
                    bridge_deposit_id,
                    route_id: route.route_id.clone(),
                    bridge_user_id: route.bridge_user_id.clone(),
                    beneficiary_ref: route.beneficiary_ref.clone(),
                    deposit_account_id: route.deposit_account_id.clone(),
                    chain_tx_id: transfer.tx_hash.clone(),
                    chain_action_id: Some(transfer.action_id),
                    from_account_id: transfer.from_account_id.clone(),
                    amount_oc: transfer.amount,
                    expected_amount_oc,
                    pricing_version: route.pricing_version.clone(),
                    topup_plan_id: route.topup_plan_id.clone(),
                    credit_units,
                    bonus_units,
                    total_credit_units,
                    confirmations,
                    required_confirmations,
                    block_height: transfer.block_height,
                    idempotency_key,
                    platform_user_id: None,
                    platform_project_id: None,
                    token_key: None,
                    external_order_id: None,
                    quota: None,
                    amount_audit: None,
                    currency: None,
                    topup_receipt: None,
                    user_snapshot: None,
                    project_snapshot: None,
                    topup_log_snapshot: None,
                    state: state_name,
                    credit_attempt_count: 0,
                    review_reason,
                    review_resolution: None,
                    operator_note: None,
                    last_error_code: None,
                    last_error: None,
                    observed_at_unix_ms: now_unix_ms,
                    updated_at_unix_ms: now_unix_ms,
                });
                Ok(ObserveOutcome {
                    new_entries: 1,
                    updated_entries: 0,
                })
            })
            .map_err(|err| map_store_error(err, "persist observed chain transfer failed"))
    }

    fn promote_pending_confirmations(
        &self,
        committed_height: u64,
        now_unix_ms: i64,
    ) -> Result<usize, BridgeServiceError> {
        self.store
            .mutate(|state| {
                let mut updated = 0usize;
                for entry in &mut state.ledger {
                    if entry.state != BridgeLedgerState::PendingConfirmations {
                        continue;
                    }
                    let confirmations = compute_confirmations(committed_height, entry.block_height);
                    let mut changed = false;
                    if confirmations != entry.confirmations {
                        entry.confirmations = confirmations;
                        entry.updated_at_unix_ms = now_unix_ms;
                        changed = true;
                    }
                    if confirmations >= entry.required_confirmations
                        && entry.state != BridgeLedgerState::Confirmed
                    {
                        entry.state = BridgeLedgerState::Confirmed;
                        entry.updated_at_unix_ms = now_unix_ms;
                        changed = true;
                    }
                    if changed {
                        updated += 1;
                    }
                }
                Ok(updated)
            })
            .map_err(|err| map_store_error(err, "persist confirmation promotion failed"))
    }

    fn pricing_rules_by_version(&self) -> BTreeMap<String, BridgePricingRuleConfig> {
        self.config
            .pricing_rules
            .iter()
            .cloned()
            .map(|rule| (rule.pricing_version.clone(), rule))
            .collect()
    }
}

fn bind_response(
    binding: &BridgeBinding,
    project_binding: &LetaiProjectBinding,
    reused_existing_binding: bool,
) -> BindBridgeUserResponse {
    BindBridgeUserResponse {
        ok: true,
        bridge_user_id: binding.bridge_user_id.clone(),
        newapi_user_ref: binding.newapi_user_ref.clone(),
        oasis_sender_account_id: binding.oasis_sender_account_id.clone(),
        letai_external_user_id: binding.letai_external_user_id.clone(),
        letai_external_project_id: project_binding.letai_external_project_id.clone(),
        project_name: project_binding.project_name.clone(),
        binding_status: binding.status.clone(),
        reused_existing_binding,
        created_at_unix_ms: binding.created_at_unix_ms,
    }
}

fn route_response(route: &DepositRoute, reused_existing_route: bool) -> CreateDepositRouteResponse {
    CreateDepositRouteResponse {
        ok: true,
        route_id: route.route_id.clone(),
        bridge_user_id: route.bridge_user_id.clone(),
        beneficiary_ref: route.beneficiary_ref.clone(),
        deposit_account_id: route.deposit_account_id.clone(),
        route_type: route.route_type.clone(),
        route_status: route.status.clone(),
        pricing_version: route.pricing_version.clone(),
        topup_plan_id: route.topup_plan_id.clone(),
        expires_at_unix_ms: route.expires_at_unix_ms,
        reused_existing_route,
    }
}

fn normalize_required(field: &str, value: &str) -> Result<String, BridgeServiceError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BridgeServiceError::bad_request(
            "invalid_request",
            format!("{field} must not be empty"),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn expire_routes(routes: &mut [DepositRoute], now_unix_ms: i64) {
    for route in routes.iter_mut() {
        if route.status == DepositRouteStatus::Issued && route.expires_at_unix_ms <= now_unix_ms {
            route.status = DepositRouteStatus::Expired;
            route.updated_at_unix_ms = now_unix_ms;
        }
    }
}

fn compute_route_expiry_unix_ms(
    route_ttl_seconds: u64,
    now_unix_ms: i64,
) -> Result<i64, BridgeServiceError> {
    let ttl_ms_u64 = route_ttl_seconds.checked_mul(1000).ok_or_else(|| {
        BridgeServiceError::internal("bridge-service route_ttl_seconds overflowed milliseconds")
    })?;
    let ttl_ms_i64 = i64::try_from(ttl_ms_u64).map_err(|_| {
        BridgeServiceError::internal("bridge-service route_ttl_seconds exceeds i64 milliseconds")
    })?;
    Ok(now_unix_ms.saturating_add(ttl_ms_i64))
}

fn map_store_error(err: StoreMutateError<BridgeServiceError>, context: &str) -> BridgeServiceError {
    match err {
        StoreMutateError::Domain(err) => err,
        StoreMutateError::Persist(err) => BridgeServiceError::internal(format!("{context}: {err}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObserveOutcome {
    new_entries: usize,
    updated_entries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PricingEvaluation<'a> {
    Matched {
        expected_amount_oc: u64,
        credit_units: u64,
        bonus_units: u64,
        total_credit_units: u64,
    },
    ManualReview {
        expected_amount_oc: Option<u64>,
        reason: &'a str,
    },
}

fn evaluate_pricing<'a>(
    pricing_version: Option<&str>,
    topup_plan_id: Option<&str>,
    amount_oc: u64,
    pricing_rules: &'a BTreeMap<String, BridgePricingRuleConfig>,
) -> PricingEvaluation<'a> {
    let Some(pricing_version) = pricing_version else {
        return if topup_plan_id.is_some() {
            PricingEvaluation::ManualReview {
                expected_amount_oc: None,
                reason: "topup_plan_auto_credit_not_supported",
            }
        } else {
            PricingEvaluation::ManualReview {
                expected_amount_oc: None,
                reason: "missing_pricing_rule",
            }
        };
    };
    let Some(rule) = pricing_rules.get(pricing_version) else {
        return PricingEvaluation::ManualReview {
            expected_amount_oc: None,
            reason: "pricing_rule_missing",
        };
    };
    if amount_oc < rule.oc_amount {
        return PricingEvaluation::ManualReview {
            expected_amount_oc: Some(rule.oc_amount),
            reason: "underpay",
        };
    }
    if amount_oc > rule.oc_amount {
        return PricingEvaluation::ManualReview {
            expected_amount_oc: Some(rule.oc_amount),
            reason: "overpay",
        };
    }
    PricingEvaluation::Matched {
        expected_amount_oc: rule.oc_amount,
        credit_units: rule.credit_units,
        bonus_units: rule.bonus_units,
        total_credit_units: rule.credit_units.saturating_add(rule.bonus_units),
    }
}

fn compute_confirmations(committed_height: u64, block_height: Option<u64>) -> u64 {
    let Some(block_height) = block_height else {
        return 0;
    };
    if committed_height < block_height {
        return 0;
    }
    committed_height
        .saturating_sub(block_height)
        .saturating_add(1)
}

fn ledger_matches_transfer(
    entry: &BridgeLedgerEntry,
    route: &DepositRoute,
    transfer: &ObservedChainTransfer,
) -> bool {
    entry.route_id == route.route_id
        && entry.chain_tx_id == transfer.tx_hash
        && match entry.chain_action_id {
            Some(chain_action_id) => chain_action_id == transfer.action_id,
            None => true,
        }
}

fn build_idempotency_key(route_id: &str, chain_tx_id: &str, chain_action_id: u64) -> String {
    format!("bridge-credit:{route_id}:{chain_tx_id}:{chain_action_id}")
}

impl BridgeService {
    pub(super) fn build_external_order_id(bridge_deposit_id: &str) -> String {
        format!("letai-topup:{bridge_deposit_id}")
    }

    pub(super) fn build_letai_external_user_id(newapi_user_ref: &str) -> String {
        format!("oasis7-user:{newapi_user_ref}")
    }

    pub(super) fn build_letai_external_project_id(bridge_user_id: &str) -> String {
        format!("oasis7-project:{bridge_user_id}")
    }

    pub(super) fn default_project_name(newapi_user_ref: &str) -> String {
        format!("{newapi_user_ref}-default")
    }
}
