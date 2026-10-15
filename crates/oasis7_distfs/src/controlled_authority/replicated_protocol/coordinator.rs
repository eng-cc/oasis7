use super::*;
use ed25519_dalek::SigningKey;

/// Owns two real locked file endpoints. Their distinct keys/ids/directories do
/// NOT establish independent fault domains. No formal runtime consumer is wired.
pub struct ReplicatedCoordinator<E = FileEndpoint> {
    primary: E,
    replica: E,
    writer: SigningKey,
    poisoned: bool,
    last_error: Option<ProtocolError>,
    decision_in_progress: bool,
}
impl<E: transport::DurableEndpoint> ReplicatedCoordinator<E> {
    pub fn new(primary: E, replica: E, writer: SigningKey) -> Result<Self, ProtocolError> {
        if primary.role() != EndpointRole::Primary
            || replica.role() != EndpointRole::Replica
            || primary.trust() != replica.trust()
            || hex::encode(writer.verifying_key().to_bytes()) != primary.trust().writer_key
        {
            return Err(invalid("coordinator roles/trust/writer"));
        }
        if primary.storage_identity()? == replica.storage_identity()? {
            return Err(invalid("same endpoint directory"));
        }
        Ok(Self {
            primary,
            replica,
            writer,
            poisoned: false,
            last_error: None,
            decision_in_progress: false,
        })
    }
    pub fn freeze_input(&mut self, body: FrozenInputBody) -> Result<FrozenOutcome, ProtocolError> {
        let request = body.request.clone();
        if self.poisoned {
            return Ok(FrozenOutcome::Unknown { request });
        }
        body.validate(self.primary.trust())?;
        let input = SignedFrozenInput::sign(body, &self.writer)?;
        let mut reserved = false;
        let result = (|| {
            let p = self.primary.query_frozen(&request)?;
            let r = self.replica.query_frozen(&request)?;
            let p = match p {
                FrozenStatus::Reserved(p) => Some(p),
                _ => None,
            };
            let r = match r {
                FrozenStatus::Reserved(r) => Some(r),
                _ => None,
            };
            for (reservation, role) in [(&p, EndpointRole::Primary), (&r, EndpointRole::Replica)] {
                if let Some(reservation) = reservation {
                    reservation
                        .validate(self.primary.trust(), role)
                        .map_err(|_| ProtocolError::Poisoned)?;
                    if reservation.input.body.request != request {
                        return Err(ProtocolError::Poisoned);
                    }
                }
            }
            if let (Some(p), Some(r)) = (&p, &r) {
                if p.input != r.input {
                    return Err(ProtocolError::Poisoned);
                }
                if let (Some(p), Some(r)) = (&p.certificate, &r.certificate) {
                    if p != r {
                        return Err(ProtocolError::Poisoned);
                    }
                    if p.input != input {
                        return Err(invalid("original frozen identity bound to different input"));
                    }
                    return Ok(p.clone());
                }
            }
            if p.as_ref().is_some_and(|p| p.input != input)
                || r.as_ref().is_some_and(|r| r.input != input)
            {
                return Err(invalid("original frozen identity bound to different input"));
            }
            let primary_receipt = self.primary.freeze_input(&input)?;
            reserved = true;
            let replica_receipt = self.replica.freeze_input(&input)?;
            let certificate = FrozenCertificate {
                input,
                primary_receipt,
                replica_receipt,
            };
            certificate
                .verify(self.primary.trust())
                .map_err(|_| ProtocolError::Poisoned)?;
            self.primary.complete_frozen(&certificate)?;
            self.replica.complete_frozen(&certificate)?;
            Ok(certificate)
        })();
        match result {
            Ok(certificate) => Ok(FrozenOutcome::Complete(Box::new(certificate))),
            Err(error @ (ProtocolError::Io(_) | ProtocolError::Poisoned)) => {
                self.last_error = Some(error);
                self.poisoned = true;
                Ok(FrozenOutcome::Unknown { request })
            }
            Err(error) if reserved => {
                self.last_error = Some(error);
                self.poisoned = true;
                Ok(FrozenOutcome::Unknown { request })
            }
            Err(error) => Err(error),
        }
    }
    fn frozen_certificate(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<Option<FrozenCertificate>, ProtocolError> {
        let p = self.primary.query_frozen(request)?;
        let r = self.replica.query_frozen(request)?;
        match (p, r) {
            (FrozenStatus::NotRecorded, FrozenStatus::NotRecorded) => Ok(None),
            (FrozenStatus::Reserved(p), FrozenStatus::Reserved(r)) => {
                p.validate(self.primary.trust(), EndpointRole::Primary)?;
                r.validate(self.replica.trust(), EndpointRole::Replica)?;
                if p.input.body.request != *request
                    || p.input != r.input
                    || p.certificate != r.certificate
                    || p.certificate.is_none()
                {
                    return Err(ProtocolError::Poisoned);
                }
                Ok(p.certificate)
            }
            _ => Err(ProtocolError::Poisoned),
        }
    }
    /// Operator diagnostics only; an error never establishes non-execution.
    pub fn last_reconciliation_error(&self) -> Option<&ProtocolError> {
        self.last_error.as_ref()
    }
    pub fn heads(&self) -> (HeadAnchor, HeadAnchor) {
        (self.primary.head(), self.replica.head())
    }
    pub fn into_endpoints(self) -> (E, E) {
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
        self.last_error = None;
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
        reject_endpoint_disagreement(&p, &r)?;
        let existing = proposal(&p).or_else(|| proposal(&r));
        let frozen_certificate = if existing.is_none()
            || existing.is_some_and(|p| p.body.frozen_input_digest.is_some())
        {
            self.frozen_certificate(request)?
        } else {
            None
        };
        let frozen_digest = frozen_certificate
            .as_ref()
            .map(|c| c.verify(self.primary.trust()))
            .transpose()?;
        let signed = if let Some(existing) = existing {
            if existing.body.record != record {
                return Err(invalid(
                    "request identity reused with changed payload/record",
                ));
            }
            existing.clone()
        } else {
            let head = self.primary.head();
            if head.position != parent.position || head.decision_hash != parent.decision_hash {
                return Err(invalid("expected parent mismatch"));
            }
            crypto::sign_proposal(
                ProposalBody {
                    schema_version: if frozen_digest.is_some() { 2 } else { 1 },
                    scope: EVIDENCE_SCOPE.into(),
                    trust: self.primary.trust().clone(),
                    position: parent
                        .position
                        .checked_add(1)
                        .ok_or_else(|| invalid("position exhausted"))?,
                    parent_hash: parent.decision_hash.clone(),
                    request: request.clone(),
                    record,
                    frozen_input_digest: frozen_digest,
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
            frozen_certificate,
            proposal: signed,
            replica_prepare,
            primary_receipt,
            replica_receipt,
        };
        let evidence_parent = HeadAnchor {
            position: evidence
                .proposal
                .body
                .position
                .checked_sub(1)
                .ok_or_else(|| invalid("zero evidence position"))?,
            decision_hash: evidence.proposal.body.parent_hash.clone(),
            qualified: false,
        };
        crypto::verify_evidence(&evidence, self.primary.trust(), &evidence_parent)
            .map_err(|_| ProtocolError::Poisoned)?;
        // All signatures attest already-synced complete decisions. No writer or
        // primary-only signature is sufficient for final evidence verification.
        // Preserve the evidence at P first; R already holds P+R decision receipts.
        self.primary.finalize(&evidence)?;
        self.replica.finalize(&evidence)?;
        Ok(ProtocolOutcome::DurabilityQualified(Box::new(evidence)))
    }
    /// Read-only reconciliation: no new execution, abort inference or automatic
    /// repair. Call submit with the SAME identity/record after explicit reopen to
    /// complete a pending/uncertain original protocol. Endpoint disagreement
    /// poisons this instance even before a decision; read-only lookup never
    /// restores its ability to submit new requests.
    pub fn lookup(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<ProtocolOutcome, ProtocolError> {
        self.decision_in_progress = false;
        request.validate().map_err(|e| invalid(e.to_string()))?;
        let result = (|| {
            let p = read_status(&mut self.primary, request)?;
            let r = read_status(&mut self.replica, request)?;
            reject_endpoint_disagreement(&p, &r)?;
            if let (EndpointStatus::Qualified(p), EndpointStatus::Qualified(r)) = (&p, &r) {
                if p != r {
                    return Err(ProtocolError::Poisoned);
                }
                return Ok(ProtocolOutcome::DurabilityQualified(p.clone()));
            }
            if let (EndpointStatus::Prepared(p), EndpointStatus::Prepared(r)) = (&p, &r) {
                if p != r {
                    return Err(ProtocolError::Poisoned);
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
            Err(error @ (ProtocolError::Io(_) | ProtocolError::Poisoned)) => {
                self.last_error = Some(error);
                self.poisoned = true;
                Ok(ProtocolOutcome::Unknown { request })
            }
            Err(error) if self.decision_in_progress => {
                self.last_error = Some(error);
                self.poisoned = true;
                Ok(ProtocolOutcome::Unknown { request })
            }
            other => other,
        }
    }
    #[cfg(test)]
    pub(super) fn endpoints_mut(&mut self) -> (&mut E, &mut E) {
        (&mut self.primary, &mut self.replica)
    }
}
// Storage/decode/integrity/sync errors are uncertainty, not malformed client
// input. Never continue appending from stale in-memory endpoint state.
fn read_status<E: transport::DurableEndpoint>(
    endpoint: &mut E,
    request: &LocalRequestIdentity,
) -> Result<EndpointStatus, ProtocolError> {
    let status = endpoint
        .lookup(request)
        .map_err(|error| ProtocolError::Io(format!("endpoint reconciliation: {error}")))?;
    if let Some(signed) = proposal(&status) {
        if signed.body.request != *request {
            return Err(ProtocolError::Poisoned);
        }
        crypto::verify_proposal(signed, endpoint.trust()).map_err(|_| ProtocolError::Poisoned)?;
        if let EndpointStatus::Qualified(evidence) = &status {
            let parent = HeadAnchor {
                position: signed
                    .body
                    .position
                    .checked_sub(1)
                    .ok_or(ProtocolError::Poisoned)?,
                decision_hash: signed.body.parent_hash.clone(),
                qualified: false,
            };
            crypto::verify_evidence(evidence, endpoint.trust(), &parent)
                .map_err(|_| ProtocolError::Poisoned)?;
        }
    }
    Ok(status)
}
fn proposal(status: &EndpointStatus) -> Option<&SignedProposal> {
    match status {
        EndpointStatus::Prepared(p) | EndpointStatus::DecisionUnqualified(p) => Some(p),
        EndpointStatus::Qualified(e) => Some(&e.proposal),
        EndpointStatus::NotRecorded => None,
    }
}

// Two authenticated durable endpoint records disagreeing about one identity
// are uncertain authority, regardless of stage. Client payload mistakes alone
// remain Invalid and do not poison an otherwise coherent writer.
fn reject_endpoint_disagreement(
    p: &EndpointStatus,
    r: &EndpointStatus,
) -> Result<(), ProtocolError> {
    if proposal(p).zip(proposal(r)).is_some_and(|(p, r)| p != r) {
        return Err(ProtocolError::Poisoned);
    }
    Ok(())
}
