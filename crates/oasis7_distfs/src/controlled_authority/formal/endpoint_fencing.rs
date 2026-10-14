use super::*;
use crate::controlled_authority::LocalRequestIdentity;
use crate::controlled_authority::replicated_protocol::HeadAnchor;

impl FileFormalEndpoint {
    pub fn checkpoint(&mut self) -> Result<fencing::EndpointCheckpointBindingV1, FormalError> {
        let _guard = self.guard()?;
        Ok(self.store.checkpoint.clone())
    }
    /// Native seal and appends share one external guard. The exact tail is
    /// captured from the authenticated checkpoint, never operator-supplied JSON.
    pub fn seal_epoch(
        &mut self,
        operation: &str,
        next_epoch: u64,
        expected: &fencing::EndpointCheckpointBindingV1,
    ) -> Result<fencing::EpochSealReceiptV1, FormalError> {
        let mut guard = self.guard()?;
        if let Some(body) = &self.store.body.seal {
            if body.recovery_operation != operation || body.next_epoch_floor != next_epoch {
                return Err(FormalError::Conflict(
                    "formal original epoch seal operation/floor".into(),
                ));
            }
            return self.signed_seal(body.clone(), guard.as_mut());
        }
        if expected != &self.store.checkpoint {
            return Err(FormalError::Conflict("formal stale seal checkpoint".into()));
        }
        let mut tail = fencing::SealedTailV1::empty();
        if let Some(id) = &self.store.body.pending {
            let entry: FormalRequestStateV1 = self.store.get(id)?;
            // This annotation retains the formal digest, not a fabricated user
            // identity. Reservation identity is the independently signed body.
            tail.request = Some(LocalRequestIdentity {
                verified_subject: "formal-request-digest".into(),
                operation_domain: "oasis7.formal.request.v1".into(),
                nonce_scope: self.authority.authority_id()?,
                request_id: entry.request_digest().into(),
            });
            tail.position = Some(entry.position());
            match entry {
                FormalRequestStateV1::Frozen {
                    input, certificate, ..
                } => {
                    tail.kind = fencing::SealedTailKindV1::Frozen;
                    tail.frozen_input_digest = Some(input.body.reservation_id()?);
                    tail.frozen_certificate_digest = certificate
                        .as_ref()
                        .map(|cert| hash("oasis7.formal.frozen_certificate.v1", cert))
                        .transpose()?;
                }
                FormalRequestStateV1::Prepared { candidate, .. }
                | FormalRequestStateV1::Decided { candidate, .. }
                | FormalRequestStateV1::Ready { candidate, .. } => {
                    tail.kind = if self.store.body.decision_head.is_some() {
                        fencing::SealedTailKindV1::Decision
                    } else {
                        fencing::SealedTailKindV1::Prepared
                    };
                    tail.proposal_digest = Some(candidate.candidate.record.record_id()?);
                    if let Some(cert) = candidate.frozen {
                        tail.frozen_input_digest = Some(cert.reservation.body.reservation_id()?);
                        tail.frozen_certificate_digest =
                            Some(hash("oasis7.formal.frozen_certificate.v1", &cert)?);
                    }
                }
                FormalRequestStateV1::Qualified { .. } => {
                    return Err(invalid("formal qualified entry remains pending"));
                }
            }
        }
        let body = fencing::EpochSealBodyV1 {
            schema_version: 1,
            instance: self.instance().clone(),
            recovery_operation: operation.into(),
            highest_complete: HeadAnchor {
                position: self.store.body.head.position,
                decision_hash: self.store.body.head.record_id.clone(),
                qualified: true,
            },
            checkpoint_before: self.store.checkpoint.clone(),
            tail,
            next_epoch_floor: next_epoch,
        };
        body.validate().map_err(fencing_error)?;
        let mut next = self.store.body.clone();
        next.seal = Some(body.clone());
        if let Err(error) = self.publish_native(next, guard.as_mut()) {
            self.poisoned = true;
            return Err(error);
        }
        self.signed_seal(body, guard.as_mut())
    }
}
