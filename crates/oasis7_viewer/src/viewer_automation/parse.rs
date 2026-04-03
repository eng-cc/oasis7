use super::*;

pub(super) fn config_from_values(
    auto_select: Option<String>,
    auto_select_target: Option<String>,
    automation_steps: Option<String>,
) -> ViewerAutomationConfig {
    let steps = parse_steps(automation_steps.as_deref());
    if !steps.is_empty() {
        return ViewerAutomationConfig {
            enabled: true,
            steps,
        };
    }

    let target = auto_select_target
        .as_deref()
        .and_then(parse_target)
        .or_else(|| auto_select.as_deref().and_then(parse_target));
    let auto_select_enabled = auto_select
        .as_deref()
        .map(parse_truthy)
        .unwrap_or(auto_select_target.is_some());
    if auto_select_enabled {
        if let Some(target) = target {
            return ViewerAutomationConfig {
                enabled: true,
                steps: vec![ViewerAutomationStep::Select(target)],
            };
        }
    }

    ViewerAutomationConfig::default()
}

pub(super) fn parse_steps(raw: Option<&str>) -> Vec<ViewerAutomationStep> {
    let mut steps = Vec::new();
    let Some(raw) = raw else {
        return steps;
    };

    for segment in raw.split(';') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let Some((key, value)) = segment.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        let parsed = match key.as_str() {
            "wait" => value
                .parse::<f64>()
                .ok()
                .map(ViewerAutomationStep::WaitSeconds),
            "mode" => parse_mode(value).map(ViewerAutomationStep::SetMode),
            "focus" => parse_focus_selection_step(value)
                .or_else(|| parse_target(value).map(ViewerAutomationStep::Focus)),
            "focus_selection" | "focus_selected" => parse_focus_selection_step(value),
            "pan" => parse_vec3(value).map(ViewerAutomationStep::Pan),
            "zoom" => value
                .parse::<f32>()
                .ok()
                .map(ViewerAutomationStep::ZoomFactor),
            "orbit" => parse_orbit(value),
            "select" => parse_target(value).map(ViewerAutomationStep::Select),
            "panel" => parse_visibility_action(value).map(ViewerAutomationStep::PanelVisibility),
            "top_panel" | "top" => {
                parse_visibility_action(value).map(ViewerAutomationStep::TopPanelVisibility)
            }
            "module" | "panel_module" | "module_visibility" => parse_module_visibility_step(value),
            "timeline_seek" | "seek_timeline" => parse_timeline_seek_step(value),
            "timeline_filter" | "timeline_mark_filter" => parse_timeline_filter_step(value),
            "timeline_jump" | "timeline_mark_jump" => parse_timeline_jump_step(value),
            "chat" | "chat_send" => parse_chat_step(value),
            "prompt_system" | "prompt_sys" => {
                parse_prompt_override_step(value, ViewerAutomationPromptField::System)
            }
            "prompt_short" | "prompt_short_term" | "prompt_stg" => {
                parse_prompt_override_step(value, ViewerAutomationPromptField::ShortTerm)
            }
            "prompt_long" | "prompt_long_term" | "prompt_ltg" => {
                parse_prompt_override_step(value, ViewerAutomationPromptField::LongTerm)
            }
            "locale" | "language" => {
                parse_locale_action(value).map(ViewerAutomationStep::SetLocale)
            }
            "layout" | "layout_preset" | "panel_layout" => {
                parse_layout_preset(value).map(ViewerAutomationStep::ApplyLayoutPreset)
            }
            "material_variant" | "variant" => parse_material_variant_step(value)
                .map(|_| ViewerAutomationStep::CycleMaterialVariant),
            _ => None,
        };
        if let Some(step) = parsed {
            steps.push(step);
        }
    }
    steps
}

pub(super) fn parse_mode(raw: &str) -> Option<ViewerCameraMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "2d" | "two_d" | "twod" => Some(ViewerCameraMode::TwoD),
        "3d" | "three_d" | "threed" => Some(ViewerCameraMode::ThreeD),
        _ => None,
    }
}

