use super::tests::{key, proposal, record, request, root, trust};
use super::*;

pub(super) fn history(count: usize, large: bool) -> Vec<DurabilityEvidence> {
    let mut head = trust().genesis_anchor().unwrap();
    (0..count)
        .map(|i| {
            let mut p = proposal(&format!("archive-{i}"), &head);
            if large {
                let mut r = record();
                let object = &mut r.objects[0];
                let old = object.content_hash.clone();
                object.bytes = vec![7; 256 * 1024];
                object.content_hash = blake3::hash(&object.bytes).to_hex().to_string();
                for hash in r.roots.values_mut() {
                    if *hash == old {
                        *hash = object.content_hash.clone();
                    }
                }
                r.payload_digest = r.roots[&ArtifactRole::Input].clone();
                r.objects
                    .sort_by(|a, b| a.content_hash.cmp(&b.content_hash));
                p.body.record = r;
                p = crypto::sign_proposal(p.body, &key(1)).unwrap();
            }
            let digest = crypto::verify_proposal(&p, &trust()).unwrap();
            let receipt = |role, stage, n| {
                crypto::sign_receipt(&trust(), role, stage, digest.clone(), &key(n)).unwrap()
            };
            let e = DurabilityEvidence {
                frozen_certificate: None,
                proposal: p,
                replica_prepare: receipt(EndpointRole::Replica, ReceiptStage::Prepared, 3),
                primary_receipt: receipt(EndpointRole::Primary, ReceiptStage::DecisionDurable, 2),
                replica_receipt: receipt(EndpointRole::Replica, ReceiptStage::DecisionDurable, 3),
            };
            head = crypto::verify_evidence(&e, &trust(), &head).unwrap();
            e
        })
        .collect()
}
#[test]
#[cfg(unix)]
fn archive_exceeds_previous_entry_and_byte_caps_and_preserves_old_identity() {
    let root = root();
    let history = history(66, true);
    assert!(
        history
            .iter()
            .map(|e| e.proposal.body.record.byte_len())
            .sum::<usize>()
            > 16 * 1024 * 1024
    );
    let last = &history.last().unwrap().proposal;
    let minimum = HeadAnchor {
        position: last.body.position,
        decision_hash: crypto::verify_proposal(last, &trust()).unwrap(),
        qualified: true,
    };
    let path = root.join("endpoint");
    let endpoint = FileEndpoint::restore(
        &path,
        trust(),
        EndpointRole::Primary,
        key(2),
        &minimum,
        &history,
    )
    .unwrap();
    drop(endpoint);
    let mut reopened =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &minimum).unwrap();
    assert_eq!(
        reopened.lookup(&request("archive-0")).unwrap(),
        EndpointStatus::Qualified(Box::new(history[0].clone()))
    );
    let mut changed = proposal("archive-0", &minimum);
    changed.body.record.after_state_root = "d".repeat(64);
    changed = crypto::sign_proposal(changed.body, &key(1)).unwrap();
    assert!(reopened.prepare(&changed).is_err());
    reopened
        .prepare(&proposal("after-archive", &minimum))
        .unwrap();
    assert!(path.join("endpoint-hot-v2.json").exists());
    assert!(reopened.resident_record_bytes() <= 512 * 1024);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn journal_rejects_corruption_and_committed_partial_frame_but_ignores_temp() {
    let root = root();
    let path = root.join("endpoint");
    let lock = storage::lock(&path).unwrap();
    let state = serde_json::to_vec(&serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[],"decisions":[]})).unwrap();
    storage::persist(&path, &state, storage::Fault::None).unwrap();
    drop(lock);
    let segment = path.join("journal/00000000000000000000");
    std::fs::write(segment.join(".replicated-pending-crash"), b"partial").unwrap();
    storage::read(&path).unwrap();
    let frame = segment.join("00000000000000000001.json");
    let original = std::fs::read(&frame).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    changed["digest"] = "0".repeat(64).into();
    std::fs::write(&frame, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(storage::read(&path).is_err());
    std::fs::write(&frame, &original[..original.len() / 2]).unwrap();
    assert!(storage::read(&path).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn legacy_snapshot_is_readable_and_migrates_without_rewriting_base() {
    let root = root();
    let path = root.join("endpoint");
    std::fs::create_dir(&path).unwrap();
    let archived = history(1, false).remove(0);
    let legacy = serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[],"decisions":[{
        "proposal":archived.proposal,"replica_prepare":archived.replica_prepare,
        "primary_receipt":archived.primary_receipt,"replica_receipt":archived.replica_receipt,"published":true
    }]});
    let bytes = serde_json::to_vec(&legacy).unwrap();
    std::fs::write(path.join("replicated-endpoint.json"), &bytes).unwrap();
    let genesis = trust().genesis_anchor().unwrap();
    let mut e =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &genesis).unwrap();
    assert!(matches!(
        e.lookup(&request("archive-0")).unwrap(),
        EndpointStatus::Qualified(_)
    ));
    e.prepare(&proposal("legacy", &e.head())).unwrap();
    drop(e);
    assert_eq!(
        std::fs::read(path.join("replicated-endpoint.json")).unwrap(),
        bytes
    );
    let mut e =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &genesis).unwrap();
    assert!(matches!(
        e.lookup(&request("legacy")).unwrap(),
        EndpointStatus::Prepared(_)
    ));
    drop(e);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn sealed_checkpoint_corruption_and_permissions_are_checked() {
    use std::os::unix::fs::PermissionsExt;
    let root = root();
    let path = root.join("endpoint");
    let lock = storage::lock(&path).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[],"decisions":[]})).unwrap();
    storage::persist(&path, &bytes, storage::Fault::None).unwrap();
    drop(lock);
    for _ in 1..64 {
        storage::persist(&path, &bytes, storage::Fault::None).unwrap();
    }
    let segment = path.join("journal/00000000000000000000");
    for dir in [&path, &path.join("journal"), &segment] {
        assert_eq!(
            std::fs::metadata(dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    for file in [
        path.join(".replicated-endpoint.lock"),
        segment.join("00000000000000000001.json"),
        segment.join("checkpoint.json"),
    ] {
        assert_eq!(
            std::fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    std::fs::write(segment.join("checkpoint.json"), br#"[64,"corrupt"]"#).unwrap();
    assert!(storage::read(&path).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "restore crash subprocess helper"]
fn restore_crash_helper() {
    let Some(path) = std::env::var_os("OASIS7_RESTORE_CRASH_PATH") else {
        return;
    };
    let bytes = std::fs::read(std::env::var_os("OASIS7_RESTORE_CRASH_STATE").unwrap()).unwrap();
    let state: super::endpoint::State = serde_json::from_slice(&bytes).unwrap();
    let history: Vec<_> = state
        .decisions
        .iter()
        .map(|e| e.evidence().unwrap())
        .collect();
    FileEndpoint::restore_with_fault(
        std::path::Path::new(&path),
        trust(),
        EndpointRole::Primary,
        key(2),
        &trust().genesis_anchor().unwrap(),
        &history,
        storage::Fault::Exit(storage::Phase::Renamed),
    )
    .unwrap();
}

#[test]
#[cfg(unix)]
fn interrupted_restore_resumes_only_matching_qualified_prefix() {
    let root = root();
    let path = root.join("endpoint");
    let evidence = history(3, false);
    let genesis = trust().genesis_anchor().unwrap();
    drop(
        FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis,
            &evidence[..1],
        )
        .unwrap(),
    );
    let decisions: Vec<_> = evidence.iter().map(|e| serde_json::json!({
        "proposal":e.proposal,"replica_prepare":e.replica_prepare,"primary_receipt":e.primary_receipt,
        "replica_receipt":e.replica_receipt,"published":true
    })).collect();
    let full = serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[],"decisions":decisions});
    let state_file = root.join("restore-state.json");
    std::fs::write(&state_file, serde_json::to_vec(&full).unwrap()).unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "controlled_authority::replicated_protocol::journal_tests::restore_crash_helper",
            "--ignored",
        ])
        .env("OASIS7_RESTORE_CRASH_PATH", &path)
        .env("OASIS7_RESTORE_CRASH_STATE", &state_file)
        .status()
        .unwrap();
    assert_eq!(child.code(), Some(73));
    let prefix =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &genesis).unwrap();
    assert_eq!(prefix.head().position, 2);
    drop(prefix);
    // A shorter/different archive cannot overwrite the durable prefix.
    assert!(
        FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis,
            &evidence[..1]
        )
        .is_err()
    );
    assert!(
        FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis,
            &history(3, true)
        )
        .is_err()
    );
    let minimum = crypto::verify_evidence(
        &evidence[2],
        &trust(),
        &HeadAnchor {
            position: 2,
            decision_hash: crypto::verify_proposal(&evidence[1].proposal, &trust()).unwrap(),
            qualified: true,
        },
    )
    .unwrap();
    let restored = FileEndpoint::restore(
        &path,
        trust(),
        EndpointRole::Primary,
        key(2),
        &minimum,
        &evidence,
    )
    .unwrap();
    assert_eq!(restored.export_evidence(), evidence);
    drop(restored);
    // A completed retry is also idempotent.
    drop(
        FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &minimum,
            &evidence,
        )
        .unwrap(),
    );
    std::fs::remove_dir_all(root).unwrap();
}
