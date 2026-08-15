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
