import { installControlProofVisualFixture } from "./control_proof_panel.jsx";
import * as core from "./legacy_core.js";
import { HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE } from "./software_safe_constants.js";
import { installMarketQuoteDecisionVisualFixture, installPowerSaleQuoteVisualFixture, installPowerSurvivalQuoteVisualFixture, installProductValidationQuoteVisualFixture, installRefineQuotePreflightVisualFixture, installScheduleRecipeQuoteVisualFixture, installTransferMaterialQuoteVisualFixture, installWaitResolutionQuoteVisualFixture, installWarDeclarationQuoteVisualFixture } from "./quote_visual_fixture_installers.js";
import { installBranchCommitmentVisualFixture } from "./branch_commitment_visual_fixture.js";
import { installAgentIntentV2VisualFixture } from "./agent_intent_visual_fixture.js";
import { installAgentContextVisualFixture, readAgentContextFixtureGameplay, readAgentContextFixtureMetadata } from "./agent_context_visual_fixture.js";
import { installMajorWorldEventCrisisVisualFixture } from "./major_world_event_visual_fixture.js";
import { fallbackTradeoffVisualFixture } from "./viewer_fallback_tradeoff_fixture.js";
import { recoveryOptionVisualFixture } from "./viewer_recovery_option_fixture.js";
const VIEWER_VISUAL_FIXTURE_GLOBAL = "__OASIS7_VIEWER_VISUAL_FIXTURES__";
function viewerVisualFixtureNameFromQuery() {
  return viewerTestApiEnabled()
    ? String(new URLSearchParams(window.location.search || "").get("viewer_visual_fixture") || "").trim() || null
    : null;
}

function viewerTestApiEnabled() {
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return __OASIS7_VISUAL_TEST__ === true && new URLSearchParams(window.location.search).get("connect") === "0" && ["1", "true", "yes", "on"].includes(value);
}

function viewerFixtureBaseSnapshot(overrides = {}) {
  const base = {
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
          resources: { alloy: 3 },
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
          resources: { iron: 0 },
        },
        "loc-1": {
          id: "loc-1",
          name: "Assembly Nexus",
          pos: { x_cm: 4_550_000, y_cm: 1_200_000, z_cm: 0 },
          profile: { radius_cm: 38_000, radiation_emission_per_tick: 0, material: "alloy" },
          resources: {},
        },
      },
      agent_prompt_profiles: {
        "agent-0": {
          agent_id: "agent-0",
          version: 3,
          updated_by: "viewer-bound",
          system_prompt: "Keep the first production line recoverable.",
          short_term_goal: "Report the blocker and wait for material recovery.",
          long_term_goal: "Restore sustainable capability without inventing extra automation.",
        },
      },
      agent_execution_debug_contexts: {
        "agent-0": {
          provider_mode: "runtime_live",
          execution_mode: "phase_1",
          environment_class: "software_safe_viewer",
          observation_schema_version: "viewer.v1",
          action_schema_version: "agent_chat.v1",
          agent_profile: "default",
          provider_check_status: "ok",
          provider_check_source: "fixture",
          fallback_reason: null,
          provider_reported_capabilities: ["agent_chat"],
          provider_reported_supported_action_sets: ["agent_chat"],
        },
      },
      agent_player_bindings: {
        "agent-0": "viewer-bound",
        "agent-1": "viewer-other",
      },
      agent_player_public_key_bindings: {
        "agent-0": "oc:pk:viewer-session-key",
        "agent-1": "oc:pk:viewer-other-session-key",
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
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      recovery_path_kind: "repair_rebuild_or_pivot",
      recovery_path_detail: "Choose the local recovery path that best fits the current constraint.",
      major_power_dependency_status: "independent_path_available",
      repair_available: true,
      rebuild_available: true,
      pivot_available: true,
      recovery_options: recoveryOptionVisualFixture(), fallback_tradeoff_preview: fallbackTradeoffVisualFixture(),
      no_safe_fallback_reason: "No repair or reroute action is currently available for this blocked intent.", required_next_decision_action_id: "return_to_goal_selection", required_next_decision_class: "return_to_goal_selection",
      available_actions: [
        {
          action_id: "build_factory_smelter_mk1",
          target_agent_id: "agent-0",
          label: "Build smelter mk1",
          protocol_action: "gameplay_action.submit",
          disabled_reason: null,
        },
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          disabled_reason: null,
        },
      ],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2,
      },
      agent_claim: null,
      micro_depot_facilities: [
        {
          facility_id: "depot-regional-01",
          owner_claim_id: "claim-regional-01",
          status: "active",
          location_id: "loc-1",
          service_radius_cm: 250_000,
          inventory_revision: 7,
          available_units_by_kind: { data: 5, repair_kit: 2 },
          throughput_epoch: 11,
          throughput_remaining_units: 13,
          throughput_limit_units_per_epoch: 16,
          supported_resource_kinds: ["data", "repair_kit"],
          module_id: "regional.micro_depot",
          module_version: "0.2.0",
          wasm_hash: "sha256:micro-depot-public-evidence-1234567890",
          upkeep_paid: true,
          last_receipt_id: "receipt-micro-depot-public-01",
          last_proposal_hash: "sha256:proposal-public-01",
          available_actions: ["service_micro_depot_repair", "reclaim_micro_depot"],
        },
      ],
    },
  };
  return {
    ...base,
    ...overrides,
    config: {
      ...base.config,
      ...(overrides.config || {}),
    },
    model: {
      ...base.model,
      ...(overrides.model || {}),
    },
    player_gameplay: {
      ...base.player_gameplay,
      ...(overrides.player_gameplay || {}),
    },
  };
}

