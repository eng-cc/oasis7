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
