import { buildTaskGame076ScenarioSnapshot } from "../gameplay_attraction_scenario.js";

function sampleSnapshot(overrides = {}) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    ...overrides,
    config: { ...base.config, ...(overrides.config || {}) },
    model: {
      ...base.model,
      agent_player_bindings: { "agent-0": "local-test-player-bound" },
      agent_player_public_key_bindings: { "agent-0": "abcdef0123456789abcdef0123456789" },
      ...(overrides.model || {}),
    },
    player_gameplay: { ...base.player_gameplay, ...(overrides.player_gameplay || {}) },
  };
}

export function slot1CandidateClaimSnapshot() {
  const base = sampleSnapshot();
  return sampleSnapshot({
    model: {
      ...base.model,
      agents: {
        ...base.model.agents,
        "agent-claim-target": {
          id: "agent-claim-target", name: "Claim Target", location_id: "loc-0", resources: {},
        },
        "agent-choice-target": {
          id: "agent-choice-target", name: "Choice Target", location_id: "loc-0", resources: {},
        },
      },
    },
    player_gameplay: {
      ...base.player_gameplay,
      agent_claim: {
        claimer_agent_id: "agent-0",
        current_epoch: 0,
        reputation_tier: 0,
        claim_cap: 1,
        owned_claim_count: 0,
        liquid_main_token_balance: 0,
        restricted_starter_claim_balance: 0,
        slot_1_auto_restricted_starter_claim_amount: 325,
        slot_1_eligible_claim_balance: 325,
        next_claim_quote: {
          slot_index: 1,
          reputation_tier: 0,
          claim_cap: 1,
          owned_claim_count: 0,
          activation_fee_amount: 100,
          claim_bond_amount: 200,
          upkeep_per_epoch: 25,
          total_upfront_amount: 325,
          transferable_liquid_balance: 0,
          restricted_starter_claim_balance: 0,
          auto_restricted_starter_claim_amount: 325,
          eligible_claim_balance: 325,
          upkeep_runway_epochs: 0,
          release_cooldown_epochs: 2,
          grace_epochs: 4,
          idle_warning_epochs: 8,
          forced_idle_reclaim_epochs: 10,
          forced_reclaim_penalty_bps: 2000,
          slot_1_claim_choice_quote: {
            status: "candidate_facts_only",
            candidates: [{
              agent_id: "agent-choice-target",
              location_x_cm: 120,
              location_y_cm: -40,
              location_z_cm: 5,
              body_kind: "industrial_worker",
              frame_kind: "light_frame",
              installed_module_ids: ["drill", "scanner"],
            }],
            fallback_reason: "candidate_rationale_missing",
            claim_choice_class: "wait_or_fund_first",
            recommended_claim_action: "wait_or_fund_first",
          },
        },
        owned_claims: [],
      },
    },
  });
}

export function slot1CandidateRationaleSnapshot() {
  const base = slot1CandidateClaimSnapshot();
  const quote = base.player_gameplay.agent_claim.next_claim_quote;
  return {
    ...base,
    player_gameplay: {
      ...base.player_gameplay,
      agent_claim: {
        ...base.player_gameplay.agent_claim,
        next_claim_quote: {
          ...quote,
          slot_1_claim_choice_quote: {
            ...quote.slot_1_claim_choice_quote,
            status: "candidate_rationale_published",
            candidate_starting_location: "(10, 20, 30) cm",
            candidate_specialty_summary: "Canonical specialties: energy, sensing/input discovery, mobility/routing, cargo/input carrying.",
            first_industrial_goal_help: "Supports the first industrial goal without guaranteeing output.",
            candidate_risk_summary: "No provable high-risk capability gap is present in the current canonical snapshot.",
            candidate_recommendation_reason: "Exactly one complete candidate is known for the first industrial goal.",
            fallback_reason: null,
            claim_choice_class: "claim_now_route_fit",
            recommended_claim_action: "claim_now_route_fit",
          },
        },
      },
    },
  };
}

export async function renderViewerApp(snapshot, authOverrides = {}, locale = "en") {
  window.history.replaceState({}, "", `/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=${locale}`);
  window.localStorage.clear();
  document.body.innerHTML = "";
  const core = await import("../legacy_core.js");
  const main = await import("../main.jsx");
  const appRoot = document.createElement("div");
  document.body.appendChild(appRoot);
  core.initializeSoftwareSafeCore();
  core.setViewerLocale(locale);
  core.injectSnapshot(snapshot);
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "local-test-player-bound",
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
    ...authOverrides,
  };
  main.__markStarterOcOnboardingCompleteForTest("agent-0");
  const dispose = main.mountViewerApp(appRoot);
  return { container: appRoot, core, dispose };
}
