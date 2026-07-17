#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  buildTaskGame076AttractionEvidence,
  buildTaskGame076ScenarioSnapshot,
} from "../software_safe_src/gameplay_attraction_scenario.js";

function assertCardShape(card) {
  assert.match(card.sample_id, /^task-game-076\./);
  assert.ok(Number.isInteger(card.hook_score));
  assert.ok(card.hook_score >= 1 && card.hook_score <= 5);
  assert.ok(Number.isInteger(card.replay_intent));
  assert.ok(card.replay_intent >= 1 && card.replay_intent <= 5);
  assert.ok(card.action_effect_feedback);
  assert.ok(card.what_did_i_cause);
  assert.ok(card.no_op_or_follow_up_route);
}

const ROLLBACK_QUOTE_FIELDS = [
  "rollback_deadline_beat",
  "rollback_cost_summary",
  "rollback_kept_benefit",
  "rollback_lost_benefit",
];

function assertCompleteRollbackQuote(routeTradeoff, label) {
  assert.equal(routeTradeoff.rollback_available, true, `${label} must expose an available rollback`);
  for (const field of ROLLBACK_QUOTE_FIELDS) {
    assert.equal(
      typeof routeTradeoff[field],
      "string",
      `${label} ${field} must be player-readable text`,
    );
    assert.ok(routeTradeoff[field].trim(), `${label} ${field} must not be empty`);
    assert.doesNotMatch(
      routeTradeoff[field],
      /\b[a-z][a-z0-9]*(?:_[a-z0-9]+)+\b/,
      `${label} ${field} must not expose an internal snake_case identifier`,
    );
  }
}

const evidence = buildTaskGame076AttractionEvidence();

