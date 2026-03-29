use super::super::{
    m1_builtin_module_artifact_identity, m4_builtin_module_artifact_identity,
    m5_builtin_module_artifact_identity, WorldError,
};

#[test]
fn builtin_identity_manifest_resolves_m1_entry() {
    let identity = m1_builtin_module_artifact_identity(
        "m1.rule.move",
        "a395547deac1bc38aef18e2d85fbbc823e268389c17a800cacc9b544dddd0e3f",
    )
    .expect("resolve m1 identity");
    assert!(identity.is_complete());
    assert_eq!(identity.signer_node_id, "builtin.module.release.signer");
    match identity.signature_scheme.as_str() {
        "ed25519" => {
            assert!(
                identity
                    .artifact_signature
                    .starts_with("modsig:ed25519:v1:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        "identity_hash_v1" => {
            assert!(
                identity.artifact_signature.starts_with("idhash:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        other => panic!("unexpected signature scheme: {other}"),
    }
}

#[test]
fn builtin_identity_manifest_resolves_m4_entry() {
    let identity = m4_builtin_module_artifact_identity(
        "m4.factory.miner.mk1",
        "6e4573f87d6c723c7a472ad1857746c3f3ea0cdaf0944851e22f9a2d7fbb28ef",
    )
    .expect("resolve m4 identity");
    assert!(identity.is_complete());
    assert_eq!(identity.signer_node_id, "builtin.module.release.signer");
    match identity.signature_scheme.as_str() {
        "ed25519" => {
            assert!(
                identity
                    .artifact_signature
                    .starts_with("modsig:ed25519:v1:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        "identity_hash_v1" => {
            assert!(
                identity.artifact_signature.starts_with("idhash:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        other => panic!("unexpected signature scheme: {other}"),
    }
}

#[test]
fn builtin_identity_manifest_resolves_m5_entry() {
    let identity = m5_builtin_module_artifact_identity(
        "m5.gameplay.war.core",
        "74f65cc151fafccdb7ce10ff47235d392539e0ebf5b8b42ec74576ce62877782",
    )
    .expect("resolve m5 identity");
    assert!(identity.is_complete());
    assert_eq!(identity.signer_node_id, "builtin.module.release.signer");
    match identity.signature_scheme.as_str() {
        "ed25519" => {
            assert!(
                identity
                    .artifact_signature
                    .starts_with("modsig:ed25519:v1:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        "identity_hash_v1" => {
            assert!(
                identity.artifact_signature.starts_with("idhash:"),
                "unexpected signature: {}",
                identity.artifact_signature
            );
        }
        other => panic!("unexpected signature scheme: {other}"),
    }
}

#[test]
fn builtin_identity_manifest_rejects_missing_module() {
    let err = m1_builtin_module_artifact_identity(
        "m1.rule.missing",
        "a395547deac1bc38aef18e2d85fbbc823e268389c17a800cacc9b544dddd0e3f",
    )
    .expect_err("missing module should fail");

    match err {
        WorldError::ModuleChangeInvalid { reason } => {
            assert!(
                reason.contains("missing module_id=m1.rule.missing"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