function emptyWorldRecoverySnapshot() {
  return viewerFixtureBaseSnapshot({
    model: {
      agents: {},
      locations: {},
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
      agent_player_bindings: {},
      agent_player_public_key_bindings: {},
    },
    player_gameplay: {
      stage_id: "world_bootstrap",
      stage_status: "blocked",
      execution_state: "blocked",
      goal_kind: "RecoverCapability",
      goal_title: "Recover world snapshot",
      objective: "Recover the world before issuing commands.",
      progress_detail: "No agents or locations are available in the current snapshot.",
      progress_percent: 0,
      blocker_kind: "runtime_snapshot_empty_entities",
      blocker_detail: "The viewer is missing a valid world snapshot.",
      causality_kind: "world_constraint",
      causality_detail: "empty snapshot contains zero agents and zero locations",
      next_step_hint: "Request a fresh snapshot; if entity counts stay at zero, repair or restart the runtime world bootstrap.",
      available_actions: [
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          disabled_reason: null,
        },
      ],
      recent_feedback: null,
      agent_claim: null,
    },
  });
}

function setFixturePlayerAuth() {
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "viewer-bound",
    publicKey: "oc:pk:viewer-session-key",
    privateKey: null,
    releaseToken: null,
    source: "visual_fixture_projection",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
  };
}

function setFixtureChatHistory() {
  core.state.chatDraft.message = "Report nearby resources.";
  core.state.chatDraft.dirty = true;
  core.state.chatHistory = [
    {
      id: "fixture-chat-5",
      source: "agent",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "agent-0",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Awaiting material recovery before the smelter can proceed.",
      tick: 12,
      intentSeq: 5,
    },
    {
      id: "fixture-chat-4",
      source: "player",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "viewer-bound",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Hold position and confirm the blocker.",
      tick: 11,
      intentSeq: 4,
    },
    {
      id: "fixture-chat-3",
      source: "agent",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "agent-0",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Factory Anchor reports iron input exhausted.",
      tick: 10,
      intentSeq: 3,
    },
  ];
  core.state.lastChatFeedback = {
    channel: "agent_chat",
    action: "agent_chat",
    stage: "acknowledged",
    ok: true,
    accepted: true,
    target: "agent-0",
    summary: "Agent chat acknowledged by the viewer fixture.",
    detail: "Recent message flow remains visible while prompt controls stay collapsed.",
    code: null,
  };
}

function setFixtureDiagnostics() {
  core.state.recentEvents = [
    { id: 24, time: 12, kind: { type: "state_sync", status: "ok" } },
    { id: 23, time: 12, kind: { type: "intent_tick", status: "blocked" } },
    { id: 22, time: 11, kind: { type: "econ_update", status: "material_shortage" } },
  ];
  core.state.eventCount = core.state.recentEvents.length;
  core.state.metrics = {
    total_ticks: 12,
    decision_trace_count: 1,
  };
}

