fn main() {
    let commands = &[
        "open_project",
        "query_view",
        "search_rules",
        "save_decision_draft",
        "save_preference",
        "read_source",
        "refresh_source",
        "rebuild_cache",
        "refresh_github",
        "get_github_snapshot",
        "open_source_target",
        "choose_project",
        "associate_work",
        "read_version",
        "get_runtime",
        "reconcile_runtime_operation",
        "set_runtime_enabled",
        "connect_runtime",
        "create_runtime_session",
        "continue_runtime_session",
        "start_runtime_turn",
        "steer_runtime_turn",
        "interrupt_runtime_turn",
        "answer_runtime_request",
        "get_exit_status",
        "request_exit_wait",
        "request_exit_stop",
        "cancel_exit_wait",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("build desktop command manifest");
}
