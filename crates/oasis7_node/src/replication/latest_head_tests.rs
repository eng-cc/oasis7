use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn latest_commit_head_does_not_read_stale_hot_files() {
    let dir = temp_dir("latest-head-o1-stale-hot");
    let world_id = "world-latest-head-o1-stale-hot";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(4096)
        .expect("hot cap");
    let _runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
    std::fs::create_dir_all(
        config
            .commit_message_path(1)
            .parent()
            .expect("commit directory"),
    )
    .expect("create commit directory");
    let stale = signed_remote_message(121, world_id, "node-b", 4095);
    let stale_bytes = serde_json::to_vec(&stale).expect("encode stale commit");
    let template = dir.join("hot-template.json");
    std::fs::write(&template, &stale_bytes).expect("write hot template");
    for height in 1..=4094 {
        std::fs::hard_link(&template, config.commit_message_path(height)).expect("link hot commit");
    }
    std::fs::write(config.commit_message_path(4095), stale_bytes).expect("write stale commit");
    let latest = signed_remote_message(122, world_id, "node-b", 4096);
    std::fs::write(
        config.commit_message_path(4096),
        serde_json::to_vec(&latest).expect("encode latest commit"),
    )
    .expect("write latest commit");
    let stale_path = config.commit_message_path(4095);
    let mut permissions = std::fs::metadata(&stale_path)
        .expect("stale metadata")
        .permissions();
    permissions.set_mode(0);
    std::fs::set_permissions(&stale_path, permissions).expect("deny stale read");

    let lookups: Vec<_> = (0..4)
        .map(|_| load_latest_commit_message_from_root(&dir, world_id, 4096))
        .collect();

    let mut restore = std::fs::metadata(&stale_path)
        .expect("stale metadata restore")
        .permissions();
    restore.set_mode(0o600);
    let _ = std::fs::set_permissions(&stale_path, restore);
    let _ = std::fs::remove_dir_all(&dir);

    for lookup in lookups {
        let latest = lookup
            .expect("latest-head lookup should not rescan stale hot payloads")
            .expect("latest commit");
        assert_eq!(latest.record.sequence, 4096);
    }
}

#[test]
fn startup_reconciles_stale_latest_head_index_before_fetch_head() {
    let dir = temp_dir("latest-head-stale-index-recovery");
    let world_id = "world-latest-head-stale-index-recovery";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(4096)
        .expect("hot cap");
    let first_path = config.commit_message_path(1);
    let commit_dir = first_path.parent().expect("commit directory");
    std::fs::create_dir_all(commit_dir).expect("create commit directory");
    let first = signed_remote_message(131, world_id, "node-b", 1);
    let first_bytes = serde_json::to_vec(&first).expect("encode first commit");
    std::fs::write(config.commit_message_path(1), &first_bytes).expect("write first commit");
    let index_dir = dir.join("replication_commit_heads");
    std::fs::create_dir_all(&index_dir).expect("create latest-head index directory");
    let index_path = index_dir.join(format!(
        "{}.json",
        oasis7_distfs::blake3_hex(world_id.as_bytes())
    ));
    let stale_index = serde_json::json!({
        "schema": 1,
        "world_id": world_id,
        "height": 1,
        "record_content_hash": first.record.content_hash,
        "message_hash": oasis7_distfs::blake3_hex(&first_bytes),
    });
    std::fs::write(
        &index_path,
        serde_json::to_vec(&stale_index).expect("encode stale index"),
    )
    .expect("write stale index");
    let second = signed_remote_message(132, world_id, "node-b", 2);
    std::fs::write(
        config.commit_message_path(2),
        serde_json::to_vec(&second).expect("encode newer commit"),
    )
    .expect("persist newer commit mirror");

    let restarted = ReplicationRuntime::new(&config, "node-a").expect("restart runtime");
    let fetch_head = load_latest_commit_message_from_root(&dir, world_id, 4096);
    drop(restarted);
    let _ = std::fs::remove_dir_all(&dir);

    let fetch_head = fetch_head
        .expect("FetchHead latest lookup should reconcile stale index")
        .expect("newest commit");
    assert_eq!(fetch_head.record.sequence, 2);
}

