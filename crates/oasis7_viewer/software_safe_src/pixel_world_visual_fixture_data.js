// Test-only snapshot fixtures for reproducible Pixel World visual evidence.
export function pixelWorldSelectedBlockerVisualFixture() {
  return {
    time: 12,
    config: {
      space: {
        width_cm: 10_000_000,
        depth_cm: 5_000_000,
        height_cm: 1_000_000,
      },
    },
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Agent 0",
          location_id: "loc-0",
          pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 },
          resources: {},
        },
        "agent-1": {
          id: "agent-1",
          name: "Agent 1",
          location_id: "loc-1",
          pos: { x_cm: 6_900_000, y_cm: 1_150_000, z_cm: 0 },
          resources: {},
        },
      },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Factory Anchor",
          pos: { x_cm: 7_150_000, y_cm: 2_200_000, z_cm: 0 },
          profile: { radius_cm: 55_000, radiation_emission_per_tick: 0, material: "silicate" },
          fragment_profile: {
            blocks: {
              blocks: [
                {
                  origin_cm: { x_cm: -36_000, y_cm: 0, z_cm: -22_000 },
                  size_cm: { x_cm: 28_000, y_cm: 7_500, z_cm: 20_000 },
                  density_kg_per_m3: 3200,
                  compounds: { ppm: { silicate_matrix: 800_000, water_ice: 200_000 } },
                },
                {
                  origin_cm: { x_cm: 4_000, y_cm: 1_000, z_cm: -12_000 },
                  size_cm: { x_cm: 42_000, y_cm: 8_000, z_cm: 18_000 },
                  density_kg_per_m3: 7800,
                  compounds: { ppm: { iron_nickel_alloy: 900_000, sulfide_ore: 100_000 } },
                },
                {
                  origin_cm: { x_cm: -18_000, y_cm: 500, z_cm: 18_000 },
                  size_cm: { x_cm: 34_000, y_cm: 6_000, z_cm: 24_000 },
                  density_kg_per_m3: 5200,
                  compounds: { ppm: { sulfide_ore: 620_000, hydrated_mineral: 380_000 } },
                },
                {
                  origin_cm: { x_cm: 30_000, y_cm: 0, z_cm: 24_000 },
                  size_cm: { x_cm: 22_000, y_cm: 4_500, z_cm: 16_000 },
                  density_kg_per_m3: 2600,
                  compounds: { ppm: { silicate_matrix: 700_000, rare_earth_oxide: 300_000 } },
                },
              ],
            },
          },
          resources: {},
        },
        "loc-1": {
          id: "loc-1",
          name: "Assembly Nexus",
          pos: { x_cm: 4_550_000, y_cm: 1_200_000, z_cm: 0 },
          profile: { radius_cm: 38_000, radiation_emission_per_tick: 0, material: "alloy" },
          resources: {},
        },
      },
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
      agent_player_bindings: { "agent-0": "player-one", "agent-1": "player-two" },
      agent_player_public_key_bindings: {
        "agent-0": "abcdef0123456789abcdef0123456789",
        "agent-1": "bbbbbb0123456789bbbbbb0123456789",
      },
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      execution_state: "blocked",
      accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
      intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
      intent_scope: "gameplay_action",
      intent_target: "agent-0",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      causality_kind: "world_constraint",
      causality_detail: "iron input exhausted at factory-0",
      last_world_change: "Smelter build request reached factory-0; iron shortage blocks construction.",
      blocker_supplemental_detail: null,
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [{
        action_id: "build_factory_smelter_mk1",
        target_agent_id: "agent-0",
        label: "Build smelter mk1",
        protocol_action: "gameplay_action.submit",
        disabled_reason: null,
      }],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2,
      },
      // Published snapshot data for the renderer's visual fixture only; this
      // does not introduce an action or alter the selected-blocker scenario.
      micro_depot_facilities: [{
        facility_id: "depot-fixture-loc-0",
        status: "active",
        location_id: "loc-0",
        service_radius_cm: 240_000,
      }],
      agent_claim: null,
    },
  };
}

// Test-only: leave the enabled recommendation visible without an action receipt.
export function pixelWorldRecommendedTargetVisualFixture() {
  const fixture = pixelWorldSelectedBlockerVisualFixture();
  const gameplay = fixture.player_gameplay;
  gameplay.stage_status = "ready";
  gameplay.execution_state = "waiting_for_intent";
  delete gameplay.accepted_intent_id;
  delete gameplay.intent_summary;
  delete gameplay.intent_scope;
  delete gameplay.intent_target;
  delete gameplay.last_world_change;
  gameplay.recent_feedback = null;
  return fixture;
}

// Test-only visual fixture. The known co-anchored pair and unknown fallback
// exercise the renderer's noninteractive module identity glyphs.
export function pixelWorldModuleVisualEntitiesFixture() {
  const fixture = pixelWorldSelectedBlockerVisualFixture();
  fixture.model.module_visual_entities = {
    "module-absolute": {
      entity_id: "module-absolute",
      module_id: "fixture-module",
      kind: "beacon",
      label: "Beacon marker",
      anchor: { type: "absolute", data: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
    },
    "module-relay": {
      entity_id: "module-relay",
      module_id: "fixture-module",
      kind: "relay",
      label: "Relay marker",
      anchor: { type: "absolute", data: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
    },
    "module-agent": {
      entity_id: "module-agent",
      module_id: "fixture-module",
      kind: "future_module_kind",
      label: "Unknown marker",
      anchor: { type: "agent", data: { agent_id: "agent-0" } },
    },
  };
  return fixture;
}
