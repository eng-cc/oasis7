use super::*;
use std::collections::HashMap;

struct TargetKindSpec<'a> {
    selection_kind: SelectionKind,
    entities: &'a HashMap<String, Entity>,
    first_filter: fn(&str) -> bool,
}

pub(super) fn module_visibility_flag(
    state: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    module: ViewerAutomationPanelModule,
) -> &mut bool {
    match module {
        ViewerAutomationPanelModule::Controls => &mut state.show_controls,
        ViewerAutomationPanelModule::Overview => &mut state.show_overview,
        ViewerAutomationPanelModule::Chat => &mut state.show_chat,
        ViewerAutomationPanelModule::Overlay => &mut state.show_overlay,
        ViewerAutomationPanelModule::Diagnosis => &mut state.show_diagnosis,
        ViewerAutomationPanelModule::EventLink => &mut state.show_event_link,
        ViewerAutomationPanelModule::Timeline => &mut state.show_timeline,
        ViewerAutomationPanelModule::Details => &mut state.show_details,
    }
}

pub(super) fn apply_visibility_action(
    current_visible: bool,
    action: ViewerAutomationVisibilityAction,
) -> bool {
    match action {
        ViewerAutomationVisibilityAction::Show => true,
        ViewerAutomationVisibilityAction::Hide => false,
        ViewerAutomationVisibilityAction::Toggle => !current_visible,
    }
}

pub(super) fn apply_layout_preset_automation(
    layout_state: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    preset: ViewerAutomationLayoutPreset,
) {
    layout_state.panel_hidden = false;
    layout_state.top_panel_collapsed = false;
    module_visibility.show_controls = false;
    module_visibility.show_overlay = false;
    module_visibility.show_diagnosis = false;

    match preset {
        ViewerAutomationLayoutPreset::Mission => {
            module_visibility.show_overview = false;
            module_visibility.show_chat = false;
            module_visibility.show_event_link = false;
            module_visibility.show_timeline = false;
            module_visibility.show_details = false;
        }
        ViewerAutomationLayoutPreset::Command => {
            module_visibility.show_overview = false;
            module_visibility.show_chat = true;
            module_visibility.show_event_link = false;
            module_visibility.show_timeline = false;
            module_visibility.show_details = false;
        }
        ViewerAutomationLayoutPreset::Intel => {
            module_visibility.show_overview = true;
            module_visibility.show_chat = false;
            module_visibility.show_event_link = true;
            module_visibility.show_timeline = true;
            module_visibility.show_details = true;
        }
    }
}

pub(super) fn apply_timeline_seek_step(
    timeline: &mut crate::timeline_controls::TimelineUiState,
    viewer_client: Option<&ViewerClient>,
    control_profile: Option<&ViewerControlProfileState>,
    tick: u64,
) {
    timeline.target_tick = tick;
    timeline.manual_override = true;
    timeline.drag_active = false;

    if let Some(client) = viewer_client {
        if !viewer_seek_supported(control_profile) {
            return;
        }
        let _ = dispatch_viewer_control(
            client,
            control_profile,
            oasis7::viewer::ViewerControl::Seek { tick },
            None,
        );
    }
}

pub(super) fn timeline_filter_flag(
    filters: &mut crate::timeline_controls::TimelineMarkFilterState,
    kind: crate::timeline_controls::TimelineMarkKindPublic,
) -> &mut bool {
    match kind {
        crate::timeline_controls::TimelineMarkKindPublic::Error => &mut filters.show_error,
        crate::timeline_controls::TimelineMarkKindPublic::Llm => &mut filters.show_llm,
        crate::timeline_controls::TimelineMarkKindPublic::Peak => &mut filters.show_peak,
    }
}

#[derive(Clone, Debug)]
struct ViewerAutomationAuthSigner {
    player_id: String,
    public_key: String,
    private_key: String,
}

pub(super) fn dispatch_agent_chat_step(
    viewer_client: Option<&ViewerClient>,
    automation_state: &mut ViewerAutomationState,
    agent_id: &str,
    message: &str,
) -> Result<(), String> {
    let client = viewer_client.ok_or_else(|| "viewer client unavailable".to_string())?;
    let signer = resolve_automation_auth_signer()?;
    let register_nonce = next_auth_nonce(automation_state);
    let mut session_register = oasis7::viewer::AuthoritativeSessionRegisterRequest {
        player_id: signer.player_id.clone(),
        public_key: Some(signer.public_key.clone()),
        auth: None,
        requested_agent_id: Some(agent_id.to_string()),
        force_rebind: false,
    };
    let register_proof = oasis7::viewer::sign_session_register_auth_proof(
        &session_register,
        register_nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )
    .map_err(|err| format!("sign session register failed: {err}"))?;
    session_register.auth = Some(register_proof);
    let nonce = next_auth_nonce(automation_state);
    let mut request = oasis7::viewer::AgentChatRequest {
        agent_id: agent_id.to_string(),
        message: message.to_string(),
        player_id: Some(signer.player_id.clone()),
        public_key: Some(signer.public_key.clone()),
        auth: None,
        intent_tick: None,
        intent_seq: Some(nonce),
    };
    let proof = oasis7::viewer::sign_agent_chat_auth_proof(
        &request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )
    .map_err(|err| format!("sign agent chat failed: {err}"))?;
    request.auth = Some(proof);
    client
        .tx
        .send(oasis7::viewer::ViewerRequest::AuthoritativeRecovery {
            command: oasis7::viewer::AuthoritativeRecoveryCommand::RegisterSession {
                request: session_register,
            },
        })
        .map_err(|err| format!("send session register failed: {err}"))?;
    client
        .tx
        .send(oasis7::viewer::ViewerRequest::AgentChat { request })
        .map_err(|err| format!("send agent chat failed: {err}"))
}

