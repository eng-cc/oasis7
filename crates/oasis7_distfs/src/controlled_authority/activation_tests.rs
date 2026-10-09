use super::super::{
    LocalRequestIdentity,
    replicated_protocol::{
        ArtifactObject, ClosedRecord, EndpointRole, FileEndpoint, ProtocolOutcome, ReceiptStage,
        ReplicatedCoordinator,
    },
};
use super::*;
use ed25519_dalek::{Signer, SigningKey};
fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn fixture() -> (
    InitialActivationPolicy,
    SignedInitialActivation,
    ClosedRecord,
) {
    // Isolated engineering fixture, not a retained-world or deployment activation.
    let trust = FixedTrust {
        world_id: "activation-fixture".into(),
        chain_id: "fixture-chain".into(),
        genesis_digest: "a".repeat(64),
        authority_epoch: 1,
        writer_key: hex::encode(key(1).verifying_key().to_bytes()),
        primary_id: "p".into(),
        primary_key: hex::encode(key(2).verifying_key().to_bytes()),
        replica_id: "r".into(),
        replica_key: hex::encode(key(3).verifying_key().to_bytes()),
    };
    let mut roots = BTreeMap::new();
    let mut objects = Vec::new();
    for (i, role) in [
        ArtifactRole::Result,
        ArtifactRole::Snapshot,
        ArtifactRole::Journal,
        ArtifactRole::NonceIndex,
        ArtifactRole::EffectOutbox,
        ArtifactRole::ExecutionManifest,
        ArtifactRole::Wasm,
        ArtifactRole::Rules,
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = vec![i as u8; 17];
        let content_hash = blake3::hash(&bytes).to_hex().to_string();
        roots.insert(role, content_hash.clone());
        objects.push(ArtifactObject {
            content_hash,
            bytes,
            references: vec![],
        });
    }
    let policy = InitialActivationPolicy {
        trust: trust.clone(),
        issuer_public_key: hex::encode(key(4).verifying_key().to_bytes()),
        initial_state_root: "b".repeat(64),
        execution_manifest_root: roots[&ArtifactRole::ExecutionManifest].clone(),
        activation_height: 0,
    };
    let body = InitialActivationBody {
        schema_version: 1,
        profile: "controlled_single_authority".into(),
        profile_version: 1,
        trust: trust.clone(),
        position: 1,
        parent_hash: trust.genesis_anchor().unwrap().decision_hash,
        activation_height: 0,
        before_state_root: policy.initial_state_root.clone(),
        after_state_root: "c".repeat(64),
        artifact_roots: roots.clone(),
    };
    let signed = sign(body);
    let bytes = serde_cbor::to_vec(&signed).unwrap();
    let digest = blake3::hash(&bytes).to_hex().to_string();
    roots.insert(ArtifactRole::Input, digest.clone());
    objects.push(ArtifactObject {
        content_hash: digest.clone(),
        bytes,
        references: vec![],
    });
    objects.sort_by(|a, b| a.content_hash.cmp(&b.content_hash));
    (
        policy,
        signed,
        ClosedRecord {
            payload_digest: digest,
            before_state_root: "b".repeat(64),
            after_state_root: "c".repeat(64),
            roots,
            objects,
        },
    )
}
fn sign(body: InitialActivationBody) -> SignedInitialActivation {
    let signature_hex = hex::encode(
        key(4)
            .sign(&initial_activation_signing_bytes(&body).unwrap())
            .to_bytes(),
    );
    SignedInitialActivation {
        body,
        signature_hex,
    }
}
#[test]
#[cfg(unix)]
fn qualified_initial_activation_and_fail_closed_boundaries() {
    let (policy, signed, record) = fixture();
    let genesis = policy.trust.genesis_anchor().unwrap();
    let root =
        std::env::temp_dir().join(format!("oasis7-initial-activation-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let p = FileEndpoint::open(
        root.join("p"),
        policy.trust.clone(),
        EndpointRole::Primary,
        key(2),
        &genesis,
    )
    .unwrap();
    let r = FileEndpoint::open(
        root.join("r"),
        policy.trust.clone(),
        EndpointRole::Replica,
        key(3),
        &genesis,
    )
    .unwrap();
    let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    let request = LocalRequestIdentity {
        verified_subject: "trusted-fixture-issuer".into(),
        operation_domain: DOMAIN.into(),
        nonce_scope: "initial".into(),
        request_id: "1".into(),
    };
    let ProtocolOutcome::DurabilityQualified(evidence) =
        coordinator.submit(1, &genesis, request, record).unwrap()
    else {
        panic!("not qualified")
    };
    let bytes = serde_cbor::to_vec(&signed).unwrap();
    let verified = verify_initial_activation(&bytes, &evidence, &policy, &genesis).unwrap();
    assert!(verified.qualified_head().qualified);
    assert_eq!(verified.body(), &signed.body);
    assert_eq!(
        verified.envelope_digest(),
        evidence.proposal.body.record.payload_digest
    );
    for field in 0..8 {
        let mut wrong = policy.clone();
        match field {
            0 => wrong.issuer_public_key = hex::encode(key(5).verifying_key().to_bytes()),
            1 => wrong.trust.world_id = "other".into(),
            2 => wrong.trust.chain_id = "other".into(),
            3 => wrong.trust.authority_epoch = 2,
            4 => wrong.trust.writer_key = wrong.trust.primary_key.clone(),
            5 => wrong.activation_height = 1,
            6 => wrong.initial_state_root = "d".repeat(64),
            _ => wrong.execution_manifest_root = "e".repeat(64),
        }
        assert!(
            verify_initial_activation(&bytes, &evidence, &wrong, &genesis).is_err(),
            "policy {field}"
        );
    }
    for field in 0..7 {
        let mut wrong = signed.body.clone();
        match field {
            0 => wrong.profile = "bft".into(),
            1 => wrong.profile_version = 2,
            2 => wrong.schema_version = 2,
            3 => wrong.position = 2,
            4 => wrong.parent_hash = verified.qualified_head().decision_hash.clone(),
            5 => wrong
                .artifact_roots
                .insert(ArtifactRole::Rules, "d".repeat(64))
                .map(|_| ())
                .unwrap(),
            _ => wrong.after_state_root = "d".repeat(64),
        }
        assert!(
            verify_initial_activation(
                &serde_cbor::to_vec(&sign(wrong)).unwrap(),
                &evidence,
                &policy,
                &genesis
            )
            .is_err(),
            "body {field}"
        );
    }
    assert!(
        verify_initial_activation(&bytes, &evidence, &policy, verified.qualified_head()).is_err()
    );
    let mut bad = (*evidence).clone();
    bad.primary_receipt.body.stage = ReceiptStage::Prepared;
    assert!(verify_initial_activation(&bytes, &bad, &policy, &genesis).is_err());
    let mut bad = (*evidence).clone();
    bad.replica_receipt = bad.primary_receipt.clone();
    assert!(verify_initial_activation(&bytes, &bad, &policy, &genesis).is_err());
    let mut bad = (*evidence).clone();
    bad.replica_receipt.signature_hex = "0".repeat(128);
    assert!(verify_initial_activation(&bytes, &bad, &policy, &genesis).is_err());
    let mut bad = (*evidence).clone();
    bad.proposal.body.record.objects[0].bytes[0] ^= 1;
    assert!(verify_initial_activation(&bytes, &bad, &policy, &genesis).is_err());
    let mut wrong_signature = signed.clone();
    wrong_signature.signature_hex = hex::encode(
        key(4)
            .sign(&serde_cbor::to_vec(&signed.body).unwrap())
            .to_bytes(),
    );
    assert!(
        verify_initial_activation(
            &serde_cbor::to_vec(&wrong_signature).unwrap(),
            &evidence,
            &policy,
            &genesis
        )
        .is_err()
    );
    let mut false_genesis = genesis.clone();
    false_genesis.decision_hash = "f".repeat(64);
    assert!(verify_initial_activation(&bytes, &evidence, &policy, &false_genesis).is_err());
    let mut unknown: serde_cbor::Value = serde_cbor::from_slice(&bytes).unwrap();
    let serde_cbor::Value::Map(ref mut map) = unknown else {
        panic!("envelope map")
    };
    map.insert(
        serde_cbor::Value::Text("unchecked_activation".into()),
        serde_cbor::Value::Bool(true),
    );
    assert!(
        matches!(verify_initial_activation(&serde_cbor::to_vec(&unknown).unwrap(), &evidence, &policy, &genesis), Err(ProtocolError::Invalid(reason)) if reason == "activation envelope decoding")
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(verify_initial_activation(&trailing, &evidence, &policy, &genesis).is_err());
    assert!(
        verify_initial_activation(
            &vec![0; MAX_ENVELOPE_BYTES + 1],
            &evidence,
            &policy,
            &genesis
        )
        .is_err()
    );
    drop(coordinator);
    std::fs::remove_dir_all(root).unwrap();
}
