use super::*;

#[test]
fn resolve_explorer_my_account_candidate_prefers_transfer_sender() {
    assert_eq!(
        resolve_explorer_my_account_candidate("player-a", "player-b", "player-c"),
        Some("player-a".to_string())
    );
    assert_eq!(
        resolve_explorer_my_account_candidate("", "player-b", "player-c"),
        Some("player-b".to_string())
    );
    assert_eq!(
        resolve_explorer_my_account_candidate("", "", "player-c"),
        Some("player-c".to_string())
    );
    assert_eq!(resolve_explorer_my_account_candidate("", "", ""), None);
}

#[test]
fn explorer_quick_shortcut_recent_txs_resets_filters_and_refreshes() {
    let mut app = ClientLauncherApp::default();
    app.explorer_panel_state.account_filter = "player-a".to_string();
    app.explorer_panel_state.action_filter_input = "42".to_string();
    app.explorer_panel_state.status_filter = ExplorerStatusFilter::Failed;
    app.explorer_panel_state.txs_cursor = 20;

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::RecentTxs);

    assert!(app.explorer_panel_state.account_filter.is_empty());
    assert!(app.explorer_panel_state.action_filter_input.is_empty());
    assert_eq!(
        app.explorer_panel_state.status_filter,
        ExplorerStatusFilter::All
    );
    assert_eq!(app.explorer_panel_state.txs_cursor, 0);
    assert!(app.explorer_panel_state.pending_txs_refresh);
}

#[test]
fn explorer_quick_shortcut_my_account_logs_when_missing_candidate() {
    let mut app = ClientLauncherApp::default();
    app.ui_language = UiLanguage::EnUs;
    let logs_before = app.logs.len();

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::MyAccount);

    assert_eq!(app.logs.len(), logs_before + 1);
    let latest_log = app.logs.back().expect("latest log should exist");
    assert!(latest_log.contains("My Account shortcut is unavailable"));
}

#[test]
fn explorer_quick_shortcut_latest_block_prefills_height_from_overview() {
    let mut app = ClientLauncherApp::default();
    app.explorer_panel_state.overview = Some(WebExplorerOverviewResponse {
        ok: true,
        observed_at_unix_ms: 1,
        node_id: "node-a".to_string(),
        world_id: "world-a".to_string(),
        latest_height: 88,
        committed_height: 88,
        network_committed_height: 88,
        last_block_hash: Some("hash-a".to_string()),
        last_execution_block_hash: Some("hash-b".to_string()),
        tracked_records: 0,
        transfer_total: 0,
        transfer_accepted: 0,
        transfer_pending: 0,
        transfer_confirmed: 0,
        transfer_failed: 0,
        transfer_timeout: 0,
        error_code: None,
        error: None,
    });

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::LatestBlock);

    assert_eq!(app.explorer_panel_state.block_height_input, "88");
    assert_eq!(app.explorer_panel_state.pending_block_height, Some(88));
    assert!(app.explorer_panel_state.pending_block_refresh);
}

#[test]
fn activate_explorer_tab_marks_explicit_refresh_without_waiting_for_poll() {
    let mut app = ClientLauncherApp::default();

    app.activate_explorer_tab(ExplorerTab::Blocks);

    assert_eq!(app.active_explorer_tab(), ExplorerTab::Blocks);
    assert!(app.explorer_panel_state.pending_blocks_refresh);
    assert!(app.explorer_panel_state.last_poll_at.is_some());
}

#[test]
fn explorer_search_block_result_prefers_height_when_key_is_numeric() {
    let mut app = ClientLauncherApp::default();

    app.apply_explorer_search_result("block", "88".to_string());

    assert_eq!(app.active_explorer_tab(), ExplorerTab::Blocks);
    assert_eq!(app.explorer_panel_state.block_height_input, "88");
    assert!(app.explorer_panel_state.block_hash_input.is_empty());
    assert_eq!(app.explorer_panel_state.pending_block_height, Some(88));
    assert_eq!(app.explorer_panel_state.pending_block_hash, None);
    assert!(app.explorer_panel_state.pending_block_refresh);
    assert!(app.explorer_panel_state.last_poll_at.is_some());
}

#[test]
fn explorer_search_block_result_uses_hash_when_key_is_not_numeric() {
    let mut app = ClientLauncherApp::default();

    app.apply_explorer_search_result("block", "block-hash-1".to_string());

    assert_eq!(app.active_explorer_tab(), ExplorerTab::Blocks);
    assert!(app.explorer_panel_state.block_height_input.is_empty());
    assert_eq!(app.explorer_panel_state.block_hash_input, "block-hash-1");
    assert_eq!(app.explorer_panel_state.pending_block_height, None);
    assert_eq!(
        app.explorer_panel_state.pending_block_hash,
        Some("block-hash-1".to_string())
    );
    assert!(app.explorer_panel_state.pending_block_refresh);
}
