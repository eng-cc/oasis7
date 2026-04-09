use super::egui_observe_section_card::{section_tone, ObserveSectionTone};
use super::egui_right_panel_player_experience::resolve_player_guide_step;
use super::*;
use crate::right_panel_module_visibility::RightPanelModuleVisibilityState;
use crate::ViewerControl;
use egui_kittest::{kittest::Queryable as _, Harness};
use egui_wgpu::wgpu;
use oasis7::simulator::{
    PlayerGameplayGoalKind, PlayerGameplaySnapshot, PlayerGameplayStageId,
    PlayerGameplayStageStatus, RejectReason, WorldEvent, WorldEventKind,
};
use std::iter::once;
use std::mem::size_of;
use std::sync::mpsc::channel;
use std::time::Duration;

const SNAPSHOT_OUTPUT_DIR: &str = "tests/snapshots";
const SNAPSHOT_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[path = "egui_right_panel_observe_tests.rs"]
mod observe_tests;
#[path = "egui_right_panel_player_achievements_tests.rs"]
mod player_achievements_tests;
#[path = "egui_right_panel_player_atmosphere_tests.rs"]
mod player_atmosphere_tests;
#[path = "egui_right_panel_player_card_motion_tests.rs"]
mod player_card_motion_tests;
#[path = "egui_right_panel_player_chatter_tests.rs"]
mod player_chatter_tests;
#[path = "egui_right_panel_player_cinematic_tests.rs"]
mod player_cinematic_tests;
#[path = "egui_right_panel_player_entry_tests.rs"]
mod player_entry_tests;
#[path = "egui_right_panel_player_guide_progress_tests.rs"]
mod player_guide_progress_tests;
#[path = "egui_right_panel_player_layout_tests.rs"]
mod player_layout_tests;
#[path = "egui_right_panel_player_minimap_tests.rs"]
mod player_minimap_tests;
#[path = "egui_right_panel_player_mission_tests.rs"]
mod player_mission_tests;
#[path = "egui_right_panel_player_reward_tests.rs"]
mod player_reward_tests;
#[path = "egui_right_panel_player_stuck_tests.rs"]
mod player_stuck_tests;
#[path = "egui_right_panel_player_summary_tests.rs"]
mod player_summary_tests;

struct SnapshotRenderer {
    render_state: egui_wgpu::RenderState,
}

impl SnapshotRenderer {
    fn try_new() -> Result<Self, String> {
        let setup = egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew::default());
        let instance = pollster::block_on(setup.new_instance());
        let render_state = pollster::block_on(egui_wgpu::RenderState::create(
            &egui_wgpu::WgpuConfiguration {
                wgpu_setup: setup,
                ..Default::default()
            },
            &instance,
            None,
            egui_wgpu::RendererOptions::PREDICTABLE,
        ))
        .map_err(|err| format!("failed to create wgpu render state for snapshots: {err}"))?;

        Ok(Self { render_state })
    }
}

fn snapshot_renderer_or_skip() -> Option<SnapshotRenderer> {
    match SnapshotRenderer::try_new() {
        Ok(renderer) => Some(renderer),
        Err(err) => {
            eprintln!("skip egui snapshot tests because wgpu is unavailable: {err}");
            None
        }
    }
}

impl egui_kittest::TestRenderer for SnapshotRenderer {
    fn handle_delta(&mut self, delta: &egui::TexturesDelta) {
        let mut renderer = self.render_state.renderer.write();
        for (texture_id, image_delta) in &delta.set {
            renderer.update_texture(
                &self.render_state.device,
                &self.render_state.queue,
                *texture_id,
                image_delta,
            );
        }
        for texture_id in &delta.free {
            renderer.free_texture(texture_id);
        }
    }

