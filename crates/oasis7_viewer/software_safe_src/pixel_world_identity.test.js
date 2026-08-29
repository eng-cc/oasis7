import { describe, expect, it } from "vitest";
import { pixelWorldReadableAgentLabel, pixelWorldReadableEntityText, pixelWorldSelectedEntityLabel } from "./pixel_world_identity.js";

const visualState = {
  agents: [{ id: "agent-0", name: "Survey Agent" }],
  locations: [{ id: "location-0" }],
};

describe("pixel world player-facing identity", () => {
  it("normalizes agent ids embedded in player leverage copy", () => {
    expect(pixelWorldReadableEntityText("Queue smelter for agent-0", visualState)).toBe("Queue smelter for Survey Agent");
  });

  it("humanizes nameless arbitrary agent slugs across labels and embedded copy", () => {
    const slugState = { ...visualState, agents: [{ id: "agent-builder" }, { id: "agent_factory_operator" }] };
    expect(pixelWorldReadableAgentLabel(slugState.agents[0])).toBe("Agent Builder");
    expect(pixelWorldReadableAgentLabel(slugState.agents[1])).toBe("Agent Factory Operator");
    expect(pixelWorldSelectedEntityLabel(slugState, { kind: "agent", id: "agent-builder" })).toBe("Agent Builder");
    expect(pixelWorldSelectedEntityLabel(slugState, { kind: "agent", id: "agent-builder" }, true)).toBe("行动体 Builder");
    expect(pixelWorldReadableEntityText("Queue smelter for agent-builder", slugState)).toBe("Queue smelter for Agent Builder");
    expect(pixelWorldReadableEntityText("Queue smelter for agent-builder", slugState, true)).toBe("Queue smelter for 行动体 Builder");
  });

  it("normalizes a location fallback instead of exposing its raw id", () => {
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" })).toBe("Location 0");
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" }, true)).toBe("地点 0");
    expect(pixelWorldSelectedEntityLabel({ ...visualState, locations: [] }, { kind: "location", id: "loc-42" })).toBe("Location 42");
  });
});
