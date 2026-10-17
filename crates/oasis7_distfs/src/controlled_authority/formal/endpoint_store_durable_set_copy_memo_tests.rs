use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct MutableSource {
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    reads: AtomicUsize,
}
impl MutableSource {
    fn put(&self, bytes: &[u8]) -> String {
        let id = blake3::hash(bytes).to_hex().to_string();
        self.objects
            .lock()
            .unwrap()
            .insert(id.clone(), bytes.to_vec());
        id
    }
}
impl FormalArchiveSourceV1 for MutableSource {
    fn read_object(&self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.objects.lock().unwrap().get(id).cloned())
    }
}

#[test]
fn healthy_copy_memo_rejects_corrupt_source_seed_and_retained_bytes() {
    let source = MutableSource::default();
    let expected = b"original immutable content";
    let id = source.put(expected);
    let mut memo = CopyReadMemo::new(&source, false);
    assert!(memo.seed(&id, b"wrong inline bytes").is_err());
    assert!(memo.objects.is_empty());
    source
        .objects
        .lock()
        .unwrap()
        .insert(id.clone(), b"wrong source bytes".to_vec());
    assert!(memo.read_object(&id).is_err());
    assert!(memo.objects.is_empty());
    source.put(expected);
    assert_eq!(memo.read_object(&id).unwrap(), Some(expected.to_vec()));
    assert_eq!(source.reads.load(Ordering::Relaxed), 2);
    memo.objects
        .insert(id.clone(), b"corrupt retained bytes".to_vec());
    assert!(memo.read_object(&id).is_err());
    assert_eq!(source.reads.load(Ordering::Relaxed), 2);
}

#[test]
fn healthy_copy_memo_does_not_retain_absence_or_cross_operation_content() {
    let source = MutableSource::default();
    let bytes = b"later available original object";
    let id = blake3::hash(bytes).to_hex().to_string();
    let mut memo = CopyReadMemo::new(&source, false);
    assert!(memo.read_object(&id).unwrap().is_none());
    source.put(bytes);
    assert_eq!(memo.read_object(&id).unwrap(), Some(bytes.to_vec()));
    assert_eq!(source.reads.load(Ordering::Relaxed), 2);
    source.objects.lock().unwrap().clear();
    // This walk already owns the correct immutable bytes; this is not a claim
    // that the original source remains available or independently durable.
    assert_eq!(memo.read_object(&id).unwrap(), Some(bytes.to_vec()));
    drop(memo);
    let mut next_operation = CopyReadMemo::new(&source, false);
    assert!(next_operation.read_object(&id).unwrap().is_none());
    assert_eq!(source.reads.load(Ordering::Relaxed), 3);
}

#[test]
fn healthy_copy_memo_bounds_objects_bytes_and_oversized_fallback() {
    let source = MutableSource::default();
    let mut memo = CopyReadMemo::new(&source, false);
    let first = source.put(&0usize.to_le_bytes());
    for index in 0..=MEMO_OBJECTS {
        let bytes = index.to_le_bytes();
        let id = source.put(&bytes);
        memo.seed(&id, &bytes).unwrap();
        assert!(memo.objects.len() <= MEMO_OBJECTS);
        assert!(memo.bytes <= MEMO_BYTES);
    }
    assert_eq!(memo.objects.len(), MEMO_OBJECTS);
    assert!(!memo.objects.contains_key(&first));
    assert_eq!(
        memo.read_object(&first).unwrap(),
        Some(0usize.to_le_bytes().to_vec())
    );
    let large = vec![81; MEMO_BYTES / 2 + 1];
    let first_large = source.put(&large);
    memo.seed(&first_large, &large).unwrap();
    let other = vec![82; MEMO_BYTES / 2 + 1];
    let second_large = source.put(&other);
    memo.seed(&second_large, &other).unwrap();
    assert!(!memo.objects.contains_key(&first_large));
    assert_eq!(memo.objects.len(), 1);
    assert_eq!(memo.bytes, other.len());
    let oversized = vec![83; MEMO_BYTES + 1];
    let id = source.put(&oversized);
    let before = source.reads.load(Ordering::Relaxed);
    for _ in 0..2 {
        assert_eq!(memo.read_object(&id).unwrap(), Some(oversized.clone()));
        assert!(!memo.objects.contains_key(&id));
        assert_eq!(memo.bytes, other.len());
    }
    assert_eq!(source.reads.load(Ordering::Relaxed), before + 2);
}