    fn render(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        let mut renderer = self.render_state.renderer.write();
        let mut encoder =
            self.render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("EguiKittestSnapshotEncoder"),
                });

        let size = ctx.content_rect().size() * ctx.pixels_per_point();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            pixels_per_point: ctx.pixels_per_point(),
            size_in_pixels: [size.x.round() as u32, size.y.round() as u32],
        };
        let clipped_primitives = ctx.tessellate(output.shapes.clone(), ctx.pixels_per_point());

        let user_cmd_bufs = renderer.update_buffers(
            &self.render_state.device,
            &self.render_state.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        let texture = self
            .render_state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("EguiKittestSnapshotTexture"),
                size: wgpu::Extent3d {
                    width: screen_descriptor.size_in_pixels[0],
                    height: screen_descriptor.size_in_pixels[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.render_state.target_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("EguiKittestSnapshotPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            renderer.render(&mut pass, &clipped_primitives, &screen_descriptor);
        }

        self.render_state
            .queue
            .submit(user_cmd_bufs.into_iter().chain(once(encoder.finish())));
        self.render_state
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(SNAPSHOT_WAIT_TIMEOUT),
            })
            .map_err(|err| format!("poll error while rendering snapshot: {err}"))?;

        Ok(texture_to_image(
            &self.render_state.device,
            &self.render_state.queue,
            &texture,
        ))
    }
}

fn texture_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> image::RgbaImage {
    let dims = BufferDimensions::new(texture.width() as usize, texture.height() as usize);
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("EguiKittestSnapshotReadback"),
        size: (dims.padded_bytes_per_row * dims.height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("EguiKittestSnapshotCopyEncoder"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(dims.padded_bytes_per_row as u32),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        },
    );

    let submit_index = queue.submit(once(encoder.finish()));
    let slice = output_buffer.slice(..);
    let (tx, rx) = channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submit_index),
            timeout: Some(SNAPSHOT_WAIT_TIMEOUT),
        })
        .expect("failed to poll wgpu device for snapshot readback");
    rx.recv()
        .expect("snapshot channel closed")
        .expect("failed to map snapshot buffer");

    let mapped = output_buffer.slice(..).get_mapped_range();
    let bytes = mapped
        .chunks_exact(dims.padded_bytes_per_row)
        .flat_map(|row| row.iter().take(dims.unpadded_bytes_per_row))
        .copied()
        .collect::<Vec<_>>();
    drop(mapped);
    output_buffer.unmap();

    image::RgbaImage::from_raw(texture.width(), texture.height(), bytes)
        .expect("failed to build image from snapshot bytes")
}

struct BufferDimensions {
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padding = (align - unpadded_bytes_per_row % align) % align;
        Self {
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row: unpadded_bytes_per_row + padding,
        }
    }
}

fn snapshot_options() -> egui_kittest::SnapshotOptions {
    egui_kittest::SnapshotOptions::new()
        .threshold(0.6)
        .failed_pixel_count_threshold(
            egui_kittest::OsThreshold::new(0)
                .macos(24)
                .linux(24)
                .windows(24),
        )
        .output_path(SNAPSHOT_OUTPUT_DIR)
}

fn sample_rejected_event(id: u64, time: u64) -> WorldEvent {
    WorldEvent {
        id,
        time,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::RuleDenied {
                notes: vec![format!("rule-{id}")],
            },
        },
        runtime_event: None,
    }
}

fn sample_agent_not_found_rejected_event(id: u64, time: u64) -> WorldEvent {
    WorldEvent {
        id,
        time,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::AgentNotFound {
                agent_id: format!("agent-{id}"),
            },
        },
        runtime_event: None,
    }
}

fn sample_agent_moved_event(id: u64, time: u64) -> WorldEvent {
    WorldEvent {
        id,
        time,
        kind: WorldEventKind::AgentMoved {
            agent_id: format!("agent-{id}"),
            from: "from-loc".to_string(),
            to: "to-loc".to_string(),
            distance_cm: 12,
            electricity_cost: 3,
        },
        runtime_event: None,
    }
}

fn sample_runtime_event(id: u64, time: u64, kind: &str, summary: &str) -> WorldEvent {
    WorldEvent {
        id,
        time,
        kind: WorldEventKind::RuntimeEvent {
            kind: kind.to_string(),
            domain_kind: Some(summary.to_string()),
        },
        runtime_event: None,
    }
}

fn sample_viewer_state(
    status: crate::ConnectionStatus,
    events: Vec<WorldEvent>,
) -> crate::ViewerState {
    crate::ViewerState {
        status,
        snapshot: None,
        events,
        decision_traces: Vec::new(),
        metrics: None,
    }
}