assert.equal(evidence.task, "TASK-GAME-076");
assert.equal(evidence.scenario_version, "task-game-076.v1");
assert.equal(evidence.sufficiency.status, "attraction_pass");
assert.equal(evidence.attraction_cards.length, 6);
for (const card of evidence.attraction_cards) {
  assertCardShape(card);
}
assert.equal(evidence.sufficiency.missing.length, 0);
assert.equal(evidence.motivation_density_card.status, "motivation_density_pass");
assert.ok(evidence.motivation_density_card.meaningful_decision_count >= 2);
assert.ok(evidence.motivation_density_card.reward_or_unlock_count >= 1);
assert.ok(evidence.motivation_density_card.stall_or_wait_periods <= 1);
assert.equal(evidence.motivation_density_card.branch_offer_clarity, "clear");
assert.ok(evidence.motivation_density_card.continue_reason);
assert.ok(evidence.motivation_density_card.return_hook);
assert.ok(evidence.motivation_density_card.leverage_class);
assert.equal(evidence.content_volume_card.status, "content_volume_pass");
assert.ok(evidence.content_volume_card.effective_play_minutes >= 30);
assert.equal(evidence.content_volume_card.target_effective_play_minutes, 30);
assert.ok(evidence.content_volume_card.player_operation_count >= 18);
assert.ok(evidence.content_volume_card.content_unit_count >= 8);
assert.ok(evidence.content_volume_card.distinct_action_family_count >= 6);
assert.equal(evidence.content_volume_card.missing.length, 0);
assert.ok(evidence.implemented_content_segments.includes("diagnosis_focus"));
assert.ok(evidence.implemented_content_segments.includes("route_tradeoff"));
assert.ok(evidence.implemented_content_segments.includes("micro_commission"));
assert.ok(evidence.implemented_content_segments.includes("incident_recovery"));
assert.ok(evidence.implemented_content_segments.includes("opportunity_scan"));
assert.ok(evidence.implemented_content_segments.includes("return_package"));
assert.ok(evidence.gameplay_truth_coverage.local_demand_id);
assert.ok(evidence.gameplay_truth_coverage.contribution_target_id);
assert.ok(evidence.gameplay_truth_coverage.world_change_due_to_player);
assert.ok(evidence.gameplay_truth_coverage.next_session_goal);
assert.ok(evidence.gameplay_truth_coverage.recovery_action_id);
assert.ok(evidence.second_run_design_card.status === "second_run_hook_pass");
assert.ok(evidence.second_run_design_card.route_tradeoff_persists_across_beats);
assert.ok(evidence.second_run_design_card.named_commission_output);
assert.ok(evidence.second_run_design_card.choice_reflective_return_goal);
assert.equal(evidence.anti_script_design_card.status, "anti_script_pass");
assert.ok(evidence.anti_script_design_card.visible_midrun_route_consequence);
assert.ok(evidence.anti_script_design_card.local_demand_progress_after_delivery);
assert.ok(evidence.anti_script_design_card.second_session_choice_memory);
assert.ok(evidence.anti_script_design_card.boredom_negative_guard);
assert.ok(evidence.anti_script_design_card.repair_tradeoff_cost_visible);
assert.equal(evidence.route_branch_regression.status, "pass");
assert.notEqual(
  evidence.route_branch_regression.accelerate.next_session_goal,
  evidence.route_branch_regression.stabilize.next_session_goal,
);
assert.notEqual(
  evidence.route_branch_regression.accelerate.incident_route_consequence_text,
  evidence.route_branch_regression.stabilize.incident_route_consequence_text,
);
assert.notEqual(
  evidence.route_branch_regression.accelerate.local_demand_id,
  evidence.route_branch_regression.stabilize.local_demand_id,
);
assert.notEqual(
  evidence.route_branch_regression.accelerate.midrun_visible_metric_delta,
  evidence.route_branch_regression.stabilize.midrun_visible_metric_delta,
);
assertCompleteRollbackQuote(
  evidence.route_branch_regression.accelerate,
  "accelerate route rollback quote",
);
assertCompleteRollbackQuote(
  evidence.route_branch_regression.stabilize,
  "stabilize route rollback quote",
);
const firstVisibleOutput = evidence.raw_snapshots.find(
  (snapshot) => snapshot.task_game_076_scenario?.variant === "first_visible_output",
);
assert.ok(firstVisibleOutput.player_gameplay.route_tradeoff.route_commitment_id);
assert.ok(firstVisibleOutput.player_gameplay.route_tradeoff.affected_future_beats.length >= 2);
assert.ok(firstVisibleOutput.player_gameplay.micro_commission.output_display_name);
assert.ok(firstVisibleOutput.player_gameplay.micro_commission.screenshot_caption);
assert.notEqual(firstVisibleOutput.player_gameplay.micro_commission.delivery_status, "unassigned");
assert.equal(firstVisibleOutput.player_gameplay.micro_commission.assigned_local_demand_id, "local_order.edge_outpost_plate_shortfall");
assert.equal(firstVisibleOutput.player_gameplay.micro_commission.local_demand_progress_preview.before, 0);
assert.equal(firstVisibleOutput.player_gameplay.micro_commission.local_demand_progress_preview.after, 3);
assert.equal(firstVisibleOutput.player_gameplay.micro_commission.local_demand_progress_preview.target, 6);
assert.ok(firstVisibleOutput.player_gameplay.route_tradeoff.midrun_feedback);
assert.equal(firstVisibleOutput.player_gameplay.route_tradeoff.midrun_feedback.timing, "before_incident_recovery");
assert.ok(firstVisibleOutput.player_gameplay.route_tradeoff.visible_consequence_text);
assert.ok(firstVisibleOutput.player_gameplay.route_tradeoff.forecast_delta_text);
assertCompleteRollbackQuote(
  firstVisibleOutput.player_gameplay.route_tradeoff,
  "first-visible route rollback quote",
);
const returnPackage = evidence.raw_snapshots.find(
  (snapshot) => snapshot.task_game_076_scenario?.variant === "return_package",
);
assert.ok(returnPackage.player_gameplay.return_package.choice_memory.length >= 2);
assert.ok(returnPackage.player_gameplay.return_package.unlocked_variant);
assert.ok(returnPackage.player_gameplay.return_package.why_this_goal);
assert.ok(returnPackage.player_gameplay.return_package.second_session_first_screen_memory);
assert.match(returnPackage.player_gameplay.return_package.second_session_first_screen_memory, /stabilized input line/i);
assert.match(returnPackage.player_gameplay.return_package.second_session_first_screen_memory, /Starter Batch/i);
const localOrderContribution = evidence.raw_snapshots.find(
  (snapshot) => snapshot.task_game_076_scenario?.variant === "local_order_contribution",
);
assert.ok(localOrderContribution.player_gameplay.local_demand_progress_after_delivery);
assert.equal(
  localOrderContribution.player_gameplay.local_demand_progress_after_delivery.local_demand_id,
  firstVisibleOutput.player_gameplay.micro_commission.assigned_local_demand_id,
);
assert.ok(localOrderContribution.player_gameplay.local_demand_progress_after_delivery.progress_delta > 0);
assert.ok(localOrderContribution.player_gameplay.local_demand_progress_after_delivery.visible_result_text);
assert.match(localOrderContribution.player_gameplay.local_demand_progress_after_delivery.visible_result_text, /0\/6.*3\/6/);
const incidentRecovery = evidence.raw_snapshots.find(
  (snapshot) => snapshot.task_game_076_scenario?.variant === "incident_recovery",
);
assert.ok(incidentRecovery.player_gameplay.incident_recovery.repair_tradeoff_costs.quick_patch);
assert.ok(incidentRecovery.player_gameplay.incident_recovery.repair_tradeoff_costs.root_cause_fix);
const quickPatch = incidentRecovery.player_gameplay.incident_recovery.repair_tradeoff_costs.quick_patch;
const rootCauseFix = incidentRecovery.player_gameplay.incident_recovery.repair_tradeoff_costs.root_cause_fix;
assert.ok(quickPatch.time_cost < rootCauseFix.time_cost);
assert.notEqual(quickPatch.risk_after, rootCauseFix.risk_after);
assert.ok(quickPatch.output_delta > rootCauseFix.output_delta || quickPatch.progress_preserved);
assert.ok(rootCauseFix.stability_delta > quickPatch.stability_delta);
assert.match(quickPatch.visible_tradeoff_text, /risk|风险/i);
assert.match(rootCauseFix.visible_tradeoff_text, /risk|风险/i);
assert.equal(evidence.weak_sample_regression.status, "pass");
assert.equal(evidence.weak_sample_regression.detected_verdict, "progression_pass_but_attraction_weak");
assert.equal(evidence.boredom_negative_regression.status, "pass");
assert.equal(evidence.boredom_negative_regression.detected_status, "attraction_weak");

