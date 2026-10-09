const text = value => typeof value === "string" && value.trim() ? value : null;
const integer = value => Number.isSafeInteger(value) && value >= 0;
const stacks = value => Array.isArray(value) && value.every(item => text(item?.kind) && integer(item?.amount)) ? value.map(item => ({ kind: item.kind, amount: item.amount })) : null;

export function normalizeIndustrialStarterProfile(value) {
  if (!value || !text(value.profile_id) || !integer(value.profile_revision) || value.profile_revision === 0) return null;
  const outcome = value.settled_outcome, settlement = outcome?.settlement;
  let settled = null;
  if (outcome && settlement && text(settlement.requester_agent_id) && integer(outcome.settlement_job_id)
    && integer(outcome.settled_at) && integer(settlement.accepted_batches) && settlement.accepted_batches > 0
    && integer(settlement.power_required) && stacks(settlement.consume) && stacks(settlement.produce)
    && settlement.produce.some(item => item.kind === "iron_ingot" && item.amount > 0)) {
    settled = { jobId: outcome.settlement_job_id, settledAt: outcome.settled_at, owner: settlement.requester_agent_id,
      acceptedBatches: settlement.accepted_batches, consume: stacks(settlement.consume), produce: stacks(settlement.produce), powerRequired: settlement.power_required };
  }
  return { profileId: value.profile_id, profileRevision: value.profile_revision, candidateAvailable: value.status === "candidate_available" && value.next_action === "build_factory_assembler_mk1" && value.progression_effect === "open_assembler_candidate_only" && value.evidence_class === "durable-milestone-backed", progressionEffect: value.progression_effect, settled };
}
