mod actions;
mod parse;

use self::actions::{
    apply_layout_preset_automation, apply_timeline_seek_step, apply_visibility_action,
    automation_focus_radius_for_target, dispatch_agent_chat_step, dispatch_prompt_override_step,
    module_visibility_flag, resolve_target_entity, timeline_filter_flag,
};
use self::parse::config_from_values;
#[cfg(any(test, target_arch = "wasm32"))]
use self::parse::{parse_steps, parse_target};
#[cfg(target_arch = "wasm32")]
use self::parse::parse_mode;
use bevy::prelude::*;
use std::collections::VecDeque;

use super::auto_focus::focus_selection_with_transform;
use super::camera_controls::orbit_min_radius;
use super::selection_linking::apply_selection;
use super::*;

const AUTOMATION_STEPS_ENV: &str = "OASIS7_VIEWER_AUTOMATION_STEPS";
const AUTO_SELECT_ENV: &str = "OASIS7_VIEWER_AUTO_SELECT";
const AUTO_SELECT_TARGET_ENV: &str = "OASIS7_VIEWER_AUTO_SELECT_TARGET";
const VIEWER_PLAYER_ID_DEFAULT: &str = "viewer-player";
const VIEWER_PLAYER_ID_ENV: &str = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
#[cfg(target_arch = "wasm32")]
const VIEWER_AUTH_BOOTSTRAP_OBJECT: &str = "__OASIS7_VIEWER_AUTH_ENV";

#[derive(Resource, Clone, Debug, PartialEq)]
pub(super) struct ViewerAutomationConfig {
    pub enabled: bool,
    pub steps: Vec<ViewerAutomationStep>,
}

