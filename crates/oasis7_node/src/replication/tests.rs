use super::*;
use oasis7_proto::storage_cold_index::{
    storage_cold_index_dir_name, STORAGE_COLD_INDEX_MANIFEST_FILE,
    STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF,
};
use std::path::PathBuf;

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-replication-tests-{prefix}-{unique}"))
}

fn deterministic_keypair_hex(seed: u8) -> (String, String) {
    let bytes = [seed; 32];
    let signing_key = SigningKey::from_bytes(&bytes);
    (
        hex::encode(signing_key.to_bytes()),
        hex::encode(signing_key.verifying_key().to_bytes()),
    )
}

fn signed_remote_message(
    seed: u8,
    world_id: &str,
    node_id: &str,
    sequence: u64,
) -> GossipReplicationMessage {
    let (private_hex, public_hex) = deterministic_keypair_hex(seed);
    let signer = ReplicationSigningKey {
        signing_key: signing_key_from_hex(private_hex.as_str()).expect("signing key"),
        public_key_hex: public_hex.clone(),
    };
    let payload = format!("payload-{seed}-{sequence}").into_bytes();
    let path = format!("{COMMIT_FILE_PREFIX}/{:020}.json", sequence.max(1));
    let record = build_replication_record_with_epoch(
        world_id,
        public_hex.as_str(),
        1,
        sequence.max(1),
        path.as_str(),
        payload.as_slice(),
        1_000,
    )
    .expect("record");
    let mut message = GossipReplicationMessage {
        version: REPLICATION_VERSION,
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        record,
        payload,
        public_key_hex: Some(public_hex),
        signature_hex: None,
    };
    message.signature_hex = Some(sign_replication_message(&message, &signer).expect("sign"));
    message
}

fn cold_entry_content_hash(
    cold_index: &commit_retention::CommitMessageColdIndex,
    height: u64,
) -> String {
    cold_index
        .by_height
        .get(&height)
        .expect("cold entry")
        .content_hash()
        .to_string()
}

