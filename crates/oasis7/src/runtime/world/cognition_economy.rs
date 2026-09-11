//! Runtime-owned cognition economy leases and receipts.
//!
//! The provider may request a quote, but it never owns the balance, lease, or
//! receipt.  This projection is kept beside the existing cognition journal so
//! reserve and terminal transitions can be persisted in the same World
//! snapshot without borrowing scheduler wake capacity as an accounting lane.

use super::super::cognition_recovery::cognition_digest_v1;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fmt;

#[path = "cognition_economy_validation.rs"]
mod validation;

pub const COGNITION_ECONOMY_SCHEMA_VERSION: &str = "cognition-economy.v1";
pub const COGNITION_LEASE_SCHEMA_VERSION: &str = "cognition-lease.v1";
pub const COGNITION_RECEIPT_SCHEMA_VERSION: &str = "cognition-receipt.v1";
const COGNITION_ECONOMY_EVENT_SCHEMA_VERSION: &str = "cognition-economy-event.v1";
const COGNITION_ECONOMY_JOURNAL_DOMAIN: &str = "oasis7.cognition.economy.journal-head.v1";
const COGNITION_ECONOMY_EVENT_DOMAIN: &str = "oasis7.cognition.economy.event.v1";
const COGNITION_ECONOMY_QUOTE_DOMAIN: &str = "oasis7.cognition.economy.quote.v1";
const COGNITION_ECONOMY_REQUEST_DOMAIN: &str = "oasis7.cognition.economy.request.v1";
const COGNITION_ECONOMY_LEASE_ID_DOMAIN: &str = "oasis7.cognition.economy.lease-id.v1";
const COGNITION_ECONOMY_OPERATION_DOMAIN: &str = "oasis7.cognition.economy.operation.v1";
const COGNITION_ECONOMY_RECEIPT_ID_DOMAIN: &str = "oasis7.cognition.economy.receipt-id.v1";
const COGNITION_ECONOMY_RECEIPT_DOMAIN: &str = "oasis7.cognition.economy.receipt.v1";
const MAX_IDENTITY_BYTES: usize = 256;

fn bounded_identity(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_IDENTITY_BYTES
        && !value.chars().any(char::is_control)
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn economy_digest<T: Serialize>(domain: &str, value: &T) -> String {
    cognition_digest_v1(domain, value)
}

/// Stable error categories emitted by the runtime economy boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CognitionEconomyError {
    InvalidInput(&'static str),
    InvalidState(&'static str),
    Conflict(&'static str),
    InsufficientBalance {
        account_id: String,
        resource: String,
        requested: u64,
        available: u64,
    },
    LeaseNotFound(String),
}

impl CognitionEconomyError {
    pub fn code(&self) -> &str {
        match self {
            Self::InvalidInput(code) | Self::InvalidState(code) | Self::Conflict(code) => code,
            Self::InsufficientBalance { .. } => "cognition_insufficient_balance",
            Self::LeaseNotFound(_) => "cognition_lease_not_found",
        }
    }
}

impl fmt::Display for CognitionEconomyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(code) | Self::InvalidState(code) | Self::Conflict(code) => {
                formatter.write_str(code)
            }
            Self::InsufficientBalance {
                account_id,
                resource,
                requested,
                available,
            } => write!(
                formatter,
                "cognition_insufficient_balance account={account_id} resource={resource} requested={requested} available={available}"
            ),
            Self::LeaseNotFound(lease_id) => {
                write!(formatter, "cognition_lease_not_found lease_id={lease_id}")
            }
        }
    }
}

impl std::error::Error for CognitionEconomyError {}

/// A host-issued immutable integer-resource quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionLeaseQuoteV1 {
    pub schema_version: String,
    pub quote_id: String,
    pub resource: String,
    pub amount: u64,
    #[serde(default)]
    pub valid_until_tick: Option<u64>,
    pub quote_digest: String,
}

/// Short compatibility name for callers that treat a quote independently of
/// the lease request.
pub type CognitionQuoteV1 = CognitionLeaseQuoteV1;