impl Default for ViewerAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            steps: Vec::new(),
        }
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub(super) struct ViewerAutomationState {
    startup_step_index: usize,
    wait_until_secs: Option<f64>,
    runtime_steps: VecDeque<ViewerAutomationStep>,
    auth_nonce_floor: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ViewerAutomationStep {
    WaitSeconds(f64),
    SetMode(ViewerCameraMode),
    Focus(ViewerAutomationTarget),
    FocusSelection,
    Pan(Vec3),
    ZoomFactor(f32),
    OrbitDeg {
        yaw: f32,
        pitch: f32,
    },
    Select(ViewerAutomationTarget),
    PanelVisibility(ViewerAutomationVisibilityAction),
    TopPanelVisibility(ViewerAutomationVisibilityAction),
    ModuleVisibility {
        module: ViewerAutomationPanelModule,
        action: ViewerAutomationVisibilityAction,
    },
    TimelineSeek {
        tick: u64,
    },
    TimelineFilter {
        kind: crate::timeline_controls::TimelineMarkKindPublic,
        action: ViewerAutomationVisibilityAction,
    },
    TimelineJump {
        kind: crate::timeline_controls::TimelineMarkKindPublic,
    },
    SendAgentChat {
        agent_id: String,
        message: String,
    },
    ApplyPromptOverride {
        agent_id: String,
        field: ViewerAutomationPromptField,
        value: ViewerAutomationPromptValue,
    },
    SetLocale(ViewerAutomationLocaleAction),
    ApplyLayoutPreset(ViewerAutomationLayoutPreset),
    CycleMaterialVariant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationTarget {
    FirstKind(&'static str),
    KindId { kind: &'static str, id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationVisibilityAction {
    Show,
    Hide,
    Toggle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationPanelModule {
    Controls,
    Overview,
    Chat,
    Overlay,
    Diagnosis,
    EventLink,
    Timeline,
    Details,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationLocaleAction {
    Zh,
    En,
    Toggle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationLayoutPreset {
    Mission,
    Command,
    Intel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationPromptField {
    System,
    ShortTerm,
    LongTerm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ViewerAutomationPromptValue {
    Set(String),
    Clear,
}

const TARGET_KIND_AGENT: &str = "agent";
const TARGET_KIND_LOCATION: &str = "location";
const TARGET_KIND_ASSET: &str = "asset";
const TARGET_KIND_MODULE_VISUAL: &str = "module_visual";
const TARGET_KIND_POWER_PLANT: &str = "power_plant";
const TARGET_KIND_CHUNK: &str = "chunk";
const TARGET_KIND_FRAGMENT: &str = "fragment";
const POWER_FOCUS_RADIUS_SCALE_FROM_BASE: f32 = 3.0;
const POWER_FOCUS_RADIUS_MIN_M: f32 = 32.0;

enum StepResult {
    Applied,
    AppliedYield,
    Pending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepSource {
    StartupConfig,
    RuntimeQueue,
}

pub(super) fn viewer_automation_config_from_env() -> ViewerAutomationConfig {
    config_from_values(
        crate::viewer_env::viewer_env_var(AUTO_SELECT_ENV),
        crate::viewer_env::viewer_env_var(AUTO_SELECT_TARGET_ENV),
        crate::viewer_env::viewer_env_var(AUTOMATION_STEPS_ENV),
    )
}

pub(super) fn run_viewer_automation(
    time: Res<Time>,
    config: Res<ViewerAutomationConfig>,
    mut state: ResMut<ViewerAutomationState>,
    mut camera_mode: ResMut<ViewerCameraMode>,
    mut right_panel_layout: ResMut<RightPanelLayoutState>,
    mut module_visibility: ResMut<
        crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    >,
    mut i18n: ResMut<crate::i18n::UiI18n>,
    mut variant_preview: ResMut<MaterialVariantPreviewState>,
    render_resources: (
        Res<Viewer3dConfig>,
        Option<Res<Viewer3dAssets>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    runtime_resources: (
        Res<Viewer3dScene>,
        Option<Res<ViewerClient>>,
        Option<Res<ViewerState>>,
        Option<Res<ViewerControlProfileState>>,
    ),
    mut timeline: ResMut<crate::timeline_controls::TimelineUiState>,
    mut timeline_filters: Option<ResMut<crate::timeline_controls::TimelineMarkFilterState>>,
    mut selection: ResMut<ViewerSelection>,
    mut queries: ParamSet<(
        Query<(&mut OrbitCamera, &mut Transform, &mut Projection), With<Viewer3dCamera>>,
        Query<(&mut Transform, Option<&BaseScale>)>,
        Query<(&Transform, Option<&BaseScale>)>,
        Query<&LocationMarker>,
    )>,
) {
    let (viewer_config, assets, mut materials) = render_resources;
    let (scene, viewer_client, viewer_state, control_profile) = runtime_resources;
    let now = time.elapsed_secs_f64();
    if let Some(wait_until_secs) = state.wait_until_secs {
        if now < wait_until_secs {
            return;
        }
        state.wait_until_secs = None;
    }

    loop {
        let Some((source, step)) = next_step(&config, &state) else {
            return;
        };

        let result = apply_step(
            step,
            now,
            &scene,
            &viewer_config,
            &mut camera_mode,
            &mut right_panel_layout,
            &mut module_visibility,
            &mut i18n,
            &mut variant_preview,
            assets.as_deref(),
            &mut materials,
            viewer_client.as_deref(),
            viewer_state.as_deref(),
            control_profile.as_deref(),
            &mut timeline,
            timeline_filters.as_deref_mut(),
            &mut selection,
            &mut queries,
            &mut state,
        );
        match result {
            StepResult::Applied => {
                advance_step(&mut state, source);
                continue;
            }
            StepResult::AppliedYield => {
                advance_step(&mut state, source);
                return;
            }
            StepResult::Pending => return,
        }
    }
}

fn next_step(
    config: &ViewerAutomationConfig,
    state: &ViewerAutomationState,
) -> Option<(StepSource, ViewerAutomationStep)> {
    if let Some(step) = state.runtime_steps.front().cloned() {
        return Some((StepSource::RuntimeQueue, step));
    }
    if !config.enabled {
        return None;
    }
    config
        .steps
        .get(state.startup_step_index)
        .cloned()
        .map(|step| (StepSource::StartupConfig, step))
}

fn advance_step(state: &mut ViewerAutomationState, source: StepSource) {
    match source {
        StepSource::StartupConfig => {
            state.startup_step_index += 1;
        }
        StepSource::RuntimeQueue => {
            let _ = state.runtime_steps.pop_front();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn enqueue_runtime_steps(
    state: &mut ViewerAutomationState,
    steps: impl IntoIterator<Item = ViewerAutomationStep>,
) {
    state.runtime_steps.extend(steps);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn parse_automation_steps(raw: &str) -> Vec<ViewerAutomationStep> {
    parse_steps(Some(raw))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn parse_automation_mode(raw: &str) -> Option<ViewerCameraMode> {
    parse_mode(raw)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn parse_automation_target(raw: &str) -> Option<ViewerAutomationTarget> {
    parse_target(raw)
}

fn apply_step(
    step: ViewerAutomationStep,
    now: f64,
    scene: &Viewer3dScene,
    viewer_config: &Viewer3dConfig,
    camera_mode: &mut ViewerCameraMode,
    right_panel_layout: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    i18n: &mut crate::i18n::UiI18n,
    variant_preview: &mut MaterialVariantPreviewState,
    assets: Option<&Viewer3dAssets>,
    materials: &mut Assets<StandardMaterial>,
    viewer_client: Option<&ViewerClient>,
    viewer_state: Option<&ViewerState>,
    control_profile: Option<&ViewerControlProfileState>,
    timeline: &mut crate::timeline_controls::TimelineUiState,
    timeline_filters: Option<&mut crate::timeline_controls::TimelineMarkFilterState>,
    selection: &mut ViewerSelection,
    queries: &mut ParamSet<(
        Query<(&mut OrbitCamera, &mut Transform, &mut Projection), With<Viewer3dCamera>>,
        Query<(&mut Transform, Option<&BaseScale>)>,
        Query<(&Transform, Option<&BaseScale>)>,
        Query<&LocationMarker>,
    )>,
    state: &mut ViewerAutomationState,
) -> StepResult {
    match step {
        ViewerAutomationStep::WaitSeconds(seconds) => {
            state.wait_until_secs = Some(now + seconds.max(0.0));
            StepResult::AppliedYield
        }
        ViewerAutomationStep::SetMode(mode) => {
            if *camera_mode != mode {
                *camera_mode = mode;
                StepResult::AppliedYield
            } else {
                StepResult::Applied
            }
        }
        ViewerAutomationStep::Focus(target) => {
            let Some((entity, selection_kind, _)) = resolve_target_entity(scene, &target) else {
                return StepResult::Pending;
            };

            let (target_translation, target_base_scale) = {
                let transform_query = queries.p2();
                let Ok((target_transform, target_base_scale)) = transform_query.get(entity) else {
                    return StepResult::Pending;
                };
                (
                    target_transform.translation,
                    target_base_scale.map(|base_scale| base_scale.0),
                )
            };

            let mut camera_query = queries.p0();
            let Ok((mut orbit, mut camera_transform, _)) = camera_query.single_mut() else {
                return StepResult::Pending;
            };
            orbit.focus = target_translation;
            if let Some(target_radius) = automation_focus_radius_for_target(
                selection_kind,
                target_base_scale,
                viewer_config.effective_cm_to_unit(),
            ) {
                let min_radius = orbit_min_radius(viewer_config.effective_cm_to_unit());
                orbit.radius = target_radius.clamp(min_radius, ORBIT_MAX_RADIUS);
            }
            orbit.apply_to_transform(&mut camera_transform);
            StepResult::Applied
        }
        ViewerAutomationStep::FocusSelection => {
            let Some(current) = selection.current.clone() else {
                return StepResult::Applied;
            };

            let focus = {
                let transform_query = queries.p2();
                let Ok((target_transform, _)) = transform_query.get(current.entity) else {
                    return StepResult::Pending;
                };
                target_transform.translation
            };

            let mut camera_query = queries.p0();
            let Ok((mut orbit, mut camera_transform, mut projection)) = camera_query.single_mut()
            else {
                return StepResult::Pending;
            };

            focus_selection_with_transform(
                &current,
                focus,
                scene,
                viewer_config,
                camera_mode,
                &mut orbit,
                &mut camera_transform,
                &mut projection,
            );
            StepResult::Applied
        }
        ViewerAutomationStep::Pan(delta) => {
            let mut camera_query = queries.p0();
            let Ok((mut orbit, mut camera_transform, _)) = camera_query.single_mut() else {
                return StepResult::Pending;
            };
            orbit.focus += delta;
            orbit.apply_to_transform(&mut camera_transform);
            StepResult::Applied
        }
        ViewerAutomationStep::ZoomFactor(factor) => {
            let mut camera_query = queries.p0();
            let Ok((mut orbit, mut camera_transform, mut projection)) = camera_query.single_mut()
            else {
                return StepResult::Pending;
            };

            let min_radius = orbit_min_radius(viewer_config.effective_cm_to_unit());
            orbit.radius = (orbit.radius * factor.max(0.01)).clamp(min_radius, ORBIT_MAX_RADIUS);
            if *camera_mode == ViewerCameraMode::TwoD {
                if let Projection::Orthographic(ortho) = &mut *projection {
                    ortho.scale =
                        (ortho.scale * factor.max(0.01)).clamp(ORTHO_MIN_SCALE, ORTHO_MAX_SCALE);
                } else {
                    *projection = camera_projection_for_mode(ViewerCameraMode::TwoD, viewer_config);
                }
            }
            orbit.apply_to_transform(&mut camera_transform);
            StepResult::Applied
        }
        ViewerAutomationStep::OrbitDeg { yaw, pitch } => {
            if *camera_mode != ViewerCameraMode::ThreeD {
                return StepResult::Applied;
            }
            let mut camera_query = queries.p0();
            let Ok((mut orbit, mut camera_transform, _)) = camera_query.single_mut() else {
                return StepResult::Pending;
            };
            orbit.yaw += yaw.to_radians();
            orbit.pitch = (orbit.pitch + pitch.to_radians()).clamp(-1.54, 1.54);
            orbit.apply_to_transform(&mut camera_transform);
            StepResult::Applied
        }
        ViewerAutomationStep::Select(target) => {
            let Some((entity, kind, id)) = resolve_target_entity(scene, &target) else {
                return StepResult::Pending;
            };
            let name = if kind == SelectionKind::Location {
                queries
                    .p3()
                    .get(entity)
                    .ok()
                    .map(|marker| marker.name.clone())
            } else {
                None
            };

            apply_selection(
                selection,
                &mut queries.p1(),
                viewer_config,
                entity,
                kind,
                id,
                name,
            );
            StepResult::Applied
        }
        ViewerAutomationStep::PanelVisibility(action) => {
            let current_visible = !right_panel_layout.panel_hidden;
            let next_visible = apply_visibility_action(current_visible, action);
            right_panel_layout.panel_hidden = !next_visible;
            StepResult::Applied
        }
        ViewerAutomationStep::TopPanelVisibility(action) => {
            let current_visible = !right_panel_layout.top_panel_collapsed;
            let next_visible = apply_visibility_action(current_visible, action);
            right_panel_layout.top_panel_collapsed = !next_visible;
            StepResult::Applied
        }
        ViewerAutomationStep::ModuleVisibility { module, action } => {
            let flag = module_visibility_flag(module_visibility, module);
            let current_visible = *flag;
            *flag = apply_visibility_action(current_visible, action);
            StepResult::Applied
        }
        ViewerAutomationStep::TimelineSeek { tick } => {
            apply_timeline_seek_step(timeline, viewer_client, control_profile, tick);
            StepResult::Applied
        }
        ViewerAutomationStep::TimelineFilter { kind, action } => {
            if let Some(filters) = timeline_filters {
                let flag = timeline_filter_flag(filters, kind);
                *flag = apply_visibility_action(*flag, action);
            } else {
                bevy::log::warn!("viewer automation timeline filter step ignored: filters missing");
            }
            StepResult::Applied
        }
        ViewerAutomationStep::TimelineJump { kind } => {
            let Some(viewer_state) = viewer_state else {
                bevy::log::warn!(
                    "viewer automation timeline jump step ignored: viewer state missing"
                );
                return StepResult::Applied;
            };
            crate::timeline_controls::timeline_mark_jump_action(
                viewer_state,
                timeline,
                timeline_filters.as_deref(),
                kind,
            );
            StepResult::Applied
        }
        ViewerAutomationStep::SendAgentChat { agent_id, message } => {
            if let Err(err) =
                dispatch_agent_chat_step(viewer_client, state, agent_id.as_str(), message.as_str())
            {
                bevy::log::warn!("viewer automation chat step ignored: {err}");
            }
            StepResult::Applied
        }
        ViewerAutomationStep::ApplyPromptOverride {
            agent_id,
            field,
            value,
        } => {
            if let Err(err) = dispatch_prompt_override_step(
                viewer_client,
                state,
                agent_id.as_str(),
                field,
                &value,
            ) {
                bevy::log::warn!("viewer automation prompt step ignored: {err}");
            }
            StepResult::Applied
        }
        ViewerAutomationStep::SetLocale(action) => {
            i18n.locale = match action {
                ViewerAutomationLocaleAction::Zh => crate::i18n::UiLocale::ZhCn,
                ViewerAutomationLocaleAction::En => crate::i18n::UiLocale::EnUs,
                ViewerAutomationLocaleAction::Toggle => i18n.locale.toggled(),
            };
            StepResult::Applied
        }
        ViewerAutomationStep::ApplyLayoutPreset(preset) => {
            apply_layout_preset_automation(right_panel_layout, module_visibility, preset);
            StepResult::Applied
        }
        ViewerAutomationStep::CycleMaterialVariant => {
            variant_preview.active = variant_preview.active.next();
            if let Some(assets) = assets {
                apply_material_variant_to_scene_materials(
                    materials,
                    assets,
                    viewer_config,
                    variant_preview.active,
                );
            }
            StepResult::Applied
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_target_supports_first_and_explicit_variants() {
        assert_eq!(
            parse_target("first_agent"),
            Some(ViewerAutomationTarget::FirstKind(TARGET_KIND_AGENT))
        );
        assert_eq!(
            parse_target("first_location"),
            Some(ViewerAutomationTarget::FirstKind(TARGET_KIND_LOCATION))
        );
        assert_eq!(
            parse_target("first:power_plant"),
            Some(ViewerAutomationTarget::FirstKind(TARGET_KIND_POWER_PLANT))
        );
        assert_eq!(
            parse_target("agent:agent-1"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_AGENT,
                id: "agent-1".to_string(),
            })
        );
        assert_eq!(
            parse_target("location:loc-2"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_LOCATION,
                id: "loc-2".to_string(),
            })
        );
        assert_eq!(
            parse_target("power-plant:plant-1"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_POWER_PLANT,
                id: "plant-1".to_string(),
            })
        );
        assert_eq!(
            parse_target("modulevisual:mv-1"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_MODULE_VISUAL,
                id: "mv-1".to_string(),
            })
        );
        assert_eq!(
            parse_target("fragment:frag-2"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_FRAGMENT,
                id: "frag-2".to_string(),
            })
        );
        assert_eq!(
            parse_target("asset:a1"),
            Some(ViewerAutomationTarget::KindId {
                kind: TARGET_KIND_ASSET,
                id: "a1".to_string(),
            })
        );
        assert_eq!(parse_target("unknown:x"), None);
        assert_eq!(parse_target(""), None);
    }

    #[test]
    fn parse_steps_supports_camera_and_selection_actions() {
        let steps = parse_steps(Some(
            "mode=3d;wait=0.6;focus=agent:agent-0;pan=1,0,-2;zoom=0.8;orbit=10,-4;select=agent:agent-0",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::SetMode(ViewerCameraMode::ThreeD),
                ViewerAutomationStep::WaitSeconds(0.6),
                ViewerAutomationStep::Focus(ViewerAutomationTarget::KindId {
                    kind: TARGET_KIND_AGENT,
                    id: "agent-0".to_string(),
                }),
                ViewerAutomationStep::Pan(Vec3::new(1.0, 0.0, -2.0)),
                ViewerAutomationStep::ZoomFactor(0.8),
                ViewerAutomationStep::OrbitDeg {
                    yaw: 10.0,
                    pitch: -4.0
                },
                ViewerAutomationStep::Select(ViewerAutomationTarget::KindId {
                    kind: TARGET_KIND_AGENT,
                    id: "agent-0".to_string(),
                }),
            ]
        );
    }

    #[test]
    fn parse_steps_supports_panel_module_focus_selection_and_variant_actions() {
        let steps = parse_steps(Some(
            "panel=toggle;module=chat:hide;focus=selection;focus_selection=current;material_variant=next",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::PanelVisibility(ViewerAutomationVisibilityAction::Toggle),
                ViewerAutomationStep::ModuleVisibility {
                    module: ViewerAutomationPanelModule::Chat,
                    action: ViewerAutomationVisibilityAction::Hide,
                },
                ViewerAutomationStep::FocusSelection,
                ViewerAutomationStep::FocusSelection,
                ViewerAutomationStep::CycleMaterialVariant,
            ]
        );
    }

    #[test]
    fn parse_steps_supports_top_panel_locale_and_layout_actions() {
        let steps = parse_steps(Some(
            "top_panel=hide;locale=en;language=toggle;layout=command;top=show",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::TopPanelVisibility(ViewerAutomationVisibilityAction::Hide),
                ViewerAutomationStep::SetLocale(ViewerAutomationLocaleAction::En),
                ViewerAutomationStep::SetLocale(ViewerAutomationLocaleAction::Toggle),
                ViewerAutomationStep::ApplyLayoutPreset(ViewerAutomationLayoutPreset::Command),
                ViewerAutomationStep::TopPanelVisibility(ViewerAutomationVisibilityAction::Show),
            ]
        );
    }

    #[test]
    fn parse_steps_supports_timeline_seek_filter_and_jump_actions() {
        let steps = parse_steps(Some(
            "timeline_seek=42;timeline_filter=err:hide;timeline_filter=llm:toggle;timeline_jump=peak",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::TimelineSeek { tick: 42 },
                ViewerAutomationStep::TimelineFilter {
                    kind: crate::timeline_controls::TimelineMarkKindPublic::Error,
                    action: ViewerAutomationVisibilityAction::Hide,
                },
                ViewerAutomationStep::TimelineFilter {
                    kind: crate::timeline_controls::TimelineMarkKindPublic::Llm,
                    action: ViewerAutomationVisibilityAction::Toggle,
                },
                ViewerAutomationStep::TimelineJump {
                    kind: crate::timeline_controls::TimelineMarkKindPublic::Peak,
                },
            ]
        );
    }

    #[test]
    fn parse_steps_supports_chat_and_prompt_actions() {
        let steps = parse_steps(Some(
            "chat=agent-1|hello+world%21;prompt_system=agent-1|clear;prompt_short=agent-1|Need%20power%20first",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::SendAgentChat {
                    agent_id: "agent-1".to_string(),
                    message: "hello world!".to_string(),
                },
                ViewerAutomationStep::ApplyPromptOverride {
                    agent_id: "agent-1".to_string(),
                    field: ViewerAutomationPromptField::System,
                    value: ViewerAutomationPromptValue::Clear,
                },
                ViewerAutomationStep::ApplyPromptOverride {
                    agent_id: "agent-1".to_string(),
                    field: ViewerAutomationPromptField::ShortTerm,
                    value: ViewerAutomationPromptValue::Set("Need power first".to_string()),
                },
            ]
        );
    }

    #[test]
    fn parse_steps_ignores_invalid_module_and_variant_actions() {
        let steps = parse_steps(Some(
            "module=chat;module=unknown:show;module=timeline:toggle;material_variant=bad;variant=cycle",
        ));
        assert_eq!(
            steps,
            vec![
                ViewerAutomationStep::ModuleVisibility {
                    module: ViewerAutomationPanelModule::Timeline,
                    action: ViewerAutomationVisibilityAction::Toggle,
                },
                ViewerAutomationStep::CycleMaterialVariant,
            ]
        );
    }

    #[test]
    fn parse_steps_ignores_invalid_locale_and_layout_actions() {
        let steps = parse_steps(Some("locale=jp;language=english;layout=unknown"));
        assert_eq!(
            steps,
            vec![ViewerAutomationStep::SetLocale(
                ViewerAutomationLocaleAction::En
            )]
        );
    }

    #[test]
    fn parse_steps_ignores_invalid_timeline_actions() {
        let steps = parse_steps(Some(
            "timeline_seek=-1;timeline_seek=abc;timeline_filter=foo:show;timeline_filter=err:unknown;timeline_jump=other;timeline_mark_jump=error",
        ));
        assert_eq!(
            steps,
            vec![ViewerAutomationStep::TimelineJump {
                kind: crate::timeline_controls::TimelineMarkKindPublic::Error,
            }]
        );
    }

    #[test]
    fn parse_steps_ignores_invalid_chat_and_prompt_actions() {
        let steps = parse_steps(Some(
            "chat=agent-1;chat=agent-2|%ZZ;prompt_system=agent-1|;prompt_long=|hello;prompt_short=agent-3|default",
        ));
        assert_eq!(
            steps,
            vec![ViewerAutomationStep::ApplyPromptOverride {
                agent_id: "agent-3".to_string(),
                field: ViewerAutomationPromptField::ShortTerm,
                value: ViewerAutomationPromptValue::Clear,
            }]
        );
    }

    #[test]
    fn apply_visibility_action_respects_show_hide_toggle() {
        assert!(apply_visibility_action(
            false,
            ViewerAutomationVisibilityAction::Show
        ));
        assert!(!apply_visibility_action(
            true,
            ViewerAutomationVisibilityAction::Hide
        ));
        assert!(!apply_visibility_action(
            true,
            ViewerAutomationVisibilityAction::Toggle
        ));
        assert!(apply_visibility_action(
            false,
            ViewerAutomationVisibilityAction::Toggle
        ));
    }

    #[test]
    fn apply_layout_preset_automation_updates_panel_and_module_visibility() {
        let mut layout_state = RightPanelLayoutState {
            top_panel_collapsed: true,
            panel_hidden: true,
        };
        let mut module_visibility =
            crate::right_panel_module_visibility::RightPanelModuleVisibilityState::default();

        apply_layout_preset_automation(
            &mut layout_state,
            &mut module_visibility,
            ViewerAutomationLayoutPreset::Intel,
        );
        assert!(!layout_state.panel_hidden);
        assert!(!layout_state.top_panel_collapsed);
        assert!(!module_visibility.show_controls);
        assert!(module_visibility.show_overview);
        assert!(!module_visibility.show_chat);
        assert!(module_visibility.show_event_link);
        assert!(module_visibility.show_timeline);
        assert!(module_visibility.show_details);
    }

    #[test]
    fn apply_layout_preset_automation_command_keeps_player_surface_compact() {
        let mut layout_state = RightPanelLayoutState {
            top_panel_collapsed: true,
            panel_hidden: true,
        };
        let mut module_visibility =
            crate::right_panel_module_visibility::RightPanelModuleVisibilityState::default();

        apply_layout_preset_automation(
            &mut layout_state,
            &mut module_visibility,
            ViewerAutomationLayoutPreset::Command,
        );

        assert!(!layout_state.panel_hidden);
        assert!(!layout_state.top_panel_collapsed);
        assert!(!module_visibility.show_controls);
        assert!(!module_visibility.show_overview);
        assert!(module_visibility.show_chat);
        assert!(!module_visibility.show_event_link);
        assert!(!module_visibility.show_timeline);
        assert!(!module_visibility.show_details);
    }

    #[test]
    fn config_from_values_uses_auto_select_when_steps_absent() {
        let config = config_from_values(
            Some("1".to_string()),
            Some("agent:agent-2".to_string()),
            None,
        );
        assert!(config.enabled);
        assert_eq!(
            config.steps,
            vec![ViewerAutomationStep::Select(
                ViewerAutomationTarget::KindId {
                    kind: TARGET_KIND_AGENT,
                    id: "agent-2".to_string(),
                }
            )]
        );
    }

    #[test]
    fn config_from_values_prioritizes_explicit_steps() {
        let config = config_from_values(
            Some("1".to_string()),
            Some("agent:agent-2".to_string()),
            Some("wait=0.2;select=first_agent".to_string()),
        );
        assert!(config.enabled);
        assert_eq!(
            config.steps,
            vec![
                ViewerAutomationStep::WaitSeconds(0.2),
                ViewerAutomationStep::Select(ViewerAutomationTarget::FirstKind(TARGET_KIND_AGENT)),
            ]
        );
    }

    #[test]
    fn resolve_target_entity_supports_extended_scene_kinds() {
        let mut scene = Viewer3dScene::default();
        scene
            .agent_entities
            .insert("agent-1".to_string(), Entity::from_bits(1));
        scene
            .location_entities
            .insert("loc-1".to_string(), Entity::from_bits(2));
        scene
            .location_entities
            .insert("frag-2".to_string(), Entity::from_bits(3));
        scene
            .asset_entities
            .insert("asset-1".to_string(), Entity::from_bits(4));
        scene
            .module_visual_entities
            .insert("mv-1".to_string(), Entity::from_bits(5));
        scene
            .power_plant_entities
            .insert("plant-1".to_string(), Entity::from_bits(6));
        scene
            .chunk_entities
            .insert("chunk-1".to_string(), Entity::from_bits(7));

        let fragment_target = ViewerAutomationTarget::FirstKind(TARGET_KIND_FRAGMENT);
        let Some((fragment_entity, fragment_kind, fragment_id)) =
            resolve_target_entity(&scene, &fragment_target)
        else {
            panic!("fragment target should resolve");
        };
        assert_eq!(fragment_entity, Entity::from_bits(3));
        assert_eq!(fragment_kind, SelectionKind::Fragment);
        assert_eq!(fragment_id, "frag-2");

        let module_target = ViewerAutomationTarget::KindId {
            kind: TARGET_KIND_MODULE_VISUAL,
            id: "mv-1".to_string(),
        };
        let Some((module_entity, module_kind, module_id)) =
            resolve_target_entity(&scene, &module_target)
        else {
            panic!("module_visual target should resolve");
        };
        assert_eq!(module_entity, Entity::from_bits(5));
        assert_eq!(module_kind, SelectionKind::Asset);
        assert_eq!(module_id, "mv-1");

        let chunk_target = ViewerAutomationTarget::KindId {
            kind: TARGET_KIND_CHUNK,
            id: "chunk-1".to_string(),
        };
        let Some((chunk_entity, chunk_kind, chunk_id)) =
            resolve_target_entity(&scene, &chunk_target)
        else {
            panic!("chunk target should resolve");
        };
        assert_eq!(chunk_entity, Entity::from_bits(7));
        assert_eq!(chunk_kind, SelectionKind::Chunk);
        assert_eq!(chunk_id, "chunk-1");
    }

    #[test]
    fn automation_focus_radius_for_power_target_has_reasonable_floor() {
        let cm_to_unit = 0.0000384;
        let radius = automation_focus_radius_for_target(
            SelectionKind::PowerPlant,
            Some(Vec3::splat(0.012)),
            cm_to_unit,
        )
        .expect("power target radius should resolve");
        let min_floor = POWER_FOCUS_RADIUS_MIN_M * cm_to_unit * 100.0;
        assert!(radius >= min_floor);
        assert!(radius > 0.0);
    }

    #[test]
    fn automation_focus_radius_for_non_power_target_is_none() {
        assert_eq!(
            automation_focus_radius_for_target(
                SelectionKind::Location,
                Some(Vec3::splat(0.5)),
                0.0000384
            ),
            None
        );
    }
}
