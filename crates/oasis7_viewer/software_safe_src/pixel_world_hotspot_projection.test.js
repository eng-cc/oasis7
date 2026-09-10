import { describe, expect, it } from "vitest";

import {
  pixelWorldHotspotGlyphSize,
  pixelWorldHotspotStyle,
  toCanvasPoint,
  pixelWorldHotspotIntersectsStage,
} from "./pixel_world_hotspot_projection.js";

const worldBounds = { width_cm: 10_000_000, depth_cm: 5_000_000 };

describe("pixel world hotspot projection", () => {
  it.each([
    ["-1%", "50%", false], ["-0.9%", "50%", true],
    ["101%", "50%", false], ["100.9%", "50%", true],
    ["50%", "-2%", false], ["50%", "-1.9%", true],
    ["50%", "102%", false], ["50%", "101.9%", true],
  ])("distinguishes full clipping from partial glyph visibility at %s,%s", (left, top, expected) => {
    expect(pixelWorldHotspotIntersectsStage({ left, top }, { width: 1000, height: 500 }, 20)).toBe(expected);
  });
  it("matches the bridge 20px inset and camera transform", () => {
    const style = pixelWorldHotspotStyle(
      { pos: { x_cm: 2_500_000, y_cm: 3_750_000 } },
      worldBounds,
      0,
      { zoom: 2, pan_x_px: 40, pan_y_px: -20 },
    );

    expect(parseFloat(style.left)).toBeCloseTo(6.25);
    expect(parseFloat(style.top)).toBeCloseTo(92.59259);
    expect(style.width).toBe("44px");
    expect(style.height).toBe("44px");
  });

  it("retains inset anchors and scales pan and centered zoom with the canvas", () => {
    const origin = { x_cm: 0, y_cm: 0 };
    expect(toCanvasPoint(origin, worldBounds, 960, 540)).toEqual({ x: 20, y: 20 });
    expect(toCanvasPoint(origin, worldBounds, 960, 540, { zoom: 0.5, pan_x_px: 30, pan_y_px: -10 })).toEqual({ x: 280, y: 135 });
    expect(toCanvasPoint({ x_cm: 10_000_000, y_cm: 5_000_000 }, worldBounds, 960, 540)).toEqual({ x: 940, y: 520 });
  });

  it("keeps a separate glyph size inside the touch target", () => {
    const hotspot = { size_hint_px: 14 };

    expect(pixelWorldHotspotGlyphSize(hotspot)).toBe(14);
    expect(pixelWorldHotspotStyle(hotspot, null, 0).width).toBe("44px");
  });
});
