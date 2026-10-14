use super::super::endpoint::NativeCheckpointVerdictV1;
use super::*;

/// Native endpoint hot restart only. The opaque checkpoint verdict attests the
/// previously verified parent context through its signed archive commitments
/// and a trusted provider's exact current monotonic floor. Ordinary readers
/// still verify the activation/history chain and cannot construct this verdict.
pub(crate) fn restore_formal_commit_from_checkpoint(
    proof: &FormalQualifiedCommitProofV1,
    original_authority: &VerifiedAuthorityV1,
    recovery: Option<&VerifiedRecovery>,
    checkpoint: &NativeCheckpointVerdictV1,
) -> Result<VerifiedFormalCommitV1, FormalError> {
    if checkpoint.proof() != proof {
        return Err(invalid("native checkpoint exact archived proof binding"));
    }
    let parent = match (
        checkpoint.verified_parent_record(),
        checkpoint.verified_parent_finalizing_authority_id(),
    ) {
        (Some(record), Some(finalizer)) => {
            record.validate()?;
            super::crypto::hash_hex(finalizer)?;
            Some(VerifiedFormalCommitV1 {
                record: record.clone(),
                finalizing_authority_id: finalizer.to_owned(),
            })
        }
        (None, None) => None,
        _ => {
            return Err(invalid(
                "native checkpoint complete original parent context required",
            ));
        }
    };
    let commit = verify_formal_qualified(proof, original_authority, parent.as_ref(), recovery)?;
    if &commit.head() != checkpoint.head()
        || commit.finalizing_authority_id() != checkpoint.authority_id()
    {
        return Err(invalid(
            "native checkpoint canonical head/finalizing authority binding",
        ));
    }
    Ok(commit)
}