fn sample_selected_viewer_selection() -> crate::ViewerSelection {
    crate::ViewerSelection {
        current: Some(crate::SelectionInfo {
            entity: Entity::from_bits(1),
            kind: crate::SelectionKind::Agent,
            id: "agent-1".to_string(),
            name: Some("agent-1".to_string()),
        }),
    }
}

#[test]
fn adaptive_panel_width_clamps_to_bounds() {
    assert_eq!(adaptive_panel_default_width(200.0), MAIN_PANEL_MIN_WIDTH);
    assert_eq!(adaptive_panel_default_width(1200.0), 264.0);
    assert_eq!(adaptive_panel_default_width(1500.0), 330.0);
    assert_eq!(adaptive_panel_default_width(4000.0), 880.0);
}

#[test]
fn adaptive_panel_max_width_scales_with_available_width() {
    assert_eq!(adaptive_panel_max_width(200.0), MAIN_PANEL_MIN_WIDTH);
    assert_eq!(adaptive_panel_max_width(1200.0), 720.0);
    assert_eq!(adaptive_panel_max_width(2000.0), 1200.0);
}

#[test]
fn adaptive_chat_panel_width_clamps_to_bounds() {
    assert_eq!(
        adaptive_chat_panel_default_width(200.0),
        CHAT_PANEL_MIN_WIDTH
    );
    assert_eq!(adaptive_chat_panel_default_width(1200.0), 300.0);
    assert_eq!(adaptive_chat_panel_default_width(1800.0), 450.0);
    assert_eq!(adaptive_chat_panel_default_width(4000.0), 1000.0);
}

#[test]
fn adaptive_chat_panel_max_width_scales_with_available_width() {
    assert_eq!(adaptive_chat_panel_max_width(200.0), CHAT_PANEL_MIN_WIDTH);
    assert_eq!(adaptive_chat_panel_max_width(1200.0), 780.0);
    assert_eq!(adaptive_chat_panel_max_width(2000.0), 1300.0);
}

#[test]
fn compact_chat_layout_switches_below_breakpoint() {
    assert!(is_compact_chat_layout(390.0));
    assert!(is_compact_chat_layout(
        CHAT_SIDE_PANEL_COMPACT_BREAKPOINT - 1.0
    ));
    assert!(!is_compact_chat_layout(
        CHAT_SIDE_PANEL_COMPACT_BREAKPOINT + 1.0
    ));
}

#[test]
fn adaptive_main_panel_min_width_uses_compact_floor_on_narrow_width() {
    assert_eq!(
        adaptive_main_panel_min_width(390.0),
        MAIN_PANEL_COMPACT_MIN_WIDTH
    );
    assert_eq!(adaptive_main_panel_min_width(1280.0), MAIN_PANEL_MIN_WIDTH);
}

#[test]
fn adaptive_chat_panel_max_width_for_side_layout_respects_viewport_budget() {
    assert_eq!(adaptive_chat_panel_max_width_for_side_layout(500.0), 20.0);
    assert_eq!(adaptive_chat_panel_max_width_for_side_layout(760.0), 280.0);
    assert_eq!(adaptive_chat_panel_max_width_for_side_layout(1200.0), 720.0);
}

#[test]
fn player_chat_panel_max_width_for_side_layout_keeps_world_first_budget() {
    assert_eq!(player_chat_panel_max_width_for_side_layout(760.0), 280.0);
    let width = player_chat_panel_max_width_for_side_layout(1365.0);
    assert!((width - 387.9).abs() < 0.1);
}

#[test]
fn adaptive_main_panel_max_width_for_layout_respects_interaction_budget() {
    assert_eq!(
        adaptive_main_panel_max_width_for_layout(1365.0, 360.0),
        765.0
    );
    assert_eq!(
        adaptive_main_panel_max_width_for_layout(390.0, 0.0),
        MAIN_PANEL_COMPACT_MIN_WIDTH
    );
}

