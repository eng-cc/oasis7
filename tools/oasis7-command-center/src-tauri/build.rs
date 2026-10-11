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
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("build desktop command manifest");
}
