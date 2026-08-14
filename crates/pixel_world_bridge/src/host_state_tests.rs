use super::*;

fn sample_input() -> Value {
    json!({
        "locale": "en",
        "snapshot": {
            "config": {
                "space": {
                    "width_cm": 10_000_000.0,
                    "depth_cm": 5_000_000.0,
                    "height_cm": 1_000_000.0
                }
            }
        },
        "lists": {
            "agents": [
                {
                    "id": "agent-0",
                    "name": "Agent 0",
                    "location_id": "loc-0",
                    "resources": {}
                }
            ],
            "locations": [
                {
                    "id": "loc-0",
                    "name": "Factory Anchor",
                    "pos": { "x_cm": 5_000_000.0, "y_cm": 2_500_000.0, "z_cm": 0.0 },
                    "profile": { "radius_cm": 25_000.0 },
                    "fragment_profile": {
                        "blocks": {
                            "blocks": [
                                {
                                    "origin_cm": { "x_cm": 0.0, "y_cm": 0.0, "z_cm": 0.0 },
                                    "size_cm": { "x_cm": 12_000.0, "y_cm": 7_500.0, "z_cm": 8_000.0 },
                                    "compounds": {
                                        "ppm": {
                                            "silicate_matrix": 800_000.0,
                                            "water_ice": 200_000.0
                                        }
                                    }
                                },
                                {
                                    "origin_cm": { "x_cm": 20_000.0, "y_cm": 1_000.0, "z_cm": 18_000.0 },
                                    "size_cm": { "x_cm": 20_000.0, "y_cm": 8_000.0, "z_cm": 10_000.0 },
                                    "compounds": {
                                        "ppm": {
                                            "iron_nickel_alloy": 900_000.0,
                                            "sulfide_ore": 100_000.0
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "resources": {}
                }
            ]
        },
        "gameplay": {
            "acceptedIntentId": "gameplay_action:build_factory_smelter_mk1",
            "acceptedIntentSummary": "Queue build_factory_smelter_mk1 for agent-0",
            "acceptedIntentScope": "gameplay_action",
            "acceptedIntentTarget": "agent-0",
            "goalTitle": "Recover sustainable capability",
            "objective": "Stabilize the first production line before expanding.",
            "progressDetail": "The primary line is blocked by missing material input.",
            "progressPercent": 68,
            "blockerKind": "material_shortage",
            "blockerLabel": "Missing Material",
            "blockerDetail": "iron input exhausted at factory-0",
            "executionState": "blocked",
            "executionStateLabel": "Blocked",
            "executionCauseKind": "world_constraint",
            "executionCauseDetail": "iron input exhausted at factory-0",
            "lastWorldChange": "Smelter build request reached factory-0; iron shortage blocks construction.",
            "nextStepHint": "Replenish upstream materials, then advance again to confirm the line resumes.",
            "recommendedAction": {
                "targetAgentId": "agent-0",
                "label": "Build smelter mk1",
                "executeKind": "gameplay_action"
            },
            "recentFeedback": {
                "action": "build_factory_smelter_mk1",
                "stage": "completed_no_progress",
                "effect": "Smelter build request reached factory-0; iron shortage blocks construction.",
                "reason": "iron input exhausted at factory-0",
                "hint": "Replenish upstream materials, then advance again.",
                "deltaLogicalTime": 1,
                "deltaEventSeq": 2
            }
        },
        "selected": Value::Null,
        "selectedKind": Value::Null,
        "selectedId": Value::Null,
        "recentEvents": [
            { "eventId": "evt-1", "title": "Transfer spike", "kind": "resource_transfer" },
            { "eventId": "evt-2", "title": "Queue update", "kind": "build_queue" }
        ],
        "presentation": {
            "world_bounds_label": "100km x 50km",
            "marker_truth_note": "scaled"
        }
    })
}

#[test]
fn rust_host_state_derives_fragment_agent_link_and_commercial_surface() {
    let state = build_render_state(&sample_input());

    assert_eq!(state["world_bounds"]["width_cm"], 10_000_000.0);
    assert_eq!(state["locations"][0]["marker_role"], "logic_anchor");
    assert_eq!(state["locations"][0]["marker_alpha"], 0.32);
    assert_eq!(state["locations"][0]["fragment_terrain_count"], 2);

    assert_eq!(state["fragment_terrain"].as_array().unwrap().len(), 2);
    assert_eq!(state["fragment_terrain"][0]["id"], "fragment:loc-0:0");
    assert_eq!(
        state["fragment_terrain"][0]["dominant_compound"],
        "silicate_matrix"
    );
    assert_eq!(state["fragment_terrain"][0]["color"], json!([126, 144, 99]));
    assert_eq!(
        state["fragment_terrain"][1]["dominant_compound"],
        "iron_nickel_alloy"
    );
    assert_eq!(state["fragment_terrain"][1]["footprint_cm"], 20_000.0);

    assert_eq!(state["agents"][0]["position_source"], "location_derived");
    assert!(state["agents"][0]["pos"].is_object());
    assert_eq!(state["links"].as_array().unwrap().len(), 1);
    assert_eq!(state["links"][0]["kind"], "agent_assignment");
    assert_eq!(state["visual_hotspots"].as_array().unwrap().len(), 4);

    let surface = &state["commercial_surface"];
    assert_eq!(surface["active_agent_id"], "agent-0");
    assert_eq!(
        surface["objective"]["title"],
        "Recover sustainable capability"
    );
    assert_eq!(surface["next_action"]["label"], "Build smelter mk1");
    assert_eq!(surface["player_leverage"]["state"], "blocked");
    assert_eq!(surface["action_receipt"]["present"], true);
    assert_eq!(surface["action_receipt"]["confidence"], "world_delta");
    assert_eq!(surface["action_receipt"]["title"], "Action blocked");
    assert_eq!(surface["world_read"]["agents"], 1);
    assert_eq!(surface["world_read"]["routes"], 1);
    assert_eq!(surface["world_read"]["fragments"], 2);
    assert_eq!(surface["world_read"]["hotspots"], 4);
}

#[test]
fn rust_host_state_projects_only_valid_agent_power_states() {
    let mut input = sample_input();
    let agents = input["lists"]["agents"].as_array_mut().expect("agents");
    agents[0]["power"] = json!({ "capacity": 100, "level": 20, "state": "low_power" });
    agents.push(json!({
        "id": "agent-critical",
        "name": "Critical",
        "pos": { "x_cm": 8_000_000.0, "y_cm": 3_000_000.0, "z_cm": 0.0 },
        "power": { "capacity": 100, "level": 5, "state": "critical" },
        "resources": {}
    }));
    agents.push(json!({
        "id": "agent-invalid",
        "name": "Invalid",
        "power": { "capacity": 100, "level": 99, "state": "depleted" },
        "resources": {}
    }));

    let state = build_render_state(&input);
    assert_eq!(state["agents"][0]["power_state"], "low_power");
    assert_eq!(state["agents"][1]["power_state"], "critical");
    assert!(state["agents"][2]["power_state"].is_null());
}

#[test]
fn rust_host_state_projects_only_active_social_edges_with_known_endpoints() {
    let mut input = sample_input();
    input["snapshot"]["time"] = json!(12);
    input["lists"]["agents"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "agent-1",
            "name": "Agent 1",
            "pos": { "x_cm": 7_000_000.0, "y_cm": 3_000_000.0, "z_cm": 0.0 },
            "resources": {}
        }));
    input["snapshot"]["model"]["social_edges"] = json!({
        "7": {
            "edge_id": 7,
            "schema_id": "cooperation",
            "relation_kind": "ally",
            "from": { "type": "Agent", "data": { "agent_id": "agent-0" } },
            "to": { "type": "Agent", "data": { "agent_id": "agent-1" } },
            "weight_bps": 4200,
            "lifecycle": "active"
        },
        "8": {
            "edge_id": 8,
            "from": { "type": "Agent", "data": { "agent_id": "missing" } },
            "to": { "type": "Agent", "data": { "agent_id": "agent-1" } },
            "lifecycle": "active"
        },
        "9": {
            "edge_id": 9,
            "from": { "type": "Agent", "data": { "agent_id": "agent-0" } },
            "to": { "type": "Agent", "data": { "agent_id": "agent-1" } },
            "expires_at_tick": 12,
            "lifecycle": "active"
        },
        "10": {
            "edge_id": 10,
            "from": { "type": "Agent", "data": { "agent_id": "agent-0" } },
            "to": { "type": "Agent", "data": { "agent_id": "agent-1" } },
            "lifecycle": "expired"
        },
        "11": {
            "edge_id": 11,
            "schema_id": "supply",
            "relation_kind": "supports",
            "from": { "type": "Location", "data": { "location_id": "loc-0" } },
            "to": { "type": "Agent", "data": { "agent_id": "agent-1" } },
            "weight_bps": 1000,
            "lifecycle": "active"
        }
    });

    let state = build_render_state(&input);
    assert_eq!(state["social_links"].as_array().unwrap().len(), 2);
    let social_links = state["social_links"].as_array().unwrap();
    let edge_7 = social_links
        .iter()
        .find(|edge| edge["id"] == "social_edge:7")
        .expect("agent-agent edge");
    assert_eq!(edge_7["relation_kind"], "ally");
    assert_eq!(edge_7["schema_id"], "cooperation");
    assert_eq!(edge_7["weight_bps"], 4200.0);
    let edge_11 = social_links
        .iter()
        .find(|edge| edge["id"] == "social_edge:11")
        .expect("location-agent edge");
    assert_eq!(edge_11["relation_kind"], "supports");
}

#[test]
fn rust_host_state_projects_module_visual_entities_with_resolved_anchors_only() {
    let mut input = sample_input();
    input["lists"]["agents"][0]["pos"] =
        json!({ "x_cm": 5_100_000.0, "y_cm": 2_600_000.0, "z_cm": 30.0 });
    input["snapshot"]["model"]["module_visual_entities"] = json!({
        "z-absolute": {
            "entity_id": "z-absolute",
            "module_id": "module-z",
            "kind": "opaque_kind",
            "label": "Stored label",
            "anchor": { "type": "absolute", "data": { "x_cm": 7100000, "y_cm": 1200000, "z_cm": 80 } }
        },
        "a-location": {
            "entity_id": "a-location",
            "module_id": "module-a",
            "kind": "opaque_kind",
            "label": null,
            "anchor": { "type": "location", "data": { "location_id": "loc-0" } }
        },
        "m-agent": {
            "entity_id": "m-agent",
            "module_id": "module-m",
            "kind": "opaque_kind",
            "label": "Agent anchor",
            "anchor": { "type": "agent", "data": { "agent_id": "agent-0" } }
        },
        "missing": {
            "entity_id": "missing",
            "module_id": "module-missing",
            "kind": "opaque_kind",
            "anchor": { "type": "location", "data": { "location_id": "no-such-location" } }
        }
    });

    let state = build_render_state(&input);
    assert_eq!(
        state["module_visual_entities"]
            .as_array()
            .expect("module visuals project as an array")
            .iter()
            .map(|entity| entity["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["a-location", "m-agent", "z-absolute"]
    );
    assert_eq!(
        state["module_visual_entities"][0]["pos"],
        input["lists"]["locations"][0]["pos"]
    );
    assert_eq!(
        state["module_visual_entities"][1]["pos"],
        input["lists"]["agents"][0]["pos"]
    );
    assert_eq!(
        state["module_visual_entities"][2]["pos"],
        json!({ "x_cm": 7100000.0, "y_cm": 1200000.0, "z_cm": 80.0 })
    );
    assert_eq!(state["module_visual_entities"][2]["kind"], "opaque_kind");
    assert_eq!(state["module_visual_entities"][2]["label"], "Stored label");
}

#[test]
fn rust_host_state_projects_published_micro_depots_at_stable_known_location_offsets() {
    let mut input = sample_input();
    input["gameplay"]["micro_depot_facilities"] = json!([
        { "facility_id": "depot-active", "status": "active", "location_id": "loc-0" },
        { "facility_id": "depot-suspended", "status": "suspended", "location_id": "loc-0" },
        { "facility_id": "depot-depleted", "status": "depleted", "location_id": "loc-0" },
        { "facility_id": "depot-unknown-location", "status": "active", "location_id": "missing" }
    ]);

    let first = build_render_state(&input);
    let second = build_render_state(&input);
    let facilities = first["micro_depot_facilities"]
        .as_array()
        .expect("facility projection must be an array");

    assert_eq!(facilities.len(), 3, "unknown anchors must be suppressed");
    assert_eq!(facilities[0]["id"], "micro_depot:depot-active");
    assert_eq!(facilities[1]["status"], "depleted");
    assert_eq!(facilities[2]["status"], "suspended");
    assert_ne!(facilities[0]["pos"], first["locations"][0]["pos"]);
    assert_eq!(
        first["micro_depot_facilities"], second["micro_depot_facilities"],
        "the same facility/location pair must retain its deterministic offset"
    );
}

#[test]
fn rust_host_state_projects_viewer_normalized_micro_depot_facilities_and_prefers_snake_array() {
    let mut camel_input = sample_input();
    camel_input["gameplay"]["microDepotFacilities"] = json!([
        { "facilityId": "depot-viewer-active", "status": "active", "locationId": "loc-0" }
    ]);
    let camel = build_render_state(&camel_input);
    assert_eq!(
        camel["micro_depot_facilities"][0]["id"], "micro_depot:depot-viewer-active",
        "the Viewer-normalized camelCase payload must reach the Bevy DTO"
    );
    assert_eq!(camel["micro_depot_facilities"][0]["location_id"], "loc-0");
    assert_eq!(camel["micro_depot_facilities"][0]["status"], "active");
    assert!(camel["micro_depot_facilities"][0]["pos"].is_object());

    camel_input["gameplay"]["micro_depot_facilities"] = json!([]);
    let snake_empty = build_render_state(&camel_input);
    assert_eq!(
        snake_empty["micro_depot_facilities"],
        json!([]),
        "an explicitly empty snake_case runtime publication must not revive camelCase compatibility data"
    );
}

#[test]
fn rust_host_state_preserves_micro_depot_stock_runway_fields_with_legacy_defaults() {
    let mut input = sample_input();
    input["gameplay"]["micro_depot_facilities"] = json!([
        {
            "facility_id": "depot-stocked",
            "status": "active",
            "location_id": "loc-0",
            "inventory_revision": 7,
            "available_units_by_kind": { "data": 8 },
            "throughput_epoch": 2,
            "throughput_remaining_units": 14,
            "throughput_limit_units_per_epoch": 16
        },
        {
            "facility_id": "depot-legacy",
            "status": "active",
            "location_id": "loc-0"
        }
    ]);

    let projected = build_render_state(&input);
    let facilities = projected["micro_depot_facilities"]
        .as_array()
        .expect("micro-depot projection must be an array");
    let stocked = facilities
        .iter()
        .find(|facility| facility["facility_id"] == "depot-stocked")
        .expect("stocked depot projection");
    assert_eq!(stocked["inventory_revision"], 7);
    assert_eq!(stocked["available_units_by_kind"], json!({ "data": 8 }));
    assert_eq!(stocked["throughput_epoch"], 2);
    assert_eq!(stocked["throughput_remaining_units"], 14);
    assert_eq!(stocked["throughput_limit_units_per_epoch"], 16);

    let legacy = facilities
        .iter()
        .find(|facility| facility["facility_id"] == "depot-legacy")
        .expect("legacy depot projection");
    assert_eq!(legacy["inventory_revision"], 0);
    assert_eq!(legacy["available_units_by_kind"], json!({}));
    assert_eq!(legacy["throughput_epoch"], 0);
    assert_eq!(legacy["throughput_remaining_units"], 0);
    assert_eq!(legacy["throughput_limit_units_per_epoch"], 0);
}

#[test]
fn rust_host_state_projects_only_targetable_action_receipts_for_pixel_world_display() {
    let blocked = build_render_state(&sample_input());
    assert_eq!(
        blocked["receipt_target"],
        json!({ "agent_id": "agent-0", "state": "blocked" }),
        "a present blocked receipt must expose a display-only target cue for its rendered Agent"
    );

    let mut accepted_input = sample_input();
    accepted_input["gameplay"]["executionState"] = json!("accepted");
    accepted_input["gameplay"]["recentFeedback"]["stage"] = json!("accepted");
    let accepted = build_render_state(&accepted_input);
    assert_eq!(
        accepted["receipt_target"],
        json!({ "agent_id": "agent-0", "state": "accepted" }),
        "accepted/submitted receipt states must retain their target and state for the renderer"
    );

    let mut absent_input = sample_input();
    absent_input["gameplay"] = json!({});
    let absent = build_render_state(&absent_input);
    assert!(
        absent.get("receipt_target").is_none() || absent["receipt_target"].is_null(),
        "an absent receipt must not create a display target"
    );

    let mut unknown_target_input = sample_input();
    unknown_target_input["gameplay"]["acceptedIntentTarget"] = json!("agent-not-rendered");
    unknown_target_input["gameplay"]["recommendedAction"]["targetAgentId"] =
        json!("agent-not-rendered");
    let unknown_target = build_render_state(&unknown_target_input);
    assert!(
        unknown_target.get("receipt_target").is_none()
            || unknown_target["receipt_target"].is_null(),
        "a receipt targeting an Agent absent from RenderState must not create a visual target"
    );
}

#[test]
fn rust_host_state_projects_only_enabled_rendered_recommended_targets_for_display() {
    let enabled = build_render_state(&sample_input());
    assert_eq!(
        enabled["recommended_target"],
        json!({ "agent_id": "agent-0" }),
        "the already-gated recommended action may point at its rendered Agent"
    );

    let mut disabled_input = sample_input();
    disabled_input["gameplay"]["recommendedAction"]["disabledReason"] = json!("Missing fuel");
    let disabled = build_render_state(&disabled_input);
    assert!(
        disabled.get("recommended_target").is_none() || disabled["recommended_target"].is_null(),
        "a disabled recommended action must not create a visual target"
    );

    let mut unknown_target_input = sample_input();
    unknown_target_input["gameplay"]["recommendedAction"]["targetAgentId"] =
        json!("agent-not-rendered");
    let unknown_target = build_render_state(&unknown_target_input);
    assert!(
        unknown_target.get("recommended_target").is_none()
            || unknown_target["recommended_target"].is_null(),
        "a recommendation for an Agent absent from RenderState must not create a visual target"
    );
}

#[test]
fn rust_host_state_localizes_command_board_surface_for_zh_locale() {
    let mut input = sample_input();
    input["locale"] = json!("zh-CN");
    input["gameplay"]["goalKind"] = json!("CreateFirstWorldFeedback");
    input["gameplay"]["goalTitle"] = json!("Create the first visible world feedback");
    input["gameplay"]["objective"] = json!(
        "Advance the world once and confirm that your action produces a visible state or event delta."
    );
    input["gameplay"]["nextStepHint"] =
        json!("Request a snapshot, advance 1 step, then inspect the new delta and events.");
    input["gameplay"]["recommendedAction"]["actionId"] = json!("build_factory_smelter_mk1");
    input["gameplay"]["recommendedAction"]["label"] = json!("Queue Smelter MK1 construction");

    let state = build_render_state(&input);
    let surface = &state["commercial_surface"];

    assert_eq!(surface["objective"]["title"], "确认第一条世界反馈");
    assert_eq!(state["goal_highlight"]["title"], "确认第一条世界反馈");
    assert_eq!(
        surface["objective"]["detail"],
        "先拿到一条明确世界反馈，再继续后续工业选择。"
    );
    assert_eq!(surface["next_action"]["label"], "排队建造一型冶炼炉");
    assert_eq!(
        surface["next_action"]["detail"],
        "先请求一次快照，推进 1 步，再检查新的世界变化和事件。"
    );

    let board_text = format!(
        "{}{}{}{}",
        surface["objective"]["title"].as_str().unwrap(),
        surface["objective"]["detail"].as_str().unwrap(),
        surface["next_action"]["label"].as_str().unwrap(),
        surface["next_action"]["detail"].as_str().unwrap()
    );
    assert!(
        !state["goal_highlight"]["title"]
            .as_str()
            .unwrap()
            .contains("Create the first visible world feedback")
    );
    assert!(!board_text.contains("Create the first visible world feedback"));
    assert!(!board_text.contains("Queue Smelter"));
    assert!(!board_text.contains("Request a snapshot"));
}

#[test]
fn rust_host_state_keeps_no_receipt_ambient_events_out_of_player_progress() {
    let mut input = sample_input();
    input["gameplay"] = json!({
        "goalTitle": "Recover sustainable capability",
        "objective": "Stabilize the first production line before expanding.",
        "progressDetail": "The first production line is waiting for a player command.",
        "executionState": "executing",
        "executionStateLabel": "Executing",
        "acceptedIntentSummary": "No player-facing accepted intent yet",
        "recentFeedback": Value::Null
    });

    let state = build_render_state(&input);
    let receipt = &state["commercial_surface"]["action_receipt"];
    assert_eq!(receipt["present"], false);
    assert_eq!(receipt["state"], "waiting_for_intent");
    assert_eq!(receipt["confidence"], "none");
    assert_eq!(receipt["title"], "No action receipt yet");
    assert_eq!(
        receipt["summary"],
        "No player-caused world change has been confirmed yet."
    );
    assert!(receipt["target_agent_id"].is_null());
}

#[test]
fn rust_host_state_suppresses_chain_sync_receipt_for_empty_world_onboarding() {
    let mut input = sample_input();
    input["lists"]["agents"] = json!([]);
    input["lists"]["locations"] = json!([]);
    input["gameplay"] = json!({
        "goalTitle": "Recover committed runtime sync",
        "objective": "Repair the committed runtime feed before retrying the first world-feedback loop.",
        "progressDetail": "The first-session loop is blocked because the viewer cannot read a committed runtime world yet.",
        "progressPercent": 0,
        "blockerKind": "runtime_snapshot_empty_entities",
        "blockerLabel": "空快照",
        "blockerDetail": "runtime exposed an empty new-user world with no agents/locations; claim the first Agent to start the onboarding flow",
        "executionState": "blocked",
        "executionStateLabel": "Blocked",
        "availableActions": [
            {
                "actionId": "request_snapshot",
                "label": "Refresh gameplay snapshot",
                "protocolAction": "request_snapshot",
                "targetAgentId": Value::Null,
                "disabledReason": Value::Null
            },
            {
                "actionId": "claim_first_agent",
                "label": "Claim first Agent",
                "protocolAction": "gameplay_action.submit",
                "targetAgentId": "starter-agent-0",
                "disabledReason": Value::Null
            }
        ],
        "recentFeedback": {
            "action": "chain_sync",
            "stage": "blocked",
            "effect": "committed runtime sync failed before the viewer could observe new world state",
            "reason": "DistributedValidationFailed { reason: \"latest state root mismatch\" }",
            "hint": "repair the chain runtime sync path, then refresh gameplay",
            "deltaLogicalTime": 0,
            "deltaEventSeq": 0
        }
    });

    let state = build_render_state(&input);
    let receipt = &state["commercial_surface"]["action_receipt"];
    let blocker_highlight = &state["blocker_highlight"];
    let blocker = &state["commercial_surface"]["blocker"];

    assert_eq!(receipt["present"], false);
    assert_eq!(receipt["state"], "waiting_for_intent");
    assert_eq!(receipt["confidence"], "none");
    assert_eq!(receipt["title"], "No action receipt yet");
    assert_eq!(
        receipt["summary"],
        "This is a new-user empty world; claim the first Agent first."
    );
    assert!(
        !receipt["summary"]
            .as_str()
            .unwrap()
            .contains("committed runtime sync failed")
    );
    assert!(
        !receipt["detail"]
            .as_str()
            .unwrap()
            .contains("DistributedValidationFailed")
    );
    assert_eq!(blocker_highlight["kind"], "runtime_snapshot_empty_entities");
    assert_eq!(blocker_highlight["label"], "Claim the first Agent");
    assert_eq!(blocker["label"], "Claim the first Agent");
    assert!(
        !blocker_highlight["label"]
            .as_str()
            .unwrap()
            .contains("runtime_snapshot_empty_entities")
    );
}

#[test]
fn rust_host_state_shows_pending_claim_receipt_for_empty_world_onboarding() {
    let mut input = sample_input();
    input["lists"]["agents"] = json!([]);
    input["lists"]["locations"] = json!([]);
    input["gameplay"] = json!({
        "goalTitle": "Claim the first Agent",
        "objective": "Start the new-user onboarding flow.",
        "progressDetail": "Waiting for committed world sync after submitting the first-Agent claim.",
        "progressPercent": 0,
        "blockerKind": "runtime_snapshot_empty_entities",
        "blockerLabel": "Claim the first Agent",
        "blockerDetail": "runtime exposed an empty new-user world with no agents/locations; claim the first Agent to start the onboarding flow",
        "executionState": "accepted",
        "executionStateLabel": "Accepted",
        "executionCauseKind": "queued_for_execution",
        "executionCauseDetail": "Waiting for committed world sync; the Agent will appear after the synced snapshot lands.",
        "acceptedIntentSummary": "submitted gameplay action claim_first_agent for starter-agent-0 to chain runtime as consensus action 1",
        "availableActions": [
            {
                "actionId": "claim_first_agent",
                "label": "Claim first Agent",
                "protocolAction": "gameplay_action.submit",
                "targetAgentId": "starter-agent-0",
                "disabledReason": Value::Null
            }
        ],
        "recentFeedback": {
            "action": "gameplay_action:claim_first_agent",
            "stage": "submitted",
            "effect": "submitted gameplay action claim_first_agent for starter-agent-0 to chain runtime as consensus action 1",
            "reason": Value::Null,
            "hint": "Waiting for committed world sync; the Agent will appear after the synced snapshot lands.",
            "deltaLogicalTime": 0,
            "deltaEventSeq": 0
        }
    });

    let state = build_render_state(&input);
    let receipt = &state["commercial_surface"]["action_receipt"];
    let blocker_highlight = &state["blocker_highlight"];

    assert_eq!(receipt["present"], true);
    assert_eq!(receipt["state"], "accepted");
    assert_eq!(receipt["confidence"], "world_delta");
    assert_eq!(receipt["title"], "Action accepted");
    assert!(
        receipt["summary"]
            .as_str()
            .unwrap()
            .contains("submitted gameplay action claim_first_agent")
    );
    assert!(
        receipt["detail"]
            .as_str()
            .unwrap()
            .contains("Waiting for committed world sync")
    );
    assert_eq!(receipt["target_agent_id"], "starter-agent-0");
    assert_eq!(blocker_highlight["kind"], "runtime_snapshot_empty_entities");
    assert_eq!(blocker_highlight["label"], "Claim the first Agent");
}
