#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const tempDir = mkdtempSync(join(tmpdir(), "task-game-076-summary-"));
const inputPath = join(tempDir, "input.json");
const summaryJsonPath = join(tempDir, "summary.json");
const summaryMdPath = join(tempDir, "summary.md");

writeFileSync(
  inputPath,
  `${JSON.stringify(
    {
      tier: "required",
      overallStatus: "pass",
      outDir: tempDir,
      commands: {
        viewer_semantic_contract: { status: "pass", log: "viewer.log" },
        software_safe_ui: { status: "pass", log: "ui.log" },
        runtime_control_feeling: { status: "pass", log: "runtime-control.log" },
        runtime_no_progress_recovery: { status: "pass", log: "runtime-recovery.log" },
        runtime_chain_sync_blocker: { status: "pass", log: "runtime-chain.log" },
        runtime_persist_backfill: { status: "pass", log: "runtime-persist.log" },
        bevy_visual_probe: { status: "pass", log: "bevy.log" },
        attraction_sufficiency_cards: { status: "pass", log: "attraction.log" },
        aw_test_completeness_guard: { status: "pass", log: "aw.log" },
      },
      skipBevy: false,
      skipRuntimeUnit: false,
    },
    null,
    2,
  )}\n`,
  "utf8",
);

execFileSync("node", [
  "crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs",
  inputPath,
  summaryJsonPath,
  summaryMdPath,
]);

const summary = JSON.parse(readFileSync(summaryJsonPath, "utf8"));
assert.equal(summary.overall_status, "pass");
assert.equal(summary.attraction_sufficiency_status, "attraction_pass");
assert.equal(summary.attraction_evidence.content_volume_card.status, "content_volume_pass");
assert.equal(summary.attraction_evidence.content_volume_card.effective_play_minutes, 34);
assert.equal(summary.attraction_evidence.content_volume_card.player_operation_count, 22);
assert.equal(summary.attraction_evidence.content_volume_supplement_complete, true);
assert.equal(summary.attraction_evidence.second_run_design_card.status, "second_run_hook_pass");
assert.equal(summary.attraction_evidence.anti_script_design_card.status, "anti_script_pass");
assert.equal(summary.attraction_evidence.anti_script_design_card.visible_midrun_route_consequence, true);
assert.equal(summary.attraction_evidence.anti_script_design_card.local_demand_progress_after_delivery, true);
assert.equal(summary.attraction_evidence.anti_script_design_card.second_session_choice_memory, true);
assert.equal(summary.attraction_evidence.anti_script_design_card.boredom_negative_guard, true);
assert.equal(summary.attraction_evidence.anti_script_design_card.repair_tradeoff_cost_visible, true);
assert.equal(summary.attraction_evidence.route_branch_regression.status, "pass");
assert.notEqual(
  summary.attraction_evidence.route_branch_regression.accelerate.next_session_goal,
  summary.attraction_evidence.route_branch_regression.stabilize.next_session_goal,
);
assert.notEqual(
  summary.attraction_evidence.route_branch_regression.accelerate.local_demand_id,
  summary.attraction_evidence.route_branch_regression.stabilize.local_demand_id,
);
assert.equal(summary.attraction_evidence.boredom_negative_regression.status, "pass");
assert.equal(summary.attraction_evidence.boredom_negative_regression.detected_status, "attraction_weak");
assert.deepEqual(summary.attraction_evidence.sufficiency.missing, []);
for (const segment of [
  "diagnosis_focus",
  "route_tradeoff",
  "micro_commission",
  "incident_recovery",
  "opportunity_scan",
  "return_package",
]) {
  assert.ok(summary.attraction_evidence.implemented_content_segments.includes(segment), segment);
}
for (const key of [
  "local_demand_id",
  "contribution_target_id",
  "world_change_due_to_player",
  "next_session_goal",
  "recovery_action_id",
]) {
  assert.equal(summary.attraction_evidence.gameplay_truth_coverage[key], true, key);
}

const summaryMd = readFileSync(summaryMdPath, "utf8");
assert.match(summaryMd, /content_volume: `content_volume_pass`/);
assert.match(summaryMd, /effective_play_minutes: `34\/30`/);
assert.match(summaryMd, /content_volume_supplement_complete: `true`/);
assert.match(summaryMd, /implemented_content_segments: .*`diagnosis_focus`/);
assert.match(summaryMd, /gameplay_truth_coverage: local_demand=`true`/);
assert.match(summaryMd, /second_run_design: `second_run_hook_pass`/);
assert.match(summaryMd, /anti_script_design: `anti_script_pass`/);
assert.match(summaryMd, /anti_script_design_coverage: midrun_route=`true`, local_demand_progress=`true`, second_session_memory=`true`, boredom_guard=`true`, repair_tradeoff=`true`/);
assert.match(summaryMd, /route_branch_regression: `pass`/);
assert.match(summaryMd, /route_branch_goals: accelerate=`handle_fast_plate_surge_risk_order`, stabilize=`deliver_starter_batch_to_edge_outpost`/);
assert.match(summaryMd, /boredom_negative_regression: `pass` \/ `attraction_weak`/);

const automationSource = readFileSync("scripts/verify-gameplay-attraction-automation.sh", "utf8");
assert.match(
  automationSource,
  /summary_writer_contract[\s\S]*gameplay-attraction-summary-writer\.test\.mjs/,
  "required automation must include the summary writer contract guard",
);

console.log("TASK-GAME-076 summary writer contract passed");