pub(super) fn dispatch_prompt_override_step(
    viewer_client: Option<&ViewerClient>,
    automation_state: &mut ViewerAutomationState,
    agent_id: &str,
    field: ViewerAutomationPromptField,
    value: &ViewerAutomationPromptValue,
) -> Result<(), String> {
    let client = viewer_client.ok_or_else(|| "viewer client unavailable".to_string())?;
    let signer = resolve_automation_auth_signer()?;
    let register_nonce = next_auth_nonce(automation_state);
    let mut session_register = oasis7::viewer::AuthoritativeSessionRegisterRequest {
        player_id: signer.player_id.clone(),
        public_key: Some(signer.public_key.clone()),
        auth: None,
        requested_agent_id: Some(agent_id.to_string()),
        force_rebind: false,
    };
    let register_proof = oasis7::viewer::sign_session_register_auth_proof(
        &session_register,
        register_nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )
    .map_err(|err| format!("sign session register failed: {err}"))?;
    session_register.auth = Some(register_proof);
    let nonce = next_auth_nonce(automation_state);
    let mut request = oasis7::viewer::PromptControlApplyRequest {
        agent_id: agent_id.to_string(),
        player_id: signer.player_id.clone(),
        public_key: Some(signer.public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version: None,
        updated_by: Some(signer.player_id.clone()),
        system_prompt_override: None,
        short_term_goal_override: None,
        long_term_goal_override: None,
    };

    let patch = Some(prompt_override_patch(value));
    match field {
        ViewerAutomationPromptField::System => request.system_prompt_override = patch,
        ViewerAutomationPromptField::ShortTerm => request.short_term_goal_override = patch,
        ViewerAutomationPromptField::LongTerm => request.long_term_goal_override = patch,
    }

    let proof = oasis7::viewer::sign_prompt_control_apply_auth_proof(
        oasis7::viewer::PromptControlAuthIntent::Apply,
        &request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )
    .map_err(|err| format!("sign prompt apply failed: {err}"))?;
    request.auth = Some(proof);
    client
        .tx
        .send(oasis7::viewer::ViewerRequest::AuthoritativeRecovery {
            command: oasis7::viewer::AuthoritativeRecoveryCommand::RegisterSession {
                request: session_register,
            },
        })
        .map_err(|err| format!("send session register failed: {err}"))?;
    client
        .tx
        .send(oasis7::viewer::ViewerRequest::PromptControl {
            command: oasis7::viewer::PromptControlCommand::Apply { request },
        })
        .map_err(|err| format!("send prompt apply failed: {err}"))
}

fn prompt_override_patch(value: &ViewerAutomationPromptValue) -> Option<String> {
    match value {
        ViewerAutomationPromptValue::Set(text) => Some(text.clone()),
        ViewerAutomationPromptValue::Clear => None,
    }
}

fn resolve_automation_auth_signer() -> Result<ViewerAutomationAuthSigner, String> {
    let player_id = resolve_viewer_player_id()?;
    let public_key = resolve_required_auth_env(VIEWER_AUTH_PUBLIC_KEY_ENV)?;
    let private_key = resolve_required_auth_env(VIEWER_AUTH_PRIVATE_KEY_ENV)?;
    Ok(ViewerAutomationAuthSigner {
        player_id,
        public_key,
        private_key,
    })
}

fn resolve_viewer_player_id() -> Result<String, String> {
    match resolve_runtime_auth_value(VIEWER_PLAYER_ID_ENV) {
        Some(value) => Ok(value),
        None => Ok(VIEWER_PLAYER_ID_DEFAULT.to_string()),
    }
}

fn resolve_required_auth_env(key: &str) -> Result<String, String> {
    resolve_runtime_auth_value(key).ok_or_else(|| format!("{key} is not set"))
}

fn resolve_runtime_auth_value(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    if let Some(value) = resolve_wasm_auth_value(key) {
        return Some(value);
    }

    std::env::var(key)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_arch = "wasm32")]
