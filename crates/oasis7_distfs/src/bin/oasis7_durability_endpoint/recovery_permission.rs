//! Endpoint archive installation consumes the shared permission verifier.
use super::*;
pub(super) use formal::RecoveryPermissionBootstrapV1 as RecoveryPermissionBootstrap;
pub(super) use formal::RecoveryPermissionViewV1 as PermissionView;
pub(super) struct VerifiedPermission {
    pub active_predecessor: formal::VerifiedAuthorityV1,
    pub cut: formal::VerifiedFormalCommitV1,
    pub recovery: formal::VerifiedRecovery,
}
pub(super) fn verify_permission(
    view: PermissionView<'_>,
    trust: &formal::InitialActivationTrustV1,
    initial: &formal::SignedInitialActivationV1,
) -> Result<VerifiedPermission, String> {
    let verified = formal::verify_recovery_permission(view, trust, initial)?;
    Ok(VerifiedPermission {
        active_predecessor: verified.active_predecessor().clone(),
        cut: verified.cut().clone(),
        recovery: verified.recovery().clone(),
    })
}
