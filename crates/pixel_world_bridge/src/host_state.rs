use serde_json::{json, Map, Value};

const FRAGMENT_TERRAIN_PALETTE: &[(&str, [u8; 3])] = &[
    ("silicate_matrix", [126, 144, 99]),
    ("iron_nickel_alloy", [176, 184, 196]),
    ("water_ice", [125, 211, 252]),
    ("hydrated_mineral", [96, 165, 250]),
    ("carbonaceous_organic", [120, 113, 108]),
    ("sulfide_ore", [202, 138, 4]),
    ("rare_earth_oxide", [167, 139, 250]),
    ("uranium_bearing_ore", [132, 204, 22]),
    ("thorium_bearing_ore", [244, 114, 182]),
    ("unknown", [148, 163, 184]),
];

fn tr(locale: &str, zh: &str, en: &str) -> String {
    if locale.to_ascii_lowercase().starts_with("zh") {
        zh.to_string()
    } else {
        en.to_string()
    }
}

fn obj<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn arr<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    obj(value, key).as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn str_value(value: &Value) -> Option<&str> {
    value.as_str().filter(|text| !text.is_empty())
}

fn str_key<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    str_value(obj(value, key))
}

fn string_key(value: &Value, key: &str) -> Option<String> {
    str_key(value, key).map(ToString::to_string)
}

fn number(value: &Value, fallback: f64) -> f64 {
    value.as_f64().filter(|n| n.is_finite()).unwrap_or(fallback)
}

fn number_key(value: &Value, key: &str, fallback: f64) -> f64 {
    number(obj(value, key), fallback)
}

