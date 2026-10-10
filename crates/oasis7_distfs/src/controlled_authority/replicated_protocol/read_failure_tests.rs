use super::*;

#[test]
#[cfg(unix)]
fn read_integrity_and_sync_failures_poison_until_explicit_reopen() {
    for mode in ["missing", "json", "identity", "sync"] {
        let root = root();
        let genesis = trust().genesis_anchor().unwrap();
        let (mut p, r) = endpoints(&root, &genesis, &genesis);
        let path = root.join("p/replicated-endpoint.json");
        let good = std::fs::read(&path).unwrap();
        match mode {
            "missing" => std::fs::remove_file(&path).unwrap(),
            "json" => std::fs::write(&path, b"{").unwrap(),
            "identity" => {
                let mut value: serde_json::Value = serde_json::from_slice(&good).unwrap();
                value["trust"]["world_id"] = serde_json::json!("wrong");
                std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            "sync" => p.inject_read_sync_failure(true),
            _ => unreachable!(),
        }
        assert!(p.lookup(&request("a")).is_err(), "{mode}");
        assert!(
            matches!(
                p.prepare(&proposal("a", &genesis)),
                Err(ProtocolError::Poisoned)
            ),
            "{mode}"
        );
        let prep = crypto::sign_receipt(
            &trust(),
            EndpointRole::Replica,
            ReceiptStage::Prepared,
            crypto::proposal_hash(&proposal("a", &genesis)).unwrap(),
            &key(3),
        )
        .unwrap();
        assert!(
            matches!(
                p.decide_primary(&proposal("a", &genesis), &prep),
                Err(ProtocolError::Poisoned)
            ),
            "{mode}"
        );
        std::fs::write(&path, &good).unwrap();
        p.inject_read_sync_failure(false);
        assert!(matches!(
            p.lookup(&request("a")).unwrap(),
            EndpointStatus::NotRecorded
        ));
        assert!(matches!(
            p.prepare(&proposal("a", &genesis)),
            Err(ProtocolError::Poisoned)
        ));
        drop(p);
        let mut reopened = FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis,
        )
        .unwrap();
        reopened.prepare(&proposal("a", &genesis)).unwrap();
        drop((reopened, r));
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
#[cfg(unix)]
fn coordinator_maps_canonical_integrity_failure_to_unknown_without_stale_append() {
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    let mut c = coordinator(&root);
    let path = root.join("p/replicated-endpoint.json");
    let good = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"{").unwrap();
    assert!(
        matches!(c.lookup(&request("a")).unwrap(),ProtocolOutcome::Unknown { request: id } if id == request("a"))
    );
    std::fs::write(&path, &good).unwrap();
    assert!(matches!(
        c.submit(4, &genesis, request("a"), record()).unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    assert_eq!(std::fs::read(&path).unwrap(), good);
    drop(c);
    let mut reopened = coordinator(&root);
    qualified(
        reopened
            .submit(4, &genesis, request("a"), record())
            .unwrap(),
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn lost_primary_after_replica_decision_recovers_original_without_prepare_promotion() {
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    let mut c = coordinator(&root);
    qualified(c.submit(4, &genesis, request("prefix"), record()).unwrap());
    let parent = c.heads().0;
    c.endpoints_mut()
        .1
        .inject_after(3, Fault::Error(Phase::DirectorySynced));
    assert!(matches!(
        c.submit(4, &parent, request("a"), record()).unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    let primary_minimum = c.heads().0;
    let (p, r) = c.into_endpoints();
    drop((p, r));
    std::fs::remove_dir_all(root.join("p")).unwrap();
    let mut r = FileEndpoint::open(
        root.join("r"),
        trust(),
        EndpointRole::Replica,
        key(3),
        &parent,
    )
    .unwrap();
    assert!(matches!(
        r.lookup(&request("a")).unwrap(),
        EndpointStatus::DecisionUnqualified(_)
    ));
    assert_eq!(r.export_evidence().len(), 1);
    r.inject_after(1, Fault::Error(Phase::DirectorySynced));
    assert!(
        matches!(r.repair_local_receipt(&request("a")).unwrap(),ReceiptRepairOutcome::Unknown { request: id } if id == request("a"))
    );
    assert!(matches!(
        r.repair_local_receipt(&request("a")).unwrap(),
        ReceiptRepairOutcome::Unknown { .. }
    ));
    let replica_minimum = r.head();
    drop(r);
    let mut r = FileEndpoint::open(
        root.join("r"),
        trust(),
        EndpointRole::Replica,
        key(3),
        &replica_minimum,
    )
    .unwrap();
    assert!(matches!(
        r.repair_local_receipt(&request("a")).unwrap(),
        ReceiptRepairOutcome::Durable(_)
    ));
    assert!(matches!(
        r.lookup(&request("a")).unwrap(),
        EndpointStatus::DecisionUnqualified(_)
    ));
    let evidence = r.export_evidence();
    assert_eq!(evidence.len(), 2);
    let p = FileEndpoint::restore(
        root.join("p"),
        trust(),
        EndpointRole::Primary,
        key(2),
        &primary_minimum,
        &evidence,
    )
    .unwrap();
    let mut recovered = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    assert!(matches!(
        recovered.lookup(&request("a")).unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    let final_evidence = qualified(
        recovered
            .submit(4, &parent, request("a"), record())
            .unwrap(),
    );
    assert_eq!(final_evidence.proposal, evidence[1].proposal);
    assert_eq!(recovered.heads().0.position, 2);
    drop(recovered);
    std::fs::remove_dir_all(root).unwrap();
}
