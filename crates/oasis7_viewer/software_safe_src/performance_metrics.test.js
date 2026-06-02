import { describe, expect, it } from "vitest";
import {
  buildViewerPerformanceMarkdown,
  evaluateViewerPerformance,
  summarizeViewerPerformance,
} from "./performance_metrics.js";

describe("viewer performance metrics", () => {
  it("summarizes healthy samples with fps and dom readiness", () => {
    const summary = summarizeViewerPerformance({
      runId: "healthy",
      profile: "smoke",
      durationMs: 1800,
      sampleDurationMs: 1800,
      frameIntervals: Array.from({ length: 110 }, (_, index) => (index % 12 === 0 ? 18 : 16)),
      longTasks: [],
      domReadiness: {
        domContentLoadedMs: 240,
        loadEventMs: 320,
        domInteractiveMs: 200,
      },
      interactionLatencies: [42, 51, 65, 58],
      dom: { nodeCount: 240, panelCount: 3, interactiveElementCount: 18 },
      thresholds: {
        minFrameSamples: 100,
        minFps: 50,
        maxFrameP95Ms: 24,
        maxFrameP99Ms: 24,
        maxLongTaskCount: 0,
        maxLongTaskTotalMs: 0,
        maxDomContentLoadedMs: 1000,
        maxLoadEventMs: 1000,
        maxInteractionP95Ms: 100,
      },
    });

    expect(summary.status).toBe("pass");
    expect(summary.metrics.frameSamples).toBe(110);
    expect(summary.metrics.fpsAvg).toBeGreaterThan(55);
    expect(summary.metrics.frameP95Ms).toBe(18);
    expect(summary.metrics.domContentLoadedMs).toBe(240);
    expect(summary.metrics.interactionP95Ms).toBe(65);
  });

  it("fails laggy samples against fps, frame, long task, and interaction gates", () => {
    const summary = summarizeViewerPerformance({
      runId: "laggy",
      profile: "smoke",
      durationMs: 1200,
      sampleDurationMs: 1200,
      frameIntervals: [16, 18, 52, 64, 80, 90, 110, 24, 18, 16],
      longTasks: [{ startTime: 12, duration: 180 }],
      domReadiness: {
        domContentLoadedMs: 4200,
        loadEventMs: 5100,
      },
      interactionLatencies: [40, 120, 280],
      thresholds: {
        minFrameSamples: 5,
        minFps: 20,
        maxFrameP95Ms: 50,
        maxFrameP99Ms: 80,
        maxLongTaskCount: 0,
        maxLongTaskTotalMs: 0,
        maxDomContentLoadedMs: 1000,
        maxLoadEventMs: 1000,
        maxInteractionP95Ms: 150,
      },
    });
    const failed = evaluateViewerPerformance(summary).gates
      .filter((gate) => gate.status === "fail")
      .map((gate) => gate.id);

    expect(summary.status).toBe("fail");
    expect(failed).toContain("frame_p95_ms");
    expect(failed).toContain("long_task_count");
    expect(failed).toContain("long_task_total_ms");
    expect(failed).toContain("dom_content_loaded_ms");
    expect(failed).toContain("load_event_ms");
    expect(failed).toContain("interaction_p95_ms");
  });

  it("renders markdown with the expanded metric rows", () => {
    const summary = summarizeViewerPerformance({
      runId: "markdown",
      url: "http://127.0.0.1/viewer",
      scenario: { name: "dense" },
      sampleDurationMs: 1600,
      frameIntervals: Array.from({ length: 90 }, () => 16),
      domReadiness: { domContentLoadedMs: 180, loadEventMs: 240 },
      interactionLatencies: [30, 35, 40],
      thresholds: { minFrameSamples: 90 },
    });

    const markdown = buildViewerPerformanceMarkdown(summary);

    expect(markdown).toContain("# Viewer Performance Probe");
    expect(markdown).toContain("| frame_p95_ms |");
    expect(markdown).toContain("DOMContentLoaded(ms)");
    expect(markdown).toContain("http://127.0.0.1/viewer");
  });

  it("keeps the probe artifact schema stable", () => {
    const summary = summarizeViewerPerformance({
      runId: "schema",
      profile: "smoke",
      url: "http://127.0.0.1/viewer",
      sampleDurationMs: 1600,
      frameIntervals: Array.from({ length: 90 }, () => 16),
      domReadiness: { domContentLoadedMs: 180, loadEventMs: 240 },
    });
    summary.artifacts = {
      summaryJson: "/tmp/summary.json",
      summaryMarkdown: "/tmp/summary.md",
      screenshot: "/tmp/viewer-performance.png",
    };
    summary.scenario = {
      name: "dense",
      agents: 288,
      locations: 96,
    };

    expect(summary.schemaVersion).toBe(2);
    expect(summary).toEqual(expect.objectContaining({
      runId: "schema",
      profile: "smoke",
      url: "http://127.0.0.1/viewer",
      status: expect.any(String),
      metrics: expect.any(Object),
      thresholds: expect.any(Object),
      domReadiness: expect.objectContaining({
        domContentLoadedMs: 180,
        loadEventMs: 240,
      }),
      gates: expect.any(Array),
      artifacts: expect.objectContaining({
        summaryJson: expect.stringContaining("summary.json"),
        summaryMarkdown: expect.stringContaining("summary.md"),
        screenshot: expect.stringContaining("viewer-performance.png"),
      }),
      scenario: {
        name: "dense",
        agents: 288,
        locations: 96,
      },
    }));
  });
});