fn normalize_position(value: &Value) -> Option<Value> {
    let x = number_key(value, "x_cm", f64::NAN);
    let y = number_key(value, "y_cm", f64::NAN);
    let z = number_key(value, "z_cm", f64::NAN);
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some(json!({ "x_cm": x, "y_cm": y, "z_cm": z }))
    } else {
        None
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn clamp_world_position(pos: &Value, world_bounds: &Value) -> Option<Value> {
    if !pos.is_object() || !world_bounds.is_object() {
        return None;
    }
    Some(json!({
        "x_cm": clamp(number_key(pos, "x_cm", 0.0), 0.0, number_key(world_bounds, "width_cm", 0.0)),
        "y_cm": clamp(number_key(pos, "y_cm", 0.0), 0.0, number_key(world_bounds, "depth_cm", 0.0)),
        "z_cm": clamp(number_key(pos, "z_cm", 0.0), 0.0, number_key(world_bounds, "height_cm", 0.0)),
    }))
}

fn world_center_position(world_bounds: &Value) -> Option<Value> {
    if !world_bounds.is_object() {
        return None;
    }
    Some(json!({
        "x_cm": number_key(world_bounds, "width_cm", 0.0) / 2.0,
        "y_cm": number_key(world_bounds, "depth_cm", 0.0) / 2.0,
        "z_cm": number_key(world_bounds, "height_cm", 0.0) / 2.0,
    }))
}

fn resource_summary(resources: &Value) -> String {
    let Some(resources) = resources.as_object() else {
        return "-".to_string();
    };
    let entries: Vec<String> = resources
        .iter()
        .map(|(key, value)| {
            if value.is_object() {
                format!("{key}:{}", value)
            } else if let Some(text) = value.as_str() {
                format!("{key}:{text}")
            } else {
                format!("{key}:{value}")
            }
        })
        .collect();
    if entries.is_empty() {
        "-".to_string()
    } else {
        entries.join(" · ")
    }
}

fn count_resource_entries(summary: &str) -> usize {
    if summary.is_empty() || summary == "-" {
        return 0;
    }
    summary
        .split(" · ")
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .count()
}

fn fragment_blocks(location: &Value) -> &[Value] {
    obj(obj(obj(location, "fragment_profile"), "blocks"), "blocks")
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn dominant_compound(block: &Value) -> String {
    let Some(ppm) = obj(obj(block, "compounds"), "ppm").as_object() else {
        return "unknown".to_string();
    };
    let mut ranked: Vec<(&String, f64)> = ppm
        .iter()
        .map(|(kind, value)| (kind, number(value, 0.0)))
        .filter(|(_, value)| *value > 0.0)
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(right.0))
    });
    ranked
        .first()
        .map(|(kind, _)| (*kind).clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn fragment_terrain_color(compound: &str) -> [u8; 3] {
    FRAGMENT_TERRAIN_PALETTE
        .iter()
        .find(|(kind, _)| *kind == compound)
        .map(|(_, color)| *color)
        .unwrap_or([148, 163, 184])
}

fn estimate_fragment_half_extent_cm(location: &Value, blocks: &[Value]) -> f64 {
    let explicit_radius = number_key(obj(location, "profile"), "radius_cm", 0.0);
    if explicit_radius > 0.0 {
        return explicit_radius;
    }
    let max_extent = blocks.iter().fold(0.0_f64, |value, block| {
        let origin = obj(block, "origin_cm");
        let size = obj(block, "size_cm");
        let origin_x = number_key(origin, "x_cm", 0.0);
        let origin_z = number_key(origin, "z_cm", number_key(origin, "y_cm", 0.0));
        let size_x = number_key(size, "x_cm", 0.0);
        let size_z = number_key(size, "z_cm", number_key(size, "y_cm", 0.0));
        value.max(origin_x + size_x).max(origin_z + size_z)
    });
    max_extent.max(2.0) / 2.0
}

fn build_fragment_terrain_for_location(location: &Value, world_bounds: &Value) -> Vec<Value> {
    let Some(pos) = normalize_position(obj(location, "pos")) else {
        return Vec::new();
    };
    let blocks = fragment_blocks(location);
    if !world_bounds.is_object() || blocks.is_empty() {
        return Vec::new();
    }
    let half_extent_cm = estimate_fragment_half_extent_cm(location, blocks);
    blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| {
            let size = obj(block, "size_cm");
            let origin = obj(block, "origin_cm");
            let size_x = number_key(size, "x_cm", 0.0);
            let size_z = number_key(size, "z_cm", number_key(size, "y_cm", 0.0));
            if size_x <= 0.0 || size_z <= 0.0 {
                return None;
            }
            let origin_x = number_key(origin, "x_cm", 0.0);
            let origin_z = number_key(origin, "z_cm", number_key(origin, "y_cm", 0.0));
            let dominant = dominant_compound(block);
            let local_x = origin_x + (size_x / 2.0) - half_extent_cm;
            let local_y = origin_z + (size_z / 2.0) - half_extent_cm;
            let terrain_pos = clamp_world_position(
                &json!({
                    "x_cm": number_key(&pos, "x_cm", 0.0) + local_x,
                    "y_cm": number_key(&pos, "y_cm", 0.0) + local_y,
                    "z_cm": number_key(&pos, "z_cm", 0.0),
                }),
                world_bounds,
            )?;
            Some(json!({
                "id": format!("fragment:{}:{index}", str_key(location, "id").unwrap_or("")),
                "location_id": str_key(location, "id").unwrap_or(""),
                "pos": terrain_pos,
                "footprint_cm": size_x.max(size_z).max(1.0),
                "dominant_compound": dominant,
                "color": fragment_terrain_color(&dominant),
                "emphasis": 0.58,
            }))
        })
        .collect()
}

fn deterministic_hash(input: &str) -> u32 {
    input.chars().fold(2_166_136_261_u32, |hash, ch| {
        hash.wrapping_mul(31).wrapping_add(ch as u32)
    })
}