#[test]
fn player_main_panel_max_width_for_layout_keeps_world_first_budget() {
    let width = player_main_panel_max_width_for_layout(1365.0, 360.0);
    assert!((width - 267.9).abs() < 0.1);
    assert_eq!(
        player_main_panel_max_width_for_layout(390.0, 0.0),
        MAIN_PANEL_COMPACT_MIN_WIDTH
    );
}

#[test]
fn show_chat_panel_requires_expanded_top_and_visibility_enabled() {
    let expanded_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: false,
    };
    assert!(should_show_chat_panel(&expanded_layout, true));
    assert!(!should_show_chat_panel(&expanded_layout, false));

    let collapsed_layout = RightPanelLayoutState {
        top_panel_collapsed: true,
        panel_hidden: false,
    };
    assert!(!should_show_chat_panel(&collapsed_layout, true));

    let hidden_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: true,
    };
    assert!(!should_show_chat_panel(&hidden_layout, true));
}

#[test]
fn total_right_panel_width_adds_main_and_chat_width() {
    assert_eq!(total_right_panel_width(320.0, 360.0), 680.0);
    assert_eq!(total_right_panel_width(320.0, 0.0), 320.0);
    assert_eq!(total_right_panel_width(-10.0, 100.0), 100.0);
}

#[test]
fn env_toggle_enabled_parses_truthy_values() {
    assert!(env_toggle_enabled(Some("1")));
    assert!(env_toggle_enabled(Some(" true ")));
    assert!(env_toggle_enabled(Some("YES")));
    assert!(env_toggle_enabled(Some("on")));
}

#[test]
fn env_toggle_enabled_rejects_falsy_values() {
    assert!(!env_toggle_enabled(None));
    assert!(!env_toggle_enabled(Some("0")));
    assert!(!env_toggle_enabled(Some("false")));
    assert!(!env_toggle_enabled(Some("off")));
    assert!(!env_toggle_enabled(Some("")));
}

#[test]
fn section_tone_maps_titles_for_zh_and_en() {
    assert_eq!(section_tone("World Summary"), ObserveSectionTone::World);
    assert_eq!(section_tone("Agent Activity"), ObserveSectionTone::Activity);
    assert_eq!(
        section_tone("Industrial Ops"),
        ObserveSectionTone::Industrial
    );
    assert_eq!(
        section_tone("Economy Dashboard"),
        ObserveSectionTone::Economy
    );
    assert_eq!(section_tone("Ops Navigator"), ObserveSectionTone::Ops);
    assert_eq!(section_tone("选中详情"), ObserveSectionTone::Details);
    assert_eq!(section_tone("事件"), ObserveSectionTone::Events);
    assert_eq!(section_tone("Random Title"), ObserveSectionTone::Default);
}

#[test]
fn connection_signal_matches_status() {
    let (text, _) = connection_signal(
        &crate::ConnectionStatus::Connected,
        crate::i18n::UiLocale::ZhCn,
    );
    assert_eq!(text, "连接正常");
    let (text, _) = connection_signal(
        &crate::ConnectionStatus::Connecting,
        crate::i18n::UiLocale::EnUs,
    );
    assert_eq!(text, "Connecting");
    let (text, _) = connection_signal(
        &crate::ConnectionStatus::Error("x".to_string()),
        crate::i18n::UiLocale::EnUs,
    );
    assert_eq!(text, "Conn Error");
}

#[test]
fn connection_signal_uses_error_palette() {
    let (_, color) = connection_signal(
        &crate::ConnectionStatus::Error("failed".to_string()),
        crate::i18n::UiLocale::ZhCn,
    );
    assert_eq!(color, egui::Color32::from_rgb(160, 52, 52));
}

#[test]
fn health_signal_uses_three_levels() {
    let (ok_text, ok_color) = health_signal(0, crate::i18n::UiLocale::EnUs);
    assert_eq!(ok_text, "Health: OK");
    assert_eq!(ok_color, egui::Color32::from_rgb(32, 112, 64));

    let (warn_text, warn_color) = health_signal(2, crate::i18n::UiLocale::ZhCn);
    assert_eq!(warn_text, "健康:告警2");
    assert_eq!(warn_color, egui::Color32::from_rgb(150, 110, 32));

    let (high_text, high_color) = health_signal(3, crate::i18n::UiLocale::EnUs);
    assert_eq!(high_text, "Health: High 3");
    assert_eq!(high_color, egui::Color32::from_rgb(154, 48, 48));
}