pub(super) fn parse_target(raw: &str) -> Option<ViewerAutomationTarget> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    let normalized = value.to_ascii_lowercase();
    if let Some(kind_token) = normalized.strip_prefix("first_") {
        let kind = canonical_target_kind(kind_token)?;
        return Some(ViewerAutomationTarget::FirstKind(kind));
    }
    if let Some(kind_token) = normalized.strip_prefix("first:") {
        let kind = canonical_target_kind(kind_token)?;
        return Some(ViewerAutomationTarget::FirstKind(kind));
    }

    let (kind_token, id) = value.split_once(':')?;
    let kind = canonical_target_kind(kind_token)?;
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    Some(ViewerAutomationTarget::KindId {
        kind,
        id: id.to_string(),
    })
}

fn canonical_target_kind(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "agent" => Some(TARGET_KIND_AGENT),
        "location" | "loc" => Some(TARGET_KIND_LOCATION),
        "fragment" | "frag" => Some(TARGET_KIND_FRAGMENT),
        "asset" => Some(TARGET_KIND_ASSET),
        "module_visual" | "module-visual" | "modulevisual" => Some(TARGET_KIND_MODULE_VISUAL),
        "power_plant" | "power-plant" | "powerplant" => Some(TARGET_KIND_POWER_PLANT),
        "chunk" => Some(TARGET_KIND_CHUNK),
        _ => None,
    }
}

fn parse_vec3(raw: &str) -> Option<Vec3> {
    let values: Vec<_> = raw
        .split(',')
        .map(|value| value.trim().parse::<f32>().ok())
        .collect();
    match values.as_slice() {
        [Some(x), Some(y), Some(z)] => Some(Vec3::new(*x, *y, *z)),
        _ => None,
    }
}

fn parse_orbit(raw: &str) -> Option<ViewerAutomationStep> {
    let values: Vec<_> = raw
        .split(',')
        .map(|value| value.trim().parse::<f32>().ok())
        .collect();
    match values.as_slice() {
        [Some(yaw), Some(pitch)] => Some(ViewerAutomationStep::OrbitDeg {
            yaw: *yaw,
            pitch: *pitch,
        }),
        _ => None,
    }
}

fn parse_focus_selection_step(raw: &str) -> Option<ViewerAutomationStep> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "selection" | "selected" | "current" | "current_selection" | "current-selection" | "1"
        | "true" | "yes" | "on" => Some(ViewerAutomationStep::FocusSelection),
        _ => None,
    }
}

fn parse_visibility_action(raw: &str) -> Option<ViewerAutomationVisibilityAction> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "show" | "visible" | "on" | "1" | "true" | "yes" => {
            Some(ViewerAutomationVisibilityAction::Show)
        }
        "hide" | "hidden" | "off" | "0" | "false" | "no" => {
            Some(ViewerAutomationVisibilityAction::Hide)
        }
        "toggle" | "switch" => Some(ViewerAutomationVisibilityAction::Toggle),
        _ => None,
    }
}

fn parse_panel_module(raw: &str) -> Option<ViewerAutomationPanelModule> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "controls" | "control" => Some(ViewerAutomationPanelModule::Controls),
        "overview" => Some(ViewerAutomationPanelModule::Overview),
        "chat" => Some(ViewerAutomationPanelModule::Chat),
        "overlay" => Some(ViewerAutomationPanelModule::Overlay),
        "diagnosis" | "diag" => Some(ViewerAutomationPanelModule::Diagnosis),
        "event_link" | "event-link" | "eventlink" => Some(ViewerAutomationPanelModule::EventLink),
        "timeline" => Some(ViewerAutomationPanelModule::Timeline),
        "details" | "detail" => Some(ViewerAutomationPanelModule::Details),
        _ => None,
    }
}

fn parse_module_visibility_step(raw: &str) -> Option<ViewerAutomationStep> {
    let (module_raw, action_raw) = raw.split_once(':')?;
    let module = parse_panel_module(module_raw)?;
    let action = parse_visibility_action(action_raw)?;
    Some(ViewerAutomationStep::ModuleVisibility { module, action })
}

