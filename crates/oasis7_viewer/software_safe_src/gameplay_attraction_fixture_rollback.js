import { TASK_GAME_076_SCENARIO_VERSION } from "./gameplay_attraction_fixture_data.js";

const FIXTURE_ROLLBACK_DEADLINE = "Before incident recovery begins.";
const FIXTURE_ROUTE_PHASES = Object.freeze([
  "before_incident_recovery", "incident_recovery", "repair_completed", "completed",
]);

function truthyText(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function playerReadableText(value) {
  return truthyText(value) &&
    !/\b[a-z][a-z0-9]*(?:_[a-z0-9]+)+\b/.test(value) &&
    !/\b[a-z][a-z0-9]*(?:\.[a-z][a-z0-9]*)+\b/.test(value);
}

// This window belongs to the TASK-GAME-076 fixture, not to runtime route policy.
function fixtureRollbackAvailable(phase) {
  return FIXTURE_ROUTE_PHASES.includes(phase) ? phase === "before_incident_recovery" : null;
}

export function fixtureRollbackQuote(mode, phase) {
  const available = fixtureRollbackAvailable(phase);
  if (available === null) throw new RangeError(`Unknown fixture route phase: ${phase}`);
  const isAccelerate = mode === "accelerate";
  return {
    rollback_available: available,
    rollback_deadline_beat: FIXTURE_ROLLBACK_DEADLINE,
    rollback_unavailable_reason: available ? null :
      "Incident recovery has begun; reopening the route would discard an already consumed consequence.",
    rollback_cost_summary: !available ? "Rollback is closed; no rollback cost is charged." : isAccelerate
      ? "Spend 1 action and forfeit 1 accelerated output to reopen the route choice."
      : "Spend 1 action and give up 1 stability to reopen the route choice.",
    rollback_kept_benefit: !available
      ? "Produced output, completed repairs, and outstanding obligations stay in effect."
      : isAccelerate ? "Keep 2 of the 3 accelerated output already produced."
        : "Keep the Starter Batch: Alloy Plate x3 already produced.",
    rollback_lost_benefit: !available
      ? "The route cannot be reopened; supported repair and replanning choices remain available."
      : isAccelerate ? "Lose access to the high-yield fast plate surge order."
        : "Lose the stabilized line's lower jam risk for the next incident.",
  };
}

export function fixtureRollbackWindowConsistent(snapshot) {
  const scenario = snapshot.task_game_076_scenario;
  const gameplay = snapshot.player_gameplay || {};
  const route = gameplay.route_tradeoff;
  if (scenario?.version !== TASK_GAME_076_SCENARIO_VERSION ||
      !["route.accelerate_output", "route.stabilize_input_line"].includes(route?.route_commitment_id)) {
    return true;
  }
  const available = fixtureRollbackAvailable(scenario.route_phase);
  if (available === null || route.rollback_available !== available ||
      route.rollback_deadline_beat !== FIXTURE_ROLLBACK_DEADLINE) return false;
  const incident = gameplay.incident_recovery;
  const sameRouteIncident = incident?.triggered_by_route === route.route_commitment_id;
  const repaired = sameRouteIncident && (truthyText(incident.selected_repair) ||
    truthyText(incident.residual_risk_after_repair) || truthyText(incident.repaired_capability_delta));
  if ((available && sameRouteIncident) || (scenario.route_phase === "incident_recovery" && repaired)) return false;
  return available ? !truthyText(route.rollback_unavailable_reason)
    : playerReadableText(route.rollback_unavailable_reason);
}
