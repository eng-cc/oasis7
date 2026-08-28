import { describe, expect, it } from "vitest";
import { resolvePixelWorldRendererRoute } from "./pixel_world_renderer_route.js";

describe("pixel world renderer route", () => {
  it("keeps the default route loading without a deferred fatal", () => {
    expect(resolvePixelWorldRendererRoute({ search: "?pixel_world_renderer=ready" })).toEqual({
      deferred: false,
      source: "loading",
      fatal: null,
    });
    expect(resolvePixelWorldRendererRoute(null)).toEqual({
      deferred: false,
      source: "loading",
      fatal: null,
    });
  });

  it("recognizes explicit deferred renderer values", () => {
    for (const value of ["0", "false", "no", "off", "defer", "fallback", "DEFER"]) {
      expect(resolvePixelWorldRendererRoute({ search: `?pixel_world_renderer=${value}` })).toEqual({
        deferred: true,
        source: "deferred",
        fatal: {
          code: "pixel_world_renderer_deferred",
          message: "pixel world renderer was explicitly deferred by the viewer route",
        },
      });
    }
  });

  it("does not treat unknown renderer values as a deferred route", () => {
    expect(resolvePixelWorldRendererRoute({ search: "?pixel_world_renderer=unknown" }).deferred).toBe(false);
  });
});
