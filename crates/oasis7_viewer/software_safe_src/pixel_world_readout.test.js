import { describe, expect, it } from "vitest";
import { resolvePixelWorldReadoutStatus } from "./pixel_world_readout.js";

describe("pixel world readout status", () => {
  it("distinguishes live, replay, empty, gap, and unavailable feed states", () => {
    expect([
      ["ready", "LIVE", "badge--good"],
      ["replay", "REPLAY", "badge--accent"],
      ["empty", "NO EVENTS", "pixel-world-readout__status--empty"],
      ["gap", "GAP", "badge--warn"],
      ["unavailable", "UNAVAILABLE", "badge--warn"],
    ].map(([status]) => resolvePixelWorldReadoutStatus("en", "connected", { status }).label)).toEqual([
      "LIVE",
      "REPLAY",
      "NO EVENTS",
      "GAP",
      "UNAVAILABLE",
    ]);
  });

  it("keeps CJK labels explicit and distinct from action success", () => {
    expect(resolvePixelWorldReadoutStatus("zh-CN", "connected", { status: "replay" })).toMatchObject({ label: "回放" });
    expect(resolvePixelWorldReadoutStatus("zh-CN", "connected", { status: "empty" })).toMatchObject({ label: "暂无动态" });
  });
});
