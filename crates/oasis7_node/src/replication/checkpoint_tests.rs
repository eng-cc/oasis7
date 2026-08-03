use super::*;
use crate::{NodeExecutionCheckpointBlob, NodeExecutionCheckpointBundle};
use std::path::PathBuf;

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oasis7-replication-checkpoint-tests-{prefix}-{unique}"
    ))
}

fn deterministic_keypair_hex(seed: u8) -> (String, String) {
    let bytes = [seed; 32];
    let signing_key = SigningKey::from_bytes(&bytes);
    (
        hex::encode(signing_key.to_bytes()),
        hex::encode(signing_key.verifying_key().to_bytes()),
    )
}

fn empty_action_root() -> String {
    crate::compute_consensus_action_root(&[]).expect("empty action root")
}

fn checkpoint_bundle(
    height: u64,
    execution_block_hash: &str,
    execution_state_root: &str,
    snapshot_bytes: &[u8],
) -> NodeExecutionCheckpointBundle {
    NodeExecutionCheckpointBundle {
        height,
        execution_block_hash: execution_block_hash.to_string(),
        execution_state_root: execution_state_root.to_string(),
        manifest_json: br#"{"checkpoint":1}"#.to_vec(),
        blobs: vec![NodeExecutionCheckpointBlob {
            content_hash: oasis7_distfs::blake3_hex(snapshot_bytes),
            bytes: snapshot_bytes.to_vec(),
        }],
    }
}

fn committed_decision(height: u64) -> PosDecision {
    PosDecision {
        height,
        slot: height,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: format!("block-{height}"),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    }
}

