import { describe, expect, it } from "vitest";
import { pixelWorldMobileFocusSelectionOffset, pixelWorldMobileSelectionOffset } from "./pixel_world_mobile_safe_area.js";

describe("pixel world mobile selection safe area", () => {
  it("clears the command band while preserving the Feed gap", () => {
    expect(pixelWorldMobileSelectionOffset({ markerTop: 520, markerBottom: 566, commandTop: 420, feedBottom: 146 })).toBe(-154);
    expect(pixelWorldMobileSelectionOffset({ markerTop: 267, markerBottom: 313, commandTop: 208, feedBottom: 146 })).toBe(-113);
  });

  it("moves the selected marker beside an expanded Focus HUD", () => {
    expect(pixelWorldMobileFocusSelectionOffset({ markerLeft: 178, hudRight: 300 })).toBe(130);
  });
});