for (const field of ROLLBACK_QUOTE_FIELDS) {
  for (const mutation of ["delete", "blank"]) {
    const invalidRollbackSamples = structuredClone(evidence.raw_snapshots);
    const quotedRoute = invalidRollbackSamples.find(
      (snapshot) => snapshot.player_gameplay?.route_tradeoff?.rollback_available === true,
    );
    assert.ok(quotedRoute, `negative fixture for ${field}/${mutation} requires a true rollback offer`);
    if (mutation === "delete") {
      delete quotedRoute.player_gameplay.route_tradeoff[field];
    } else {
      quotedRoute.player_gameplay.route_tradeoff[field] = "   ";
    }
    const invalidRollbackEvidence = buildTaskGame076AttractionEvidence({
      samples: invalidRollbackSamples,
    });
    assert.equal(
      invalidRollbackEvidence.second_run_design_card.status,
      "second_run_hook_weak",
      `${field}/${mutation} must fail the design gate`,
    );
    assert.ok(
      invalidRollbackEvidence.second_run_design_card.missing.includes("route_rollback_quote_missing"),
      `${field}/${mutation} must identify the incomplete rollback quote`,
    );
  }
}

for (const mutation of ["delete", "null", "non_boolean"]) {
  const invalidRollbackAvailabilitySamples = structuredClone(evidence.raw_snapshots);
  const routeSample = invalidRollbackAvailabilitySamples.find(
    (snapshot) => snapshot.player_gameplay?.route_tradeoff,
  );
  assert.ok(routeSample, `negative fixture for rollback_available/${mutation} requires a real route sample`);
  if (mutation === "delete") {
    delete routeSample.player_gameplay.route_tradeoff.rollback_available;
  } else if (mutation === "null") {
    routeSample.player_gameplay.route_tradeoff.rollback_available = null;
  } else {
    routeSample.player_gameplay.route_tradeoff.rollback_available = "available";
  }
  const invalidRollbackAvailabilityEvidence = buildTaskGame076AttractionEvidence({
    samples: invalidRollbackAvailabilitySamples,
  });
  assert.equal(
    invalidRollbackAvailabilityEvidence.second_run_design_card.status,
    "second_run_hook_weak",
    `rollback_available/${mutation} must fail the design gate`,
  );
  assert.ok(
    invalidRollbackAvailabilityEvidence.second_run_design_card.missing.includes(
      "route_rollback_quote_missing",
    ),
    `rollback_available/${mutation} must identify the invalid rollback availability`,
  );
}

const weakEvidence = buildTaskGame076AttractionEvidence({
  samples: [
    buildTaskGame076ScenarioSnapshot({ variant: "weak_high_progress" }),
    buildTaskGame076ScenarioSnapshot({ variant: "weak_high_progress" }),
    buildTaskGame076ScenarioSnapshot({ variant: "weak_high_progress" }),
  ],
});

assert.equal(weakEvidence.sufficiency.status, "attraction_weak");
assert.ok(weakEvidence.sufficiency.missing.includes("meaningful_decision_count"));
assert.ok(weakEvidence.sufficiency.missing.includes("reward_or_unlock_count"));
assert.ok(weakEvidence.sufficiency.missing.includes("continue_reason"));
assert.ok(weakEvidence.sufficiency.missing.includes("effective_play_minutes"));
assert.equal(weakEvidence.weak_sample_regression.detected_verdict, "progression_pass_but_attraction_weak");

console.log("TASK-GAME-076 attraction scenario tests passed");
