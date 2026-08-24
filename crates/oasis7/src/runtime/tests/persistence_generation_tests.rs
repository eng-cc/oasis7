use super::*;

#[test]
fn load_from_dir_uses_tick_archive_bound_to_selected_sidecar_generation() {
    let mut older_world = World::new();
    for _ in 0..140 {
        older_world.step().expect("older step");
    }

    let mut newer_world = World::new();
    for _ in 0..141 {
        newer_world.step().expect("newer step");
    }

    let dir = temp_dir("persist-generation-bound-tick-archive");
    older_world
        .save_to_dir(&dir)
        .expect("save older generation");
    let index_path = dir
        .join(".distfs-state")
        .join("sidecar-generations")
        .join("index.json");
    let older_generation_id = serde_json::from_slice::<serde_json::Value>(
        &fs::read(&index_path).expect("read older generation index"),
    )
    .expect("decode older generation index")["latest_generation"]
        .as_str()
        .expect("older generation id")
        .to_string();

    newer_world
        .save_to_dir(&dir)
        .expect("save newer generation");

    // Simulate a rollback selector choosing the older generation while the
    // directory-level archive still contains the newer world's records.
    let mut index: serde_json::Value =
        serde_json::from_slice(&fs::read(&index_path).expect("read generation index"))
            .expect("decode generation index");
    index["latest_generation"] = serde_json::Value::String(older_generation_id);
    fs::write(
        &index_path,
        serde_json::to_vec_pretty(&index).expect("encode generation index"),
    )
    .expect("select older generation");

    let restored = World::load_from_dir(&dir).expect("restore selected generation");
    assert_eq!(
        restored.tick_consensus_records(),
        older_world.tick_consensus_records()
    );
    restored
        .verify_tick_consensus_chain()
        .expect("verify selected generation tick consensus chain");

    let _ = fs::remove_dir_all(&dir);
}