fn parse_timeline_seek_step(raw: &str) -> Option<ViewerAutomationStep> {
    let tick = raw.trim().parse::<u64>().ok()?;
    Some(ViewerAutomationStep::TimelineSeek { tick })
}

fn parse_timeline_filter_step(raw: &str) -> Option<ViewerAutomationStep> {
    let (kind_raw, action_raw) = raw.split_once(':')?;
    let kind = parse_timeline_mark_kind(kind_raw)?;
    let action = parse_visibility_action(action_raw)?;
    Some(ViewerAutomationStep::TimelineFilter { kind, action })
}

fn parse_timeline_jump_step(raw: &str) -> Option<ViewerAutomationStep> {
    let kind = parse_timeline_mark_kind(raw)?;
    Some(ViewerAutomationStep::TimelineJump { kind })
}

fn parse_timeline_mark_kind(raw: &str) -> Option<crate::timeline_controls::TimelineMarkKindPublic> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "err" | "error" => Some(crate::timeline_controls::TimelineMarkKindPublic::Error),
        "llm" => Some(crate::timeline_controls::TimelineMarkKindPublic::Llm),
        "peak" | "resource_peak" | "resource-peak" | "resourcepeak" => {
            Some(crate::timeline_controls::TimelineMarkKindPublic::Peak)
        }
        _ => None,
    }
}

fn parse_chat_step(raw: &str) -> Option<ViewerAutomationStep> {
    let (agent_id, message) = parse_agent_and_text(raw)?;
    Some(ViewerAutomationStep::SendAgentChat { agent_id, message })
}

fn parse_prompt_override_step(
    raw: &str,
    field: ViewerAutomationPromptField,
) -> Option<ViewerAutomationStep> {
    let (agent_id, text_raw) = parse_agent_and_text(raw)?;
    let value = parse_prompt_value(text_raw.as_str())?;
    Some(ViewerAutomationStep::ApplyPromptOverride {
        agent_id,
        field,
        value,
    })
}

fn parse_agent_and_text(raw: &str) -> Option<(String, String)> {
    let (agent_id_raw, text_raw) = raw.split_once('|')?;
    let agent_id = agent_id_raw.trim();
    if agent_id.is_empty() {
        return None;
    }
    let text = decode_percent_text(text_raw)?;
    Some((agent_id.to_string(), text))
}

fn parse_prompt_value(raw: &str) -> Option<ViewerAutomationPromptValue> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "clear" | "none" | "null" | "default" => Some(ViewerAutomationPromptValue::Clear),
        _ => Some(ViewerAutomationPromptValue::Set(trimmed.to_string())),
    }
}

fn decode_percent_text(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let current = bytes[index];
        if current == b'%' {
            if index + 2 >= bytes.len() {
                return None;
            }
            let high = from_hex_nibble(bytes[index + 1])?;
            let low = from_hex_nibble(bytes[index + 2])?;
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }
        if current == b'+' {
            decoded.push(b' ');
        } else {
            decoded.push(current);
        }
        index += 1;
    }
    let text = String::from_utf8(decoded).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn from_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_material_variant_step(raw: &str) -> Option<()> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "next" | "cycle" | "toggle" | "f8" => Some(()),
        _ => None,
    }
}

fn parse_locale_action(raw: &str) -> Option<ViewerAutomationLocaleAction> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "zh" | "zh_cn" | "zh-cn" | "cn" | "chinese" => Some(ViewerAutomationLocaleAction::Zh),
        "en" | "en_us" | "en-us" | "english" => Some(ViewerAutomationLocaleAction::En),
        "toggle" | "switch" => Some(ViewerAutomationLocaleAction::Toggle),
        _ => None,
    }
}

fn parse_layout_preset(raw: &str) -> Option<ViewerAutomationLayoutPreset> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "mission" => Some(ViewerAutomationLayoutPreset::Mission),
        "command" => Some(ViewerAutomationLayoutPreset::Command),
        "intel" => Some(ViewerAutomationLayoutPreset::Intel),
        _ => None,
    }
}

fn parse_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
