use std::sync::Arc;

use super::model::{
    BindBridgeUserRequest, BindBridgeUserResponse, BridgeBinding, BridgeBindingStatus,
    BridgeHealthResponse, CreateDepositRouteRequest, CreateDepositRouteResponse, DepositRoute,
    DepositRouteStatus,
};
use super::store::{BridgeStateStore, StoreMutateError};

const ROUTE_TYPE_OPERATOR_ASSIGNED_ACCOUNT: &str = "operator_assigned_account";

#[derive(Debug, Clone)]
pub(super) struct BridgeServiceConfig {
    pub(super) route_ttl_seconds: u64,
    pub(super) deposit_account_prefix: String,
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
            route_count: snapshot.routes.len(),
            active_route_count,
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
        self.store
            .mutate(|state| {
                expire_routes(state.routes.as_mut_slice(), now_unix_ms);
                if let Some(existing) = state.bindings.iter().find(|binding| {
                    binding.status == BridgeBindingStatus::Active
                        && binding.newapi_user_ref == newapi_user_ref
                        && binding.oasis_sender_account_id == oasis_sender_account_id
                }) {
                    return Ok(BindBridgeUserResponse {
                        ok: true,
                        bridge_user_id: existing.bridge_user_id.clone(),
                        newapi_user_ref: existing.newapi_user_ref.clone(),
                        oasis_sender_account_id: existing.oasis_sender_account_id.clone(),
                        binding_status: existing.status.clone(),
                        reused_existing_binding: true,
                        created_at_unix_ms: existing.created_at_unix_ms,
                    });
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
                    status: BridgeBindingStatus::Active,
                    created_at_unix_ms: now_unix_ms,
                    updated_at_unix_ms: now_unix_ms,
                };
                state.bindings.push(binding.clone());
                Ok(BindBridgeUserResponse {
                    ok: true,
                    bridge_user_id,
                    newapi_user_ref: binding.newapi_user_ref,
                    oasis_sender_account_id: binding.oasis_sender_account_id,
                    binding_status: binding.status,
                    reused_existing_binding: false,
                    created_at_unix_ms: binding.created_at_unix_ms,
                })
            })
            .map_err(|err| map_store_error(err, "persist bind bridge user failed"))
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
                    now_unix_ms.saturating_add((self.config.route_ttl_seconds * 1000) as i64);
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

    #[cfg(test)]
    pub(super) fn snapshot(&self) -> super::model::PersistedBridgeState {
        self.store.snapshot()
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

fn map_store_error(err: StoreMutateError<BridgeServiceError>, context: &str) -> BridgeServiceError {
    match err {
        StoreMutateError::Domain(err) => err,
        StoreMutateError::Persist(err) => BridgeServiceError::internal(format!("{context}: {err}")),
    }
}
