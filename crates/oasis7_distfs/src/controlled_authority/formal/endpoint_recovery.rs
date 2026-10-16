use super::*;
use crate::controlled_authority::replicated_protocol::ClosedRecord;
impl FileFormalEndpoint {
    /// Both fresh finalizers retain the original certified input before replay.
    /// No replacement reservation or old-key receipt is produced.
    pub fn install_recovery_frozen(
        &mut self,
        certificate: FormalFrozenCertificateV1,
        material: ClosedRecord,
    ) -> Result<(), FormalError> {
        let mut guard = self.guard()?;
        let recovery = self.recovery.as_ref().ok_or_else(|| {
            invalid("formal retained frozen installation missing verified recovery")
        })?;
        certificate.verify(self.original.definition(), self.original.activation_ref())?;
        verify_frozen_material(&certificate.reservation.body, &material)?;
        let body = &certificate.reservation.body;
        let RecoveryTailV1::Frozen {
            position,
            reservation_id,
            handoff,
        } = &recovery.body().tail
        else {
            return Err(invalid(
                "formal retained candidate cannot be downgraded to frozen replay",
            ));
        };
        if body.parent != recovery.body().sealed_cut
            || body.next_position != *position
            || body.reservation_id()? != *reservation_id
            || &body.handoff != handoff
        {
            return Err(invalid("formal retained original frozen recovery binding"));
        }
        let request = body.request_digest.clone();
        if let Some(entry) = self.load(&request)? {
            match entry {
                FormalRequestStateV1::Frozen {
                    input,
                    material: old,
                    certificate: existing,
                    ..
                } if input == certificate.reservation && old == material => {
                    if existing.as_ref().is_some_and(|old| old != &certificate) {
                        return Err(FormalError::Conflict(
                            "formal retained original frozen certificate".into(),
                        ));
                    }
                    if existing.is_some()
                        && self
                            .store
                            .body
                            .pending
                            .as_ref()
                            .is_some_and(|id| self.store.local_object_exists(id))
                    {
                        return Ok(());
                    }
                }
                _ => {
                    return Err(FormalError::Conflict(
                        "formal retained original candidate already bound".into(),
                    ));
                }
            }
        }
        self.fresh_slot(&request, body.next_position, &body.parent.record_id)?;
        let receipt = match self.role {
            EndpointRole::Primary => certificate.primary.clone(),
            EndpointRole::Replica => certificate.replica.clone(),
        };
        self.write(
            FormalRequestStateV1::Frozen {
                input: certificate.reservation.clone(),
                material,
                receipt: Some(receipt),
                certificate: Some(certificate),
            },
            false,
            guard.as_mut(),
        )
    }
}
