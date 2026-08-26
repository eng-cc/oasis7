//! Administrative admission APIs for proof-bearing capability authority state.
//!
//! These APIs deliberately keep the legacy record/certificate entry point
//! fail-closed.  Authority is a trust root and can only enter the journal
//! together with a quorum-signed binding that replay can verify.

use super::super::capability_authorization::{
    CapabilityAgentIdentity, CapabilityAuthorityFinalityProof, CapabilityAuthorityRecord,
};
use super::super::governance::GovernanceFinalityCertificate;
use super::super::{CapabilityAuthorizationEvent, WorldError, WorldEventBody};
use super::World;
use super::capability_authorization::{deny, validate_agent_identity, validate_authority_record};

impl World {
    /// Attempt to install an authority record without the proof that binds
    /// every signed authority field to historical finality.
    ///
    /// This compatibility entry point intentionally remains rejected.  A
    /// process-local certificate is not a sufficient trust root.
    pub fn install_capability_authority_record_with_finality(
        &mut self,
        record: CapabilityAuthorityRecord,
        certificate: GovernanceFinalityCertificate,
    ) -> Result<(), WorldError> {
        let _ = (record, certificate);
        Err(deny(
            "authority installation requires a capability authority finality proof",
        ))
    }

    /// Install an authority record with a quorum-signed capability binding.
    /// The proof repeats the historical governance certificate and signs all
    /// authority metadata so replay can verify the exact record admitted.
    pub fn install_capability_authority_record_with_finality_proof(
        &mut self,
        record: CapabilityAuthorityRecord,
        proof: CapabilityAuthorityFinalityProof,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        validate_authority_record(&record)?;
        self.verify_capability_authority_finality_proof(&record, &proof)?;
        if self.chain_resource_manifest.world_id != "unbound"
            && self.chain_resource_manifest.world_id != record.world_id
        {
            return Err(deny("authority record world does not match live world"));
        }
        if let Some(existing) = self
            .capability_revocation_state
            .authority_records
            .get(&record.issuer_id)
            && existing != &record
        {
            return Err(deny("authority record is immutable"));
        }
        self.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::AuthorityInstalledWithProof { record, proof },
            ),
            None,
        )?;
        Ok(())
    }

    /// Install the durable owner/generation binding for a live Agent subject.
    /// Agent gameplay state does not carry capability credentials.  The host
    /// must therefore install this binding before an Agent grant can execute;
    /// each generation is journaled and immutable, while a strictly newer
    /// generation may replace it after a transfer/reset/re-claim.
    pub fn install_capability_agent_identity(
        &mut self,
        agent_id: impl Into<String>,
        owner_binding: impl Into<String>,
        generation: u64,
    ) -> Result<(), WorldError> {
        self.verify_capability_authorization_root()?;
        let agent_id = agent_id.into();
        let identity = CapabilityAgentIdentity {
            owner_binding: owner_binding.into(),
            generation,
        };
        validate_agent_identity(agent_id.as_str(), &identity)?;
        if !self.state.agents.contains_key(agent_id.as_str()) {
            return Err(deny("capability agent identity requires a live agent"));
        }
        if let Some(existing) = self
            .capability_revocation_state
            .agent_identities
            .get(agent_id.as_str())
        {
            if generation < existing.generation {
                return Err(deny("capability agent identity generation regressed"));
            }
            if generation == existing.generation && existing != &identity {
                return Err(deny(
                    "capability agent identity changed without a new generation",
                ));
            }
        }
        if self
            .capability_revocation_state
            .agent_identities
            .get(agent_id.as_str())
            == Some(&identity)
        {
            return Ok(());
        }
        self.append_event(
            WorldEventBody::CapabilityAuthorization(
                CapabilityAuthorizationEvent::AgentIdentityInstalled { agent_id, identity },
            ),
            None,
        )?;
        Ok(())
    }
}