#[test]
fn persist_commit_message_rejects_same_height_hot_or_cold_binding_without_index() {
    for (suffix, offload_existing) in [("hot", false), ("cold", true)] {
        let dir = temp_dir(format!("latest-head-same-height-conflict-{suffix}").as_str());
        let world_id = format!("world-latest-head-same-height-conflict-{suffix}");
        let config = NodeReplicationConfig::new(&dir)
            .expect("config")
            .with_max_hot_commit_messages(if offload_existing { 1 } else { 4096 })
            .expect("hot cap");
        let runtime = ReplicationRuntime::new(&config, "node-a").expect("runtime");
        let existing = signed_remote_message(141, world_id.as_str(), "node-b", 1);
        runtime
            .persist_commit_message(1, &existing)
            .expect("persist existing commit");
        if offload_existing {
            runtime
                .persist_commit_message(
                    2,
                    &signed_remote_message(142, world_id.as_str(), "node-b", 2),
                )
                .expect("persist newer commit");
            assert!(
                !config.commit_message_path(1).exists(),
                "height one should be retained only in the cold mirror"
            );
        }

        let index_path = dir.join("replication_commit_heads").join(format!(
            "{}.json",
            oasis7_distfs::blake3_hex(world_id.as_bytes())
        ));
        std::fs::remove_file(&index_path).expect("remove latest-head index");
        let replacement = signed_remote_message(143, world_id.as_str(), "node-b", 1);
        let err = runtime
            .persist_commit_message(1, &replacement)
            .expect_err("same-height binding conflict must fail before overwrite");
        assert!(
            matches!(err, NodeError::Replication { ref reason } if reason.contains("latest commit head conflict")),
            "unexpected same-height conflict error: {err:?}"
        );
        let persisted = load_commit_message_from_root(&dir, world_id.as_str(), 1)
            .expect("read existing commit")
            .expect("existing commit");
        assert_eq!(persisted.record.content_hash, existing.record.content_hash);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn startup_reconstructs_missing_world_index_in_shared_replication_root() {
    let dir = temp_dir("latest-head-shared-root-missing-world-index");
    let world_a = "world-latest-head-shared-root-a";
    let world_b = "world-latest-head-shared-root-b";
    let config = NodeReplicationConfig::new(&dir)
        .expect("config")
        .with_max_hot_commit_messages(4096)
        .expect("hot cap");
    let first_commit_path = config.commit_message_path(1);
    let commit_dir = first_commit_path.parent().expect("commit directory");
    std::fs::create_dir_all(commit_dir).expect("create commit directory");
    let first_a = signed_remote_message(151, world_a, "node-b", 1);
    let first_b = signed_remote_message(152, world_b, "node-b", 2);
    let first_a_bytes = serde_json::to_vec(&first_a).expect("encode world a commit");
    std::fs::write(config.commit_message_path(1), &first_a_bytes).expect("write world a commit");
    let index_dir = dir.join("replication_commit_heads");
    std::fs::create_dir_all(&index_dir).expect("create latest-head index directory");
    let index_path = index_dir.join(format!(
        "{}.json",
        oasis7_distfs::blake3_hex(world_a.as_bytes())
    ));
    let index = serde_json::json!({
        "schema": 1,
        "world_id": world_a,
        "height": 1,
        "record_content_hash": first_a.record.content_hash,
        "message_hash": oasis7_distfs::blake3_hex(&first_a_bytes),
    });
    std::fs::write(
        &index_path,
        serde_json::to_vec(&index).expect("encode world a index"),
    )
    .expect("write world a index");
    let world_b_commit_path = config.commit_message_path(2);
    std::fs::write(
        &world_b_commit_path,
        serde_json::to_vec(&first_b).expect("encode world b commit mirror"),
    )
    .expect("write world b commit mirror");
    let _runtime = ReplicationRuntime::new(&config, "node-a").expect("startup runtime");
    let world_b_index_path = index_dir.join(format!(
        "{}.json",
        oasis7_distfs::blake3_hex(world_b.as_bytes())
    ));
    assert!(
        world_b_index_path.exists(),
        "startup should reconstruct every world index present in a shared root"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
