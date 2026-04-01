use super::super::{
    m1_builtin_module_artifact_identity, m4_builtin_module_artifact_identity,
    m5_builtin_module_artifact_identity, WorldError,
};

#[test]
fn builtin_identity_manifest_resolves_m1_entry() {
    let identity = m1_builtin_module_artifact_identity(
        "m1.rule.move",
        "96d22c5767ac1a5bd992c5b7443c8d83291e093c5253202f4a4706d97c19b458",
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
        "5232b320279daeb36851857093e7ce5e94a32a8506f73021f180752ae8d6d2ed",
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
        "bb76a393b8c3d6f815d29baf55d8ea957776aa15299d16be0a84f72d29e5ca4b",
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
        "96d22c5767ac1a5bd992c5b7443c8d83291e093c5253202f4a4706d97c19b458",
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
