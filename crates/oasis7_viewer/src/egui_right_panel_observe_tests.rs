use super::*;

#[test]
fn egui_kittest_overview_renders_status_badges() {
    let state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let selection = crate::ViewerSelection::default();
    let timeline = TimelineUiState::default();

    let mut harness = Harness::new_ui(move |ui| {
        render_overview_section(
            ui,
            crate::i18n::UiLocale::ZhCn,
            &state,
            &selection,
            &timeline,
            None,
        );
    });

    harness.fit_contents();
    harness.get_by_label_contains("连接正常");
    harness.get_by_label_contains("健康:正常");
    harness.get_by_label_contains("观察:实时");
    harness.get_by_label_contains("状态: 已连接");
}

#[test]
fn egui_kittest_overview_reacts_to_warn_and_manual_mode() {
    let state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_rejected_event(1, 1)],
    );
    let selection = crate::ViewerSelection::default();
    let timeline = TimelineUiState {
        manual_override: true,
        ..Default::default()
    };

    let mut harness = Harness::new_ui(move |ui| {
        render_overview_section(
            ui,
            crate::i18n::UiLocale::EnUs,
            &state,
            &selection,
            &timeline,
            None,
        );
    });

    harness.fit_contents();
    harness.get_by_label_contains("Health: Warn 1");
    harness.get_by_label_contains("View: Manual");
    harness.get_by_label_contains("Status: connected");
}

#[test]
fn egui_kittest_overview_renders_render_perf_summary() {
    let state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let selection = crate::ViewerSelection::default();
    let timeline = TimelineUiState::default();
    let perf = crate::RenderPerfSummary {
        frame_ms_avg: 16.7,
        frame_ms_p95: 24.2,
        world_entities: 180,
        visible_labels: 46,
        overlay_entities: 22,
        event_window_size: 88,
        auto_degrade_active: true,
        ..crate::RenderPerfSummary::default()
    };

    let mut harness = Harness::new_ui(move |ui| {
        render_overview_section(
            ui,
            crate::i18n::UiLocale::EnUs,
            &state,
            &selection,
            &timeline,
            Some(&perf),
        );
    });

    harness.fit_contents();
    harness.get_by_label_contains("Render: avg/p95 16.7/24.2 ms");
    harness.get_by_label_contains("Entities:180");
    harness.get_by_label_contains("Budget: auto degrade active");
    harness.get_by_label_contains("Hotspot: render_frame");
    harness.get_by_label_contains("Runtime: unknown/none");
}

#[derive(Default)]
struct TimelineFilterHarnessState {
    viewer_state: crate::ViewerState,
    timeline: TimelineUiState,
    filters: TimelineMarkFilterState,
}

#[test]
fn egui_kittest_timeline_filter_button_toggles_state() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut TimelineFilterHarnessState| {
            render_timeline_section(
                ui,
                crate::i18n::UiLocale::ZhCn,
                &state.viewer_state,
                &mut state.timeline,
                &mut state.filters,
                None,
                None,
            );
        },
        TimelineFilterHarnessState::default(),
    );

    harness.fit_contents();
    harness.get_by_label("错误:开").click();
    harness.run();
    assert!(!harness.state().filters.show_error);

    harness.get_by_label("错误:关").click();
    harness.run();
    assert!(harness.state().filters.show_error);
}

#[test]
fn egui_kittest_overlay_section_renders_chunk_legend_and_width_hint() {
    let state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let viewer_3d_config: Option<Res<crate::Viewer3dConfig>> = None;
    let mut overlay_config = crate::WorldOverlayConfig::default();
    let mut industry_zoom = crate::industry_graph_view_model::IndustrySemanticZoomState::default();

    let mut harness = Harness::new_ui(move |ui| {
        render_overlay_section(
            ui,
            crate::i18n::UiLocale::ZhCn,
            crate::ViewerCameraMode::TwoD,
            &state,
            &viewer_3d_config,
            &mut overlay_config,
            &mut industry_zoom,
        );
    });

    harness.fit_contents();
    harness.get_by_label_contains("分块图例");
    harness.get_by_label_contains("未探索");
    harness.get_by_label_contains("已生成");
    harness.get_by_label_contains("已耗尽");
    harness.get_by_label_contains("背景网格");
    harness.get_by_label_contains("线宽(2D)");
}

