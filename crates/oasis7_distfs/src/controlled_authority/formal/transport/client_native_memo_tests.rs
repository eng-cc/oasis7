use super::*;

#[test]
fn corrupted_memo_content_cannot_replace_authenticated_cas_bytes() {
    let bytes = b"authenticated checkpoint object";
    let id = blake3::hash(bytes).to_hex().to_string();
    let mut memo = RestoreMemo::default();
    assert!(memo.insert(&id, b"different object").is_err());
    assert!(memo.objects.is_empty());
    memo.insert(&id, bytes).unwrap();
    memo.objects.get_mut(&id).unwrap()[0] ^= 1;
    assert!(
        memo.get(&id)
            .unwrap_err()
            .to_string()
            .contains("memo CAS hash mismatch")
    );
    assert_eq!(memo.hits, 0);
}

#[test]
fn memo_entry_limit_preserves_prior_objects_and_leaves_new_objects_for_fetch() {
    let mut memo = RestoreMemo::default();
    let mut first = String::new();
    for ordinal in 0..MAX_RESTORE_MEMO_ENTRIES {
        let bytes = ordinal.to_le_bytes();
        let id = blake3::hash(&bytes).to_hex().to_string();
        if ordinal == 0 {
            first = id.clone();
        }
        memo.insert(&id, &bytes).unwrap();
    }
    let extra = b"object beyond entry limit";
    let extra_id = blake3::hash(extra).to_hex().to_string();
    memo.insert(&extra_id, extra).unwrap();
    assert!(memo.get(&extra_id).unwrap().is_none());
    assert_eq!(memo.get(&first).unwrap().unwrap(), 0usize.to_le_bytes());
    assert_eq!(memo.objects.len(), MAX_RESTORE_MEMO_ENTRIES);
}

#[test]
fn memo_byte_limit_accepts_exact_boundary_and_does_not_cache_overflow() {
    let mut memo = RestoreMemo::default();
    let bytes = vec![7; MAX_RESTORE_MEMO_BYTES];
    let id = blake3::hash(&bytes).to_hex().to_string();
    memo.insert(&id, &bytes).unwrap();
    let extra = [8];
    let extra_id = blake3::hash(&extra).to_hex().to_string();
    memo.insert(&extra_id, &extra).unwrap();
    assert_eq!(memo.bytes, MAX_RESTORE_MEMO_BYTES);
    assert!(memo.get(&extra_id).unwrap().is_none());
    assert_eq!(memo.get(&id).unwrap().unwrap(), bytes);
    // Duplicate reads cannot consume another allocation/accounted entry.
    memo.insert(&id, &bytes).unwrap();
    assert_eq!(memo.objects.len(), 1);
    assert_eq!(memo.bytes, MAX_RESTORE_MEMO_BYTES);
}
