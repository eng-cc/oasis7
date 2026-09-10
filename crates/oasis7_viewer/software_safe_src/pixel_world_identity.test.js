import { describe, expect, it } from "vitest";
import {
  pixelWorldEntityMarkerCode,
  pixelWorldReadableAgentLabel,
  pixelWorldReadableEntityText,
  pixelWorldSelectedEntityLabel,
} from "./pixel_world_identity.js";

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

  it("does not rewrite natural-language agent compounds that are not entity ids", () => {
    expect(pixelWorldReadableEntityText("Use agent-based planning for agent-to-agent handoffs", visualState)).toBe(
      "Use agent-based planning for agent-to-agent handoffs",
    );
  });

  it("normalizes a location fallback instead of exposing its raw id", () => {
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" })).toBe("Location 0");
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" }, true)).toBe("地点 0");
    expect(pixelWorldSelectedEntityLabel({ ...visualState, locations: [] }, { kind: "location", id: "loc-42" })).toBe("Location 42");
  });

  it("derives deterministic type-scoped marker codes from stable ids", () => {
    const agents = [
      { id: "agent-builder", name: "Shared Name" },
      { id: "agent-factory", name: "Shared Name" },
      { id: "agent-0", name: "Shared Name" },
    ];
    const reorderedCodes = agents
      .slice()
      .reverse()
      .map((agent) => pixelWorldEntityMarkerCode(agent, "", "agent"));
    const originalCodes = agents.map((agent) => pixelWorldEntityMarkerCode(agent, "", "agent"));

    expect(new Set(originalCodes).size).toBe(originalCodes.length);
    const originalById = new Map(agents.map((agent, index) => [agent.id, originalCodes[index]]));
    agents.slice().reverse().forEach((agent, index) => {
      expect(reorderedCodes[index]).toBe(originalById.get(agent.id));
    });
    expect(pixelWorldEntityMarkerCode({ id: "loc-0" }, "", "location")).not.toBe(
      pixelWorldEntityMarkerCode({ id: "agent-0" }, "", "agent"),
    );
  });

  it("keeps canonical id variants collision-free", () => {
    const canonical = pixelWorldEntityMarkerCode({ id: "agent-0" }, "", "agent");
    const underscoreVariant = pixelWorldEntityMarkerCode({ id: "agent_0" }, "", "agent");
    expect(canonical).toBe("A0");
    expect(underscoreVariant).not.toBe(canonical);
  });
});
