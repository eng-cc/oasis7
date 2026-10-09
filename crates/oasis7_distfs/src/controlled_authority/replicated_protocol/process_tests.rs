use super::*;
const HELPER: &str =
    "controlled_authority::replicated_protocol::tests::process_tests::subprocess_helper";
fn child(
    root: &std::path::Path,
    mode: &str,
    endpoint: &str,
    write: usize,
    phase: usize,
) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", HELPER, "--ignored", "--nocapture"])
        .env("O7_REPLICATED_TEST_ROOT", root)
        .env("O7_REPLICATED_TEST_MODE", mode)
        .env("O7_REPLICATED_TEST_ENDPOINT", endpoint)
        .env("O7_REPLICATED_TEST_WRITE", write.to_string())
        .env("O7_REPLICATED_TEST_PHASE", phase.to_string())
        .status()
        .unwrap()
}
#[test]
#[ignore = "subprocess test harness only"]
fn subprocess_helper() {
    let Ok(root) = std::env::var("O7_REPLICATED_TEST_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let genesis = trust().genesis_anchor().unwrap();
    if std::env::var("O7_REPLICATED_TEST_MODE").unwrap() == "locked" {
        assert!(matches!(
            FileEndpoint::open(
                root.join("p"),
                trust(),
                EndpointRole::Primary,
                key(2),
                &genesis
            ),
            Err(ProtocolError::Locked)
        ));
        return;
    }
    let writes = std::env::var("O7_REPLICATED_TEST_WRITE")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let phase = std::env::var("O7_REPLICATED_TEST_PHASE")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let mut c = coordinator(&root);
    let (p, r) = c.endpoints_mut();
    let endpoint = if std::env::var("O7_REPLICATED_TEST_ENDPOINT").unwrap() == "p" {
        p
    } else {
        r
    };
    endpoint.inject_after(writes, Fault::Exit(PHASES[phase]));
    c.submit(4, &genesis, request("original"), record())
        .unwrap();
    panic!("crash hook did not run");
}
#[test]
#[cfg(unix)]
fn process_lock_and_every_crash_phase_preserve_original_protocol() {
    let root = root();
    let c = coordinator(&root);
    assert!(child(&root, "locked", "p", 1, 0).success());
    drop(c);
    std::fs::remove_dir_all(root).unwrap();
    for endpoint in ["p", "r"] {
        for writes in 1..=5 {
            for phase in 0..PHASES.len() {
                let root = super::root();
                assert_eq!(
                    child(&root, "crash", endpoint, writes, phase).code(),
                    Some(73)
                );
                let mut recovered = coordinator(&root);
                let genesis = trust().genesis_anchor().unwrap();
                qualified(
                    recovered
                        .submit(4, &genesis, request("original"), record())
                        .unwrap(),
                );
                drop(recovered);
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
#[test]
#[cfg(unix)]
fn concurrent_expected_parent_allows_only_one_irrevocable_decision() {
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    let (p, mut r) = endpoints(&root, &genesis, &genesis);
    let a = proposal("a", &genesis);
    let b = proposal("b", &genesis);
    let ra = r.prepare(&a).unwrap();
    let rb = r.prepare(&b).unwrap();
    let service = std::sync::Arc::new(std::sync::Mutex::new(p));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers = [(a, ra), (b, rb)]
        .into_iter()
        .map(|(proposal, receipt)| {
            let service = service.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.lock().unwrap().decide_primary(&proposal, &receipt)
            })
        })
        .collect::<Vec<_>>();
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(service.lock().unwrap().head().position, 1);
    drop((service, r));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn prepared_and_primary_only_anchors_cannot_be_restored_as_qualified() {
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    let (mut p, mut r) = endpoints(&root, &genesis, &genesis);
    let a = proposal("a", &genesis);
    p.prepare(&a).unwrap();
    let prep = r.prepare(&a).unwrap();
    let mut c = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    assert!(matches!(
        c.lookup(&request("a")).unwrap(),
        ProtocolOutcome::Pending { .. }
    ));
    let (mut p, mut r) = c.into_endpoints();
    let pr = p.decide_primary(&a, &prep).unwrap();
    let decision_anchor = p.head();
    assert!(!decision_anchor.qualified);
    assert!(p.export_evidence().is_empty());
    assert!(matches!(
        r.lookup(&request("a")).unwrap(),
        EndpointStatus::Prepared(_)
    ));
    drop(p);
    let mut false_anchor = decision_anchor.clone();
    false_anchor.qualified = true;
    assert!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &false_anchor
        )
        .is_err()
    );
    let mut p = FileEndpoint::open(
        root.join("p"),
        trust(),
        EndpointRole::Primary,
        key(2),
        &decision_anchor,
    )
    .unwrap();
    let rr = r.decide_replica(&a, &prep, &pr).unwrap();
    let e = DurabilityEvidence {
        proposal: a,
        replica_prepare: prep,
        primary_receipt: pr,
        replica_receipt: rr,
    };
    p.finalize(&e).unwrap();
    r.finalize(&e).unwrap();
    let anchor = p.head();
    drop((p, r));
    let saved = std::fs::read(root.join("p/replicated-endpoint.json")).unwrap();
    std::fs::remove_dir_all(root.join("p")).unwrap();
    assert!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &anchor
        )
        .is_err()
    );
    std::fs::write(root.join("p/replicated-endpoint.json"), saved).unwrap();
    let reopened = FileEndpoint::open(
        root.join("p"),
        trust(),
        EndpointRole::Primary,
        key(2),
        &anchor,
    )
    .unwrap();
    assert_eq!(reopened.head(), anchor);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn endpoint_paths_and_snapshot_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    std::fs::create_dir(root.join("outside")).unwrap();
    symlink(root.join("outside"), root.join("p")).unwrap();
    assert!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis
        )
        .is_err()
    );
    std::fs::remove_file(root.join("p")).unwrap();
    std::fs::create_dir(root.join("p")).unwrap();
    symlink(
        root.join("outside"),
        root.join("p/replicated-endpoint.json"),
    )
    .unwrap();
    assert!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis
        )
        .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(not(unix))]
fn unsupported_platform_does_not_create_endpoint_files() {
    let root = root();
    let genesis = trust().genesis_anchor().unwrap();
    assert!(matches!(
        FileEndpoint::open(
            root.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &genesis
        ),
        Err(ProtocolError::UnsupportedPlatform)
    ));
    assert!(!root.join("p").exists());
    std::fs::remove_dir_all(root).unwrap();
}
