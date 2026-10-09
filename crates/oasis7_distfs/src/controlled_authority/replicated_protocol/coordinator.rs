use super::*;
use ed25519_dalek::SigningKey;

/// Owns two real locked file endpoints. Their distinct keys/ids/directories do
/// NOT establish independent fault domains. No formal runtime consumer is wired.
pub struct ReplicatedCoordinator {
    primary: FileEndpoint,
    replica: FileEndpoint,
    writer: SigningKey,
    poisoned: bool,
    decision_in_progress: bool,
}
impl ReplicatedCoordinator {
    pub fn new(
        primary: FileEndpoint,
        replica: FileEndpoint,
        writer: SigningKey,
    ) -> Result<Self, ProtocolError> {
        if primary.role() != EndpointRole::Primary
            || replica.role() != EndpointRole::Replica
            || primary.trust() != replica.trust()
            || hex::encode(writer.verifying_key().to_bytes()) != primary.trust().writer_key
        {
            return Err(invalid("coordinator roles/trust/writer"));
        }
        if std::fs::canonicalize(primary.path()).map_err(io)?
            == std::fs::canonicalize(replica.path()).map_err(io)?
        {
            return Err(invalid("same endpoint directory"));
        }
        Ok(Self {
            primary,
            replica,
            writer,
            poisoned: false,
            decision_in_progress: false,
        })
    }
    pub fn heads(&self) -> (HeadAnchor, HeadAnchor) {
        (self.primary.head(), self.replica.head())
    }
    pub fn into_endpoints(self) -> (FileEndpoint, FileEndpoint) {
        (self.primary, self.replica)
    }
    pub fn submit(
        &mut self,
        epoch: u64,
        expected_parent: &HeadAnchor,
        request: LocalRequestIdentity,
        record: ClosedRecord,
    ) -> Result<ProtocolOutcome, ProtocolError> {
        if self.poisoned {
            return Ok(ProtocolOutcome::Unknown { request });
        }
        if epoch != self.primary.trust().authority_epoch {
            return Err(invalid("fixed epoch mismatch"));
        }
        request.validate().map_err(|e| invalid(e.to_string()))?;
        record.validate()?;
        self.decision_in_progress = false;
        let result = self.submit_inner(expected_parent, &request, record);
        self.outcome(result, request)
    }
    fn submit_inner(
        &mut self,
        parent: &HeadAnchor,
        request: &LocalRequestIdentity,
        record: ClosedRecord,
    ) -> Result<ProtocolOutcome, ProtocolError> {
        let p = read_status(&mut self.primary, request)?;
        let r = read_status(&mut self.replica, request)?;
        let existing = proposal(&p).or_else(|| proposal(&r));
        let signed = if let Some(existing) = existing {
            if existing.body.record != record {
                return Err(invalid(
                    "request identity reused with changed payload/record",
                ));
            }
            if proposal(&r).is_some_and(|x| x != existing) {
                return Err(invalid("endpoint request disagreement"));
            }
            existing.clone()
        } else {
            let head = self.primary.head();
            if head.position != parent.position || head.decision_hash != parent.decision_hash {
                return Err(invalid("expected parent mismatch"));
            }
            crypto::sign_proposal(
                ProposalBody {
                    schema_version: 1,
                    scope: EVIDENCE_SCOPE.into(),
                    trust: self.primary.trust().clone(),
                    position: parent
                        .position
                        .checked_add(1)
                        .ok_or_else(|| invalid("position exhausted"))?,
                    parent_hash: parent.decision_hash.clone(),
                    request: request.clone(),
                    record,
                },
                &self.writer,
            )?
        };
        if matches!(p, EndpointStatus::NotRecorded | EndpointStatus::Prepared(_)) {
            self.primary.prepare(&signed)?;
        }
        let replica_prepare = self.replica.prepare(&signed)?;
        let primary_receipt = self.primary.decide_primary(&signed, &replica_prepare)?;
        self.decision_in_progress = true;
        let replica_receipt =
            self.replica
                .decide_replica(&signed, &replica_prepare, &primary_receipt)?;
        let evidence = DurabilityEvidence {
            proposal: signed,
            replica_prepare,
            primary_receipt,
            replica_receipt,
        };
        // All signatures attest already-synced complete decisions. No writer or
        // primary-only signature is sufficient for final evidence verification.
        // Preserve the evidence at P first; R already holds P+R decision receipts.
        self.primary.finalize(&evidence)?;
        self.replica.finalize(&evidence)?;
        Ok(ProtocolOutcome::DurabilityQualified(Box::new(evidence)))
    }
    /// Read-only reconciliation: no new execution, abort inference or automatic
    /// repair. Call submit with the SAME identity/record after explicit reopen to
    /// complete a pending/uncertain original protocol.
    pub fn lookup(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<ProtocolOutcome, ProtocolError> {
        self.decision_in_progress = false;
        request.validate().map_err(|e| invalid(e.to_string()))?;
        let result = (|| {
            let p = read_status(&mut self.primary, request)?;
            let r = read_status(&mut self.replica, request)?;
            if let (EndpointStatus::Qualified(p), EndpointStatus::Qualified(r)) = (&p, &r) {
                if p != r {
                    return Err(invalid("qualified endpoint fork"));
                }
                return Ok(ProtocolOutcome::DurabilityQualified(p.clone()));
            }
            if let (EndpointStatus::Prepared(p), EndpointStatus::Prepared(r)) = (&p, &r) {
                if p != r {
                    return Err(invalid("prepared endpoint fork"));
                }
                return Ok(ProtocolOutcome::Pending {
                    request: request.clone(),
                });
            }
            Ok(ProtocolOutcome::Unknown {
                request: request.clone(),
            })
        })();
        self.outcome(result, request.clone())
    }
    fn outcome(
        &mut self,
        result: Result<ProtocolOutcome, ProtocolError>,
        request: LocalRequestIdentity,
    ) -> Result<ProtocolOutcome, ProtocolError> {
        match result {
            Err(ProtocolError::Io(_) | ProtocolError::Poisoned) => {
                self.poisoned = true;
                Ok(ProtocolOutcome::Unknown { request })
            }
            Err(_) if self.decision_in_progress => {
                self.poisoned = true;
                Ok(ProtocolOutcome::Unknown { request })
            }
            other => other,
        }
    }
    #[cfg(test)]
    pub(super) fn endpoints_mut(&mut self) -> (&mut FileEndpoint, &mut FileEndpoint) {
        (&mut self.primary, &mut self.replica)
    }
}
// Storage/decode/integrity/sync errors are uncertainty, not malformed client
// input. Never continue appending from stale in-memory endpoint state.
fn read_status(
    endpoint: &mut FileEndpoint,
    request: &LocalRequestIdentity,
) -> Result<EndpointStatus, ProtocolError> {
    endpoint
        .lookup(request)
        .map_err(|_| ProtocolError::Poisoned)
}
fn proposal(status: &EndpointStatus) -> Option<&SignedProposal> {
    match status {
        EndpointStatus::Prepared(p) | EndpointStatus::DecisionUnqualified(p) => Some(p),
        EndpointStatus::Qualified(e) => Some(&e.proposal),
        EndpointStatus::NotRecorded => None,
    }
}
