#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
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

assert.equal(summary.evidence_sources.fixture_structure.scope, "fixture_structure_only");
assert.equal(summary.evidence_sources.live_operations.status, "unverified");
assert.equal(summary.evidence_sources.human_observation.status, "unverified");
assert.equal(summary.evidence_sources.human_observation.meaningful_decision_count, null);
const observedInput = JSON.parse(readFileSync(inputPath, "utf8"));
observedInput.tier = "live";
observedInput.commands.live_pure_api_gameplay = { status: "pass", log: "candidate-api.log" };
observedInput.humanObservations = [
  { participant_id: "p1", session_id: "s1", decision_id: "reduce-batch", choice: "small batch", consequence: "kept power reserve" },
  { participant_id: "p1", session_id: "s2", decision_id: "reduce-batch", choice: "small batch", consequence: "kept power reserve" },
];
writeFileSync(inputPath, JSON.stringify(observedInput));
execFileSync("node", ["crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs", inputPath, summaryJsonPath, summaryMdPath]);
const observed = JSON.parse(readFileSync(summaryJsonPath, "utf8"));
assert.equal(observed.evidence_sources.live_operations.commands.live_pure_api_gameplay.log, "candidate-api.log");
assert.equal(observed.evidence_sources.human_observation.meaningful_decision_count, 1);
assert.equal(observed.evidence_sources.human_observation.retention_status, "unverified");
observedInput.commands.live_pure_api_gameplay.status = "unknown";
observedInput.humanObservations = [{ choice: "two buttons" }];
writeFileSync(inputPath, JSON.stringify(observedInput));
execFileSync("node", ["crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs", inputPath, summaryJsonPath, summaryMdPath]);
const unknown = JSON.parse(readFileSync(summaryJsonPath, "utf8"));
assert.equal(unknown.overall_status, "unverified");
assert.equal(unknown.evidence_sources.human_observation.status, "unverified");
assert.equal(unknown.evidence_sources.human_observation.meaningful_decision_count, null);
console.log("TASK-GAME-076 summary writer contract passed");

