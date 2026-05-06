use serde::{Deserialize, Serialize};

pub(super) const BRIDGE_STATE_SCHEMA_V1: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum BridgeBindingStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct BridgeBinding {
    pub(super) bridge_user_id: String,
    pub(super) newapi_user_ref: String,
    pub(super) oasis_sender_account_id: String,
    pub(super) status: BridgeBindingStatus,
    pub(super) created_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum DepositRouteStatus {
    Issued,
    Settled,
    Expired,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct DepositRoute {
    pub(super) route_id: String,
    pub(super) bridge_user_id: String,
    pub(super) beneficiary_ref: String,
    pub(super) deposit_account_id: String,
    pub(super) route_type: String,
    pub(super) pricing_version: Option<String>,
    pub(super) topup_plan_id: Option<String>,
    pub(super) expires_at_unix_ms: i64,
    pub(super) status: DepositRouteStatus,
    pub(super) created_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct PersistedBridgeState {
    pub(super) schema_version: u32,
    pub(super) next_binding_seq: u64,
    pub(super) next_route_seq: u64,
    pub(super) bindings: Vec<BridgeBinding>,
    pub(super) routes: Vec<DepositRoute>,
}

impl Default for PersistedBridgeState {
    fn default() -> Self {
        Self {
            schema_version: BRIDGE_STATE_SCHEMA_V1,
            next_binding_seq: 1,
            next_route_seq: 1,
            bindings: Vec::new(),
            routes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct BindBridgeUserRequest {
    pub(super) newapi_user_ref: String,
    pub(super) oasis_sender_account_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct BindBridgeUserResponse {
    pub(super) ok: bool,
    pub(super) bridge_user_id: String,
    pub(super) newapi_user_ref: String,
    pub(super) oasis_sender_account_id: String,
    pub(super) binding_status: BridgeBindingStatus,
    pub(super) reused_existing_binding: bool,
    pub(super) created_at_unix_ms: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct CreateDepositRouteRequest {
    pub(super) bridge_user_id: String,
    pub(super) pricing_version: Option<String>,
    pub(super) topup_plan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct CreateDepositRouteResponse {
    pub(super) ok: bool,
    pub(super) route_id: String,
    pub(super) bridge_user_id: String,
    pub(super) beneficiary_ref: String,
    pub(super) deposit_account_id: String,
    pub(super) route_type: String,
    pub(super) route_status: DepositRouteStatus,
    pub(super) pricing_version: Option<String>,
    pub(super) topup_plan_id: Option<String>,
    pub(super) expires_at_unix_ms: i64,
    pub(super) reused_existing_route: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct BridgeHealthResponse {
    pub(super) ok: bool,
    pub(super) service: String,
    pub(super) observed_at_unix_ms: i64,
    pub(super) binding_count: usize,
    pub(super) active_binding_count: usize,
    pub(super) route_count: usize,
    pub(super) active_route_count: usize,
}