impl CognitionLeaseQuoteV1 {
    pub fn new(quote_id: impl Into<String>, resource: impl Into<String>, amount: u64) -> Self {
        let mut quote = Self {
            schema_version: COGNITION_LEASE_SCHEMA_VERSION.to_string(),
            quote_id: quote_id.into(),
            resource: resource.into(),
            amount,
            valid_until_tick: None,
            quote_digest: String::new(),
        };
        quote.refresh_digest();
        quote
    }

    pub fn with_valid_until_tick(mut self, tick: u64) -> Self {
        self.valid_until_tick = Some(tick);
        self.refresh_digest();
        self
    }

    pub fn recompute_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("lease quote is serializable");
        value
            .as_object_mut()
            .expect("lease quote is an object")
            .remove("quote_digest");
        economy_digest(COGNITION_ECONOMY_QUOTE_DOMAIN, &value)
    }

    pub fn refresh_digest(&mut self) {
        self.quote_digest = self.recompute_digest();
    }

    pub fn validate(&self) -> Result<(), CognitionEconomyError> {
        if self.schema_version != COGNITION_LEASE_SCHEMA_VERSION
            || !bounded_identity(&self.quote_id)
            || !bounded_identity(&self.resource)
            || self.amount == 0
            || !valid_digest(&self.quote_digest)
            || self.quote_digest != self.recompute_digest()
        {
            return Err(CognitionEconomyError::InvalidInput(
                "cognition_quote_invalid",
            ));
        }
        Ok(())
    }
}

/// Correlation supplied by an admission adapter.  Runtime derives lease and
/// receipt identities from this value and the immutable quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionLeaseRequestV1 {
    pub schema_version: String,
    pub idempotency_key: String,
    pub account_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub quote: CognitionLeaseQuoteV1,
}

impl CognitionLeaseRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        idempotency_key: impl Into<String>,
        account_id: impl Into<String>,
        agent_id: impl Into<String>,
        agent_session_id: impl Into<String>,
        agent_turn_id: impl Into<String>,
        decision_request_id: impl Into<String>,
        request_digest: impl Into<String>,
        quote: CognitionLeaseQuoteV1,
    ) -> Self {
        Self {
            schema_version: COGNITION_LEASE_SCHEMA_VERSION.to_string(),
            idempotency_key: idempotency_key.into(),
            account_id: account_id.into(),
            agent_id: agent_id.into(),
            agent_session_id: agent_session_id.into(),
            agent_turn_id: agent_turn_id.into(),
            decision_request_id: decision_request_id.into(),
            request_digest: request_digest.into(),
            quote,
        }
    }

    pub fn validate(&self) -> Result<(), CognitionEconomyError> {
        if self.schema_version != COGNITION_LEASE_SCHEMA_VERSION
            || !bounded_identity(&self.idempotency_key)
            || !bounded_identity(&self.account_id)
            || !bounded_identity(&self.agent_id)
            || !bounded_identity(&self.agent_session_id)
            || !bounded_identity(&self.agent_turn_id)
            || !bounded_identity(&self.decision_request_id)
            || !bounded_identity(&self.request_digest)
        {
            return Err(CognitionEconomyError::InvalidInput(
                "cognition_lease_request_invalid",
            ));
        }
        self.quote.validate()
    }

    fn fingerprint_digest(&self) -> String {
        economy_digest(
            COGNITION_ECONOMY_REQUEST_DOMAIN,
            &(
                &self.schema_version,
                &self.account_id,
                &self.agent_id,
                &self.agent_session_id,
                &self.agent_turn_id,
                &self.decision_request_id,
                &self.request_digest,
                &self.quote,
            ),
        )
    }

    pub fn derived_lease_id(&self) -> String {
        economy_digest(
            COGNITION_ECONOMY_LEASE_ID_DOMAIN,
            &(self.idempotency_key.as_str(), self.fingerprint_digest()),
        )
    }
}

