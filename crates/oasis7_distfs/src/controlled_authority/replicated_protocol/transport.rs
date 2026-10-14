//! Authenticated endpoint transport for the durability prerequisite protocol.
//! TCP listeners and clients are loopback-only: cross-host deployment requires
//! an authenticated encrypted tunnel. Signed frames do not provide secrecy.
mod client;
mod server;
#[cfg(test)]
mod tests;
mod wire;

use super::*;
pub use client::RemoteEndpoint;
pub use server::EndpointServer;

/// Both local and remote endpoints expose the same durable operations. Remote
/// clients hold the writer key only; each server retains its own endpoint key.
pub trait DurableEndpoint: Send {
    fn freeze_input(&mut self, input: &SignedFrozenInput) -> Result<FrozenReceipt, ProtocolError>;
    fn complete_frozen(&mut self, certificate: &FrozenCertificate) -> Result<(), ProtocolError>;
    fn query_frozen(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<FrozenStatus, ProtocolError>;
    fn role(&self) -> EndpointRole;
    fn trust(&self) -> &FixedTrust;
    fn head(&self) -> HeadAnchor;
    fn storage_identity(&self) -> Result<String, ProtocolError>;
    fn lookup(&mut self, request: &LocalRequestIdentity) -> Result<EndpointStatus, ProtocolError>;
    fn prepare(&mut self, proposal: &SignedProposal) -> Result<DurableReceipt, ProtocolError>;
    fn decide_primary(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError>;
    fn decide_replica(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
        primary: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError>;
    fn finalize(&mut self, evidence: &DurabilityEvidence) -> Result<(), ProtocolError>;
}

impl DurableEndpoint for FileEndpoint {
    fn freeze_input(&mut self, input: &SignedFrozenInput) -> Result<FrozenReceipt, ProtocolError> {
        self.freeze_input(input)
    }
    fn complete_frozen(&mut self, certificate: &FrozenCertificate) -> Result<(), ProtocolError> {
        self.complete_frozen(certificate)
    }
    fn query_frozen(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<FrozenStatus, ProtocolError> {
        self.query_frozen(request)
    }

    fn role(&self) -> EndpointRole {
        self.role()
    }
    fn trust(&self) -> &FixedTrust {
        self.trust()
    }
    fn head(&self) -> HeadAnchor {
        self.head()
    }
    fn storage_identity(&self) -> Result<String, ProtocolError> {
        Ok(std::fs::canonicalize(self.path())
            .map_err(io)?
            .to_string_lossy()
            .into_owned())
    }
    fn lookup(&mut self, request: &LocalRequestIdentity) -> Result<EndpointStatus, ProtocolError> {
        self.lookup(request)
    }
    fn prepare(&mut self, proposal: &SignedProposal) -> Result<DurableReceipt, ProtocolError> {
        self.prepare(proposal)
    }
    fn decide_primary(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.decide_primary(proposal, prepared)
    }
    fn decide_replica(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
        primary: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.decide_replica(proposal, prepared, primary)
    }
    fn finalize(&mut self, evidence: &DurabilityEvidence) -> Result<(), ProtocolError> {
        self.finalize(evidence)
    }
}

impl<T: DurableEndpoint + ?Sized> DurableEndpoint for Box<T> {
    fn freeze_input(&mut self, input: &SignedFrozenInput) -> Result<FrozenReceipt, ProtocolError> {
        self.as_mut().freeze_input(input)
    }
    fn complete_frozen(&mut self, certificate: &FrozenCertificate) -> Result<(), ProtocolError> {
        self.as_mut().complete_frozen(certificate)
    }
    fn query_frozen(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<FrozenStatus, ProtocolError> {
        self.as_mut().query_frozen(request)
    }

    fn role(&self) -> EndpointRole {
        self.as_ref().role()
    }
    fn trust(&self) -> &FixedTrust {
        self.as_ref().trust()
    }
    fn head(&self) -> HeadAnchor {
        self.as_ref().head()
    }
    fn storage_identity(&self) -> Result<String, ProtocolError> {
        self.as_ref().storage_identity()
    }
    fn lookup(&mut self, request: &LocalRequestIdentity) -> Result<EndpointStatus, ProtocolError> {
        self.as_mut().lookup(request)
    }
    fn prepare(&mut self, proposal: &SignedProposal) -> Result<DurableReceipt, ProtocolError> {
        self.as_mut().prepare(proposal)
    }
    fn decide_primary(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.as_mut().decide_primary(proposal, prepared)
    }
    fn decide_replica(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
        primary: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.as_mut().decide_replica(proposal, prepared, primary)
    }
    fn finalize(&mut self, evidence: &DurabilityEvidence) -> Result<(), ProtocolError> {
        self.as_mut().finalize(evidence)
    }
}
