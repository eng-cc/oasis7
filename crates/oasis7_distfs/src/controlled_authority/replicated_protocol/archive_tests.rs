use super::journal_tests::history;
use super::tests::{key, proposal, request, root, trust};
use super::*;
use std::path::{Path, PathBuf};
fn object(path: &Path, kind: &str, hash: &str) -> PathBuf {
    path.join("archive-v2")
        .join(kind)
        .join(&hash[..2])
        .join(format!("{hash}.cbor"))
}
fn checkpoint(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path.join("endpoint-hot-v2.json")).unwrap()).unwrap()
}
fn anchor(e: &DurabilityEvidence) -> HeadAnchor {
    HeadAnchor {
        position: e.proposal.body.position,
        decision_hash: crypto::verify_proposal(&e.proposal, &trust()).unwrap(),
        qualified: true,
    }
}
#[test]
#[cfg(unix)]
fn signed_checkpoint_proves_skip_ancestry_and_rejects_fork_and_rollback() {
    let root = root();
    let path = root.join("endpoint");
    let history = history(9, false);
    drop(
        FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &anchor(&history[2]),
            &history[..3],
        )
        .unwrap(),
    );
    let old = std::fs::read(path.join("endpoint-hot-v2.json")).unwrap();
    let minimum = anchor(&history[8]);
    let mut endpoint = FileEndpoint::restore(
        &path,
        trust(),
        EndpointRole::Primary,
        key(2),
        &minimum,
        &history,
    )
    .unwrap();
    for e in &history {
        endpoint.verify_rpc_minimum(&anchor(e)).unwrap();
    }
    let mut fork = history[2].proposal.clone();
    fork.body.record.after_state_root = "d".repeat(64);
    fork = crypto::sign_proposal(fork.body, &key(1)).unwrap();
    assert!(
        endpoint
            .verify_rpc_minimum(&HeadAnchor {
                position: 3,
                decision_hash: crypto::verify_proposal(&fork, &trust()).unwrap(),
                qualified: false
            })
            .is_err()
    );
    std::fs::write(path.join("endpoint-hot-v2.json"), old).unwrap();
    assert!(endpoint.lookup(&request("archive-0")).is_err());
    assert!(matches!(
        endpoint.prepare(&proposal("new", &minimum)),
        Err(ProtocolError::Poisoned)
    ));
    drop(endpoint);
    assert!(FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &minimum).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn archive_and_authenticated_absence_fail_closed_on_referenced_corruption() {
    for kind in ["entry", "index", "skip"] {
        let root = root();
        let path = root.join("endpoint");
        let history = history(9, false);
        let minimum = anchor(&history[8]);
        let mut endpoint = FileEndpoint::restore(
            &path,
            trust(),
            EndpointRole::Primary,
            key(2),
            &minimum,
            &history,
        )
        .unwrap();
        let checkpoint = checkpoint(&path);
        let hash = checkpoint["body"]["node"].as_str().unwrap();
        let head: serde_json::Value =
            serde_cbor::from_slice(&std::fs::read(object(&path, "node", hash)).unwrap()).unwrap();
        let oldest = head["ancestors"][3].as_str().unwrap();
        match kind {
            "entry" => {
                let node: serde_json::Value =
                    serde_cbor::from_slice(&std::fs::read(object(&path, "node", oldest)).unwrap())
                        .unwrap();
                std::fs::remove_file(object(&path, "entry", node["entry"].as_str().unwrap()))
                    .unwrap();
                assert!(endpoint.lookup(&request("archive-0")).is_err());
            }
            "index" => {
                let index = object(
                    &path,
                    "index",
                    checkpoint["body"]["index"].as_str().unwrap(),
                );
                std::fs::write(index, b"corrupt radix").unwrap();
                assert!(endpoint.lookup(&request("never-recorded")).is_err());
            }
            "skip" => {
                std::fs::write(object(&path, "node", oldest), b"corrupt skip").unwrap();
                assert!(endpoint.verify_rpc_minimum(&anchor(&history[0])).is_err());
                assert!(endpoint.lookup(&request("archive-0")).is_err());
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            endpoint.prepare(&proposal("new", &minimum)),
            Err(ProtocolError::Poisoned)
        ));
        drop(endpoint);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
#[cfg(unix)]
fn v1_delta_journal_migrates_to_bounded_hot_state_without_deleting_history() {
    let root = root();
    let path = root.join("endpoint");
    let lock = storage::lock(&path).unwrap();
    let history = history(5, false);
    let entries: Vec<_> = history.iter().map(|e|serde_json::json!({"proposal":e.proposal,"replica_prepare":e.replica_prepare,"primary_receipt":e.primary_receipt,"replica_receipt":e.replica_receipt,"published":true})).collect();
    let mut state = serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[],"decisions":[]});
    storage::persist(
        &path,
        &serde_json::to_vec(&state).unwrap(),
        storage::Fault::None,
    )
    .unwrap();
    for entry in entries {
        state["decisions"].as_array_mut().unwrap().push(entry);
        storage::persist(
            &path,
            &serde_json::to_vec(&state).unwrap(),
            storage::Fault::None,
        )
        .unwrap();
    }
    drop(lock);
    let minimum = anchor(&history[4]);
    let mut endpoint =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &minimum).unwrap();
    assert!(
        path.join("journal/00000000000000000000/00000000000000000001.json")
            .exists()
    );
    assert_eq!(
        endpoint.resident_record_bytes(),
        history[4].proposal.body.record.byte_len()
    );
    assert!(
        std::fs::metadata(path.join("endpoint-hot-v2.json"))
            .unwrap()
            .len()
            < 16 * 1024
    );
    assert_eq!(
        endpoint.lookup(&request("archive-0")).unwrap(),
        EndpointStatus::Qualified(Box::new(history[0].clone()))
    );
    drop(endpoint);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn legacy_stale_preparation_remains_pending_without_growing_hot_history() {
    let root = root();
    let path = root.join("endpoint");
    std::fs::create_dir(&path).unwrap();
    let history = history(2, false);
    let pending = proposal("old-pending", &trust().genesis_anchor().unwrap());
    let decisions: Vec<_> = history.iter().map(|e|serde_json::json!({"proposal":e.proposal,"replica_prepare":e.replica_prepare,"primary_receipt":e.primary_receipt,"replica_receipt":e.replica_receipt,"published":true})).collect();
    let old = serde_json::json!({"schema_version":1,"trust":trust(),"role":EndpointRole::Primary,"prepared":[{"proposal":pending,"receipt":null}],"decisions":decisions});
    std::fs::write(
        path.join("replicated-endpoint.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let minimum = anchor(&history[1]);
    let mut endpoint =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &minimum).unwrap();
    assert_eq!(
        endpoint.lookup(&request("old-pending")).unwrap(),
        EndpointStatus::Prepared(Box::new(pending.clone()))
    );
    endpoint
        .prepare(&proposal("current-slot", &minimum))
        .unwrap();
    assert_eq!(
        endpoint.lookup(&request("old-pending")).unwrap(),
        EndpointStatus::Prepared(Box::new(pending))
    );
    assert!(endpoint.resident_record_bytes() < 1024);
    drop(endpoint);
    let mut endpoint =
        FileEndpoint::open(&path, trust(), EndpointRole::Primary, key(2), &minimum).unwrap();
    assert!(matches!(
        endpoint.lookup(&request("old-pending")).unwrap(),
        EndpointStatus::Prepared(_)
    ));
    drop(endpoint);
    std::fs::remove_dir_all(root).unwrap();
}
