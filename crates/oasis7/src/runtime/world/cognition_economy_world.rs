//! World persistence boundary for the typed cognition economy projection.

use super::super::error::WorldError;
use super::World;
use super::cognition_economy::{
    CognitionEconomyError, CognitionEconomyStateV1, CognitionLeaseRequestV1, CognitionLeaseV1,
    CognitionProvisioningReceiptV1, CognitionProvisioningRequestV1, CognitionReceiptV1,
};

fn world_economy_error(error: CognitionEconomyError) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: error.code().to_string(),
    }
}

impl World {
    /// Read back and validate the Runtime-owned cognition economy projection.
    pub fn cognition_economy(&self) -> Result<CognitionEconomyStateV1, WorldError> {
        let value = self
            .cognition
            .get("cognition_economy")
            .cloned()
            .unwrap_or_else(|| {
                serde_json::to_value(CognitionEconomyStateV1::default())
                    .expect("default economy is serializable")
            });
        CognitionEconomyStateV1::from_snapshot_json(value).map_err(world_economy_error)
    }

    /// Explicit bootstrap balance setup. This is deliberately separate from
    /// scheduler capacity and does not refill an account implicitly.
    pub fn set_cognition_resource_balance(
        &mut self,
        account_id: impl Into<String>,
        resource: impl Into<String>,
        available: u64,
    ) -> Result<(), WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        economy
            .set_resource_balance(account_id, resource, available)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(())
    }

    /// Install one Runtime-authorized cognition allowance for a live Agent's
    /// capability owner.  The owner binding and current world identity are
    /// derived from Runtime state so callers cannot fund an arbitrary account
    /// or carry an allowance across a reorg/generation boundary.
    pub fn provision_cognition_for_agent(
        &mut self,
        agent_id: &str,
        provision_id: impl Into<String>,
        authority_context: impl Into<String>,
        allowance: u64,
    ) -> Result<CognitionProvisioningReceiptV1, WorldError> {
        if !self.state.agents.contains_key(agent_id) {
            return Err(WorldError::DistributedValidationFailed {
                reason: "cognition provisioning requires a live agent".to_string(),
            });
        }
        let identity = self
            .capability_revocation_state
            .agent_identities
            .get(agent_id)
            .ok_or_else(|| WorldError::DistributedValidationFailed {
                reason: "cognition provisioning requires a live capability identity".to_string(),
            })?
            .clone();
        let binding = self.current_cognition_runtime_binding()?;
        let request = CognitionProvisioningRequestV1::new(
            provision_id,
            identity.owner_binding.clone(),
            identity.owner_binding,
            identity.generation,
            binding.world_id,
            binding.branch_id,
            binding.reorg_epoch,
            allowance,
            authority_context,
        );
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .provision(request, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }

    pub fn reserve_cognition_lease(
        &mut self,
        request: CognitionLeaseRequestV1,
    ) -> Result<CognitionLeaseV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let lease = economy
            .reserve(request, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(lease)
    }

    pub fn settle_cognition_lease(
        &mut self,
        lease_id: &str,
        consumed_amount: u64,
    ) -> Result<CognitionReceiptV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .settle(lease_id, consumed_amount, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }

    pub fn release_cognition_lease(
        &mut self,
        lease_id: &str,
    ) -> Result<CognitionReceiptV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .release(lease_id, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }

    pub fn expire_cognition_lease(
        &mut self,
        lease_id: &str,
    ) -> Result<CognitionReceiptV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .expire(lease_id, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }

    pub fn refund_cognition_lease(
        &mut self,
        lease_id: &str,
    ) -> Result<CognitionReceiptV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .refund(lease_id, transaction.state.time)
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }

    pub fn refund_settled_cognition_lease(
        &mut self,
        lease_id: &str,
        refunded_amount: u64,
        parent_receipt_id: &str,
        reason: &str,
    ) -> Result<CognitionReceiptV1, WorldError> {
        let mut transaction = self.clone();
        let mut economy = transaction.cognition_economy()?;
        let receipt = economy
            .refund_settled(
                lease_id,
                refunded_amount,
                parent_receipt_id,
                reason,
                transaction.state.time,
            )
            .map_err(world_economy_error)?;
        transaction.cognition["cognition_economy"] =
            economy.snapshot_json().map_err(world_economy_error)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(receipt)
    }
}