#[test]
fn send_control_request_updates_playing_state() {
    let state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let mut loading = crate::button_feedback::StepControlLoadingState::default();
    let mut control_ui = ControlPanelUiState::default();

    send_control_request(
        ViewerControl::Play,
        &state,
        &mut loading,
        &mut control_ui,
        None,
        None,
    );
    assert!(control_ui.playing);

    send_control_request(
        ViewerControl::Pause,
        &state,
        &mut loading,
        &mut control_ui,
        None,
        None,
    );
    assert!(!control_ui.playing);

    send_control_request(
        ViewerControl::Play,
        &state,
        &mut loading,
        &mut control_ui,
        None,
        None,
    );
    assert!(control_ui.playing);

    send_control_request(
        ViewerControl::Step { count: 1 },
        &state,
        &mut loading,
        &mut control_ui,
        None,
        None,
    );
    assert!(!control_ui.playing);
}

#[derive(Default)]
struct ControlButtonsHarnessState {
    viewer_state: crate::ViewerState,
    loading: crate::button_feedback::StepControlLoadingState,
    control_ui: ControlPanelUiState,
}

#[derive(Default)]
struct ThemeRuntimeHarnessState {
    runtime: crate::app_bootstrap::ThemeRuntimeState,
}

#[test]
fn egui_kittest_control_buttons_merge_play_pause_and_fold_advanced_debug() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ControlButtonsHarnessState| {
            render_control_buttons(
                ui,
                crate::i18n::UiLocale::ZhCn,
                false,
                &state.viewer_state,
                &mut state.loading,
                &mut state.control_ui,
                None,
                None,
            );
        },
        ControlButtonsHarnessState::default(),
    );

    harness.fit_contents();
    harness.get_by_label("播放");
    harness.get_by_label("高级调试:关");

    harness.get_by_label("播放").click();
    harness.run();
    assert!(harness.state().control_ui.playing);
    harness.get_by_label("暂停");

    harness.get_by_label("高级调试:关").click();
    harness.run();
    assert!(harness.state().control_ui.advanced_debug_expanded);
    harness.get_by_label("单步");
}

#[test]
fn egui_kittest_theme_runtime_apply_and_hot_reload_controls_work() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ThemeRuntimeHarnessState| {
            super::egui_right_panel_theme_runtime::render_theme_runtime_section(
                ui,
                crate::i18n::UiLocale::ZhCn,
                &mut state.runtime,
            );
        },
        ThemeRuntimeHarnessState::default(),
    );

    harness.fit_contents();
    harness.get_by_label("应用主题").click();
    harness.run();
    assert!(harness.state().runtime.pending_apply);

    harness.get_by_label("自动热重载").click();
    harness.run();
    assert!(harness.state().runtime.hot_reload_enabled);
}

#[test]
fn mode_signal_reflects_timeline_state() {
    let live_timeline = TimelineUiState::default();
    let (live_text, live_color) = mode_signal(&live_timeline, crate::i18n::UiLocale::EnUs);
    assert_eq!(live_text, "View: Live");
    assert_eq!(live_color, egui::Color32::from_rgb(38, 94, 148));

    let manual_timeline = TimelineUiState {
        drag_active: true,
        ..Default::default()
    };
    let (manual_text, manual_color) = mode_signal(&manual_timeline, crate::i18n::UiLocale::ZhCn);
    assert_eq!(manual_text, "观察:手动");
    assert_eq!(manual_color, egui::Color32::from_rgb(125, 96, 28));
}

