use super::web_test_api::{
    consume_web_test_api_commands, publish_web_test_api_state, setup_web_test_api,
};
use super::*;
use crate::i18n::{UiI18n, UiLocale};
use crate::right_panel_module_visibility::RightPanelModuleVisibilityState;
#[cfg(target_arch = "wasm32")]
use bevy::asset::{AssetMetaCheck, AssetPlugin};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

#[path = "theme_runtime.rs"]
mod theme_runtime;
pub(super) use theme_runtime::{
    apply_theme_runtime_updates, resolve_theme_runtime_state, ThemePresetSelection,
    ThemeRuntimeState,
};

pub(super) fn run_ui(addr: String, offline: bool) {
    let viewer_3d_config = resolve_viewer_3d_config();
    let viewer_external_mesh_config = resolve_viewer_external_mesh_config();
    let viewer_external_material_config = resolve_viewer_external_material_config();
    let viewer_external_texture_config = resolve_viewer_external_texture_config();
    let material_variant_preview_state = resolve_material_variant_preview_state();
    let auto_degrade_config = auto_degrade_config_from_env();
    let perf_probe_config = perf_probe::perf_probe_config_from_env();
    let auto_focus_config = auto_focus_config_from_env();
    let viewer_automation_config = viewer_automation_config_from_env();
    let event_window = event_window_policy_from_env(DEFAULT_MAX_EVENTS);
    let panel_mode = resolve_panel_mode_from_env();
    let experience_mode = resolve_experience_mode_from_env();
    let right_panel_layout_state = default_right_panel_layout_state(experience_mode);
    let module_visibility_defaults = default_module_visibility_state(experience_mode);
    let theme_runtime = resolve_theme_runtime_state();
    let (module_visibility_state, module_visibility_path) =
        resolve_right_panel_module_visibility_resources(module_visibility_defaults);
    let default_plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(primary_window_config()),
        ..default()
    });
    #[cfg(target_arch = "wasm32")]
    let default_plugins = default_plugins.set(AssetPlugin {
        // Web runtime may receive empty fallback bodies for missing `.meta` files.
        // Skip meta probing so font/asset loading stays on the unprocessed path.
        meta_check: AssetMetaCheck::Never,
        ..default()
    });
    let default_plugins = default_plugins.set(default_pbr_plugin_for_runtime());

    App::new()
        .insert_resource(ViewerConfig {
            addr,
            max_events: event_window.max_events,
            event_window,
        })
        .insert_resource(viewer_3d_config)
        .insert_resource(viewer_external_mesh_config)
        .insert_resource(viewer_external_material_config)
        .insert_resource(viewer_external_texture_config)
        .insert_resource(material_variant_preview_state)
        .insert_resource(Viewer3dScene::default())
        .insert_resource(ViewerCameraMode::default())
        .insert_resource(panel_mode)
        .insert_resource(experience_mode)
        .insert_resource(theme_runtime)
        .insert_resource(ViewerSelection::default())
        .insert_resource(ChatInputFocusSignal::default())
        .insert_resource(world_overlay_config_from_env())
        .insert_resource(crate::industry_graph_view_model::IndustrySemanticZoomState::default())
        .insert_resource(WorldOverlayUiState::default())
        .insert_resource(OverlayRenderRuntime::default())
        .insert_resource(DiagnosisState::default())
        .insert_resource(EventObjectLinkState::default())
        .insert_resource(TimelineUiState::default())
        .insert_resource(TimelineMarkFilterState::default())
        .insert_resource(CopyableTextPanelState::default())
        .insert_resource(OrbitDragState::default())
        .insert_resource(TwoDZoomTier::default())
        .insert_resource(resolve_initial_ui_i18n())
        .insert_resource(auto_degrade_config)
        .insert_resource(AutoDegradeState::default())
        .insert_resource(perf_probe_config)
        .insert_resource(perf_probe::PerfProbeState::default())
        .insert_resource(auto_focus_config)
        .insert_resource(AutoFocusState::default())
        .insert_resource(viewer_automation_config)
        .insert_resource(ViewerAutomationState::default())
        .insert_resource(SelectionEmphasisState::default())
        .insert_resource(internal_capture_config_from_env())
        .insert_resource(InternalCaptureState::default())
        .insert_resource(right_panel_layout_state)
        .insert_resource(RightPanelWidthState::default())
        .insert_resource(RenderPerfSummary::default())
        .insert_resource(RenderPerfHistory::default())
        .insert_resource(LabelLodStats::default())
        .insert_resource(module_visibility_state)
        .insert_resource(module_visibility_path)
        .insert_resource(StepControlLoadingState::default())
        .insert_resource(ViewerControlProfileState::default())
        .add_plugins(default_plugins)
        .add_plugins(EguiPlugin::default())
        .insert_resource(OfflineConfig { offline })
        .add_systems(
            Startup,
            (
                setup_startup_state,
                setup_3d_scene,
                setup_wasm_egui_input_bridge,
                setup_web_test_api,
            ),
        )
        .add_systems(Update, pump_wasm_egui_input_bridge_events)
        .add_systems(
            Update,
            consume_web_test_api_commands.before(run_viewer_automation),
        )
        .add_systems(
            Update,
            (
                poll_viewer_messages,
                headless_auto_play_once,
                sync_timeline_state_from_world,
                handle_timeline_adjust_buttons,
                handle_timeline_mark_filter_buttons,
                handle_timeline_bar_drag,
                handle_timeline_mark_jump_buttons,
                handle_timeline_seek_submit,
                handle_world_overlay_toggle_buttons,
                handle_event_click_buttons,
                handle_locate_focus_event_button,
                selection_linking::handle_quick_locate_agent_button,
                handle_jump_selection_events_button,
                update_event_object_link_text,
                update_world_overlay_status_text,
                update_diagnosis_panel,
                update_event_click_list_ui,
                update_timeline_ui,
                trigger_internal_capture,
                persist_right_panel_module_visibility,
            )
                .chain(),
        )
        .add_systems(
            Update,
            attempt_viewer_reconnect
                .after(poll_viewer_messages)
                .before(sync_timeline_state_from_world),
        )
        .add_systems(Update, track_step_loading_state)
        .add_systems(Update, apply_theme_runtime_updates.before(update_3d_scene))
        .add_systems(
            Update,
            (
                update_3d_scene,
                update_selection_emphasis.after(update_3d_scene),
                apply_startup_auto_focus.after(update_3d_scene),
                update_world_overlays_3d.after(update_3d_scene),
                orbit_camera_controls,
                handle_focus_selection_hotkey.after(orbit_camera_controls),
                run_viewer_automation
                    .after(update_3d_scene)
                    .after(apply_startup_auto_focus)
                    .after(orbit_camera_controls)
                    .before(sync_camera_mode),
                sync_camera_mode
                    .after(orbit_camera_controls)
                    .after(handle_focus_selection_hotkey),
                camera_controls::sync_two_d_zoom_tier
                    .after(sync_camera_mode)
                    .after(orbit_camera_controls),
                camera_controls::sync_two_d_map_marker_visibility
                    .after(camera_controls::sync_two_d_zoom_tier),
                camera_controls::sync_two_d_map_marker_scale
                    .after(camera_controls::sync_two_d_map_marker_visibility),
                camera_controls::sync_detail_zoom_visibility
                    .after(camera_controls::sync_two_d_zoom_tier),
                update_grid_line_lod_visibility.after(sync_camera_mode),
                sync_world_background_visibility.after(sync_camera_mode),
                update_floating_origin.after(orbit_camera_controls),
                sample_render_perf_summary.after(update_grid_line_lod_visibility),
                update_auto_degrade_policy.after(sample_render_perf_summary),
                perf_probe::update_perf_probe.after(update_auto_degrade_policy),
                update_3d_viewport,
                handle_control_buttons,
            ),
        )
        .add_systems(
            Update,
            handle_material_variant_preview_hotkey.after(update_3d_scene),
        )
        .add_systems(
            PostUpdate,
            (
                pick_3d_selection.after(TransformSystems::Propagate),
                update_label_lod.after(pick_3d_selection),
            ),
        )
        .add_systems(
            EguiPrimaryContextPass,
            (
                render_right_side_panel_egui,
                sync_wasm_egui_input_bridge_focus,
            )
                .chain(),
        )
        .add_systems(
            Update,
            publish_web_test_api_state
                .after(poll_viewer_messages)
                .after(run_viewer_automation),
        )
        .run();
}