fn derive_agent_position(
    agent: &Value,
    location_by_id: &Map<String, Value>,
    world_bounds: &Value,
) -> Option<Value> {
    let location_id = str_key(agent, "location_id")?;
    let location = location_by_id.get(location_id)?;
    let location_pos = obj(location, "pos");
    if !location_pos.is_object() || !world_bounds.is_object() {
        return None;
    }
    let hash = deterministic_hash(&format!(
        "{}:{location_id}",
        str_key(agent, "id").unwrap_or("")
    ));
    let angle = ((hash % 360) as f64).to_radians();
    let max_dim =
        number_key(world_bounds, "width_cm", 0.0).max(number_key(world_bounds, "depth_cm", 0.0));
    let explicit_radius = number_key(location, "radius_cm", 0.0);
    let radius_hint = if explicit_radius > 0.0 {
        explicit_radius
    } else {
        35_000.0
    };
    let radius_cm = 10_000.0_f64.max((max_dim * 0.015).min(radius_hint));
    clamp_world_position(
        &json!({
            "x_cm": number_key(location_pos, "x_cm", 0.0) + angle.cos() * radius_cm,
            "y_cm": number_key(location_pos, "y_cm", 0.0) + angle.sin() * radius_cm,
            "z_cm": number_key(location_pos, "z_cm", 0.0),
        }),
        world_bounds,
    )
}

fn resolve_agent_position(
    agent: &Value,
    selected: &Value,
    location_by_id: &Map<String, Value>,
    world_bounds: &Value,
) -> (Value, &'static str) {
    let selected_pos = if str_key(selected, "id") == str_key(agent, "id") {
        obj(selected, "pos")
    } else {
        &Value::Null
    };
    if let Some(pos) =
        normalize_position(obj(agent, "pos")).or_else(|| normalize_position(selected_pos))
    {
        return (pos, "snapshot");
    }
    if let Some(pos) = derive_agent_position(agent, location_by_id, world_bounds) {
        return (pos, "location_derived");
    }
    (Value::Null, "missing")
}

fn resolve_selection_position(
    selection: &Value,
    agents: &[Value],
    locations: &[Value],
) -> Option<Value> {
    let kind = str_key(selection, "kind")?;
    let id = str_key(selection, "id")?;
    match kind {
        "agent" => agents
            .iter()
            .find(|agent| str_key(agent, "id") == Some(id))
            .and_then(|agent| normalize_position(obj(agent, "pos"))),
        "location" => locations
            .iter()
            .find(|location| str_key(location, "id") == Some(id))
            .and_then(|location| normalize_position(obj(location, "pos"))),
        _ => None,
    }
}

fn build_pixel_world_links(agents: &[Value], location_by_id: &Map<String, Value>) -> Vec<Value> {
    agents
        .iter()
        .filter_map(|agent| {
            let location_id = str_key(agent, "location_id")?;
            let location = location_by_id.get(location_id)?;
            let agent_pos = obj(agent, "pos");
            let location_pos = obj(location, "pos");
            if !agent_pos.is_object() || !location_pos.is_object() {
                return None;
            }
            Some(json!({
                "id": format!("link:{}:{location_id}", str_key(agent, "id").unwrap_or("")),
                "kind": "agent_assignment",
                "from": agent_pos,
                "to": location_pos,
                "emphasis": 0.72,
            }))
        })
        .collect()
}

fn build_recent_event_hotspots(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .take(4)
        .enumerate()
        .map(|(index, event)| {
            json!({
                "id": str_key(event, "eventId")
                    .or_else(|| str_key(event, "event_id"))
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("recent-{index}")),
                "title": str_key(event, "title")
                    .or_else(|| str_key(event, "summary"))
                    .or_else(|| str_key(event, "kind"))
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("event-{index}")),
                "kind": str_key(event, "kind").unwrap_or("recent_event"),
            })
        })
        .collect()
}

fn offset_world_position(
    anchor: Option<Value>,
    world_bounds: &Value,
    x_ratio: f64,
    y_ratio: f64,
) -> Option<Value> {
    if !world_bounds.is_object() {
        return None;
    }
    let base = anchor.or_else(|| world_center_position(world_bounds))?;
    clamp_world_position(
        &json!({
            "x_cm": number_key(&base, "x_cm", 0.0) + number_key(world_bounds, "width_cm", 0.0) * x_ratio,
            "y_cm": number_key(&base, "y_cm", 0.0) + number_key(world_bounds, "depth_cm", 0.0) * y_ratio,
            "z_cm": number_key(&base, "z_cm", 0.0),
        }),
        world_bounds,
    )
}