// Exercise the actual wrapper with isolated gate stubs; no live/human evidence is generated.
const wrapperRoot = join(tempDir, "wrapper repo");
mkdirSync(join(wrapperRoot, "scripts"), { recursive: true });
mkdirSync(join(wrapperRoot, "crates"));
mkdirSync(join(wrapperRoot, "bin"));
writeFileSync(join(wrapperRoot, "scripts/verify-gameplay-attraction-automation.sh"), automationSource);
writeFileSync(join(wrapperRoot, "scripts/cargo-dev-lib.sh"), "oasis7_cargo_dev() { exit 99; }\n");
writeFileSync(join(wrapperRoot, "scripts/viewer-dependency-preflight.sh"), 'viewer_dependency_preflight() { printf preflight >> "$WRAPPER_GATE_MARKER"; }\n');
symlinkSync(join(process.cwd(), "crates/oasis7_viewer"), join(wrapperRoot, "crates/oasis7_viewer"));
const realNode = process.execPath;
for (const [command, body] of Object.entries({
  npm: "#!/bin/sh\nexit 0\n",
  node: `#!/bin/sh
case "$1" in *write-gameplay-attraction-automation-summary.mjs) exec "$WRAPPER_REAL_NODE" "$@" ;; esac
exit 0
`,
})) {
  const path = join(wrapperRoot, "bin", command);
  writeFileSync(path, body);
  chmodSync(path, 0o755);
}
const samplesPath = join(tempDir, "user samples.json");
const humanPath = join(tempDir, "user observations.json");
writeFileSync(samplesPath, JSON.stringify(summary.attraction_evidence.raw_snapshots));
const suppliedHumanRecords = [
  { participant_id: "unit-participant", session_id: "unit-session", decision_id: "unit-choice", choice: "unit choice", consequence: "unit consequence" },
  { choice: "excluded incomplete unit record" },
];
writeFileSync(humanPath, JSON.stringify(suppliedHumanRecords));
const opaqueCandidate = "unit candidate $HOME `literal` \"quote\"";
function runWrapper(name, extra = []) {
  const out = join(tempDir, name);
  const gateMarker = join(tempDir, `${name}-gates`);
  const result = spawnSync("bash", [join(wrapperRoot, "scripts/verify-gameplay-attraction-automation.sh"), "--out-dir", out, "--skip-bevy", "--skip-runtime-unit", ...extra], {
    env: { ...process.env, WRAPPER_GATE_MARKER: gateMarker, WRAPPER_REAL_NODE: realNode, PATH: `${join(wrapperRoot, "bin")}:${process.env.PATH}` }, encoding: "utf8",
  });
  return { result, out, gateMarker };
}
function wrapperSummary(run) {
  assert.equal(run.result.status, 0, run.result.stderr);
  const path = run.result.stdout.match(/automation summary: (.*)\n/)[1];
  return JSON.parse(readFileSync(path, "utf8"));
}
const supplied = wrapperSummary(runWrapper("supplied report", [
  "--completion-profile", "production_only", "--candidate", opaqueCandidate, "--fixture-samples-json", samplesPath, "--human-observations-json", humanPath,
]));
assert.equal(supplied.attraction_evidence.completion_profile, "production_only");
assert.equal(supplied.tier, "required");
assert.equal(supplied.evidence_sources.fixture_structure.source, "input.fixtureSamples");
assert.equal(supplied.evidence_sources.live_operations.candidate, opaqueCandidate);
assert.equal(supplied.evidence_sources.human_observation.status, "observed");
assert.equal(supplied.evidence_sources.human_observation.excluded_record_count, 1);
assert.deepEqual(supplied.attraction_evidence.raw_snapshots, summary.attraction_evidence.raw_snapshots);
assert.deepEqual(supplied.evidence_sources.human_observation.records, [suppliedHumanRecords[0]]);
assert.equal(supplied.evidence_sources.human_observation.meaningful_decision_count, 1);
assert.equal(supplied.evidence_sources.human_observation.retention_status, "unverified");
assert.equal(supplied.evidence_sources.live_operations.status, "unverified");
const defaults = wrapperSummary(runWrapper("default report"));
assert.equal(defaults.attraction_evidence.completion_profile, "production_and_delivery");
assert.equal(defaults.evidence_sources.fixture_structure.source, "default_scenario_fixture");
assert.equal(defaults.evidence_sources.live_operations.candidate, null);
assert.equal(defaults.evidence_sources.human_observation.status, "unverified");
const malformedPath = join(tempDir, "malformed.json");
const objectPath = join(tempDir, "object.json");
const nullRecordPath = join(tempDir, "null-record.json");
const nonJsonNumberPath = join(tempDir, "non-json-number.json");
writeFileSync(malformedPath, "{bad JSON");
writeFileSync(objectPath, "{}");
writeFileSync(nullRecordPath, "[null]");
writeFileSync(nonJsonNumberPath, '[{"value": NaN}]');
for (const [index, args] of [
  ["--completion-profile", "unknown"], ["--candidate"], ["--candidate", "  "],
  ["--fixture-samples-json", join(tempDir, "missing.json")],
  ["--fixture-samples-json", malformedPath], ["--fixture-samples-json", objectPath],
  ["--human-observations-json", nullRecordPath],
  ["--fixture-samples-json", nonJsonNumberPath],
].entries()) {
  const rejected = runWrapper(`rejected-${index}`, args);
  assert.notEqual(rejected.result.status, 0, JSON.stringify(args));
  assert.match(rejected.result.stderr, /error:/);
  assert.throws(() => readFileSync(rejected.gateMarker), /ENOENT/, "invalid input must fail before dependency preflight or gates");
}
rmSync(tempDir, { recursive: true, force: true });
console.log("TASK-GAME-076 wrapper report inputs passed");
