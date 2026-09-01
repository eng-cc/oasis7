import { describe, expect, it } from "vitest";
import { buildAgentContextDisplayModel } from "./viewer_agent_context_display_model.js";

const AGENT_ID = "agent-0";
const LOCATION_ID = "loc-0";

const MATCHED_INTENT = {
  schema_version: 2,
  intent_id: "intent:agent-0:accepted",
  agent_id: AGENT_ID,
  target_agent_id: AGENT_ID,
  world_id: "world-test",
  reorg_epoch: 0,
  logical_time: 12,
  event_seq: 4,
  updated_at: 12,
  status: "accepted",
  source_class: "runtime_projection",
  freshness: "current",
  control_state: "controllable",
  copy_schema_version: 1,
  summary: "Agent guidance accepted; the Agent will evaluate its next world action.",
};

const AGENT = {
  id: AGENT_ID,
  name: "Atlas",
  label: "Northline Scout",
  location_id: LOCATION_ID,
  state: "executing",
  freshness: "current",
  activity: {
    status: "executing",
    operation: "route",
    target: "Factory Anchor",
    updated_at: 0,
  },
};

const LOCATION = {
  id: LOCATION_ID,
  name: "Factory Anchor",
  label: "Factory Anchor",
};

const GAMEPLAY = {
  agent_id: AGENT_ID,
  objective: "Stabilize the first production line before expanding.",
  nextStepHint: "Replenish upstream materials, then advance again to confirm the line resumes.",
  blockerKind: "material_shortage",
  blockerDetail: "iron input exhausted at factory-0",
  progressionProof: {
    leverageVerdict: "watch: recovery can restore the first capability",
    leverageClass: "repair_elasticity",
  },
  primaryIntent: MATCHED_INTENT,
};

function snapshot({ agent = AGENT, location = LOCATION } = {}) {
  return {
    model: {
      agents: agent ? { [AGENT_ID]: agent } : {},
      locations: location ? { [LOCATION_ID]: location } : {},
    },
  };
}

function buildInput(overrides = {}) {
  return {
    snapshot: snapshot(),
    selected: { kind: "agent", id: AGENT_ID },
    gameplay: GAMEPLAY,
    connectionStatus: "connected",
    freshness: "current",
    feedback: null,
    receipt: null,
    locale: "en",
    ...overrides,
  };
}

