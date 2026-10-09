use super::*;

#[test]
fn writer_key_is_distinct_from_each_endpoint_key() {
    trust().validate().unwrap();
    for role in [EndpointRole::Primary, EndpointRole::Replica] {
        let mut alias = trust();
        alias.writer_key = alias.endpoint(role).1.into();
        assert!(alias.validate().is_err(), "writer aliases {role:?}");
    }
}
fn changed_record() -> ClosedRecord {
    let mut changed = record();
    changed.after_state_root = "d".repeat(64);
    changed
}
fn forked_proposal(parent: &HeadAnchor) -> SignedProposal {
    let mut body = proposal("fork", parent).body;
    body.record = changed_record();
    crypto::sign_proposal(body, &key(1)).unwrap()
}
#[test]
#[cfg(unix)]
fn prepared_endpoint_disagreement_poison_lookup() {
    assert_prepared_disagreement(true);
}
#[test]
#[cfg(unix)]
fn prepared_endpoint_disagreement_poison_submit() {
    assert_prepared_disagreement(false);
}
#[cfg(unix)]
fn assert_prepared_disagreement(through_lookup: bool) {
    {
        let root = root();
        let genesis = trust().genesis_anchor().unwrap();
        let (mut p, mut r) = endpoints(&root, &genesis, &genesis);
        p.prepare(&proposal("fork", &genesis)).unwrap();
        r.prepare(&forked_proposal(&genesis)).unwrap();
        let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
        let before = coordinator.heads();
        let outcome = if through_lookup {
            coordinator.lookup(&request("fork"))
        } else {
            coordinator.submit(4, &genesis, request("fork"), record())
        };
        assert!(
            matches!(outcome, Ok(ProtocolOutcome::Unknown {request: id}) if id == request("fork"))
        );
        // A read-only query may still report verified data, but never unpoisons.
        let _ = coordinator.lookup(&request("not-recorded"));
        assert!(
            matches!(coordinator.submit(4, &genesis, request("new"), record()).unwrap(), ProtocolOutcome::Unknown {request: id} if id == request("new"))
        );
        assert_eq!(coordinator.heads(), before);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
#[cfg(unix)]
fn qualified_endpoint_disagreement_poison_lookup_without_advancing_heads() {
    let left = root();
    let right = root();
    let genesis = trust().genesis_anchor().unwrap();
    let mut a = coordinator(&left);
    let mut b = coordinator(&right);
    qualified(a.submit(4, &genesis, request("fork"), record()).unwrap());
    qualified(
        b.submit(4, &genesis, request("fork"), changed_record())
            .unwrap(),
    );
    let (p, unused_r) = a.into_endpoints();
    let (unused_p, r) = b.into_endpoints();
    drop((unused_p, unused_r));
    let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    let before = coordinator.heads();
    assert!(
        matches!(coordinator.lookup(&request("fork")), Ok(ProtocolOutcome::Unknown {request: id}) if id == request("fork"))
    );
    assert!(matches!(
        coordinator
            .submit(4, &before.0, request("new"), record())
            .unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    assert_eq!(coordinator.heads(), before);
    drop(coordinator);
    std::fs::remove_dir_all(left).unwrap();
    std::fs::remove_dir_all(right).unwrap();
}