fn build_visual_hotspots(
    world_bounds: &Value,
    anchor: Option<Value>,
    goal_highlight: &Value,
    blocker_highlight: &Value,
    recent_event_hotspots: &[Value],
) -> Vec<Value> {
    if !world_bounds.is_object() {
        return Vec::new();
    }
    let offsets = [
        (-0.18, -0.14),
        (0.18, -0.12),
        (0.22, 0.14),
        (-0.2, 0.16),
        (0.0, -0.22),
        (0.0, 0.22),
    ];
    let mut staged = Vec::new();
    if let Some(title) = str_key(goal_highlight, "title") {
        staged.push(json!({
            "id": "goal-highlight",
            "label": title,
            "kind": "goal",
            "emphasis": 1.0,
            "size_hint_px": 14.0,
        }));
    }
    if let Some(kind) = str_key(blocker_highlight, "kind") {
        staged.push(json!({
            "id": "blocker-highlight",
            "label": kind,
            "kind": "blocker",
            "emphasis": 1.0,
            "size_hint_px": 16.0,
        }));
    }
    for hotspot in recent_event_hotspots.iter().take(4) {
        staged.push(json!({
            "id": format!("recent:{}", str_key(hotspot, "id").unwrap_or("")),
            "label": str_key(hotspot, "title").unwrap_or(""),
            "kind": str_key(hotspot, "kind").unwrap_or("recent_event"),
            "emphasis": 0.72,
            "size_hint_px": 10.0,
        }));
    }
    staged
        .into_iter()
        .enumerate()
        .filter_map(|(index, mut entry)| {
            let (x_ratio, y_ratio) = offsets[index % offsets.len()];
            let pos = offset_world_position(anchor.clone(), world_bounds, x_ratio, y_ratio)?;
            entry.as_object_mut()?.insert("pos".to_string(), pos);
            Some(entry)
        })
        .collect()
}

fn pick_known_agent_id(candidate_ids: Vec<Option<String>>, agents: &[Value]) -> Option<String> {
    candidate_ids.into_iter().flatten().find(|candidate| {
        agents
            .iter()
            .any(|agent| str_key(agent, "id") == Some(candidate.as_str()))
    })
}

fn action_receipt_title(locale: &str, state: &str, present: bool) -> String {
    if !present {
        return tr(locale, "暂无行动回执", "No action receipt yet");
    }
    match state {
        "accepted" => tr(locale, "行动已接受", "Action accepted"),
        "blocked" => tr(locale, "行动被阻塞", "Action blocked"),
        "completed" => tr(locale, "世界已改变", "World changed"),
        "rejected" => tr(locale, "行动被拒绝", "Action rejected"),
        _ => tr(locale, "行动进行中", "Action in progress"),
    }
}

