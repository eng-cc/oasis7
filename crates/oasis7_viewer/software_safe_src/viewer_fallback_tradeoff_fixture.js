export function fallbackTradeoffVisualFixture() {
  return [
    {
      value_class: "safe_wait",
      available: false,
      cost: "No bounded wait trigger is currently available.",
      progress_kept: "Keeps the current intent unchanged.",
      opportunity_cost: "Waiting cannot verify or repair the blocker.",
      reason: "The runtime has no canonical tick or event trigger that bounds a safe wait.",
      recommended: false,
    },
    {
      value_class: "repair_now",
      available: false,
      cost: "Refresh the gameplay snapshot and inspect the current blocker.",
      progress_kept: "Keeps the current intent while checking recovery state.",
      opportunity_cost: "Uses the next decision on diagnosis instead of a new goal.",
      reason: "No repair action is currently available for the published blocker.",
      recommended: false,
    },
    {
      value_class: "reroute_now",
      available: false,
      cost: "Replace the current Agent short-term goal.",
      progress_kept: "Preserves the recorded intent for comparison, not execution progress.",
      opportunity_cost: "Moves attention from repairing the current blocked intent.",
      reason: "No enabled reprioritize action is currently available.",
      recommended: false,
    },
  ];
}