/// Terminal state of a lease.  `Reserved` is the only state that can be
/// settled or released.  Refund may close either a reserved or settled lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitionLeaseStatusV1 {
    Reserved,
    Settled,
    Released,
    Refunded,
}

/// Durable lease identity and its immutable quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionLeaseV1 {
    pub schema_version: String,
    pub lease_id: String,
    pub idempotency_key: String,
    pub account_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub quote: CognitionLeaseQuoteV1,
    pub reserved_amount: u64,
    pub settled_amount: u64,
    pub refunded_amount: u64,
    pub status: CognitionLeaseStatusV1,
    pub reserved_at_tick: u64,
    #[serde(default)]
    pub closed_at_tick: Option<u64>,
    #[serde(default)]
    pub receipt_id: Option<String>,
}

/// Stable short name for the durable lease record.
pub type CognitionLease = CognitionLeaseV1;

impl CognitionLeaseV1 {
    pub fn validate(&self) -> Result<(), CognitionEconomyError> {
        if self.schema_version != COGNITION_LEASE_SCHEMA_VERSION
            || !bounded_identity(&self.lease_id)
            || !bounded_identity(&self.idempotency_key)
            || !bounded_identity(&self.account_id)
            || !bounded_identity(&self.agent_id)
            || !bounded_identity(&self.agent_session_id)
            || !bounded_identity(&self.agent_turn_id)
            || !bounded_identity(&self.decision_request_id)
            || !bounded_identity(&self.request_digest)
            || self.reserved_amount == 0
            || self.reserved_amount != self.quote.amount
        {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_lease_invalid",
            ));
        }
        self.quote.validate()?;
        let request = CognitionLeaseRequestV1::new(
            self.idempotency_key.clone(),
            self.account_id.clone(),
            self.agent_id.clone(),
            self.agent_session_id.clone(),
            self.agent_turn_id.clone(),
            self.decision_request_id.clone(),
            self.request_digest.clone(),
            self.quote.clone(),
        );
        if request.derived_lease_id() != self.lease_id {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_lease_identity_invalid",
            ));
        }
        if self.settled_amount > self.reserved_amount
            || self.refunded_amount > self.reserved_amount
            || (self.status == CognitionLeaseStatusV1::Reserved
                && (self.settled_amount != 0
                    || self.refunded_amount != 0
                    || self.closed_at_tick.is_some()
                    || self.receipt_id.is_some()))
            || (self.status == CognitionLeaseStatusV1::Released
                && (self.settled_amount != 0 || self.refunded_amount != self.reserved_amount))
            || (self.status == CognitionLeaseStatusV1::Refunded
                && self.refunded_amount != self.reserved_amount)
            || (self.status != CognitionLeaseStatusV1::Reserved
                && (self.closed_at_tick.is_none()
                    || self
                        .receipt_id
                        .as_deref()
                        .is_none_or(|id| !valid_digest(id))))
            || (self.status == CognitionLeaseStatusV1::Settled
                && self.settled_amount.checked_add(self.refunded_amount)
                    != Some(self.reserved_amount))
        {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_lease_state_invalid",
            ));
        }
        Ok(())
    }
}

/// Immutable economic receipt for one terminal operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionReceiptV1 {
    pub schema_version: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub lease_id: String,
    pub idempotency_key: String,
    pub account_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub quote: CognitionLeaseQuoteV1,
    pub reserved_amount: u64,
    pub consumed_amount: u64,
    pub refunded_amount: u64,
    pub status: CognitionLeaseStatusV1,
    pub issued_at_tick: u64,
}

/// Stable short name for the durable economic receipt.
pub type CognitionReceipt = CognitionReceiptV1;

