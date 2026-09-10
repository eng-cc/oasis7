import { describe, expect, it } from "vitest";
import { hotspotTooltipSafeBand } from "./pixel_world_tooltip_placement.js";

describe("hotspot tooltip safe band", () => {
  it.each([
    [844, 160, 224, 420],
    [390, 73, 137, 174.8],
    [360, 73, 137, 171.2],
    [360, 104, 220, 171.2],
  ])("keeps summary and command actions clear at viewport height %s", (height, summaryTop, summaryBottom, commandTop) => {
    const band = hotspotTooltipSafeBand({ top: summaryTop, bottom: summaryBottom }, { top: commandTop }, height);
    expect(band.top >= summaryBottom || band.top + band.maxHeight <= summaryTop).toBe(true);
    expect(band.maxHeight).toBeGreaterThanOrEqual(56);
    expect(band.top + band.maxHeight).toBeLessThan(commandTop);
    expect(band.top + band.maxHeight).toBeLessThan(height);
  });
});
