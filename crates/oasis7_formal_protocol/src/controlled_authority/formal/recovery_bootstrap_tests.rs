use super::*;
#[test]
fn recovery_bootstrap_issuer_floor_cannot_downgrade_recursive_prior_witness() {
    let retained = IssuerAnchorV1 {
        revision: 2,
        digest: "12".repeat(32),
    };
    let old = IssuerAnchorV1 {
        revision: 1,
        digest: "11".repeat(32),
    };
    assert!(require_forward_floor(&old, &retained).is_err());
    let fork = IssuerAnchorV1 {
        revision: 2,
        digest: "13".repeat(32),
    };
    assert!(require_forward_floor(&fork, &retained).is_err());
    assert!(require_forward_floor(&retained, &retained).is_ok());
    let advanced = IssuerAnchorV1 {
        revision: 3,
        digest: "14".repeat(32),
    };
    assert!(require_forward_floor(&advanced, &retained).is_ok());
}
