import { describe, expect, it } from "vitest";
import { normalizeIndustrialStarterProfile } from "./industrial_starter_outcome.js";
const fixture = () => ({ profile_id: "starter-industrial-smelter-to-assembler-v1", profile_revision: 1, status: "candidate_available", progression_effect: "open_assembler_candidate_only", next_action: "build_factory_assembler_mk1", evidence_class: "durable-milestone-backed", settled_outcome: { settlement_job_id: 5, settled_at: 27, settlement: { requester_agent_id: "starter-agent-0", accepted_batches: 12, consume: [{ kind: "iron_ore", amount: 48 }], produce: [{ kind: "iron_ingot", amount: 36 }], power_required: 24 } } });
describe("starter industrial outcome", () => {
  it("uses actual settled inputs/output and separates next candidate", () => {
    const result = normalizeIndustrialStarterProfile(fixture());
    expect(result.settled.produce).toEqual([{kind:"iron_ingot",amount:36}]);
    expect(result.settled.consume).toEqual([{kind:"iron_ore",amount:48}]);
    expect(result.settled.powerRequired).toBe(24);
    expect(result.candidateAvailable).toBe(true);
  });
  it("does not advertise Assembler eligibility for a preproduction chain candidate", () => {
    const data = fixture(); delete data.settled_outcome;
    data.next_action = "build_factory_smelter_mk1"; data.progression_effect = "none"; data.evidence_class = "candidate-only";
    expect(normalizeIndustrialStarterProfile(data).candidateAvailable).toBe(false);
  });
  it("keeps legacy absent settlement unpublished without deriving from balance", () => {
    const data = fixture(); delete data.settled_outcome; data.iron_ingot_balance = 999;
    expect(normalizeIndustrialStarterProfile(data).settled).toBeNull();
  });
  it("rejects malformed numeric settlement provenance", () => {
    for (const invalid of [true, 1.5, -1, "36"]) {
      const data = fixture(); data.settled_outcome.settlement.produce[0].amount = invalid;
      expect(normalizeIndustrialStarterProfile(data).settled).toBeNull();
    }
  });
});