fn build_action_receipt(locale: &str, gameplay: &Value, active_agent_id: Option<&str>) -> Value {
    let recent_feedback = obj(gameplay, "recentFeedback");
    let has_world_delta = str_key(gameplay, "lastWorldChange").is_some()
        || str_key(recent_feedback, "effect").is_some();
    let has_player_intent = str_key(gameplay, "acceptedIntentId").is_some()
        || str_key(gameplay, "acceptedIntentScope").is_some()
        || str_key(gameplay, "acceptedIntentTarget").is_some()
        || str_key(recent_feedback, "action").is_some();
    let present =
        has_world_delta || has_player_intent || str_key(recent_feedback, "reason").is_some();
    let raw_state = str_key(gameplay, "executionState")
        .or_else(|| str_key(recent_feedback, "stage"))
        .unwrap_or("waiting_for_intent");
    let state = if present {
        raw_state
    } else {
        "waiting_for_intent"
    };
    let confidence = if has_world_delta {
        "world_delta"
    } else if has_player_intent {
        "accepted_intent"
    } else {
        "none"
    };
    let summary = if present {
        str_key(gameplay, "lastWorldChange")
            .or_else(|| str_key(recent_feedback, "effect"))
            .or_else(|| str_key(gameplay, "acceptedIntentSummary"))
            .or_else(|| str_key(recent_feedback, "action"))
            .or_else(|| str_key(gameplay, "executionSummary"))
            .unwrap_or("")
            .to_string()
    } else {
        tr(
            locale,
            "还没有一条玩家行动产生可确认的世界变化。",
            "No player-caused world change has been confirmed yet.",
        )
    };
    let detail = if present {
        str_key(gameplay, "executionCauseDetail")
            .or_else(|| str_key(recent_feedback, "reason"))
            .or_else(|| str_key(recent_feedback, "hint"))
            .or_else(|| str_key(gameplay, "acceptedIntentDetail"))
            .or_else(|| str_key(gameplay, "progressDetail"))
            .map(ToString::to_string)
    } else {
        Some(tr(
            locale,
            "先提交玩法动作或推进世界，再查看系统确认、阻塞或完成的回执。",
            "Submit a gameplay action or advance the world, then read whether the system accepted, blocked, or completed it.",
        ))
    };
    json!({
        "present": present,
        "state": state,
        "confidence": confidence,
        "title": action_receipt_title(locale, state, present),
        "summary": summary,
        "detail": detail,
        "target_agent_id": if present {
            string_key(gameplay, "acceptedIntentTarget")
                .or_else(|| string_key(obj(gameplay, "recommendedAction"), "targetAgentId"))
                .or_else(|| active_agent_id.map(ToString::to_string))
                .map(Value::String)
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        },
        "effect_kind": if present {
            string_key(gameplay, "executionCauseKind")
                .or_else(|| string_key(recent_feedback, "stage"))
                .map(Value::String)
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        },
        "delta_logical_time": if present { obj(recent_feedback, "deltaLogicalTime").clone() } else { Value::Null },
        "delta_event_seq": if present { obj(recent_feedback, "deltaEventSeq").clone() } else { Value::Null },
    })
}

fn build_commercial_surface(
    locale: &str,
    gameplay: &Value,
    agents: &[Value],
    links: &[Value],
    fragment_terrain: &[Value],
    visual_hotspots: &[Value],
    selection: &Value,
) -> Value {
    let active_agent_id = pick_known_agent_id(
        vec![
            string_key(obj(gameplay, "recommendedAction"), "targetAgentId"),
            string_key(gameplay, "acceptedIntentTarget"),
            if str_key(selection, "kind") == Some("agent") {
                string_key(selection, "id")
            } else {
                None
            },
            agents.first().and_then(|agent| string_key(agent, "id")),
        ],
        agents,
    );
    let objective_title = str_key(gameplay, "goalTitle")
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            tr(
                locale,
                "进入世界，建立第一条能力链",
                "Enter the world and build the first capability chain",
            )
        });
    let objective_detail = str_key(gameplay, "objective")
        .or_else(|| str_key(gameplay, "progressDetail"))
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            tr(
                locale,
                "先让 Agent、路线和资源关系变得可读，再推进下一步。",
                "Read the agent, route, and resource relationship before pushing the next move.",
            )
        });
    let next_action_label = str_key(obj(gameplay, "recommendedAction"), "label")
        .or_else(|| str_key(gameplay, "nextStepHint"))
        .or_else(|| str_key(gameplay, "narrativeNextStep"))
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            tr(
                locale,
                "选择一个 Agent 或推进世界一步",
                "Select an agent or advance the world one step",
            )
        });
    let next_action_detail = str_key(obj(gameplay, "recommendedAction"), "disabledReason")
        .or_else(|| str_key(gameplay, "nextStepHint"))
        .or_else(|| str_key(gameplay, "executionSummary"))
        .map(ToString::to_string);
    let leverage_summary = str_key(gameplay, "acceptedIntentSummary")
        .or_else(|| str_key(gameplay, "lastWorldChange"))
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            tr(
                locale,
                "还没有一条被正式接受的玩家意图",
                "No player-facing accepted intent yet",
            )
        });
    let leverage_detail = str_key(gameplay, "lastWorldChange")
        .or_else(|| str_key(gameplay, "executionCauseDetail"))
        .or_else(|| str_key(gameplay, "acceptedIntentDetail"))
        .or_else(|| str_key(gameplay, "progressDetail"))
        .map(ToString::to_string);
    let action_receipt = build_action_receipt(locale, gameplay, active_agent_id.as_deref());

    json!({
        "objective": {
            "title": objective_title,
            "detail": objective_detail,
            "progress_percent": obj(gameplay, "progressPercent").clone(),
        },
        "next_action": {
            "label": next_action_label,
            "detail": next_action_detail,
            "target_agent_id": string_key(obj(gameplay, "recommendedAction"), "targetAgentId")
                .or_else(|| active_agent_id.clone())
                .map(Value::String)
                .unwrap_or(Value::Null),
            "execute_kind": string_key(obj(gameplay, "recommendedAction"), "executeKind")
                .map(Value::String)
                .unwrap_or(Value::Null),
        },
        "active_agent_id": active_agent_id,
        "player_leverage": {
            "state": str_key(gameplay, "executionState").unwrap_or("waiting_for_intent"),
            "label": str_key(gameplay, "executionStateLabel")
                .map(ToString::to_string)
                .unwrap_or_else(|| tr(locale, "等待玩家意图", "Waiting for Intent")),
            "summary": leverage_summary,
            "detail": leverage_detail,
        },
        "action_receipt": action_receipt,
        "blocker": {
            "label": string_key(gameplay, "blockerLabel")
                .or_else(|| string_key(gameplay, "blockerKind")),
            "detail": string_key(gameplay, "narrativeBlockerDetail")
                .or_else(|| string_key(gameplay, "blockerDetail")),
        },
        "world_read": {
            "agents": agents.len(),
            "routes": links.len(),
            "fragments": fragment_terrain.len(),
            "hotspots": visual_hotspots.len(),
        },
    })
}

