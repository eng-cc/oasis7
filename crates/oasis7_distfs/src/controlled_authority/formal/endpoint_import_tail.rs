//! Install the original unresolved slot in the initial protected checkpoint.
//! Merely importing an authenticated index must not erase its reserved tail.
use super::*;
pub(super) fn install_imported_tail(
    hot: &mut HotBody,
    archive: &VerifiedFormalArchiveTransferV1,
    recovery: &VerifiedRecovery,
) -> Result<(), FormalError> {
    if recovery.body().tail == RecoveryTailV1::Empty {
        if archive.tail.kind != fencing::SealedTailKindV1::Empty {
            return Err(invalid(
                "formal imported empty recovery has unresolved tail",
            ));
        }
        return Ok(());
    }
    let request = archive
        .tail
        .request
        .as_ref()
        .ok_or_else(|| invalid("formal imported nonempty tail request commitment"))?;
    if request.verified_subject != "formal-request-digest"
        || request.operation_domain != "oasis7.formal.request.v1"
    {
        return Err(invalid("formal imported tail uses only digest identity"));
    }
    hash_hex(&request.request_id)?;
    let mut load = |id: &str| {
        archive
            .source
            .read_object(id)?
            .ok_or_else(|| invalid("formal imported referenced tail object missing"))
    };
    let id = Store::lookup_from_root(archive.index.as_deref(), &request.request_id, &mut load)?
        .ok_or_else(|| invalid("formal sealed index omitted its reserved tail"))?;
    let bytes = load(&id)?;
    if blake3::hash(&bytes).to_hex().as_str() != id || bytes.len() > 64 * 1024 * 1024 {
        return Err(invalid("formal imported tail object checksum/limit"));
    }
    let entry: FormalRequestStateV1 =
        serde_cbor::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    if entry.request_digest() != request.request_id
        || Some(entry.position()) != hot.head.position.checked_add(1)
        || matches!(entry, FormalRequestStateV1::Qualified { .. })
    {
        return Err(invalid("formal imported reserved exact next slot"));
    }
    let (reservation, record, parent, decided) = match &entry {
        FormalRequestStateV1::Frozen {
            input, certificate, ..
        } => {
            let certificate = certificate
                .as_ref()
                .ok_or_else(|| invalid("formal imported frozen lacks completed certificate"))?;
            if certificate.reservation != *input {
                return Err(invalid("formal imported frozen certificate input"));
            }
            (
                input.body.reservation_id()?,
                None,
                input.body.parent.clone(),
                false,
            )
        }
        FormalRequestStateV1::Prepared { candidate, .. }
        | FormalRequestStateV1::Decided { candidate, .. }
        | FormalRequestStateV1::Ready { candidate, .. } => {
            let cert = candidate
                .frozen
                .as_ref()
                .ok_or_else(|| invalid("formal imported candidate lacks original frozen"))?;
            (
                cert.reservation.body.reservation_id()?,
                Some(candidate.candidate.record.record_id()?),
                cert.reservation.body.parent.clone(),
                matches!(
                    entry,
                    FormalRequestStateV1::Decided { .. } | FormalRequestStateV1::Ready { .. }
                ),
            )
        }
        FormalRequestStateV1::Qualified { .. } => unreachable!(),
    };
    let (expected_reservation, expected_record) = match &recovery.body().tail {
        RecoveryTailV1::Frozen { reservation_id, .. } => (reservation_id, None),
        RecoveryTailV1::Prepared {
            reservation_id,
            record_id,
            ..
        }
        | RecoveryTailV1::Decision {
            reservation_id,
            record_id,
            ..
        } => (reservation_id, Some(record_id)),
        RecoveryTailV1::Empty => unreachable!(),
    };
    if parent != hot.head
        || &reservation != expected_reservation
        || record.as_ref() != expected_record
    {
        return Err(invalid(
            "formal imported slot differs from verified recovery tail",
        ));
    }
    if decided {
        hot.decision_head = Some(FormalHeadRefV1 {
            position: entry.position(),
            record_id: record.unwrap(),
        });
    }
    hot.pending = Some(id);
    Ok(())
}
