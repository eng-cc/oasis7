use serde::{Deserialize, Serialize};
use serde_json::Value;

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
#[serde(rename_all = "snake_case")]
pub(super) enum BridgeLedgerState {
    Detected,
    PendingConfirmations,
    Confirmed,
    Crediting,
    Credited,
    Reconciled,
    Failed,
    ManualReview,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super) struct BridgeLedgerEntry {
    pub(super) bridge_deposit_id: String,
    pub(super) route_id: String,
    pub(super) bridge_user_id: String,
    pub(super) beneficiary_ref: String,
    pub(super) deposit_account_id: String,
    pub(super) chain_tx_id: String,
    pub(super) chain_action_id: Option<u64>,
    pub(super) from_account_id: String,
    pub(super) amount_oc: u64,
    pub(super) expected_amount_oc: Option<u64>,
    pub(super) pricing_version: Option<String>,
    pub(super) topup_plan_id: Option<String>,
    pub(super) credit_units: u64,
    pub(super) bonus_units: u64,
    pub(super) total_credit_units: u64,
    pub(super) confirmations: u64,
    pub(super) required_confirmations: u64,
    pub(super) block_height: Option<u64>,
    pub(super) target_type: String,
    pub(super) idempotency_key: String,
    pub(super) state: BridgeLedgerState,
    #[serde(default)]
    pub(super) credit_attempt_count: u32,
    #[serde(default)]
    pub(super) review_reason: Option<String>,
    #[serde(default)]
    pub(super) review_resolution: Option<String>,
    #[serde(default)]
    pub(super) operator_note: Option<String>,
    #[serde(default)]
    pub(super) adapter_receipt: Option<Value>,
    #[serde(default)]
    pub(super) last_error_code: Option<String>,
    #[serde(default)]
    pub(super) last_error: Option<String>,
    pub(super) observed_at_unix_ms: i64,
    pub(super) updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super) struct PersistedBridgeState {
    pub(super) schema_version: u32,
    pub(super) next_binding_seq: u64,
    pub(super) next_route_seq: u64,
    #[serde(default = "default_next_deposit_seq")]
    pub(super) next_deposit_seq: u64,
    #[serde(default)]
    pub(super) bindings: Vec<BridgeBinding>,
    #[serde(default)]
    pub(super) routes: Vec<DepositRoute>,
    #[serde(default)]
    pub(super) ledger: Vec<BridgeLedgerEntry>,
}

impl Default for PersistedBridgeState {
    fn default() -> Self {
        Self {
            schema_version: BRIDGE_STATE_SCHEMA_V1,
            next_binding_seq: 1,
            next_route_seq: 1,
            next_deposit_seq: default_next_deposit_seq(),
            bindings: Vec::new(),
            routes: Vec::new(),
            ledger: Vec::new(),
        }
    }
}

fn default_next_deposit_seq() -> u64 {
    1
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct OperatorReviewRequest {
    pub(super) resolution: String,
    pub(super) operator_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct OperatorReviewResponse {
    pub(super) ok: bool,
    pub(super) bridge_deposit_id: String,
    pub(super) previous_state: BridgeLedgerState,
    pub(super) state: BridgeLedgerState,
    pub(super) resolution: String,
    pub(super) operator_note: Option<String>,
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
    pub(super) ledger_count: usize,
    pub(super) pending_confirmation_count: usize,
    pub(super) manual_review_count: usize,
    pub(super) failed_credit_count: usize,
    pub(super) reconciled_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct BridgeReconcileResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) latest_committed_height: Option<u64>,
    pub(super) scanned_route_count: usize,
    pub(super) observed_new_deposit_count: usize,
    pub(super) updated_deposit_count: usize,
    pub(super) reconciled_credit_count: usize,
    pub(super) manual_review_count: usize,
    pub(super) failed_credit_count: usize,
}
