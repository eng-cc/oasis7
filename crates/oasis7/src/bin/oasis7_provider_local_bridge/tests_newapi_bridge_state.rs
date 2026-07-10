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