#[derive(Default)]
struct CameraModeHarnessState {
    camera_mode: crate::ViewerCameraMode,
}

#[test]
fn egui_kittest_camera_mode_toggle_switches_state() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut CameraModeHarnessState| {
            ui.horizontal_wrapped(|ui| {
                let is_two_d = state.camera_mode == crate::ViewerCameraMode::TwoD;
                if ui
                    .selectable_label(
                        is_two_d,
                        crate::i18n::camera_mode_button_label(
                            crate::ViewerCameraMode::TwoD,
                            crate::i18n::UiLocale::ZhCn,
                        ),
                    )
                    .clicked()
                {
                    state.camera_mode = crate::ViewerCameraMode::TwoD;
                }
                if ui
                    .selectable_label(
                        !is_two_d,
                        crate::i18n::camera_mode_button_label(
                            crate::ViewerCameraMode::ThreeD,
                            crate::i18n::UiLocale::ZhCn,
                        ),
                    )
                    .clicked()
                {
                    state.camera_mode = crate::ViewerCameraMode::ThreeD;
                }
            });
        },
        CameraModeHarnessState::default(),
    );

    harness.fit_contents();
    assert_eq!(harness.state().camera_mode, crate::ViewerCameraMode::TwoD);

    harness.get_by_label("3D").click();
    harness.run();
    assert_eq!(harness.state().camera_mode, crate::ViewerCameraMode::ThreeD);

    harness.get_by_label("2D").click();
    harness.run();
    assert_eq!(harness.state().camera_mode, crate::ViewerCameraMode::TwoD);
}

#[derive(Default)]
struct ModuleToggleHarnessState {
    visibility: RightPanelModuleVisibilityState,
    copyable_visible: bool,
}

#[test]
fn egui_kittest_module_toggle_switches_visibility() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ModuleToggleHarnessState| {
            ui.horizontal_wrapped(|ui| {
                render_module_toggle_button(
                    ui,
                    "controls",
                    &mut state.visibility.show_controls,
                    crate::i18n::UiLocale::ZhCn,
                );

                let mut details_visible = state.visibility.show_details;
                render_module_toggle_button(
                    ui,
                    "details",
                    &mut details_visible,
                    crate::i18n::UiLocale::ZhCn,
                );
                state.visibility.show_details = details_visible;
                state.copyable_visible = details_visible;
            });
        },
        ModuleToggleHarnessState {
            visibility: RightPanelModuleVisibilityState::default(),
            copyable_visible: true,
        },
    );

    harness.fit_contents();
    harness.get_by_label("控制:开").click();
    harness.run();
    assert!(!harness.state().visibility.show_controls);

    harness.get_by_label("明细:关").click();
    harness.run();
    assert!(harness.state().visibility.show_details);
    assert!(harness.state().copyable_visible);

    harness.get_by_label("明细:开").click();
    harness.run();
    assert!(!harness.state().visibility.show_details);
    assert!(!harness.state().copyable_visible);
}

#[test]
fn egui_kittest_snapshot_overview_live() {
    let Some(renderer) = snapshot_renderer_or_skip() else {
        return;
    };

    let state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let selection = crate::ViewerSelection::default();
    let timeline = TimelineUiState::default();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 150.0))
        .renderer(renderer)
        .build_ui(move |ui| {
            render_overview_section(
                ui,
                crate::i18n::UiLocale::EnUs,
                &state,
                &selection,
                &timeline,
                None,
            );
        });

    harness.fit_contents();
    harness.snapshot_options("viewer_overview_live", &snapshot_options());
}

#[test]
fn egui_kittest_snapshot_overview_manual_high_risk() {
    let Some(renderer) = snapshot_renderer_or_skip() else {
        return;
    };

    let state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![
            sample_rejected_event(1, 1),
            sample_rejected_event(2, 2),
            sample_rejected_event(3, 3),
        ],
    );
    let selection = crate::ViewerSelection::default();
    let timeline = TimelineUiState {
        manual_override: true,
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 160.0))
        .renderer(renderer)
        .build_ui(move |ui| {
            render_overview_section(
                ui,
                crate::i18n::UiLocale::EnUs,
                &state,
                &selection,
                &timeline,
                None,
            );
        });

    harness.fit_contents();
    harness.snapshot_options("viewer_overview_manual_high_risk", &snapshot_options());
}