#[test]
fn feedback_tone_for_event_maps_warning_positive_and_info() {
    let warning = feedback_tone_for_event(&sample_rejected_event(1, 1).kind);
    assert_eq!(warning, FeedbackTone::Warning);

    let positive = feedback_tone_for_event(&sample_agent_moved_event(2, 2).kind);
    assert_eq!(positive, FeedbackTone::Positive);

    let runtime_warning = feedback_tone_for_event(
        &sample_runtime_event(3, 3, "runtime.economy.factory_production_blocked", "factory=factory.alpha recipe=recipe.motor reason=material_shortage detail=material_shortage:iron_ingot").kind,
    );
    assert_eq!(runtime_warning, FeedbackTone::Warning);

    let runtime_positive = feedback_tone_for_event(
        &sample_runtime_event(
            4,
            4,
            "runtime.economy.recipe_completed",
            "factory=factory.alpha recipe=recipe.motor outputs=motor_mk1x2",
        )
        .kind,
    );
    assert_eq!(runtime_positive, FeedbackTone::Positive);

    let info = feedback_tone_for_event(&WorldEventKind::LocationRegistered {
        location_id: "loc-1".to_string(),
        name: "alpha".to_string(),
        pos: oasis7::geometry::GeoPos::new(0.0, 0.0, 0.0),
        profile: Default::default(),
    });
    assert_eq!(info, FeedbackTone::Info);
}

#[test]
fn push_feedback_toast_clamps_queue_and_removes_oldest() {
    let mut feedback = FeedbackToastState::default();
    let locale = crate::i18n::UiLocale::EnUs;
    for id in 1..=(feedback_toast_cap() as u64 + 2) {
        push_feedback_toast(
            &mut feedback,
            &sample_rejected_event(id, id),
            10.0 + id as f64,
            locale,
        );
    }

    assert_eq!(feedback_toast_len(&feedback), feedback_toast_cap());
    let ids = feedback_toast_ids(&feedback);
    assert_eq!(ids, vec![3, 4, 5]);
}

#[test]
fn push_feedback_toast_uses_runtime_industry_friendly_detail() {
    let mut feedback = FeedbackToastState::default();
    let locale = crate::i18n::UiLocale::ZhCn;
    let event = sample_runtime_event(9, 9, "runtime.economy.factory_production_blocked", "factory=factory.alpha recipe=recipe.motor requester=agent.alpha reason=material_shortage detail=material_shortage:iron_ingot");

    push_feedback_toast(&mut feedback, &event, 12.0, locale);

    assert_eq!(
        feedback_toast_snapshot(&feedback, 0),
        Some((9, FeedbackTone::Warning, "操作受阻"))
    );
    let detail = feedback_toast_detail(&feedback, 0).expect("runtime toast detail");
    assert!(detail.contains("代价已显现"));
    assert!(detail.contains("factory.alpha"));
}

#[test]
fn push_feedback_toast_surfaces_reward_language_for_completed_output() {
    let mut feedback = FeedbackToastState::default();
    let locale = crate::i18n::UiLocale::EnUs;
    let event = sample_runtime_event(
        10,
        10,
        "runtime.economy.recipe_completed",
        "factory=factory.alpha recipe=recipe.motor requester=agent.alpha outputs=motor_mk1x2",
    );

    push_feedback_toast(&mut feedback, &event, 13.0, locale);

    let detail = feedback_toast_detail(&feedback, 0).expect("runtime reward toast detail");
    assert!(detail.contains("Reward earned"));
    assert!(detail.contains("produced"));
}

#[test]
fn sync_feedback_toasts_skips_history_then_tracks_new_events_only() {
    let mut feedback = FeedbackToastState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_rejected_event(1, 1), sample_agent_moved_event(2, 2)],
    );
    let locale = crate::i18n::UiLocale::ZhCn;

    sync_feedback_toasts(&mut feedback, &state, 20.0, locale);
    assert_eq!(feedback_toast_len(&feedback), 0);
    assert_eq!(feedback_last_seen_event_id(&feedback), Some(2));
    assert!(!feedback_action_feedback_seen(&feedback));

    state.events.push(sample_rejected_event(3, 3));
    sync_feedback_toasts(&mut feedback, &state, 21.0, locale);

    assert_eq!(feedback_last_seen_event_id(&feedback), Some(3));
    assert!(feedback_action_feedback_seen(&feedback));
    assert_eq!(feedback_toast_len(&feedback), 1);
    assert_eq!(
        feedback_toast_snapshot(&feedback, 0),
        Some((3, FeedbackTone::Warning, "操作受阻"))
    );
}