fn resolve_initial_ui_i18n() -> UiI18n {
    UiI18n {
        locale: resolve_initial_ui_locale().unwrap_or(UiI18n::default().locale),
    }
}

#[cfg(target_arch = "wasm32")]
fn resolve_initial_ui_locale() -> Option<UiLocale> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    parse_ui_locale_from_search(search.as_str())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_initial_ui_locale() -> Option<UiLocale> {
    None
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn parse_ui_locale_from_search(search: &str) -> Option<UiLocale> {
    #[cfg(target_arch = "wasm32")]
    {
        let params = web_sys::UrlSearchParams::new_with_str(search).ok()?;
        return parse_ui_locale_param(
            params
                .get("locale")
                .or_else(|| params.get("language"))
                .as_deref(),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if search.trim().is_empty() {
            return None;
        }
        let trimmed = search.trim_start_matches('?');
        for pair in trimmed.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or_default();
            let value = parts.next().unwrap_or_default();
            if key == "locale" || key == "language" {
                return parse_ui_locale_param(Some(value));
            }
        }
        None
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn parse_ui_locale_param(raw: Option<&str>) -> Option<UiLocale> {
    match raw.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "zh_cn" | "cn" | "chinese" => Some(UiLocale::ZhCn),
        "en" | "en-us" | "en_us" | "english" => Some(UiLocale::EnUs),
        _ => None,
    }
}

fn primary_window_config() -> Window {
    let window = Window {
        title: "oasis7 Viewer".to_string(),
        resolution: (1200, 800).into(),
        ..default()
    };

    #[cfg(target_arch = "wasm32")]
    let window = Window {
        fit_canvas_to_parent: true,
        // Preserve browser composition flow so CJK IME can commit into egui TextEdit.
        prevent_default_event_handling: false,
        ..window
    };

    window
}

fn default_pbr_plugin_for_runtime() -> bevy::pbr::PbrPlugin {
    #[cfg(not(target_arch = "wasm32"))]
    let plugin = bevy::pbr::PbrPlugin::default();
    #[cfg(target_arch = "wasm32")]
    let mut plugin = bevy::pbr::PbrPlugin::default();
    #[cfg(target_arch = "wasm32")]
    {
        // SwiftShader/WebGL2 can fail deferred lighting pipeline creation in headless CI/browser
        // runs. Keep 3D rendering on the forward path for wasm stability.
        plugin.add_default_deferred_lighting_plugin = false;
    }
    plugin
}

fn resolve_panel_mode_from_env() -> ViewerPanelMode {
    let Some(raw) = crate::viewer_env::viewer_env_var("OASIS7_VIEWER_PANEL_MODE") else {
        return ViewerPanelMode::default();
    };

    match raw.trim().to_ascii_lowercase().as_str() {
        "observe" | "obs" | "default" | "prompt_ops" | "prompt-ops" | "promptops" | "ops" => {
            ViewerPanelMode::Observe
        }
        _ => ViewerPanelMode::default(),
    }
}

fn resolve_experience_mode_from_env() -> ViewerExperienceMode {
    let Some(raw) = crate::viewer_env::viewer_env_var("OASIS7_VIEWER_EXPERIENCE_MODE") else {
        return ViewerExperienceMode::default();
    };

    parse_experience_mode(raw.as_str()).unwrap_or_default()
}

fn parse_experience_mode(raw: &str) -> Option<ViewerExperienceMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "player" => Some(ViewerExperienceMode::Player),
        "director" => Some(ViewerExperienceMode::Director),
        _ => None,
    }
}

