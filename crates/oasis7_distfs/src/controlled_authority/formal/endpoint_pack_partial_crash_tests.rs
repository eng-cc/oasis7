//! Real subprocess CAS crash; oracle custody/floor remain controlled simulations.
use super::*;
use crate::controlled_authority::formal::business_index::BusinessIndexNodeWriteV1;
#[derive(serde::Serialize, serde::Deserialize)]
struct CrashFixture {
    directory: std::path::PathBuf,
    certificate: SignedInitialActivationV1,
    trust: InitialActivationTrustV1,
    oracle_address: std::net::SocketAddr,
    writes: Vec<BusinessIndexNodeWriteV1>,
}
fn open(fixture: &CrashFixture) -> FileFormalEndpoint {
    let authority = verify_initial_activation(&fixture.certificate, &fixture.trust).unwrap();
    let provider = Arc::new(
        RemoteMonotonicEndpointProvider::new(
            fixture.oracle_address,
            Duration::from_secs(3),
            key(17),
            TrustedFencingPolicyV1 {
                provider_public_key: public(3),
                authorized_resource_bindings: authority
                    .policy()
                    .authorized_fencing_resource_bindings
                    .clone(),
            },
        )
        .unwrap(),
    );
    FileFormalEndpoint::open_protected(
        &fixture.directory,
        FormalEndpointAuthorizationV1 {
            original: authority.clone(),
            authority,
            recovery: None,
            parent: None,
        },
        EndpointRole::Primary,
        provider,
        None,
    )
    .unwrap()
}
#[test]
#[ignore = "internal pack first-link SIGKILL child; parent supplies controlled fixture"]
fn endpoint_pack_first_link_crash_child() {
    let path = std::env::var_os("OASIS7_ENDPOINT_PACK_CRASH_CONFIG").expect("parent fixture");
    let fixture: CrashFixture = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let endpoint = open(&fixture);
    super::super::super::super::business_pack::persist_kill_after_first_link_for_test(
        &endpoint.store.path.join("formal-objects-v1"),
        &fixture.writes,
    )
    .unwrap();
    panic!("must SIGKILL before all links/checkpoint/receipt");
}
#[test]
fn endpoint_pack_first_link_sigkill_reopens_real_store_and_preserves_retry() {
    use std::os::unix::process::ExitStatusExt;
    let fixture = Fixture::new();
    let oracle = Oracle::start(&fixture, EndpointRole::Primary);
    let body = fixture.authority.initial_body().cloned().unwrap();
    let certificate = SignedInitialActivationV1 {
        signature_hex: signed(initial_activation_signing_bytes(&body).unwrap(), 9),
        body,
    };
    let trust = InitialActivationTrustV1 {
        issuer_public_key: public(9),
        world: fixture.authority.definition().world.clone(),
        initial_state_root: certificate.body.initial_state_root.clone(),
        governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
        recovery_policy_digest: fixture.authority.policy().digest().unwrap(),
    };
    let mut writes: Vec<_> = [
        b"old CAS object".to_vec(),
        b"new CAS first".to_vec(),
        b"new CAS last".to_vec(),
    ]
    .into_iter()
    .map(|bytes| BusinessIndexNodeWriteV1 {
        digest: blake3::hash(&bytes).to_hex().to_string(),
        bytes,
    })
    .collect();
    writes.sort_by(|a, b| a.digest.cmp(&b.digest));
    let crash = CrashFixture {
        directory: fixture.pdir.clone(),
        certificate,
        trust,
        oracle_address: oracle.address,
        writes,
    };
    let endpoint = open(&crash);
    endpoint
        .store
        .persist_business_nodes(&crash.writes[..1])
        .unwrap();
    let original_checkpoint = endpoint.store.checkpoint.clone();
    drop(endpoint);
    let config = fixture.path.join("pack-crash-fixture.json");
    fs::write(&config, serde_json::to_vec(&crash).unwrap()).unwrap();
    let status=std::process::Command::new(std::env::current_exe().unwrap()).args(["--ignored","--exact","controlled_authority::formal::endpoint::tests::protected_tests::pack_partial_crash_tests::endpoint_pack_first_link_crash_child","--test-threads=1"]).env("OASIS7_ENDPOINT_PACK_CRASH_CONFIG",config).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    let staging = fixture
        .pdir
        .join("formal-objects-v1/.business-pack-staging-v1");
    assert_eq!(fs::read_dir(&staging).unwrap().count(), 1);
    assert!(
        !fixture
            .pdir
            .join("formal-objects-v1")
            .join(&crash.writes[2].digest)
            .exists()
    );
    // The real open_protected -> Store::open_signed path acquires its endpoint
    // exclusive lock, removes staging, and verifies the unchanged provider floor.
    let endpoint = open(&crash);
    assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
    assert_eq!(endpoint.store.checkpoint, original_checkpoint);
    for write in &crash.writes[..2] {
        assert!(endpoint.store.raw_object(&write.digest).unwrap().unwrap() == write.bytes);
    }
    endpoint
        .store
        .persist_business_nodes(&crash.writes)
        .unwrap();
    endpoint
        .store
        .persist_business_nodes(&crash.writes)
        .unwrap();
    for write in &crash.writes {
        assert!(endpoint.store.raw_object(&write.digest).unwrap().unwrap() == write.bytes);
    }
    assert_eq!(endpoint.store.checkpoint, original_checkpoint);
    assert_eq!(endpoint.store.body.head.position, 0);
}
