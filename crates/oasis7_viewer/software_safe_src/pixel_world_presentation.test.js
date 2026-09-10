import { describe, expect, it } from "vitest";
import {
  pixelWorldBlockerPresentation,
  pixelWorldConnectionPresentation,
  pixelWorldFeedFreshnessPresentation,
  pixelWorldMajorEventPresentation,
  pixelWorldSparseScenePresentation,
} from "./pixel_world_presentation.js";

describe("pixel world player presentation", () => {
  it("localizes the empty entity blocker and keeps its protocol code out of player copy", () => {
    const english = pixelWorldBlockerPresentation("runtime_snapshot_empty_entities", "en");
    expect(english).toMatchObject({
      label: "No published entities yet",
      reason: "The authoritative snapshot has no published agents or locations yet.",
      nextAction: "Reload the authoritative snapshot or wait for the first published entity.",
    });
    expect(`${english.label} ${english.reason} ${english.nextAction}`).not.toContain("runtime_snapshot_empty_entities");

    const chinese = pixelWorldBlockerPresentation("runtime_snapshot_empty_entities", "zh-CN");
    expect(chinese.label).toBe("尚未发布世界实体");
    expect(`${chinese.reason} ${chinese.nextAction}`).not.toContain("runtime_snapshot_empty_entities");
  });

  it("maps other known blocker codes to readable labels without exposing raw enums", () => {
    expect(pixelWorldBlockerPresentation("material_shortage", "en").label).toBe("Missing Material");
    expect(pixelWorldBlockerPresentation("power_shortage", "zh-CN").label).toBe("缺电");
    expect(pixelWorldBlockerPresentation("unknown_internal_code", "en").label).toBe("Current blocker");
  });

  it("keeps world connection and feed freshness as separate status scopes", () => {
    expect(pixelWorldConnectionPresentation("connecting", "en")).toMatchObject({
      label: "World connection: CONNECTING",
      state: "connecting",
    });
    expect(pixelWorldConnectionPresentation("closed", "zh-CN").label).toBe("世界连接：已关闭");
    expect(pixelWorldFeedFreshnessPresentation("ready", false, "en")).toMatchObject({
      label: "Feed freshness: LIVE",
      state: "live",
    });
    expect(pixelWorldFeedFreshnessPresentation("ready", true, "en").label).toBe("Feed freshness: STALE");
  });

  it("presents published crisis severity and lifecycle without implying action success", () => {
    const active = pixelWorldMajorEventPresentation({ severity: 4, lifecycle: "active" }, "en");
    expect(active).toMatchObject({
      label: "Crisis active · severity 4",
      severity: "4",
      lifecycle: "active",
      shape: "severity-4 lifecycle-active",
    });
    expect(active.label).not.toMatch(/accepted|success|completed/i);

    const resolved = pixelWorldMajorEventPresentation({ severity: 2, lifecycle: "resolved" }, "zh-CN");
    expect(resolved.label).toBe("危机已解决 · 严重度 2");
    expect(resolved.shape).toBe("severity-2 lifecycle-resolved");
  });

  it("reports absent routes and terrain from actual counts and preserves published bounds", () => {
    const sparse = pixelWorldSparseScenePresentation({
      routeCount: 0,
      terrainCount: 0,
      locationCount: 1,
      agentCount: 1,
      worldBounds: { width_cm: 1000000, depth_cm: 500000 },
    }, "en");
    expect(sparse).toMatchObject({
      routes: "No published routes in this snapshot",
      terrain: "No published terrain in this snapshot",
      bounds: "Published bounds: 1000000 × 500000 cm",
    });
    expect(sparse.routes).not.toMatch(/route object|terrain object|synthetic/i);
  });
});