impl CognitionReceiptV1 {
    pub fn recompute_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("cognition receipt is serializable");
        value
            .as_object_mut()
            .expect("cognition receipt is an object")
            .remove("receipt_digest");
        economy_digest(COGNITION_ECONOMY_RECEIPT_DOMAIN, &value)
    }

    pub fn validate(&self) -> Result<(), CognitionEconomyError> {
        if self.schema_version != COGNITION_RECEIPT_SCHEMA_VERSION
            || !bounded_identity(&self.receipt_id)
            || !valid_digest(&self.receipt_id)
            || !valid_digest(&self.receipt_digest)
            || !bounded_identity(&self.lease_id)
            || !bounded_identity(&self.idempotency_key)
            || !bounded_identity(&self.account_id)
            || !bounded_identity(&self.agent_id)
            || !bounded_identity(&self.agent_session_id)
            || !bounded_identity(&self.agent_turn_id)
            || !bounded_identity(&self.decision_request_id)
            || !bounded_identity(&self.request_digest)
            || self.reserved_amount == 0
            || self.consumed_amount > self.reserved_amount
            || self.refunded_amount > self.reserved_amount
            || self.status == CognitionLeaseStatusV1::Reserved
            || (self.status == CognitionLeaseStatusV1::Settled
                && self.consumed_amount.checked_add(self.refunded_amount)
                    != Some(self.reserved_amount))
            || (self.status == CognitionLeaseStatusV1::Released
                && (self.consumed_amount != 0 || self.refunded_amount != self.reserved_amount))
            || (self.status == CognitionLeaseStatusV1::Refunded && self.consumed_amount != 0)
            || self.receipt_digest != self.recompute_digest()
        {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_receipt_invalid",
            ));
        }
        self.quote.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionResourceBalanceV1 {
    pub available: u64,
    pub reserved: u64,
}

