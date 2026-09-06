import { describe, expect, it } from "vitest";
import {
  AGENT_CONTEXT_FIXTURE_COPIES,
  AGENT_CONTEXT_FIXTURE_MODES,
  AGENT_CONTEXT_FIXTURE_STATES,
  buildAgentContextFixtureState,
  buildAgentContextRichFixtureSnapshot,
} from "./agent_context_visual_fixture.js";

function baseSnapshot() {
  return {
    time: 12,
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Atlas",
          label: "Northline Scout",
          location_id: "loc-0",
          state: "idle",
          freshness: "current",
          activity: { status: "idle" },
        },
      },
      locations: { "loc-0": { id: "loc-0", name: "Factory Anchor" } },
    },
    player_gameplay: {},
  };
}

describe("Agent Context headed visual fixture contract", () => {
  it("accepts only the explicit rich/unavailable mode and state/copy selectors", () => {
    expect(AGENT_CONTEXT_FIXTURE_MODES).toEqual(["rich", "unavailable"]);
    expect(AGENT_CONTEXT_FIXTURE_STATES).toEqual(["current", "stale", "reconnecting"]);
    expect(AGENT_CONTEXT_FIXTURE_COPIES).toEqual(["short", "long"]);
    expect(buildAgentContextFixtureState(
      "?agent_context_mode=rich&agent_context_state=reconnecting&agent_context_copy=long",
    )).toEqual({ mode: "rich", state: "reconnecting", copy: "long" });
    expect(buildAgentContextFixtureState(
      "?agent_context_mode=unknown&agent_context_state=unknown&agent_context_copy=unknown",
    )).toEqual({ mode: "unavailable", state: "current", copy: "short" });
  });

  it("publishes a rich explicitly Agent-bound projection with long player-safe copy", () => {
    const snapshot = buildAgentContextRichFixtureSnapshot(
      baseSnapshot,
      { mode: "rich", state: "stale", copy: "long" },
      "en",
    );
    const fixture = snapshot.viewer_test_agent_context;
    expect(fixture).toMatchObject({ mode: "rich", state: "stale", copy: "long" });
    expect(fixture.gameplay).toMatchObject({
      agent_id: "agent-0",
      objective: expect.stringContaining("Stabilize the first production line"),
      nextStepHint: expect.any(String),
      blockerDetail: expect.stringContaining("iron input"),
      progressionProof: { leverageVerdict: expect.any(String) },
    });
    expect(fixture.gameplay.objective.length).toBeGreaterThan(80);
    expect(snapshot.player_gameplay.primary_intent).toMatchObject({
      agent_id: "agent-0",
      target_agent_id: "agent-0",
      freshness: "stale",
      status: "accepted",
    });
    expect(snapshot.model.agents["agent-0"]).toMatchObject({ freshness: "stale" });
  });

  it("covers the deterministic en/zh current-stale-reconnecting headed matrix", () => {
    for (const locale of ["en", "zh"]) {
      for (const state of AGENT_CONTEXT_FIXTURE_STATES) {
        const snapshot = buildAgentContextRichFixtureSnapshot(
          baseSnapshot,
          { mode: "rich", state, copy: "long" },
          locale,
        );
        const fixture = snapshot.viewer_test_agent_context;
        expect(fixture).toMatchObject({
          schema: "agent-context-fixture/v1",
          measurement: "groups-fields",
          mode: "rich",
          state,
          copy: "long",
        });
        expect(fixture.gameplay.objective.length).toBeGreaterThan(20);
        expect(fixture.gameplay.blockerDetail.length).toBeGreaterThan(20);
        expect(snapshot.player_gameplay.primary_intent).toMatchObject({
          agent_id: "agent-0",
          target_agent_id: "agent-0",
          freshness: state,
        });
        expect(snapshot.model.agents["agent-0"]).toMatchObject({ freshness: state });
      }
    }
  });

  it("keeps the unavailable mode projection-free instead of falling back to player-global gameplay", () => {
    const snapshot = buildAgentContextRichFixtureSnapshot(
      baseSnapshot,
      { mode: "unavailable", state: "current", copy: "short" },
      "zh",
    );
    expect(snapshot.viewer_test_agent_context).toMatchObject({ mode: "unavailable", state: "current", copy: "short" });
    expect(snapshot.viewer_test_agent_context.gameplay).toBeNull();
    expect(snapshot.player_gameplay.primary_intent).toBeNull();
    expect(snapshot.player_gameplay.objective).toBeUndefined();
  });
});