#[test]
fn sync_feedback_toasts_ignores_agent_not_found_noise() {
    let mut feedback = FeedbackToastState::default();
    let mut state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let locale = crate::i18n::UiLocale::ZhCn;

    sync_feedback_toasts(&mut feedback, &state, 20.0, locale);
    assert_eq!(feedback_toast_len(&feedback), 0);

    state
        .events
        .push(sample_agent_not_found_rejected_event(1, 1));
    sync_feedback_toasts(&mut feedback, &state, 21.0, locale);

    assert_eq!(feedback_last_seen_event_id(&feedback), Some(1));
    assert!(!feedback_action_feedback_seen(&feedback));
    assert_eq!(feedback_toast_len(&feedback), 0);
}

#[test]
fn resolve_player_guide_step_prioritizes_connection_panel_and_selection() {
    let hidden_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: true,
    };
    let open_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: false,
    };
    let empty_selection = crate::ViewerSelection::default();
    let selected = sample_selected_viewer_selection();

    assert_eq!(
        resolve_player_guide_step(
            &crate::ConnectionStatus::Connecting,
            &hidden_layout,
            &empty_selection
        ),
        PlayerGuideStep::ConnectWorld
    );
    assert_eq!(
        resolve_player_guide_step(
            &crate::ConnectionStatus::Connected,
            &hidden_layout,
            &empty_selection
        ),
        PlayerGuideStep::OpenPanel
    );
    assert_eq!(
        resolve_player_guide_step(
            &crate::ConnectionStatus::Connected,
            &open_layout,
            &empty_selection
        ),
        PlayerGuideStep::SelectTarget
    );
    assert_eq!(
        resolve_player_guide_step(&crate::ConnectionStatus::Connected, &open_layout, &selected),
        PlayerGuideStep::ExploreAction
    );
}

#[test]
fn player_onboarding_visibility_tracks_dismissed_step_only() {
    let mut onboarding = PlayerOnboardingState::default();
    assert!(should_show_player_onboarding_card(
        &onboarding,
        PlayerGuideStep::OpenPanel
    ));

    dismiss_player_onboarding_step(&mut onboarding, PlayerGuideStep::OpenPanel);
    assert!(!should_show_player_onboarding_card(
        &onboarding,
        PlayerGuideStep::OpenPanel
    ));
    assert!(should_show_player_onboarding_card(
        &onboarding,
        PlayerGuideStep::SelectTarget
    ));
}

#[test]
fn player_goal_hint_visibility_requires_hidden_panel_and_dismissed_step() {
    let hidden_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: true,
    };
    let open_layout = RightPanelLayoutState {
        top_panel_collapsed: false,
        panel_hidden: false,
    };
    let mut onboarding = PlayerOnboardingState::default();

    assert!(!should_show_player_goal_hint(
        &onboarding,
        PlayerGuideStep::OpenPanel,
        &hidden_layout
    ));

    dismiss_player_onboarding_step(&mut onboarding, PlayerGuideStep::OpenPanel);
    assert!(should_show_player_goal_hint(
        &onboarding,
        PlayerGuideStep::OpenPanel,
        &hidden_layout
    ));
    assert!(!should_show_player_goal_hint(
        &onboarding,
        PlayerGuideStep::OpenPanel,
        &open_layout
    ));
    assert!(!should_show_player_goal_hint(
        &onboarding,
        PlayerGuideStep::SelectTarget,
        &hidden_layout
    ));
}

#[test]
fn build_player_hud_snapshot_formats_connected_selected_state() {
    let state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_rejected_event(1, 1), sample_agent_moved_event(2, 2)],
    );
    let selection = sample_selected_viewer_selection();
    let snapshot = build_player_hud_snapshot(
        &state,
        &selection,
        PlayerGuideStep::ExploreAction,
        false,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.connection, "Connected");
    assert_eq!(snapshot.role, "Action commander");
    assert_eq!(snapshot.tick, 0);
    assert_eq!(snapshot.events, 2);
    assert_eq!(snapshot.objective, "Advance The Run");
    assert!(snapshot.selection.contains("agent-1"));
}