impl Default for CognitionResourceBalanceV1 {
    fn default() -> Self {
        Self {
            available: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionEconomyIdempotencyRecordV1 {
    pub request_fingerprint: String,
    pub lease_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionEconomyOperationRecordV1 {
    pub operation_key: String,
    pub operation_digest: String,
    pub lease_id: String,
    pub receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionEconomyEventV1 {
    pub schema_version: String,
    pub journal_seq: u64,
    pub parent_event_digest: String,
    pub event_kind: String,
    pub lease_id: String,
    pub operation_key: String,
    pub idempotency_key: String,
    pub resource: String,
    pub reserved_amount: u64,
    pub consumed_amount: u64,
    pub refunded_amount: u64,
    pub status: CognitionLeaseStatusV1,
    pub receipt_id: String,
    pub event_digest: String,
}

impl CognitionEconomyEventV1 {
    fn recompute_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("economy event is serializable");
        value
            .as_object_mut()
            .expect("economy event is an object")
            .remove("event_digest");
        economy_digest(COGNITION_ECONOMY_EVENT_DOMAIN, &value)
    }
}

/// The complete durable cognition economy projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionEconomyStateV1 {
    pub schema_version: String,
    #[serde(default)]
    pub balances: BTreeMap<String, BTreeMap<String, CognitionResourceBalanceV1>>,
    #[serde(default)]
    pub leases: BTreeMap<String, CognitionLeaseV1>,
    #[serde(default)]
    pub receipts: BTreeMap<String, CognitionReceiptV1>,
    #[serde(default)]
    pub idempotency: BTreeMap<String, CognitionEconomyIdempotencyRecordV1>,
    #[serde(default)]
    pub operations: BTreeMap<String, CognitionEconomyOperationRecordV1>,
    #[serde(default)]
    pub journal: Vec<CognitionEconomyEventV1>,
    #[serde(default)]
    pub head_seq: u64,
    #[serde(default)]
    pub head_digest: String,
}

pub type CognitionEconomyV1 = CognitionEconomyStateV1;

impl Default for CognitionEconomyStateV1 {
    fn default() -> Self {
        Self {
            schema_version: COGNITION_ECONOMY_SCHEMA_VERSION.to_string(),
            balances: BTreeMap::new(),
            leases: BTreeMap::new(),
            receipts: BTreeMap::new(),
            idempotency: BTreeMap::new(),
            operations: BTreeMap::new(),
            journal: Vec::new(),
            head_seq: 0,
            head_digest: economy_digest(
                COGNITION_ECONOMY_JOURNAL_DOMAIN,
                &(0_u64, Vec::<CognitionEconomyEventV1>::new()),
            ),
        }
    }
}

impl CognitionEconomyStateV1 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a validated typed projection from persisted cognition JSON.
    pub fn from_snapshot_json(value: JsonValue) -> Result<Self, CognitionEconomyError> {
        let state: Self = serde_json::from_value(value).map_err(|_| {
            CognitionEconomyError::InvalidState("cognition_economy_projection_invalid")
        })?;
        state.validate()?;
        Ok(state)
    }

    pub fn snapshot_json(&self) -> Result<JsonValue, CognitionEconomyError> {
        self.validate()?;
        serde_json::to_value(self).map_err(|_| {
            CognitionEconomyError::InvalidState("cognition_economy_projection_serialize_failed")
        })
    }

    pub fn available_balance(&self, account_id: &str, resource: &str) -> u64 {
        self.balances
            .get(account_id)
            .and_then(|resources| resources.get(resource))
            .map(|balance| balance.available)
            .unwrap_or_default()
    }

    pub fn reserved_balance(&self, account_id: &str, resource: &str) -> u64 {
        self.balances
            .get(account_id)
            .and_then(|resources| resources.get(resource))
            .map(|balance| balance.reserved)
            .unwrap_or_default()
    }

    /// Explicit bootstrap/credit operation.  It never refills implicitly and
    /// cannot overwrite an account while an active reservation exists.
    pub fn set_resource_balance(
        &mut self,
        account_id: impl Into<String>,
        resource: impl Into<String>,
        available: u64,
    ) -> Result<(), CognitionEconomyError> {
        let mut next = self.clone();
        next.set_resource_balance_inner(account_id.into(), resource.into(), available)?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn reserve(
        &mut self,
        request: CognitionLeaseRequestV1,
        tick: u64,
    ) -> Result<CognitionLeaseV1, CognitionEconomyError> {
        request.validate()?;
        let mut next = self.clone();
        let lease = next.reserve_inner(request, tick)?;
        next.validate()?;
        *self = next;
        Ok(lease)
    }

    pub fn settle(
        &mut self,
        lease_id: &str,
        consumed_amount: u64,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let mut next = self.clone();
        let receipt = next.settle_inner(lease_id, consumed_amount, tick)?;
        next.validate()?;
        *self = next;
        Ok(receipt)
    }

    pub fn release(
        &mut self,
        lease_id: &str,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let mut next = self.clone();
        let receipt = next.release_inner(lease_id, tick)?;
        next.validate()?;
        *self = next;
        Ok(receipt)
    }

    pub fn refund(
        &mut self,
        lease_id: &str,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let mut next = self.clone();
        let receipt = next.refund_inner(lease_id, tick)?;
        next.validate()?;
        *self = next;
        Ok(receipt)
    }

    fn set_resource_balance_inner(
        &mut self,
        account_id: String,
        resource: String,
        available: u64,
    ) -> Result<(), CognitionEconomyError> {
        if !bounded_identity(&account_id) || !bounded_identity(&resource) {
            return Err(CognitionEconomyError::InvalidInput(
                "cognition_balance_identity_invalid",
            ));
        }
        if self.reserved_balance(account_id.as_str(), resource.as_str()) != 0 {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_balance_has_active_reservation",
            ));
        }
        self.balances.entry(account_id).or_default().insert(
            resource,
            CognitionResourceBalanceV1 {
                available,
                reserved: 0,
            },
        );
        Ok(())
    }

    fn balance_mut(&mut self, account_id: &str, resource: &str) -> &mut CognitionResourceBalanceV1 {
        self.balances
            .entry(account_id.to_string())
            .or_default()
            .entry(resource.to_string())
            .or_default()
    }

    fn reserve_inner(
        &mut self,
        request: CognitionLeaseRequestV1,
        tick: u64,
    ) -> Result<CognitionLeaseV1, CognitionEconomyError> {
        let fingerprint = request.fingerprint_digest();
        if let Some(existing) = self.idempotency.get(&request.idempotency_key) {
            if existing.request_fingerprint != fingerprint {
                return Err(CognitionEconomyError::Conflict(
                    "cognition_idempotency_conflict",
                ));
            }
            return self.leases.get(&existing.lease_id).cloned().ok_or_else(|| {
                CognitionEconomyError::InvalidState("cognition_idempotency_lease_missing")
            });
        }
        if request
            .quote
            .valid_until_tick
            .is_some_and(|expires| tick > expires)
        {
            return Err(CognitionEconomyError::InvalidInput(
                "cognition_quote_expired",
            ));
        }
        let lease_id = request.derived_lease_id();
        let balance = self.balance_mut(&request.account_id, &request.quote.resource);
        if balance.available < request.quote.amount {
            return Err(CognitionEconomyError::InsufficientBalance {
                account_id: request.account_id,
                resource: request.quote.resource,
                requested: request.quote.amount,
                available: balance.available,
            });
        }
        balance.available -= request.quote.amount;
        balance.reserved = balance.reserved.checked_add(request.quote.amount).ok_or(
            CognitionEconomyError::InvalidState("cognition_balance_reservation_overflow"),
        )?;
        let lease = CognitionLeaseV1 {
            schema_version: COGNITION_LEASE_SCHEMA_VERSION.to_string(),
            lease_id: lease_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            account_id: request.account_id.clone(),
            agent_id: request.agent_id.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            request_digest: request.request_digest.clone(),
            quote: request.quote.clone(),
            reserved_amount: request.quote.amount,
            settled_amount: 0,
            refunded_amount: 0,
            status: CognitionLeaseStatusV1::Reserved,
            reserved_at_tick: tick,
            closed_at_tick: None,
            receipt_id: None,
        };
        self.leases.insert(lease_id.clone(), lease.clone());
        self.idempotency.insert(
            request.idempotency_key.clone(),
            CognitionEconomyIdempotencyRecordV1 {
                request_fingerprint: fingerprint,
                lease_id: lease_id.clone(),
            },
        );
        self.append_event(
            "reserve",
            &lease,
            &operation_key(&lease_id, "reserve"),
            "",
            0,
            0,
        )?;
        Ok(lease)
    }

    fn settle_inner(
        &mut self,
        lease_id: &str,
        consumed_amount: u64,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let lease = self
            .leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| CognitionEconomyError::LeaseNotFound(lease_id.to_string()))?;
        let key = operation_key(lease_id, "settle");
        let digest = economy_digest(
            COGNITION_ECONOMY_OPERATION_DOMAIN,
            &(key.as_str(), consumed_amount),
        );
        if let Some(existing) = self.operations.get(&key) {
            if existing.operation_digest != digest {
                return Err(CognitionEconomyError::Conflict(
                    "cognition_settlement_idempotency_conflict",
                ));
            }
            return self.receipts.get(&existing.receipt_id).cloned().ok_or(
                CognitionEconomyError::InvalidState("cognition_operation_receipt_missing"),
            );
        }
        if lease.status != CognitionLeaseStatusV1::Reserved {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_lease_already_closed",
            ));
        }
        if consumed_amount > lease.reserved_amount {
            return Err(CognitionEconomyError::InvalidInput(
                "cognition_settlement_exceeds_reservation",
            ));
        }
        let refund = lease.reserved_amount - consumed_amount;
        let balance = self.balance_mut(&lease.account_id, &lease.quote.resource);
        if balance.reserved < lease.reserved_amount {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_reserved_balance_missing",
            ));
        }
        balance.reserved -= lease.reserved_amount;
        balance.available =
            balance
                .available
                .checked_add(refund)
                .ok_or(CognitionEconomyError::InvalidState(
                    "cognition_available_balance_overflow",
                ))?;
        let receipt = self.close_lease(
            lease,
            CognitionLeaseStatusV1::Settled,
            consumed_amount,
            refund,
            tick,
            key,
            digest,
        )?;
        Ok(receipt)
    }

    fn release_inner(
        &mut self,
        lease_id: &str,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let lease = self
            .leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| CognitionEconomyError::LeaseNotFound(lease_id.to_string()))?;
        let key = operation_key(lease_id, "release");
        let digest = economy_digest(COGNITION_ECONOMY_OPERATION_DOMAIN, &key);
        if let Some(existing) = self.operations.get(&key) {
            if existing.operation_digest != digest {
                return Err(CognitionEconomyError::Conflict(
                    "cognition_release_idempotency_conflict",
                ));
            }
            return self.receipts.get(&existing.receipt_id).cloned().ok_or(
                CognitionEconomyError::InvalidState("cognition_operation_receipt_missing"),
            );
        }
        if lease.status != CognitionLeaseStatusV1::Reserved {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_lease_already_closed",
            ));
        }
        let balance = self.balance_mut(&lease.account_id, &lease.quote.resource);
        if balance.reserved < lease.reserved_amount {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_reserved_balance_missing",
            ));
        }
        balance.reserved -= lease.reserved_amount;
        balance.available = balance.available.checked_add(lease.reserved_amount).ok_or(
            CognitionEconomyError::InvalidState("cognition_available_balance_overflow"),
        )?;
        let reserved_amount = lease.reserved_amount;
        self.close_lease(
            lease,
            CognitionLeaseStatusV1::Released,
            0,
            reserved_amount,
            tick,
            key,
            digest,
        )
    }

    fn refund_inner(
        &mut self,
        lease_id: &str,
        tick: u64,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let lease = self
            .leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| CognitionEconomyError::LeaseNotFound(lease_id.to_string()))?;
        let key = operation_key(lease_id, "refund");
        let digest = economy_digest(COGNITION_ECONOMY_OPERATION_DOMAIN, &key);
        if let Some(existing) = self.operations.get(&key) {
            if existing.operation_digest != digest {
                return Err(CognitionEconomyError::Conflict(
                    "cognition_refund_idempotency_conflict",
                ));
            }
            return self.receipts.get(&existing.receipt_id).cloned().ok_or(
                CognitionEconomyError::InvalidState("cognition_operation_receipt_missing"),
            );
        }
        let (refund, reserved_balance_to_release) = match lease.status {
            CognitionLeaseStatusV1::Reserved => (lease.reserved_amount, lease.reserved_amount),
            CognitionLeaseStatusV1::Settled => (lease.settled_amount, 0),
            CognitionLeaseStatusV1::Released | CognitionLeaseStatusV1::Refunded => {
                return Err(CognitionEconomyError::InvalidState(
                    "cognition_lease_already_closed",
                ));
            }
        };
        let balance = self.balance_mut(&lease.account_id, &lease.quote.resource);
        if balance.reserved < reserved_balance_to_release {
            return Err(CognitionEconomyError::InvalidState(
                "cognition_reserved_balance_missing",
            ));
        }
        balance.reserved -= reserved_balance_to_release;
        balance.available =
            balance
                .available
                .checked_add(refund)
                .ok_or(CognitionEconomyError::InvalidState(
                    "cognition_available_balance_overflow",
                ))?;
        self.close_lease(
            lease,
            CognitionLeaseStatusV1::Refunded,
            0,
            refund,
            tick,
            key,
            digest,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn close_lease(
        &mut self,
        mut lease: CognitionLeaseV1,
        status: CognitionLeaseStatusV1,
        consumed_amount: u64,
        refunded_amount: u64,
        tick: u64,
        operation_key: String,
        operation_digest: String,
    ) -> Result<CognitionReceiptV1, CognitionEconomyError> {
        let receipt_id = economy_digest(
            COGNITION_ECONOMY_RECEIPT_ID_DOMAIN,
            &(lease.lease_id.as_str(), operation_key.as_str()),
        );
        let mut receipt = CognitionReceiptV1 {
            schema_version: COGNITION_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: receipt_id.clone(),
            receipt_digest: String::new(),
            lease_id: lease.lease_id.clone(),
            idempotency_key: lease.idempotency_key.clone(),
            account_id: lease.account_id.clone(),
            agent_id: lease.agent_id.clone(),
            agent_session_id: lease.agent_session_id.clone(),
            agent_turn_id: lease.agent_turn_id.clone(),
            decision_request_id: lease.decision_request_id.clone(),
            request_digest: lease.request_digest.clone(),
            quote: lease.quote.clone(),
            reserved_amount: lease.reserved_amount,
            consumed_amount,
            refunded_amount,
            status,
            issued_at_tick: tick,
        };
        receipt.receipt_digest = receipt.recompute_digest();
        lease.status = status;
        lease.settled_amount = match status {
            CognitionLeaseStatusV1::Settled => consumed_amount,
            CognitionLeaseStatusV1::Released => 0,
            CognitionLeaseStatusV1::Refunded => lease.settled_amount,
            CognitionLeaseStatusV1::Reserved => 0,
        };
        lease.refunded_amount = match status {
            CognitionLeaseStatusV1::Settled => refunded_amount,
            CognitionLeaseStatusV1::Released => lease.reserved_amount,
            CognitionLeaseStatusV1::Refunded => {
                lease.refunded_amount.checked_add(refunded_amount).ok_or(
                    CognitionEconomyError::InvalidState("cognition_refund_overflow"),
                )?
            }
            CognitionLeaseStatusV1::Reserved => 0,
        };
        lease.closed_at_tick = Some(tick);
        lease.receipt_id = Some(receipt_id.clone());
        lease.validate()?;
        self.leases.insert(lease.lease_id.clone(), lease.clone());
        self.receipts.insert(receipt_id.clone(), receipt.clone());
        self.operations.insert(
            operation_key.clone(),
            CognitionEconomyOperationRecordV1 {
                operation_key: operation_key.clone(),
                operation_digest,
                lease_id: lease.lease_id.clone(),
                receipt_id: receipt_id.clone(),
            },
        );
        self.append_event(
            match status {
                CognitionLeaseStatusV1::Settled => "settle",
                CognitionLeaseStatusV1::Released => "release",
                CognitionLeaseStatusV1::Refunded => "refund",
                CognitionLeaseStatusV1::Reserved => "reserve",
            },
            &lease,
            &operation_key,
            &receipt_id,
            consumed_amount,
            refunded_amount,
        )?;
        Ok(receipt)
    }

    fn append_event(
        &mut self,
        event_kind: &str,
        lease: &CognitionLeaseV1,
        operation_key: &str,
        receipt_id: &str,
        consumed_amount: u64,
        refunded_amount: u64,
    ) -> Result<(), CognitionEconomyError> {
        let journal_seq = self.head_seq.saturating_add(1);
        let parent_event_digest = self
            .journal
            .last()
            .map(|event| event.event_digest.clone())
            .unwrap_or_default();
        let mut event = CognitionEconomyEventV1 {
            schema_version: COGNITION_ECONOMY_EVENT_SCHEMA_VERSION.to_string(),
            journal_seq,
            parent_event_digest,
            event_kind: event_kind.to_string(),
            lease_id: lease.lease_id.clone(),
            operation_key: operation_key.to_string(),
            idempotency_key: lease.idempotency_key.clone(),
            resource: lease.quote.resource.clone(),
            reserved_amount: lease.reserved_amount,
            consumed_amount,
            refunded_amount,
            status: lease.status,
            receipt_id: receipt_id.to_string(),
            event_digest: String::new(),
        };
        event.event_digest = event.recompute_digest();
        self.journal.push(event);
        self.head_seq = journal_seq;
        self.head_digest = economy_digest(
            COGNITION_ECONOMY_JOURNAL_DOMAIN,
            &(self.head_seq, &self.journal),
        );
        Ok(())
    }
}

fn operation_key(lease_id: &str, operation: &str) -> String {
    economy_digest(COGNITION_ECONOMY_OPERATION_DOMAIN, &(lease_id, operation))
}