#[test]
fn checkpoint_verification_receipt_is_full_closure_bound_and_atomic() {
    let dir = temp_dir("verification-receipt");
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let runtime = ReplicationRuntime::new(&config, "observer-a").expect("runtime");
    let bundle = checkpoint_bundle(42, "exec-block-42", "exec-state-42", b"verified-snapshot");
    let descriptor = runtime
        .store_execution_checkpoint_bundle(&bundle)
        .expect("store checkpoint closure");
    let receipt_path = dir.join("checkpoint-verification/42.json");
    let verified_fetch_observations = [
        serde_json::json!({
            "source": "network_fetch", "content_hash": descriptor.manifest_ref,
            "observed_content_hash": descriptor.manifest_ref,
            "observed_size_bytes": descriptor.manifest_size_bytes,
            "response_found": true, "signed_request": true,
            "connected_candidate_ids": ["storage-a"],
        }),
        serde_json::json!({
            "source": "network_fetch", "content_hash": descriptor.blobs[0].content_hash,
            "observed_content_hash": descriptor.blobs[0].content_hash,
            "observed_size_bytes": descriptor.blobs[0].size_bytes,
            "response_found": true, "signed_request": true,
            "connected_candidate_ids": ["storage-a"],
        }),
    ];

    runtime
        .persist_checkpoint_verification_receipt("world-receipt", None, &descriptor, None, &[])
        .expect("ordinary sync does not write a probe receipt");
    assert!(
        !receipt_path.exists(),
        "ordinary sync must not emit a reusable probe receipt"
    );

    runtime
        .persist_checkpoint_verification_receipt(
            "world-receipt",
            Some("probe-nonce-0123456789abcdef0123456789abcdef"),
            &descriptor,
            Some(&bundle),
            &verified_fetch_observations,
        )
        .expect("publish verified receipt");
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).expect("receipt bytes"))
            .expect("receipt json");
    assert_eq!(receipt["world_id"], "world-receipt");
    assert_eq!(
        receipt["probe_nonce"],
        "probe-nonce-0123456789abcdef0123456789abcdef"
    );
    assert_eq!(receipt["manifest_hash"], descriptor.manifest_ref);
    assert_eq!(receipt["objects"].as_array().expect("objects").len(), 2);
    assert_eq!(
        receipt["objects"][0]["expected_content_hash"],
        receipt["objects"][0]["observed_content_hash"]
    );
    let err = runtime
        .persist_checkpoint_verification_receipt(
            "world-receipt",
            Some("second-probe-nonce-0123456789abcdef0123456789"),
            &descriptor,
            Some(&bundle),
            &verified_fetch_observations,
        )
        .expect_err("a probe must not overwrite an existing receipt");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("without overwrite"))
    );
    let unchanged: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).expect("receipt bytes"))
            .expect("unchanged receipt json");
    assert_eq!(unchanged["probe_nonce"], receipt["probe_nonce"]);

    let mut mismatched = bundle.clone();
    mismatched.blobs[0].bytes = b"tampered-snapshot".to_vec();
    let err = runtime
        .persist_checkpoint_verification_receipt(
            "world-receipt",
            Some("probe-nonce-0123456789abcdef0123456789abcdef"),
            &descriptor,
            Some(&mismatched),
            &verified_fetch_observations,
        )
        .expect_err("tampered closure must not publish a receipt");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("blob binding mismatch"))
    );
    assert!(
        receipt_path.is_file(),
        "a failed later attempt must not replace the verified receipt"
    );
    assert!(
        !dir.join(format!(
            "checkpoint-verification/42.{}.tmp",
            std::process::id()
        ))
        .exists(),
        "failed validation must not create a partial receipt"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn attach_execution_checkpoint_descriptor_resigns_remote_writer_legacy_message() {
    let dir_a = temp_dir("remote-writer-a");
    let dir_b = temp_dir("remote-writer-b");
    let world_id = "world-checkpoint-attach-remote-writer";
    let (private_a, public_a) = deterministic_keypair_hex(211);
    let (private_b, public_b) = deterministic_keypair_hex(212);
    let config_a = NodeReplicationConfig::new(&dir_a)
        .expect("config a")
        .with_signing_keypair(private_a, public_a)
        .expect("signing a");
    let config_b = NodeReplicationConfig::new(&dir_b)
        .expect("config b")
        .with_signing_keypair(private_b, public_b.clone())
        .expect("signing b");
    let mut runtime_a = ReplicationRuntime::new(&config_a, "node-a").expect("runtime a");
    let mut runtime_b = ReplicationRuntime::new(&config_b, "node-b").expect("runtime b");
    let message = runtime_a
        .build_local_commit_message(
            "node-a",
            world_id,
            1_000,
            &committed_decision(1),
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build remote writer legacy commit")
        .expect("message");
    let checkpoint = checkpoint_bundle(1, "exec-block-1", "exec-state-1", b"remote-writer");

    let augmented = runtime_b
        .attach_execution_checkpoint_descriptor_to_message("node-b", &message, &checkpoint)
        .expect("attach should re-sign remote writer message");

    assert_ne!(augmented, message);
    assert_eq!(augmented.node_id, "node-a");
    assert_eq!(augmented.record.writer_id, public_b);
    assert_eq!(augmented.public_key_hex.as_deref(), Some(public_b.as_str()));
    assert!(augmented.signature_hex.is_some());
    let payload = serde_json::from_slice::<ReplicatedCommitPayload>(augmented.payload.as_slice())
        .expect("payload");
    assert_eq!(payload.node_id, "node-a");
    assert_eq!(payload.height, 1);
    assert!(payload.execution_checkpoint.is_some());
    let dir_observer = temp_dir("remote-writer-observer");
    let observer_config = NodeReplicationConfig::new(&dir_observer)
        .expect("observer config")
        .with_remote_writer_allowlist(vec![public_b])
        .expect("observer allowlist");
    let observer_runtime =
        ReplicationRuntime::new(&observer_config, "node-observer").expect("observer runtime");
    assert!(
        observer_runtime
            .validate_remote_message_for_apply("node-observer", world_id, &augmented)
            .expect("augmented message validates")
    );

    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
    let _ = std::fs::remove_dir_all(&dir_observer);
}

#[test]
fn attach_execution_checkpoint_descriptor_pins_augmented_payload_blob() {
    let dir = temp_dir("pins-augmented-payload");
    let world_id = "world-checkpoint-attach-pins-augmented-payload";
    let (private_hex, public_hex) = deterministic_keypair_hex(213);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(private_hex, public_hex)
        .expect("signing")
        .with_max_hot_commit_messages(1)
        .expect("hot commit cap");
    let mut runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
    let message = runtime
        .build_local_commit_message(
            "node-a",
            world_id,
            1_000,
            &committed_decision(1),
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build legacy local message")
        .expect("message");
    let checkpoint = checkpoint_bundle(1, "exec-block-1", "exec-state-1", b"pinned-snapshot");
    let augmented = runtime
        .attach_execution_checkpoint_descriptor_to_message("node-a", &message, &checkpoint)
        .expect("attach checkpoint descriptor");
    assert_ne!(augmented.record.content_hash, message.record.content_hash);
    assert!(
        runtime
            .load_blob_by_hash(augmented.record.content_hash.as_str())
            .expect("load augmented payload before prune")
            .is_some()
    );

    runtime
        .build_local_commit_message(
            "node-a",
            world_id,
            2_000,
            &committed_decision(2),
            Some("exec-block-2"),
            Some("exec-state-2"),
        )
        .expect("build second commit")
        .expect("message");

    assert!(
        runtime
            .load_blob_by_hash(augmented.record.content_hash.as_str())
            .expect("load pinned augmented payload after prune")
            .is_some()
    );

    let _ = std::fs::remove_dir_all(&dir);
}
