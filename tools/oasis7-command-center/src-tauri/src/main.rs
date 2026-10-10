#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::Host;
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
};

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            app.manage(Host::open(directory.join("command-center.sqlite"))?);
            let show = MenuItem::with_id(app, "show", "打开指挥中心", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出指挥中心", true, Some("CmdOrCtrl+Q"))?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let application_menu =
                Submenu::with_items(app, "Oasis7 指挥中心", true, &[&show, &quit])?;
            let edit_menu = Submenu::with_items(
                app,
                "编辑",
                true,
                &[
                    &PredefinedMenuItem::undo(app, Some("撤销"))?,
                    &PredefinedMenuItem::redo(app, Some("重做"))?,
                    &PredefinedMenuItem::cut(app, Some("剪切"))?,
                    &PredefinedMenuItem::copy(app, Some("复制"))?,
                    &PredefinedMenuItem::paste(app, Some("粘贴"))?,
                    &PredefinedMenuItem::select_all(app, Some("全选"))?,
                ],
            )?;
            app.set_menu(Menu::with_items(app, &[&application_menu, &edit_menu])?)?;
            let icon = app
                .default_window_icon()
                .ok_or("packaged application icon missing")?
                .clone();
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Oasis7 指挥中心")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    // This foundation owns no external processes or model tasks.
                    // Core has no unresolved runtime operations to coordinate.
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_project,
            commands::query_view,
            commands::search_rules,
            commands::save_decision_draft,
            commands::save_preference,
            commands::read_source,
            commands::refresh_source,
            commands::rebuild_cache,
            commands::refresh_github,
            commands::get_github_snapshot,
            commands::open_source_target,
            commands::choose_project,
            commands::associate_work,
            commands::read_version,
        ])
        .build(tauri::generate_context!())
        .expect("initialize Command Center desktop")
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
        });
}
