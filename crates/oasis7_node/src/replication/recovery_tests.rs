use super::commit_retention::{self, CommitMessageColdIndex};
use super::*;
use oasis7_proto::storage_cold_index::{
    storage_cold_index_dir_name, STORAGE_COLD_INDEX_MANIFEST_FILE,
};
use std::path::PathBuf;

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oasis7-replication-recovery-tests-{prefix}-{unique}"
    ))
}

fn signed_remote_message(
    seed: u8,
    world_id: &str,
    node_id: &str,
    sequence: u64,
) -> GossipReplicationMessage {
    let bytes = [seed; 32];
    let signing_key = SigningKey::from_bytes(&bytes);
    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(signing_key.verifying_key().to_bytes());
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

#[test]
fn cold_index_alias_recovery_does_not_prune_newer_pack_segments() {
    let dir = temp_dir("cold-index-alias-recovery-no-prune");
    let world_id = "world-cold-index-alias-recovery-no-prune";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(2)
        .expect("hot commit cap");
    let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");

    for (seed, height) in [(100, 1), (101, 300)] {
        runtime
            .persist_commit_message(
                height,
                &signed_remote_message(seed, world_id, "node-b", height),
            )
            .expect("persist initial message");
    }
    let compat_alias_path = dir.join("replication_commit_messages_cold_index.json");
    let stale_alias = std::fs::read(&compat_alias_path).expect("read stale compat alias");

    for (seed, height) in [(102, 301), (103, 302)] {
        runtime
            .persist_commit_message(
                height,
                &signed_remote_message(seed, world_id, "node-b", height),
            )
            .expect("persist later message");
    }

    let canonical_path = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join(STORAGE_COLD_INDEX_MANIFEST_FILE);
    let canonical_before =
        load_json_or_default::<CommitMessageColdIndex>(&canonical_path).expect("canonical before");
    let newer_pack = match canonical_before
        .by_height
        .get(&300)
        .expect("newer cold height")
    {
        commit_retention::CommitMessageColdEntry::PackRef(pack_ref) => pack_ref,
        _ => panic!("newer cold height should be packed"),
    };
    let newer_segment_path = dir
        .join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
        .join("segments")
        .join(format!("{}.pack", newer_pack.segment_id));
    assert!(
        newer_segment_path.exists(),
        "newer pack segment should exist"
    );

    std::fs::write(&compat_alias_path, stale_alias).expect("restore stale compat alias");
    std::fs::write(&canonical_path, b"{\"by_height\":{\"1\"").expect("truncate canonical");
    let recovered = load_commit_message_cold_index_from_root(dir.as_path())
        .expect("recover from stale compat alias");

    assert!(
        recovered.by_height.contains_key(&1),
        "stale alias should recover height 1"
    );
    assert!(
        !recovered.by_height.contains_key(&300),
        "stale alias should not prove height 300"
    );
    assert!(
        newer_segment_path.exists(),
        "alias recovery must not prune pack segments that the stale alias cannot reference"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
