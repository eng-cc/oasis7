use super::*;

#[test]
fn cached_newapi_bridge_state_reuses_arc_for_unchanged_file() {
    let _guard = newapi_bridge_state_env_guard();
    let state_path = std::env::temp_dir().join(format!(
        "oasis7-provider-bridge-cache-state-{}.json",
        std::process::id()
    ));
    fs::write(state_path.as_path(), br#"{"bindings":[]}"#).expect("write bridge state");
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(
            "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH",
            state_path.as_os_str(),
        );
    }

    let cache = Arc::new(Mutex::new(NewapiBridgeStateCache::default()));
    let first = load_cached_newapi_bridge_state(&cache).expect("first cached bridge state");
    let second = load_cached_newapi_bridge_state(&cache).expect("second cached bridge state");

    assert_eq!(first.as_ref(), second.as_ref());
    assert!(Arc::ptr_eq(&first, &second));

    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var("OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH");
    }
    let _ = fs::remove_file(state_path);
}

#[test]
fn cached_newapi_bridge_state_reloads_replaced_content_with_same_mtime() {
    let _guard = newapi_bridge_state_env_guard();
    let state_path = std::env::temp_dir().join(format!(
        "oasis7-provider-bridge-cache-same-mtime-state-{}.json",
        std::process::id()
    ));
    fs::write(state_path.as_path(), br#"{"authorization":"first"}"#)
        .expect("write initial bridge state");
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(
            "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH",
            state_path.as_os_str(),
        );
    }

    let cache = Arc::new(Mutex::new(NewapiBridgeStateCache::default()));
    let first = load_cached_newapi_bridge_state(&cache).expect("load initial bridge state");
    assert_eq!(first["authorization"], "first");
    let initial_modified_at = fs::metadata(state_path.as_path())
        .expect("read initial bridge state metadata")
        .modified()
        .expect("read initial bridge state mtime");

    fs::write(state_path.as_path(), br#"{"authorization":"replacement"}"#)
        .expect("replace bridge state");
    fs::File::open(state_path.as_path())
        .expect("open replacement bridge state")
        .set_times(fs::FileTimes::new().set_modified(initial_modified_at))
        .expect("restore replacement bridge state mtime");
    assert_eq!(
        fs::metadata(state_path.as_path())
            .expect("read replacement bridge state metadata")
            .modified()
            .expect("read replacement bridge state mtime"),
        initial_modified_at,
        "replacement must have the same mtime as the cached state"
    );

    let refreshed =
        load_cached_newapi_bridge_state(&cache).expect("reload replacement bridge state");
    assert_eq!(refreshed["authorization"], "replacement");

    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var("OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH");
    }
    let _ = fs::remove_file(state_path);
}
