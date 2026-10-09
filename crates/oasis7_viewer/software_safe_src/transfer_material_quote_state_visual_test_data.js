export const VISUAL_FIXTURE_NAME = "transfer_material_quote";

export const visualFixtureQuote = Object.freeze({
  requester_agent_id: "agent-0", from_ledger: "site:source", to_ledger: "site:destination", kind: "iron_ingot", requested_amount: 20,
  submission_feasible: true, max_transferable_amount: 40, sent_amount: 20, distance_km: 200, loss_bps: 5, expected_loss_amount: 2,
  expected_received_amount: 18, source_amount_before: 40, source_amount_after: 20, destination_amount_before: 0, destination_expected_amount_after: 18,
  ticks_until_arrival: 2, ready_at: 3, effective_priority: "standard", priority_reason: "material_default_priority", inflight_before: 0,
  inflight_capacity: 2, path_id: "path:source-relay-destination", route_ids: ["route:source-relay", "route:relay-destination"], tariff_electricity_total: 12, reroute_count: 0, recommendation: "submit_transfer", conditional: true,
});