fn build_world_bounds(input: &Value) -> Value {
    let space = obj(obj(input, "snapshot"), "config")
        .get("space")
        .unwrap_or(&Value::Null);
    if !space.is_object() {
        return Value::Null;
    }
    json!({
        "width_cm": number_key(space, "width_cm", 0.0),
        "depth_cm": number_key(space, "depth_cm", 0.0),
        "height_cm": number_key(space, "height_cm", 0.0),
    })
}

pub(crate) fn build_render_state(input: &Value) -> Value {
    let locale = str_key(input, "locale").unwrap_or("en");
    let world_bounds = build_world_bounds(input);
    let world_scale_base = number_key(&world_bounds, "width_cm", 1.0)
        .min(number_key(&world_bounds, "depth_cm", 1.0))
        .max(1.0);
    let selected = obj(input, "selected");
    let mut fragment_terrain = Vec::new();
    let mut locations = Vec::new();

    for location in arr(obj(input, "lists"), "locations") {
        let terrain = build_fragment_terrain_for_location(location, &world_bounds);
        fragment_terrain.extend(terrain.iter().cloned());
        let resource_summary = resource_summary(obj(location, "resources"));
        let resource_score = count_resource_entries(&resource_summary);
        let has_terrain = !terrain.is_empty();
        if let Some(pos) = normalize_position(obj(location, "pos")) {
            let radius_cm = number_key(obj(location, "profile"), "radius_cm", 0.0);
            let size_hint_px = if has_terrain {
                10.0
            } else {
                16.0 + 18.0_f64
                    .min((radius_cm / world_scale_base) * 420.0 + (resource_score * 2) as f64)
            };
            locations.push(json!({
                "id": str_key(location, "id").unwrap_or(""),
                "label": str_key(location, "name").or_else(|| str_key(location, "id")).unwrap_or(""),
                "pos": pos,
                "radius_cm": radius_cm,
                "resource_summary": resource_summary,
                "resource_score": resource_score,
                "fragment_terrain_count": terrain.len(),
                "marker_role": if has_terrain { "logic_anchor" } else { "primary_marker" },
                "marker_alpha": if has_terrain { 0.32 } else { 0.72 },
                "size_hint_px": size_hint_px,
            }));
        }
    }
    let location_by_id: Map<String, Value> = locations
        .iter()
        .filter_map(|location| Some((str_key(location, "id")?.to_string(), location.clone())))
        .collect();

    let agents: Vec<Value> = arr(obj(input, "lists"), "agents")
        .iter()
        .map(|agent| {
            let (pos, position_source) =
                resolve_agent_position(agent, selected, &location_by_id, &world_bounds);
            let resource_summary = resource_summary(obj(agent, "resources"));
            let resource_score = count_resource_entries(&resource_summary);
            let mut status_badges = Vec::new();
            if let Some(location_id) = str_key(agent, "location_id") {
                status_badges.push(Value::String(format!("location={location_id}")));
            }
            if let Some(kind) = str_key(agent, "kind") {
                status_badges.push(Value::String(format!("kind={kind}")));
            }
            if position_source == "location_derived" {
                status_badges.push(Value::String("position=location_derived".to_string()));
            }
            json!({
                "id": str_key(agent, "id").unwrap_or(""),
                "label": str_key(agent, "name").or_else(|| str_key(agent, "id")).unwrap_or(""),
                "location_id": string_key(agent, "location_id"),
                "pos": pos,
                "position_source": position_source,
                "resource_summary": resource_summary,
                "resource_score": resource_score,
                "status_badges": status_badges,
                "size_hint_px": 12.0 + 10.0_f64.min(
                    (resource_score * 2) as f64
                    + if str_key(agent, "location_id").is_some() { 2.0 } else { 0.0 }
                    + if str_key(agent, "kind").is_some() { 1.0 } else { 0.0 }
                ),
            })
        })
        .collect();

    let selection = match (str_key(input, "selectedKind"), str_key(input, "selectedId")) {
        (Some(kind), Some(id)) => json!({ "kind": kind, "id": id }),
        _ => Value::Null,
    };
    let links = build_pixel_world_links(&agents, &location_by_id);
    let anchor = resolve_selection_position(&selection, &agents, &locations)
        .or_else(|| {
            agents
                .iter()
                .find(|agent| obj(agent, "pos").is_object())
                .and_then(|agent| normalize_position(obj(agent, "pos")))
        })
        .or_else(|| {
            locations
                .first()
                .and_then(|location| normalize_position(obj(location, "pos")))
        })
        .or_else(|| world_center_position(&world_bounds));
    let gameplay = obj(input, "gameplay");
    let goal_highlight = str_key(gameplay, "goalTitle")
        .map(|title| json!({ "title": title, "objective": string_key(gameplay, "objective") }))
        .unwrap_or(Value::Null);
    let blocker_highlight = if str_key(gameplay, "blockerKind").is_some()
        || str_key(gameplay, "blockerDetail").is_some()
    {
        json!({
            "kind": str_key(gameplay, "blockerKind").unwrap_or("blocked"),
            "detail": string_key(gameplay, "blockerDetail"),
        })
    } else {
        Value::Null
    };
    let recent_event_hotspots = build_recent_event_hotspots(arr(input, "recentEvents"));
    let visual_hotspots = build_visual_hotspots(
        &world_bounds,
        anchor,
        &goal_highlight,
        &blocker_highlight,
        &recent_event_hotspots,
    );
    let commercial_surface = build_commercial_surface(
        locale,
        gameplay,
        &agents,
        &links,
        &fragment_terrain,
        &visual_hotspots,
        &selection,
    );
    let presentation = obj(input, "presentation");

    json!({
        "locale": locale,
        "world_bounds": world_bounds,
        "locations": locations,
        "fragment_terrain": fragment_terrain,
        "agents": agents,
        "links": links,
        "selection": selection,
        "goal_highlight": goal_highlight,
        "blocker_highlight": blocker_highlight,
        "recent_event_hotspots": recent_event_hotspots,
        "visual_hotspots": visual_hotspots,
        "commercial_surface": commercial_surface,
        "presentation": {
            "world_bounds_label": obj(presentation, "world_bounds_label").clone(),
            "marker_truth_note": obj(presentation, "marker_truth_note").clone(),
        },
    })
}

#[cfg(test)]
#[path = "host_state_tests.rs"]
mod tests;
