//! Publication/unknown/partial-reclaim fault fixtures use modeled endpoints.
use super::*;

#[test]
fn published_attempt_reclamation_retries_independent_readback_and_partial_unlink() {
    let policy = ReleaseSecurityPolicy::default();
    let (world, _, anchor, _) = fixtures::actual_world_initial_fixture_with_execution_policy(
        formal_package::execution_policy_material(&policy)
            .unwrap()
            .2,
    );
    let original = continuation(&world);
    let primary = endpoint(&anchor, EndpointRole::Primary, false);
    let replica = endpoint(&anchor, EndpointRole::Replica, false);
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("oasis7-attempt-reclaim-{}", std::process::id()));
    assert!(!root.exists());
    let open = || {
        FormalWriter::open(
            root.clone(),
            fixtures::key(4),
            policy.clone(),
            world.clone(),
            anchor.clone(),
            primary.clone(),
            replica.clone(),
        )
    };
    let mut writer = open().unwrap();
    writer.prepare(&original).unwrap();
    let proof = writer.resume_original().unwrap();
    let attempt = std::fs::read_dir(root.join("attempts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    #[derive(Deserialize)]
    struct Summary {
        phase: String,
        duplicate_outputs: Vec<String>,
    }
    let summary = || -> Summary {
        serde_cbor::from_slice(&std::fs::read(attempt.join("attempt-lifecycle-v1.cbor")).unwrap())
            .unwrap()
    };
    let outputs = summary().duplicate_outputs;
    assert!(!outputs.is_empty());
    assert_eq!(summary().phase, "RecoveryPinned");
    assert!(
        root.join("pending.cbor").is_file(),
        "pending acknowledgement remains independent"
    );
    for id in &outputs {
        assert!(
            attempt
                .join("storage/blobs")
                .join(format!("{id}.blob"))
                .is_file()
        );
    }
    // One healthy destination cannot unpin any output while the other is unknown.
    primary.0.lock().unwrap().closure_readback = true;
    assert_eq!(writer.resume_original().unwrap(), proof);
    assert_eq!(summary().phase, "RecoveryPinned");
    // Same signed candidate with altered local proof bytes is not the exact
    // independently stored publication and cannot authorize source deletion.
    replica.0.lock().unwrap().closure_readback = true;
    let publication_path = root
        .join("published")
        .join(format!("{:020}.cbor", proof.decision.record.position));
    let exact: Published = read(&publication_path).unwrap();
    let mut altered = exact.clone();
    altered.proof.decision.primary.signature_hex = "00".repeat(64);
    durable_transaction::write_private_file_durable(&publication_path, &to_cbor(&altered).unwrap())
        .unwrap();
    writer.reclaim_published_attempts(&altered);
    assert_eq!(summary().phase, "RecoveryPinned");
    for id in &outputs {
        assert!(
            attempt
                .join("storage/blobs")
                .join(format!("{id}.blob"))
                .is_file()
        );
    }
    durable_transaction::write_private_file_durable(&publication_path, &to_cbor(&exact).unwrap())
        .unwrap();
    drop(writer);
    // Model interruption after an unlink: the durable exact delete list permits
    // retry, and independently stored material must be read again on reopening.
    let missing = attempt
        .join("storage/blobs")
        .join(format!("{}.blob", outputs[0]));
    std::fs::remove_file(missing).unwrap();
    let unique = attempt.join("unique-recovery-evidence");
    std::fs::write(&unique, b"unknown external delivery receipt").unwrap();
    replica.0.lock().unwrap().closure_readback = true;
    let mut reopened = open().unwrap();
    assert_eq!(summary().phase, "Reclaimed");
    for id in &outputs {
        assert!(
            !attempt
                .join("storage/blobs")
                .join(format!("{id}.blob"))
                .exists()
        );
    }
    assert_eq!(
        std::fs::read(&unique).unwrap(),
        b"unknown external delivery receipt"
    );
    assert!(attempt.join("world").is_dir());
    assert!(attempt.join("records").is_dir());
    assert!(root.join("pending.cbor").is_file());
    assert_eq!(
        reopened.resume_original().unwrap(),
        proof,
        "original publication works without attempt output"
    );
    reopened
        .acknowledge(&proof.decision.record.record_id().unwrap())
        .unwrap();
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
