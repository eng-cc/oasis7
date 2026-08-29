import { describe, expect, it } from "vitest";
import { pixelWorldReadableEntityText, pixelWorldSelectedEntityLabel } from "./pixel_world_identity.js";

const visualState = {
  agents: [{ id: "agent-0", name: "Survey Agent" }],
  locations: [{ id: "location-0" }],
};

describe("pixel world player-facing identity", () => {
  it("normalizes agent ids embedded in player leverage copy", () => {
    expect(pixelWorldReadableEntityText("Queue smelter for agent-0", visualState)).toBe("Queue smelter for Survey Agent");
  });

  it("normalizes a location fallback instead of exposing its raw id", () => {
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" })).toBe("Location 0");
    expect(pixelWorldSelectedEntityLabel(visualState, { kind: "location", id: "location-0" }, true)).toBe("地点 0");
    expect(pixelWorldSelectedEntityLabel({ ...visualState, locations: [] }, { kind: "location", id: "loc-42" })).toBe("Location 42");
  });
});