#[test]
fn next_local_record_position_rejects_sequence_overflow_for_existing_writer() {
    let dir = temp_dir("existing-writer-sequence-overflow");
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let mut runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
    runtime.guard = SingleWriterReplicationGuard {
        writer_id: Some("node-a".to_string()),
        writer_epoch: 7,
        last_sequence: u64::MAX,
    };
    runtime.writer_state = LocalWriterState {
        writer_epoch: 7,
        last_sequence: u64::MAX,
        last_replicated_height: 0,
    };

    let err = runtime
        .next_local_record_position("node-a")
        .expect_err("sequence overflow should fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("sequence overflow"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn next_local_record_position_rejects_writer_epoch_overflow_on_writer_switch() {
    let dir = temp_dir("writer-switch-epoch-overflow");
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let mut runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
    runtime.guard = SingleWriterReplicationGuard {
        writer_id: Some("node-b".to_string()),
        writer_epoch: u64::MAX,
        last_sequence: 8,
    };
    runtime.writer_state = LocalWriterState {
        writer_epoch: u64::MAX,
        last_sequence: 12,
        last_replicated_height: 0,
    };

    let err = runtime
        .next_local_record_position("node-a")
        .expect_err("writer epoch overflow should fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("writer_epoch overflow"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn next_local_record_position_rejects_sequence_overflow_without_guard_writer() {
    let dir = temp_dir("no-guard-sequence-overflow");
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let mut runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
    runtime.guard = SingleWriterReplicationGuard {
        writer_id: None,
        writer_epoch: DEFAULT_WRITER_EPOCH,
        last_sequence: 0,
    };
    runtime.writer_state = LocalWriterState {
        writer_epoch: 19,
        last_sequence: u64::MAX,
        last_replicated_height: 0,
    };

    let err = runtime
        .next_local_record_position("node-a")
        .expect_err("sequence overflow should fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("sequence overflow"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_remote_message_for_observe_rejects_writer_outside_allowlist() {
    let dir = temp_dir("allowlist-reject");
    let (local_private_hex, local_public_hex) = deterministic_keypair_hex(21);
    let (_, allowed_public_hex) = deterministic_keypair_hex(22);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(local_private_hex, local_public_hex)
        .expect("signing keypair")
        .with_remote_writer_allowlist(vec![allowed_public_hex])
        .expect("allowlist");
    let runtime = ReplicationRuntime::new(&config, "node-b").expect("runtime");
    let unauthorized_message = signed_remote_message(23, "world-allowlist", "node-a", 1);

    let err = runtime
        .validate_remote_message_for_observe("node-b", "world-allowlist", &unauthorized_message)
        .expect_err("unauthorized writer should fail");
    assert!(matches!(err, NodeError::Replication { reason } if reason.contains("not authorized")));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_remote_message_for_observe_accepts_writer_in_allowlist() {
    let dir = temp_dir("allowlist-accept");
    let (local_private_hex, local_public_hex) = deterministic_keypair_hex(31);
    let (_, allowed_public_hex) = deterministic_keypair_hex(32);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(local_private_hex, local_public_hex)
        .expect("signing keypair")
        .with_remote_writer_allowlist(vec![allowed_public_hex])
        .expect("allowlist");
    let runtime = ReplicationRuntime::new(&config, "node-b").expect("runtime");
    let allowed_message = signed_remote_message(32, "world-allowlist", "node-a", 1);

    let accepted = runtime
        .validate_remote_message_for_observe("node-b", "world-allowlist", &allowed_message)
        .expect("authorized writer should pass");
    assert!(accepted);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_fetch_commit_request_signs_with_runtime_signer() {
    let dir = temp_dir("fetch-commit-sign");
    let (local_private_hex, local_public_hex) = deterministic_keypair_hex(41);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(local_private_hex, local_public_hex.clone())
        .expect("signing keypair")
        .with_remote_writer_allowlist(vec![local_public_hex.clone()])
        .expect("allowlist");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let request = runtime
        .build_fetch_commit_request("world-fetch-sign", 7)
        .expect("build request");
    assert_eq!(
        request.requester_public_key_hex.as_deref(),
        Some(local_public_hex.as_str())
    );
    assert!(request.requester_signature_hex.is_some());
    config
        .authorize_fetch_commit_request(&request)
        .expect("signed request should pass authorization");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn authorize_fetch_commit_request_rejects_missing_signature_when_required() {
    let dir = temp_dir("fetch-commit-missing-signature");
    let (local_private_hex, local_public_hex) = deterministic_keypair_hex(42);
    let (_, allowed_public_hex) = deterministic_keypair_hex(43);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(local_private_hex, local_public_hex)
        .expect("signing keypair")
        .with_remote_writer_allowlist(vec![allowed_public_hex.clone()])
        .expect("allowlist");
    let request = FetchCommitRequest {
        world_id: "world-fetch-sign".to_string(),
        height: 9,
        requester_public_key_hex: Some(allowed_public_hex),
        requester_signature_hex: None,
    };

    let err = config
        .authorize_fetch_commit_request(&request)
        .expect_err("unsigned request should fail");
    assert!(matches!(
        err,
        NodeError::Replication { reason }
            if reason.contains("missing requester_signature_hex")
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn authorize_fetch_blob_request_rejects_requester_outside_allowlist() {
    let dir = temp_dir("fetch-blob-allowlist");
    let (local_private_hex, local_public_hex) = deterministic_keypair_hex(44);
    let (_, allowed_public_hex) = deterministic_keypair_hex(45);
    let (requester_private_hex, requester_public_hex) = deterministic_keypair_hex(46);
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_signing_keypair(local_private_hex, local_public_hex)
        .expect("signing keypair")
        .with_remote_writer_allowlist(vec![allowed_public_hex])
        .expect("allowlist");
    let signer = ReplicationSigningKey {
        signing_key: signing_key_from_hex(requester_private_hex.as_str()).expect("signing key"),
        public_key_hex: requester_public_hex.clone(),
    };
    let mut request = FetchBlobRequest {
        content_hash: "hash-1".to_string(),
        requester_public_key_hex: Some(requester_public_hex),
        requester_signature_hex: None,
    };
    request.requester_signature_hex =
        Some(sign_fetch_blob_request(&request, &signer).expect("sign"));

    let err = config
        .authorize_fetch_blob_request(&request)
        .expect_err("out-of-allowlist requester should fail");
    assert!(matches!(err, NodeError::Replication { reason } if reason.contains("not authorized")));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_commit_message_by_height_reads_from_cold_index_after_hot_prune() {
    let dir = temp_dir("commit-cold-index");
    let world_id = "world-commit-cold-index";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let message_1 = signed_remote_message(61, world_id, "node-b", 1);
    let message_2 = signed_remote_message(62, world_id, "node-b", 2);
    let message_3 = signed_remote_message(63, world_id, "node-b", 3);
    runtime
        .persist_commit_message(1, &message_1)
        .expect("persist message 1");
    runtime
        .persist_commit_message(2, &message_2)
        .expect("persist message 2");
    runtime
        .persist_commit_message(3, &message_3)
        .expect("persist message 3");

    let hot_1 = config.commit_message_path(1);
    let hot_2 = config.commit_message_path(2);
    let hot_3 = config.commit_message_path(3);
    assert!(
        !hot_1.exists(),
        "oldest hot commit should be pruned after exceeding cap"
    );
    assert!(hot_2.exists(), "recent commit 2 should remain hot");
    assert!(hot_3.exists(), "recent commit 3 should remain hot");

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    assert!(
        cold_index.by_height.contains_key(&1),
        "cold index should contain pruned height"
    );

    let retention_plan =
        build_commit_message_retention_plan(dir.as_path(), 2).expect("retention plan");
    assert_eq!(retention_plan.hot_window.latest_height, Some(3));
    assert_eq!(retention_plan.hot_window.hot_window_start_height, Some(2));

    let loaded_1 = runtime
        .load_commit_message_by_height(world_id, 1)
        .expect("load commit height 1")
        .expect("cold commit height 1 should exist");
    assert_eq!(loaded_1.record.content_hash, message_1.record.content_hash);
    assert_eq!(loaded_1.world_id, world_id);

    let loaded_2 = runtime
        .load_commit_message_by_height(world_id, 2)
        .expect("load commit height 2")
        .expect("hot commit height 2 should exist");
    assert_eq!(loaded_2.record.content_hash, message_2.record.content_hash);

    let loaded_3 = runtime
        .load_commit_message_by_height(world_id, 3)
        .expect("load commit height 3")
        .expect("hot commit height 3 should exist");
    assert_eq!(loaded_3.record.content_hash, message_3.record.content_hash);

    let hot_bytes = std::fs::read(&hot_3).expect("read hot commit json");
    assert_eq!(
        hot_bytes.iter().filter(|byte| **byte == b'\n').count(),
        0,
        "hot commit mirror should be written without pretty-print newlines"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn commit_cold_index_uses_canonical_layout_and_refreshes_hot_range() {
    let dir = temp_dir("commit-cold-index-layout");
    let world_id = "world-commit-cold-index-layout";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    runtime
        .persist_commit_message(1, &signed_remote_message(81, world_id, "node-b", 1))
        .expect("persist message 1");
    runtime
        .persist_commit_message(100, &signed_remote_message(82, world_id, "node-b", 100))
        .expect("persist message 100");
    runtime
        .persist_commit_message(101, &signed_remote_message(83, world_id, "node-b", 101))
        .expect("persist message 101");

    let canonical_path = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join(STORAGE_COLD_INDEX_MANIFEST_FILE);
    assert!(
        canonical_path.exists(),
        "canonical cold index manifest should exist"
    );
    assert!(
        dir.join("replication_commit_messages_cold_index.json")
            .exists(),
        "legacy cold index alias should remain available during protocol rollout"
    );

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    assert_eq!(cold_index.manifest.namespace, COMMIT_MESSAGE_DIR);
    assert_eq!(cold_index.manifest.key_kind, "height");
    assert_eq!(
        cold_index.manifest.value_kind,
        STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF
    );
    assert_eq!(
        cold_index.manifest.hot_range,
        Some(oasis7_proto::storage_cold_index::StorageColdIndexRange {
            from_key: 100,
            to_key: 101,
        })
    );
    assert_eq!(
        cold_index.manifest.cold_range_anchor,
        Some(
            oasis7_proto::storage_cold_index::StorageColdIndexRangeAnchor {
                from_key: 1,
                to_key: 1,
                first_content_hash: cold_entry_content_hash(&cold_index, 1),
                last_content_hash: cold_entry_content_hash(&cold_index, 1),
                entry_count: 1,
            }
        )
    );

    let canonical_bytes = std::fs::read(&canonical_path).expect("read canonical cold index");
    assert!(
        !canonical_bytes.contains(&b'\n'),
        "cold index manifest should be written without pretty-print newlines"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn commit_cold_index_packs_multiple_heights_into_shared_segment_file() {
    let dir = temp_dir("commit-cold-pack-shared-segment");
    let world_id = "world-commit-cold-pack-shared-segment";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    for height in 1..=4 {
        runtime
            .persist_commit_message(
                height,
                &signed_remote_message(120 + height as u8, world_id, "node-b", height),
            )
            .expect("persist packed commit message");
    }

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    let entry_1 = cold_index.by_height.get(&1).expect("height 1");
    let entry_2 = cold_index.by_height.get(&2).expect("height 2");
    match (entry_1, entry_2) {
        (
            commit_retention::CommitMessageColdEntry::PackRef(pack_1),
            commit_retention::CommitMessageColdEntry::PackRef(pack_2),
        ) => {
            assert_eq!(
                pack_1.segment_id, pack_2.segment_id,
                "nearby cold heights should share one pack segment"
            );
            assert!(
                pack_2.offset > pack_1.offset,
                "later packed commit should append after earlier one"
            );
            let segment_path = dir
                .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
                .join("segments")
                .join(format!("{}.pack", pack_1.segment_id));
            assert!(segment_path.exists(), "pack segment should exist");
        }
        _ => panic!("cold entries should be rewritten as pack refs"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prune_hot_commit_messages_sweeps_existing_orphan_blobs_after_offload() {
    let dir = temp_dir("commit-prune-orphan-sweep");
    let world_id = "world-commit-prune-orphan-sweep";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let message_1 = signed_remote_message(71, world_id, "node-b", 1);
    let message_2 = signed_remote_message(72, world_id, "node-b", 2);
    runtime
        .persist_commit_message(1, &message_1)
        .expect("persist message 1");
    runtime
        .persist_commit_message(2, &message_2)
        .expect("persist message 2");

    let legacy_bytes = serde_json::to_vec_pretty(&message_1).expect("legacy pretty payload");
    let legacy_hash = runtime
        .store
        .put_bytes(legacy_bytes.as_slice())
        .expect("store orphan payload");
    let legacy_blob_path = dir
        .join("store")
        .join("blobs")
        .join(format!("{legacy_hash}.blob"));
    assert!(
        legacy_blob_path.exists(),
        "legacy orphan blob should exist before sweep"
    );

    runtime
        .persist_commit_message(3, &signed_remote_message(73, world_id, "node-b", 3))
        .expect("persist message 3");

    assert!(
        !legacy_blob_path.exists(),
        "hot-window offload should opportunistically prune legacy orphan blobs"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_commit_message_by_height_migrates_legacy_only_cold_index_to_canonical_layout() {
    let dir = temp_dir("commit-cold-index-legacy-only");
    let world_id = "world-commit-cold-index-legacy-only";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let message_1 = signed_remote_message(91, world_id, "node-b", 1);
    let message_2 = signed_remote_message(92, world_id, "node-b", 2);
    let message_1_bytes = serde_json::to_vec(&message_1).expect("serialize message 1");
    let message_2_bytes = serde_json::to_vec(&message_2).expect("serialize message 2");
    let hash_1 = runtime
        .store
        .put_bytes(message_1_bytes.as_slice())
        .expect("store legacy message 1");
    let hash_2 = runtime
        .store
        .put_bytes(message_2_bytes.as_slice())
        .expect("store legacy message 2");
    let legacy_blob_1 = runtime.store.blobs_dir().join(format!("{hash_1}.blob"));
    let legacy_blob_2 = runtime.store.blobs_dir().join(format!("{hash_2}.blob"));
    assert!(legacy_blob_1.exists(), "legacy blob 1 should exist");
    assert!(legacy_blob_2.exists(), "legacy blob 2 should exist");

    let legacy_index_path = dir.join("replication_commit_messages_cold_index.json");
    std::fs::write(
        &legacy_index_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "namespace": COMMIT_MESSAGE_DIR,
            "key_kind": "height",
            "value_kind": "content_hash",
            "by_height": {
                "1": hash_1,
                "2": hash_2,
            }
        }))
        .expect("serialize legacy cold index"),
    )
    .expect("write legacy cold index");
    let canonical_dir = dir.join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR));
    assert!(
        !canonical_dir.exists(),
        "canonical cold index dir should not exist before migration"
    );

    let loaded_1 = runtime
        .load_commit_message_by_height(world_id, 1)
        .expect("load commit height 1")
        .expect("cold commit height 1 should exist");
    assert_eq!(loaded_1.record.content_hash, message_1.record.content_hash);
    assert!(
        canonical_dir
            .join(STORAGE_COLD_INDEX_MANIFEST_FILE)
            .exists(),
        "legacy-only cold index should backfill canonical manifest on read"
    );
    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    assert_eq!(
        cold_index.manifest.value_kind,
        STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF
    );
    assert!(
        matches!(
            cold_index.by_height.get(&1),
            Some(commit_retention::CommitMessageColdEntry::PackRef(_))
        ),
        "legacy hash entry should migrate into pack ref"
    );
    assert!(
        !legacy_blob_1.exists() && !legacy_blob_2.exists(),
        "migrated legacy CAS blobs should be deleted once pack refs are persisted"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_commit_message_cold_index_restores_compat_alias_from_canonical_manifest() {
    let dir = temp_dir("commit-cold-index-restore-compat");
    let world_id = "world-commit-cold-index-restore-compat";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    runtime
        .persist_commit_message(1, &signed_remote_message(94, world_id, "node-b", 1))
        .expect("persist message 1");
    runtime
        .persist_commit_message(100, &signed_remote_message(95, world_id, "node-b", 100))
        .expect("persist message 100");

    let compat_alias_path = dir.join("replication_commit_messages_cold_index.json");
    std::fs::remove_file(&compat_alias_path).expect("remove compat alias");
    assert!(
        !compat_alias_path.exists(),
        "compat alias should be removed for test"
    );

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    assert_eq!(cold_index.manifest.namespace, COMMIT_MESSAGE_DIR);
    assert!(
        compat_alias_path.exists(),
        "canonical load should restore compat alias"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_commit_message_by_height_rejects_invalid_pack_segment_id() {
    let dir = temp_dir("commit-cold-pack-invalid-segment-id");
    let world_id = "world-commit-cold-pack-invalid-segment-id";
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let cold_index_path = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join(STORAGE_COLD_INDEX_MANIFEST_FILE);
    std::fs::create_dir_all(cold_index_path.parent().expect("cold index parent"))
        .expect("create cold index dir");
    std::fs::write(
        &cold_index_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "namespace": COMMIT_MESSAGE_DIR,
            "key_kind": "height",
            "value_kind": STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF,
            "by_height": {
                "1": {
                    "segment_id": "../escape",
                    "offset": 0,
                    "len": 16,
                    "content_hash": "deadbeef"
                }
            }
        }))
        .expect("serialize invalid cold index"),
    )
    .expect("write invalid cold index");

    let err = runtime
        .load_commit_message_by_height(world_id, 1)
        .expect_err("invalid segment id should fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("invalid commit message pack segment id"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_commit_message_by_height_rejects_out_of_bounds_pack_entry() {
    let dir = temp_dir("commit-cold-pack-out-of-bounds");
    let world_id = "world-commit-cold-pack-out-of-bounds";
    let config = NodeReplicationConfig::new(&dir).expect("config");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let segment_id = "00000000000000000001-00000000000000000256";
    let segment_dir = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join("segments");
    std::fs::create_dir_all(&segment_dir).expect("create segments dir");
    std::fs::write(segment_dir.join(format!("{segment_id}.pack")), [0u8; 8])
        .expect("write short pack file");

    let cold_index_path = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join(STORAGE_COLD_INDEX_MANIFEST_FILE);
    std::fs::write(
        &cold_index_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "namespace": COMMIT_MESSAGE_DIR,
            "key_kind": "height",
            "value_kind": STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF,
            "by_height": {
                "1": {
                    "segment_id": segment_id,
                    "offset": 0,
                    "len": 32,
                    "content_hash": "deadbeef"
                }
            }
        }))
        .expect("serialize out-of-bounds cold index"),
    )
    .expect("write out-of-bounds cold index");

    let err = runtime
        .load_commit_message_by_height(world_id, 1)
        .expect_err("out-of-bounds pack entry should fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("pack entry out of bounds"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn commit_cold_index_scan_anchor_matches_readback_boundaries() {
    let dir = temp_dir("commit-cold-index-scan-anchor");
    let world_id = "world-commit-cold-index-scan-anchor";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let mut messages = Vec::new();
    for height in 1..=5 {
        let message = signed_remote_message(100 + height as u8, world_id, "node-b", height);
        runtime
            .persist_commit_message(height, &message)
            .expect("persist commit message");
        messages.push(message);
    }

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    let anchor = cold_index
        .manifest
        .cold_range_anchor
        .clone()
        .expect("cold range anchor");
    assert_eq!(anchor.from_key, 1);
    assert_eq!(anchor.to_key, 3);
    assert_eq!(anchor.entry_count, 3);

    let first_cold = runtime
        .load_commit_message_by_height(world_id, anchor.from_key)
        .expect("load first cold height")
        .expect("first cold commit exists");
    let last_cold = runtime
        .load_commit_message_by_height(world_id, anchor.to_key)
        .expect("load last cold height")
        .expect("last cold commit exists");
    assert_eq!(
        cold_entry_content_hash(&cold_index, anchor.from_key),
        anchor.first_content_hash
    );
    assert_eq!(
        cold_entry_content_hash(&cold_index, anchor.to_key),
        anchor.last_content_hash
    );
    assert_eq!(
        first_cold.record.content_hash,
        messages[0].record.content_hash
    );
    assert_eq!(
        last_cold.record.content_hash,
        messages[2].record.content_hash
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prune_hot_commit_messages_uses_latest_height_window_range() {
    let dir = temp_dir("commit-hot-window-range");
    let world_id = "world-commit-hot-window-range";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    let message_1 = signed_remote_message(71, world_id, "node-b", 1);
    let message_100 = signed_remote_message(72, world_id, "node-b", 100);
    runtime
        .persist_commit_message(1, &message_1)
        .expect("persist message 1");
    runtime
        .persist_commit_message(100, &message_100)
        .expect("persist message 100");

    assert!(
        !config.commit_message_path(1).exists(),
        "sparse height outside latest-based hot window should be offloaded"
    );
    assert!(
        config.commit_message_path(100).exists(),
        "latest height should remain in hot mirror"
    );

    let retention_plan =
        build_commit_message_retention_plan(dir.as_path(), 2).expect("retention plan");
    assert_eq!(retention_plan.hot_window.latest_height, Some(100));
    assert_eq!(retention_plan.hot_window.hot_window_start_height, Some(99));

    let cold_index = load_commit_message_cold_index_from_root(dir.as_path()).expect("cold index");
    assert!(
        cold_index.by_height.contains_key(&1),
        "cold index should retain offloaded sparse height"
    );

    let loaded_1 = runtime
        .load_commit_message_by_height(world_id, 1)
        .expect("load commit height 1")
        .expect("cold commit height 1 should exist");
    assert_eq!(loaded_1.record.content_hash, message_1.record.content_hash);

    let loaded_100 = runtime
        .load_commit_message_by_height(world_id, 100)
        .expect("load commit height 100")
        .expect("hot commit height 100 should exist");
    assert_eq!(
        loaded_100.record.content_hash,
        message_100.record.content_hash
    );

    let _ = std::fs::remove_dir_all(&dir);
}