function setFixtureHostedGate() {
  core.state.hostedAccess = {
    deployment_mode: HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
    action_matrix: [
      {
        action_id: "prompt_control_apply",
        required_auth: "strong_auth",
        availability: "public_player_plane_with_backend_reauth_preview",
        reason: "prompt_control_apply is available after browser player-session registration plus backend re-authorization",
      },
      {
        action_id: "main_token_transfer",
        required_auth: "strong_auth",
        availability: "blocked_until_strong_auth",
        reason: "main_token_transfer remains blocked; this viewer exposes no transfer form.",
      },
    ],
  };
  core.state.auth = {
    ...core.state.auth,
    available: false,
    playerId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    source: "guest_only",
    registrationStatus: "guest",
    runtimeStatus: "guest",
    error: "session validation requires hosted login",
  };
  core.state.hostedLogin.handle = "player@example.com";
  core.state.hostedLogin.challengeId = "fixture-challenge";
  core.state.hostedLogin.maskedLoginHint = "p***@example.com";
  core.state.hostedLogin.deliveryMode = "email";
  core.state.hostedLogin.accountExists = true;
  core.state.hostedLogin.error = "Enter the latest verification code to continue.";
  core.state.hostedLogin.retryAfterSeconds = 18;
}

function openFixtureDetails(name) {
  queueMicrotask(() => {
    if (name === "gameplay_diagnostics_expanded" || name === "factory_production_failure_disposition" || name === "control_proof_applied") {
      document.getElementById("viewer-gameplay-details")?.setAttribute("open", "");
      if (name === "gameplay_diagnostics_expanded") {
        document.getElementById("viewer-diagnostics-panel")?.setAttribute("open", "");
      }
    }
  });
}

function installViewerVisualFixture() {
  if (!viewerTestApiEnabled()) {
    delete window[VIEWER_VISUAL_FIXTURE_GLOBAL];
    document.body.removeAttribute("data-viewer-visual-fixture");
    return null;
  }
  const fixtures = Object.assign(Object.create(null), {
    shell_selected_blocker() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
    },
    agent_chat_history() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      core.setPromptOverridesVisible(false);
    },
    gameplay_diagnostics_expanded() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      setFixtureDiagnostics();
    },
    factory_production_failure_disposition() {
      core.injectSnapshot(viewerFixtureBaseSnapshot({
        player_gameplay: {
          available_actions: [
            {
              action_id: "schedule_recipe_smelter_iron_ingot",
              label: "Queue iron ingot run",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "agent-0",
              disabled_reason: "insufficient iron_ore in site ledger",
            },
            {
              action_id: "request_snapshot",
              label: "Refresh gameplay snapshot",
              protocol_action: "request_snapshot",
            },
          ],
          factory_production_failure_disposition: {
            action_id: "19",
            requester_agent_id: "agent-0",
            factory_id: "factory.target",
            recipe_id: "recipe.smelter.iron_ingot",
            blocker_kind: "product_validation_rejected",
            blocker_detail: "product profile rejected the committed output",
            disposition_kind: "consumed_lost",
            consumed_inputs: [{ kind: "iron_ore", amount: 3 }],
            lost_inputs: [{ kind: "iron_ore", amount: 3 }],
            consumed_power: 7,
            lost_power: 7,
            next_action: "inspect_product_validation_and_reschedule",
            next_recheck: null,
          },
        },
      }), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
    },
    hosted_login_gate() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixtureHostedGate();
    },
    empty_world_recovery() {
      core.injectSnapshot(emptyWorldRecoverySnapshot(), { returnState: false });
      core.state.selectedKind = null;
      core.state.selectedId = null;
      core.state.selectedObject = null;
    },
  });
  installAgentContextVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installControlProofVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installAgentIntentV2VisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installMajorWorldEventCrisisVisualFixture(fixtures, { core, viewerFixtureBaseSnapshot });
  installRefineQuotePreflightVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }); installScheduleRecipeQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }); installTransferMaterialQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installProductValidationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installPowerSaleQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installPowerSurvivalQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installMarketQuoteDecisionVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installWaitResolutionQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installWarDeclarationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installBranchCommitmentVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  window[VIEWER_VISUAL_FIXTURE_GLOBAL] = fixtures;

  const fixtureName = viewerVisualFixtureNameFromQuery();
  if (!fixtureName || !Object.hasOwn(fixtures, fixtureName) || typeof fixtures[fixtureName] !== "function") {
    return null;
  }
  fixtures[fixtureName]();
  document.body.setAttribute("data-viewer-visual-fixture", fixtureName);
  openFixtureDetails(fixtureName);
  return fixtureName;
}


export const viewerVisualTestAdapter = { enabled: viewerTestApiEnabled, fixtureName: viewerVisualFixtureNameFromQuery, install: installViewerVisualFixture, readMetadata: readAgentContextFixtureMetadata, readGameplay: readAgentContextFixtureGameplay };