fn resolve_wasm_auth_value(key: &str) -> Option<String> {
    let window = web_sys::window()?;
    let store = js_sys::Reflect::get(
        window.as_ref(),
        &wasm_bindgen::JsValue::from_str(VIEWER_AUTH_BOOTSTRAP_OBJECT),
    )
    .ok()?;
    if store.is_null() || store.is_undefined() {
        return None;
    }
    js_sys::Reflect::get(&store, &wasm_bindgen::JsValue::from_str(key))
        .ok()?
        .as_string()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn next_auth_nonce(state: &mut ViewerAutomationState) -> u64 {
    let nonce = state.auth_nonce_floor.max(1);
    state.auth_nonce_floor = nonce.saturating_add(1);
    nonce
}

pub(super) fn resolve_target_entity(
    scene: &Viewer3dScene,
    target: &ViewerAutomationTarget,
) -> Option<(Entity, SelectionKind, String)> {
    match target {
        ViewerAutomationTarget::FirstKind(kind) => {
            let spec = target_kind_spec(scene, kind)?;
            let id = first_sorted_matching(spec.entities, spec.first_filter)
                .or_else(|| first_sorted_id(spec.entities))?;
            let entity = spec.entities.get(id.as_str()).copied()?;
            Some((entity, spec.selection_kind, id))
        }
        ViewerAutomationTarget::KindId { kind, id } => {
            let spec = target_kind_spec(scene, kind)?;
            spec.entities
                .get(id.as_str())
                .copied()
                .map(|entity| (entity, spec.selection_kind, id.clone()))
        }
    }
}

fn target_kind_spec<'a>(scene: &'a Viewer3dScene, kind: &str) -> Option<TargetKindSpec<'a>> {
    match kind {
        TARGET_KIND_AGENT => Some(TargetKindSpec {
            selection_kind: SelectionKind::Agent,
            entities: &scene.agent_entities,
            first_filter: always_true,
        }),
        TARGET_KIND_LOCATION => Some(TargetKindSpec {
            selection_kind: SelectionKind::Location,
            entities: &scene.location_entities,
            first_filter: always_true,
        }),
        TARGET_KIND_FRAGMENT => Some(TargetKindSpec {
            selection_kind: SelectionKind::Fragment,
            entities: &scene.location_entities,
            first_filter: is_fragment_id,
        }),
        TARGET_KIND_ASSET => Some(TargetKindSpec {
            selection_kind: SelectionKind::Asset,
            entities: &scene.asset_entities,
            first_filter: always_true,
        }),
        TARGET_KIND_MODULE_VISUAL => Some(TargetKindSpec {
            selection_kind: SelectionKind::Asset,
            entities: &scene.module_visual_entities,
            first_filter: always_true,
        }),
        TARGET_KIND_POWER_PLANT => Some(TargetKindSpec {
            selection_kind: SelectionKind::PowerPlant,
            entities: &scene.power_plant_entities,
            first_filter: always_true,
        }),
        TARGET_KIND_CHUNK => Some(TargetKindSpec {
            selection_kind: SelectionKind::Chunk,
            entities: &scene.chunk_entities,
            first_filter: always_true,
        }),
        _ => None,
    }
}

fn first_sorted_id(items: &HashMap<String, Entity>) -> Option<String> {
    let mut ids: Vec<_> = items.keys().cloned().collect();
    ids.sort();
    ids.into_iter().next()
}

fn first_sorted_matching(
    items: &HashMap<String, Entity>,
    predicate: fn(&str) -> bool,
) -> Option<String> {
    let mut ids: Vec<_> = items
        .keys()
        .filter(|id| predicate(id.as_str()))
        .cloned()
        .collect();
    ids.sort();
    ids.into_iter().next()
}

fn always_true(_id: &str) -> bool {
    true
}

fn is_fragment_id(id: &str) -> bool {
    id.starts_with("frag-")
}

pub(super) fn automation_focus_radius_for_target(
    selection_kind: SelectionKind,
    base_scale: Option<Vec3>,
    cm_to_unit: f32,
) -> Option<f32> {
    match selection_kind {
        SelectionKind::PowerPlant => {
            let units_per_meter = cm_to_unit.max(f32::EPSILON) * 100.0;
            let min_radius = POWER_FOCUS_RADIUS_MIN_M * units_per_meter;
            let scale_extent = base_scale
                .map(|scale| {
                    scale
                        .x
                        .abs()
                        .max(scale.y.abs())
                        .max(scale.z.abs())
                        .max(f32::EPSILON)
                })
                .unwrap_or(min_radius);
            Some((scale_extent * POWER_FOCUS_RADIUS_SCALE_FROM_BASE).max(min_radius))
        }
        _ => None,
    }
}