fn parse_env_toggle(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn resolve_panel_hidden_override_from_env() -> Option<bool> {
    crate::viewer_env::viewer_env_var("OASIS7_VIEWER_PANEL_HIDDEN")
        .and_then(|raw| parse_env_toggle(raw.as_str()))
}

fn default_right_panel_layout_state_with_override(
    mode: ViewerExperienceMode,
    panel_hidden_override: Option<bool>,
) -> RightPanelLayoutState {
    let mut state = match mode {
        ViewerExperienceMode::Player => RightPanelLayoutState {
            top_panel_collapsed: false,
            panel_hidden: false,
        },
        ViewerExperienceMode::Director => RightPanelLayoutState::default(),
    };

    if let Some(panel_hidden) = panel_hidden_override {
        state.panel_hidden = panel_hidden;
    }

    state
}

fn default_right_panel_layout_state(mode: ViewerExperienceMode) -> RightPanelLayoutState {
    default_right_panel_layout_state_with_override(mode, resolve_panel_hidden_override_from_env())
}

fn default_module_visibility_state(mode: ViewerExperienceMode) -> RightPanelModuleVisibilityState {
    match mode {
        ViewerExperienceMode::Player => RightPanelModuleVisibilityState {
            show_controls: true,
            show_overview: false,
            show_chat: true,
            show_overlay: false,
            show_diagnosis: false,
            show_event_link: false,
            show_timeline: false,
            show_details: false,
        },
        ViewerExperienceMode::Director => RightPanelModuleVisibilityState::default(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn run_headless(addr: String, offline: bool) {
    let event_window = event_window_policy_from_env(DEFAULT_MAX_EVENTS);
    let perf_probe_config = perf_probe::perf_probe_config_from_env();
    App::new()
        .insert_resource(ViewerConfig {
            addr,
            max_events: event_window.max_events,
            event_window,
        })
        .insert_resource(HeadlessStatus::default())
        .insert_resource(OfflineConfig { offline })
        .insert_resource(perf_probe_config)
        .insert_resource(perf_probe::PerfProbeState::default())
        .insert_resource(RenderPerfSummary::default())
        .insert_resource(RenderPerfHistory::default())
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, setup_startup_state)
        .add_systems(
            Update,
            (
                poll_viewer_messages,
                attempt_viewer_reconnect,
                headless_auto_play_once,
                sample_headless_perf_summary,
                perf_probe::update_perf_probe.after(sample_headless_perf_summary),
                headless_report,
            )
                .chain(),
        )
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_addr() -> String {
    crate::viewer_env::viewer_env_var("OASIS7_VIEWER_ADDR")
        .or_else(|| std::env::args().nth(1))
        .unwrap_or_else(|| DEFAULT_ADDR.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_offline(headless: bool) -> bool {
    let offline_env = crate::viewer_env::viewer_env_present("OASIS7_VIEWER_OFFLINE");
    let force_online = crate::viewer_env::viewer_env_present("OASIS7_VIEWER_FORCE_ONLINE");
    decide_offline(headless, offline_env, force_online)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn decide_offline(headless: bool, offline_env: bool, force_online: bool) -> bool {
    if force_online {
        return false;
    }
    if offline_env {
        return true;
    }
    headless
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ui_locale_from_search_accepts_locale_and_language_keys() {
        assert_eq!(
            parse_ui_locale_from_search("?locale=zh"),
            Some(UiLocale::ZhCn)
        );
        assert_eq!(
            parse_ui_locale_from_search("?language=en-US"),
            Some(UiLocale::EnUs)
        );
        assert_eq!(
            parse_ui_locale_from_search("?ws=ws://127.0.0.1:5011&locale=chinese"),
            Some(UiLocale::ZhCn)
        );
    }

    #[test]
    fn parse_ui_locale_from_search_ignores_unknown_values() {
        assert_eq!(parse_ui_locale_from_search("?locale=fr"), None);
        assert_eq!(parse_ui_locale_from_search("?ws=ws://127.0.0.1:5011"), None);
    }

    #[test]
    fn primary_window_config_sets_title_and_resolution() {
        let window = primary_window_config();
        assert_eq!(window.title, "oasis7 Viewer");
        assert_eq!(window.resolution.physical_width(), 1200);
        assert_eq!(window.resolution.physical_height(), 800);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn primary_window_config_keeps_default_prevent_default_on_native() {
        let window = primary_window_config();
        assert!(window.prevent_default_event_handling);
    }

    #[test]
    fn decide_offline_force_online_overrides_other_flags() {
        assert!(!decide_offline(true, true, true));
    }

    #[test]
    fn parse_experience_mode_accepts_player_and_director() {
        assert_eq!(
            parse_experience_mode("player"),
            Some(ViewerExperienceMode::Player)
        );
        assert_eq!(
            parse_experience_mode(" director "),
            Some(ViewerExperienceMode::Director)
        );
    }

    #[test]
    fn parse_experience_mode_rejects_unknown_values() {
        assert_eq!(parse_experience_mode(""), None);
        assert_eq!(parse_experience_mode("ops"), None);
    }

    #[test]
    fn default_right_panel_layout_state_matches_experience_mode() {
        assert_eq!(
            default_right_panel_layout_state(ViewerExperienceMode::Player),
            RightPanelLayoutState {
                top_panel_collapsed: false,
                panel_hidden: false,
            }
        );
        assert_eq!(
            default_right_panel_layout_state(ViewerExperienceMode::Director),
            RightPanelLayoutState::default()
        );
    }

    #[test]
    fn parse_env_toggle_supports_on_and_off_values() {
        assert_eq!(parse_env_toggle("true"), Some(true));
        assert_eq!(parse_env_toggle(" On "), Some(true));
        assert_eq!(parse_env_toggle("0"), Some(false));
        assert_eq!(parse_env_toggle("off"), Some(false));
        assert_eq!(parse_env_toggle("invalid"), None);
    }

    #[test]
    fn right_panel_layout_state_applies_panel_hidden_override() {
        assert_eq!(
            default_right_panel_layout_state_with_override(
                ViewerExperienceMode::Director,
                Some(true)
            ),
            RightPanelLayoutState {
                top_panel_collapsed: false,
                panel_hidden: true,
            }
        );
        assert_eq!(
            default_right_panel_layout_state_with_override(
                ViewerExperienceMode::Player,
                Some(false)
            ),
            RightPanelLayoutState {
                top_panel_collapsed: false,
                panel_hidden: false,
            }
        );
    }

    #[test]
    fn default_module_visibility_state_player_is_lightweight() {
        let state = default_module_visibility_state(ViewerExperienceMode::Player);
        assert!(state.show_controls);
        assert!(!state.show_overview);
        assert!(state.show_chat);
        assert!(!state.show_overlay);
        assert!(!state.show_diagnosis);
        assert!(!state.show_event_link);
        assert!(!state.show_timeline);
        assert!(!state.show_details);
    }

    #[test]
    fn default_pbr_plugin_for_runtime_tracks_target_compat_mode() {
        let plugin = default_pbr_plugin_for_runtime();
        assert!(plugin.prepass_enabled);
        #[cfg(target_arch = "wasm32")]
        assert!(!plugin.add_default_deferred_lighting_plugin);
        #[cfg(not(target_arch = "wasm32"))]
        assert!(plugin.add_default_deferred_lighting_plugin);
    }
}