describe("Agent Context Lite display model", () => {
  it("composes selected-Agent identity, activity, freshness, gameplay context, and matching Intent", () => {
    const model = buildAgentContextDisplayModel(buildInput());

    expect(model).toMatchObject({
      kind: "agent",
      id: AGENT_ID,
      identity: {
        name: "Atlas",
        label: "Northline Scout",
        id: AGENT_ID,
      },
      state: expect.objectContaining({ kind: "executing" }),
      freshness: expect.objectContaining({ kind: "current" }),
      activity: expect.objectContaining({
        kind: "executing",
        operation: "Route",
        targetLabel: "Factory Anchor",
      }),
      objective: expect.objectContaining({ value: GAMEPLAY.objective }),
      nextMove: expect.objectContaining({ value: GAMEPLAY.nextStepHint }),
      blocker: expect.objectContaining({ value: GAMEPLAY.blockerDetail }),
      playerLeverage: expect.objectContaining({
        value: GAMEPLAY.progressionProof.leverageVerdict,
      }),
      intent: expect.objectContaining({
        kind: "matched",
        agentId: AGENT_ID,
        status: "accepted",
      }),
    });
  });

  it.each([
    ["current", "current"],
    ["last-known", "last-known"],
    ["stale", "stale"],
    ["reconnecting", "reconnecting"],
    ["unknown", "unknown"],
    ["conflict", "conflict"],
    ["replay", "replay"],
    ["gap", "gap"],
    ["reorg", "reorg"],
    ["unavailable", "unavailable"],
  ])("preserves explicit %s freshness without using wall-clock or activity timestamps", (freshness, expectedKind) => {
    const model = buildAgentContextDisplayModel(buildInput({
      freshness,
      connectionStatus: freshness === "reconnecting" ? "reconnecting" : "connected",
      snapshot: snapshot({
        agent: { ...AGENT, freshness },
      }),
    }));

    expect(model.freshness).toEqual(expect.objectContaining({ kind: expectedKind }));
  });

  it("does not wrap a player-global or mismatched Intent as selected-Agent Intent", () => {
    const mismatchedIntent = {
      ...MATCHED_INTENT,
      intent_id: "intent:other-agent:accepted",
      agent_id: "agent-1",
      target_agent_id: "agent-1",
    };
    const model = buildAgentContextDisplayModel(buildInput({
      gameplay: { ...GAMEPLAY, primaryIntent: mismatchedIntent },
    }));

    expect(model.intent).toMatchObject({ kind: "unavailable" });
    expect(model.intent).not.toMatchObject({ status: "accepted", agentId: AGENT_ID });
    expect(JSON.stringify(model)).not.toContain("intent:other-agent:accepted");
  });

  it("does not project an unbound global gameplay summary into a selected Agent in a multi-Agent world", () => {
    const globalGameplay = {
      ...GAMEPLAY,
      agent_id: null,
      objective: "Global player objective must stay outside Agent Context.",
      nextStepHint: "Global player next step must stay outside Agent Context.",
      blockerDetail: "Global player blocker must stay outside Agent Context.",
      progressionProof: {
        ...GAMEPLAY.progressionProof,
        leverageVerdict: "Global player leverage must stay outside Agent Context.",
      },
      primaryIntent: MATCHED_INTENT,
    };
    const model = buildAgentContextDisplayModel(buildInput({
      snapshot: {
        model: {
          agents: {
            [AGENT_ID]: AGENT,
            "agent-1": { ...AGENT, id: "agent-1", name: "Other Agent", label: "Other Scout" },
          },
          locations: { [LOCATION_ID]: LOCATION },
        },
      },
      gameplay: globalGameplay,
    }));

    expect(model.objective).toMatchObject({ value: null, state: "unavailable" });
    expect(model.nextMove).toMatchObject({ value: null, state: "unavailable" });
    expect(model.blocker).toMatchObject({ value: null, state: "unavailable" });
    expect(model.playerLeverage).toMatchObject({ value: null, state: "unavailable" });
    expect(model.intent).toMatchObject({ kind: "matched", agentId: AGENT_ID, status: "accepted" });
    expect(JSON.stringify(model)).not.toContain("Global player objective must stay outside Agent Context.");
    expect(JSON.stringify(model)).not.toContain("Global player next step must stay outside Agent Context.");
    expect(JSON.stringify(model)).not.toContain("Global player blocker must stay outside Agent Context.");
    expect(JSON.stringify(model)).not.toContain("Global player leverage must stay outside Agent Context.");
  });

  it("keeps a null Intent unavailable instead of inventing a plan or accepted lifecycle", () => {
    const model = buildAgentContextDisplayModel(buildInput({
      gameplay: { ...GAMEPLAY, primaryIntent: null, primary_intent: null },
    }));

    expect(model.intent).toMatchObject({ kind: "unavailable" });
    expect(model.intent).not.toHaveProperty("status", "accepted");
    expect(JSON.stringify(model)).not.toMatch(/accepted|primary.?intent/i);
  });

  it("keeps non-causal feedback separate from causal Receipt state", () => {
    const model = buildAgentContextDisplayModel(buildInput({
      gameplay: {
        ...GAMEPLAY,
        recentFeedback: {
          source: "runtime",
          action: "step",
          stage: "ack",
          effect: "The request was queued for runtime evaluation.",
        },
      },
      feedback: {
        kind: "gameplay_action",
        stage: "ack",
        effect: "The request was queued for runtime evaluation.",
      },
      receipt: null,
    }));

    expect(model.feedback).toMatchObject({ kind: "status" });
    expect(model.receipt).toMatchObject({ state: "none" });
    expect(model.receipt).not.toMatchObject({ present: true, confidence: "world_delta" });
  });

  it("uses only a supplied authoritative Receipt and never derives one from feedback", () => {
    const receipt = {
      present: true,
      state: "confirmed",
      confidence: "world_delta",
      title: "Action Receipt",
      summary: "The world change was confirmed.",
      target_agent_id: AGENT_ID,
    };
    const model = buildAgentContextDisplayModel(buildInput({ receipt }));

    expect(model.receipt).toMatchObject({
      present: true,
      state: "confirmed",
      confidence: "world_delta",
      targetAgentId: AGENT_ID,
    });
  });

  it.each([
    ["another Agent", { target_agent_id: "agent-1" }],
    ["no Agent", {}],
  ])("rejects an explicit Receipt bound to %s", (_description, target) => {
    const model = buildAgentContextDisplayModel(buildInput({
      receipt: {
        present: true,
        state: "confirmed",
        confidence: "world_delta",
        ...target,
      },
    }));

    expect(model.receipt).toEqual({ present: false, state: "none" });
  });

  it("clears Agent context for a selected non-Agent and reports honest unavailable state", () => {
    const model = buildAgentContextDisplayModel(buildInput({
      selected: { kind: "location", id: LOCATION_ID },
      receipt: {
        present: true,
        state: "confirmed",
        confidence: "world_delta",
        target_agent_id: AGENT_ID,
      },
    }));

    expect(model).toMatchObject({
      kind: "location",
      id: LOCATION_ID,
      identity: { name: "Factory Anchor", id: LOCATION_ID },
      state: expect.objectContaining({ kind: "unavailable" }),
      unavailableReason: expect.any(String),
    });
    expect(model.activity).toBeNull();
    expect(model.intent).toBeNull();
    expect(model.playerLeverage).toBeNull();
    expect(JSON.stringify(model)).not.toMatch(/Atlas|Northline Scout|repair_elasticity|intent:agent-0/);
  });
});
