//! Real signed closure copies count source reads, then require an independent
//! local readback. The fixture source never grants authority or supplies a memo.
use super::*;
use std::sync::Mutex;

struct CountedSource {
    source: FileFormalArchiveSourceV1,
    reads: Mutex<BTreeMap<String, usize>>,
}
impl CountedSource {
    fn new(directory: PathBuf) -> Self {
        Self {
            source: FileFormalArchiveSourceV1::new(directory).unwrap(),
            reads: Mutex::new(BTreeMap::new()),
        }
    }
    fn reads(&self, id: &str) -> usize {
        self.reads.lock().unwrap().get(id).copied().unwrap_or(0)
    }
}
impl FormalArchiveSourceV1 for CountedSource {
    fn read_object(&self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        *self.reads.lock().unwrap().entry(id.into()).or_default() += 1;
        self.source.read_object(id)
    }
}
fn target(fixture: &Fixture) -> PathBuf {
    let target = fixture.path.join("independent-copy-target");
    fs::create_dir(&target).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
    }
    target
}

#[test]
fn healthy_copy_source_fetches_shared_history_index_once() {
    let fixture = Fixture::new();
    let mut coordinator = fixture.coordinator();
    coordinator.submit(fixture.initial.clone()).unwrap();
    let primary = coordinator.endpoints_mut().0;
    let checkpoint = primary.checkpoint().unwrap();
    primary.seal_epoch(&h(210), 2, &checkpoint).unwrap();
    let index = primary.store.body.index.clone().unwrap();
    let native = Store::read_native_checkpoint(&fixture.pdir).unwrap();
    let source = CountedSource::new(fixture.pdir.clone());
    let target = target(&fixture);
    let copied = super::super::store::durable_set::copy_authenticated_closure(
        &target,
        &native,
        &public(5),
        &source,
        1,
        64 * 1024 * 1024,
        true,
    )
    .unwrap();
    let local = FileFormalArchiveSourceV1::new(target.clone()).unwrap();
    let verified = super::super::store::durable_set::copy_authenticated_closure(
        &target,
        &native,
        &public(5),
        &local,
        1,
        64 * 1024 * 1024,
        false,
    )
    .unwrap();
    assert_eq!(copied, verified);
    assert_eq!(
        source.reads(&index),
        1,
        "history lookup and index walk must reuse the same hash-verified source bytes"
    );
}

#[test]
fn healthy_copy_source_reuses_authenticated_inline_business_nodes() {
    let snapshot = serde_cbor::to_vec(&serde_json::json!({
        "state": { "retained_history_root": null }
    }))
    .unwrap();
    let (fixture, envelope) = super::business_tests::fixture_with_snapshot(snapshot);
    let mut coordinator = fixture.coordinator();
    coordinator.submit(fixture.initial.clone()).unwrap();
    let primary = coordinator.endpoints_mut().0;
    let checkpoint = primary.checkpoint().unwrap();
    primary.seal_epoch(&h(210), 2, &checkpoint).unwrap();
    let native = Store::read_native_checkpoint(&fixture.pdir).unwrap();
    let source = CountedSource::new(fixture.pdir.clone());
    let target = target(&fixture);
    let copied = super::super::store::durable_set::copy_authenticated_closure(
        &target,
        &native,
        &public(5),
        &source,
        2,
        64 * 1024 * 1024,
        true,
    )
    .unwrap();
    let local = FileFormalArchiveSourceV1::new(target.clone()).unwrap();
    let verified = super::super::store::durable_set::copy_authenticated_closure(
        &target,
        &native,
        &public(5),
        &local,
        2,
        64 * 1024 * 1024,
        false,
    )
    .unwrap();
    assert_eq!(copied, verified);
    assert_eq!(envelope.delta.node_writes.len(), 257);
    let repeated_reads: usize = envelope
        .delta
        .node_writes
        .iter()
        .map(|write| source.reads(&write.digest))
        .sum();
    assert_eq!(
        repeated_reads, 0,
        "the authenticated Result already carries every node; copying must not refetch them"
    );
    // A verified inline copy is not an install verdict. Independent readback
    // must still detect damage to the newly written physical pack.
    fs::write(
        target
            .join("formal-objects-v1")
            .join(&envelope.delta.node_writes[0].digest),
        b"corrupt independently stored business pack",
    )
    .unwrap();
    assert!(
        super::super::store::durable_set::copy_authenticated_closure(
            &target,
            &native,
            &public(5),
            &local,
            2,
            64 * 1024 * 1024,
            false,
        )
        .is_err()
    );
}

#[test]
fn healthy_copy_source_rechecks_new_manifest_and_stale_source_on_each_walk() {
    let fixture = Fixture::new();
    let mut coordinator = fixture.coordinator();
    coordinator.submit(fixture.initial.clone()).unwrap();
    let primary = coordinator.endpoints_mut().0;
    let checkpoint = primary.checkpoint().unwrap();
    primary.seal_epoch(&h(210), 2, &checkpoint).unwrap();
    let mut body = primary.store.body.clone();
    let index = body.index.clone().unwrap();
    let native = Store::read_native_checkpoint(&fixture.pdir).unwrap();
    let source = CountedSource::new(fixture.pdir.clone());
    let target = target(&fixture);
    let copy = |checkpoint: &[u8]| {
        super::super::store::durable_set::copy_authenticated_closure(
            &target,
            checkpoint,
            &public(5),
            &source,
            1,
            64 * 1024 * 1024,
            true,
        )
    };
    copy(&native).unwrap();
    // Even an otherwise correctly signed new checkpoint cannot borrow the
    // prior walk's index roots, bytes or membership verdict.
    body.index = Some(h(211));
    let changed = Store::encode_provider_checkpoint(
        &body,
        &signed(Store::checkpoint_signing_bytes(&body).unwrap(), 5),
        &public(5),
    )
    .unwrap();
    assert!(copy(&changed).is_err());
    let before = source.reads(&index);
    fs::write(
        fixture.pdir.join("formal-objects-v1").join(&index),
        b"corrupt source after prior successful walk",
    )
    .unwrap();
    assert!(copy(&native).is_err());
    assert_eq!(source.reads(&index), before + 1);
}
