#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import {
  buildTaskGame076AttractionEvidence,
  buildTaskGame076AutomationSummary,
  TASK_GAME_076_BEATS,
  TASK_GAME_076_SCENARIO_VERSION,
} from "../software_safe_src/gameplay_attraction_scenario.js";

function usage() {
  console.error("Usage: write-gameplay-attraction-automation-summary.mjs <input.json> <summary.json> <summary.md>");
}

const [inputPath, summaryJsonPath, summaryMdPath] = process.argv.slice(2);
if (!inputPath || !summaryJsonPath || !summaryMdPath) {
  usage();
  process.exit(2);
}

const input = JSON.parse(readFileSync(inputPath, "utf8"));
const summary = buildTaskGame076AutomationSummary(input);
const attractionEvidence = buildTaskGame076AttractionEvidence();
summary.attraction_evidence = attractionEvidence;
summary.attraction_sufficiency_status = attractionEvidence.sufficiency.status;
writeFileSync(summaryJsonPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");

const lines = [
  "# TASK-GAME-076 Gameplay Attraction Automation Summary",
  "",
  `- scenario_version: \`${TASK_GAME_076_SCENARIO_VERSION}\``,
  `- tier: \`${summary.tier}\``,
  `- overall_status: \`${summary.overall_status}\``,
  `- attraction_sufficiency_status: \`${summary.attraction_sufficiency_status}\``,
  `- out_dir: \`${summary.out_dir}\``,
  "",
  "## Commands",
  "",
];

for (const key of Object.keys(summary.commands).sort()) {
  const item = summary.commands[key];
  lines.push(`- \`${key}\`: \`${item.status}\` (${item.log})`);
}

lines.push("", "## Beat Coverage", "");
for (const beat of summary.beats) {
  const note = beat.status_note ? ` / ${beat.status_note}` : "";
  lines.push(`- \`${beat.time}\` \`${beat.beat}\`: \`${beat.status}\` / \`${beat.provenance}\`${note}`);
  lines.push(`  - assertion: ${beat.assertion}`);
  if (beat.missing_or_failed_commands.length > 0) {
    lines.push(`  - missing_or_failed_commands: ${beat.missing_or_failed_commands.join(", ")}`);
  }
}

lines.push("", "## Attraction Sufficiency", "");
lines.push(`- status: \`${attractionEvidence.sufficiency.status}\``);
lines.push(`- average_hook_score: \`${attractionEvidence.sufficiency.average_hook_score}\``);
lines.push(`- average_replay_intent: \`${attractionEvidence.sufficiency.average_replay_intent}\``);
lines.push(`- motivation_density: \`${attractionEvidence.motivation_density_card.status}\``);
lines.push(`- content_volume: \`${attractionEvidence.content_volume_card.status}\``);
lines.push(
  `- effective_play_minutes: \`${attractionEvidence.content_volume_card.effective_play_minutes}/${attractionEvidence.content_volume_card.target_effective_play_minutes}\``,
);
lines.push(`- player_operation_count: \`${attractionEvidence.content_volume_card.player_operation_count}\``);
lines.push(`- content_unit_count: \`${attractionEvidence.content_volume_card.content_unit_count}\``);
lines.push(`- distinct_action_family_count: \`${attractionEvidence.content_volume_card.distinct_action_family_count}\``);
lines.push(`- content_volume_supplement_complete: \`${attractionEvidence.content_volume_supplement_complete}\``);
lines.push(`- implemented_content_segments: ${attractionEvidence.implemented_content_segments.map((item) => `\`${item}\``).join(", ")}`);
lines.push(
  `- gameplay_truth_coverage: local_demand=\`${attractionEvidence.gameplay_truth_coverage.local_demand_id}\`, contribution_target=\`${attractionEvidence.gameplay_truth_coverage.contribution_target_id}\`, world_change=\`${attractionEvidence.gameplay_truth_coverage.world_change_due_to_player}\`, next_session_goal=\`${attractionEvidence.gameplay_truth_coverage.next_session_goal}\`, recovery_action=\`${attractionEvidence.gameplay_truth_coverage.recovery_action_id}\``,
);
lines.push(`- second_run_design: \`${attractionEvidence.second_run_design_card.status}\``);
lines.push(
  `- second_run_design_coverage: route_persistence=\`${attractionEvidence.second_run_design_card.route_tradeoff_persists_across_beats}\`, named_output=\`${attractionEvidence.second_run_design_card.named_commission_output}\`, generated_opportunity=\`${attractionEvidence.second_run_design_card.opportunity_generated_from_choice}\`, choice_return=\`${attractionEvidence.second_run_design_card.choice_reflective_return_goal}\``,
);
lines.push(`- anti_script_design: \`${attractionEvidence.anti_script_design_card.status}\``);
lines.push(
  `- anti_script_design_coverage: midrun_route=\`${attractionEvidence.anti_script_design_card.visible_midrun_route_consequence}\`, local_demand_progress=\`${attractionEvidence.anti_script_design_card.local_demand_progress_after_delivery}\`, second_session_memory=\`${attractionEvidence.anti_script_design_card.second_session_choice_memory}\`, boredom_guard=\`${attractionEvidence.anti_script_design_card.boredom_negative_guard}\`, repair_tradeoff=\`${attractionEvidence.anti_script_design_card.repair_tradeoff_cost_visible}\``,
);
lines.push(`- route_branch_regression: \`${attractionEvidence.route_branch_regression.status}\``);
lines.push(
  `- route_branch_goals: accelerate=\`${attractionEvidence.route_branch_regression.accelerate.next_session_goal}\`, stabilize=\`${attractionEvidence.route_branch_regression.stabilize.next_session_goal}\``,
);
lines.push(`- weak_sample_regression: \`${attractionEvidence.weak_sample_regression.status}\` / \`${attractionEvidence.weak_sample_regression.detected_verdict}\``);
lines.push(`- boredom_negative_regression: \`${attractionEvidence.boredom_negative_regression.status}\` / \`${attractionEvidence.boredom_negative_regression.detected_status}\``);
if (attractionEvidence.sufficiency.missing.length > 0) {
  lines.push(`- missing: ${attractionEvidence.sufficiency.missing.join(", ")}`);
}
lines.push("", "### Attraction Cards", "");
for (const card of attractionEvidence.attraction_cards) {
  lines.push(
    `- \`${card.sample_id}\`: hook=\`${card.hook_score}\`, replay=\`${card.replay_intent}\`, verdict=\`${card.verdict}\`, provenance=\`${card.provenance}\``,
  );
  lines.push(`  - caused: ${card.what_did_i_cause}`);
  lines.push(`  - continue: ${card.continue_reason}`);
}

lines.push(
  "",
  "## Scenario Beats",
  "",
);
for (const beat of TASK_GAME_076_BEATS) {
  lines.push(`- \`${beat.time}\` \`${beat.beat}\`: ${beat.assertion}`);
}

lines.push(
  "",
  "## Policy",
  "",
  "- Manual attraction cards can explain fun, boredom, and replay intent, but do not replace automation.",
  "- Bevy/pixel-world automation verifies spatial/visual/canvas readability only; gameplay causality must come from pure API or Rust runtime harnesses.",
  "- Viewer and visual fixtures must be derived from the TASK-GAME-076 scenario driver, not hand-written as independent truth.",
  "- Deterministic-provider-backed attraction evidence can support the design sufficiency gate; real player retention still needs live/provider playtest samples.",
  "- Run `--tier live` before using this summary as real player-path or pure API gameplay evidence.",
);

writeFileSync(summaryMdPath, `${lines.join("\n")}\n`, "utf8");