#[test]
fn build_player_hud_snapshot_uses_unselected_fallback_text() {
    let state = sample_viewer_state(crate::ConnectionStatus::Connecting, Vec::new());
    let selection = crate::ViewerSelection::default();
    let snapshot = build_player_hud_snapshot(
        &state,
        &selection,
        PlayerGuideStep::OpenPanel,
        false,
        crate::i18n::UiLocale::ZhCn,
    );

    assert_eq!(snapshot.connection, "连接中");
    assert_eq!(snapshot.role, "前线指挥员");
    assert_eq!(snapshot.tick, 0);
    assert_eq!(snapshot.events, 0);
    assert_eq!(snapshot.selection, "未选择");
    assert_eq!(snapshot.objective, "展开操作面板");
}

#[test]
fn build_player_hud_snapshot_prefers_post_onboarding_goal_and_role() {
    let mut state = sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    state.snapshot = Some(oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 42,
        config: oasis7::simulator::WorldConfig::default(),
        model: oasis7::simulator::WorldModel::default(),
        runtime_snapshot: None,
        player_gameplay: Some(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::BranchReady,
            goal_id: "post_onboarding.choose_first_expansion_tradeoff".to_string(),
            goal_kind: PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff,
            goal_title: "Choose the first expansion tradeoff".to_string(),
            objective: "canonical objective".to_string(),
            progress_detail: "canonical progress".to_string(),
            progress_percent: 92,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "canonical next step".to_string(),
            branch_hint: Some("Tradeoffs unlocked: throughput expansion".to_string()),
            available_actions: Vec::new(),
            recent_feedback: None,
            agent_claim: None,
        }),
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
    });

    let snapshot = build_player_hud_snapshot(
        &state,
        &crate::ViewerSelection::default(),
        PlayerGuideStep::ExploreAction,
        true,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.role, "First-line lead");
    assert_eq!(
        snapshot.objective,
        "Next Stage: Choose the First Expansion Tradeoff"
    );
    assert_eq!(snapshot.tick, 42);
}

#[test]
fn build_player_hud_snapshot_uses_post_onboarding_fallback_without_gameplay_snapshot() {
    let state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_runtime_event(
            7,
            7,
            "runtime.economy.recipe_started",
            "factory=factory.alpha recipe=recipe.motor requester=agent.alpha outputs=motor_mk1x2",
        )],
    );

    let snapshot = build_player_hud_snapshot(
        &state,
        &crate::ViewerSelection::default(),
        PlayerGuideStep::ExploreAction,
        true,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.role, "First-line lead");
    assert_eq!(
        snapshot.objective,
        "PostOnboarding: Stabilize Your First Line"
    );
}

#[test]
fn truncate_observe_text_keeps_short_text() {
    let text = "观察";
    assert_eq!(truncate_observe_text(text, 8), text);
}

#[test]
fn truncate_observe_text_supports_multibyte_chars() {
    let text = "观察模式状态很长很长";
    let truncated = truncate_observe_text(text, 6);
    assert_eq!(truncated.chars().count(), 6);
    assert!(truncated.ends_with('…'));
}

#[test]
fn event_row_preview_limit_uses_constant() {
    let long_line = "x".repeat(EVENT_ROW_LABEL_MAX_CHARS + 20);
    let preview = truncate_observe_text(&long_line, EVENT_ROW_LABEL_MAX_CHARS);
    assert_eq!(preview.chars().count(), EVENT_ROW_LABEL_MAX_CHARS);
    assert!(preview.ends_with('…'));
}

#[test]
fn rejection_event_count_only_counts_rejected_events() {
    use oasis7::geometry::GeoPos;

    let events = vec![
        WorldEvent {
            id: 1,
            time: 1,
            kind: WorldEventKind::LocationRegistered {
                location_id: "loc-1".to_string(),
                name: "Alpha".to_string(),
                pos: GeoPos::new(0.0, 0.0, 0.0),
                profile: Default::default(),
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 2,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound {
                    agent_id: "a-1".to_string(),
                },
            },
            runtime_event: None,
        },
    ];

    assert_eq!(rejection_event_count(&events), 1);
}
