import { afterEach, describe, expect, it, vi } from "vitest";

import { installPixelWorldVisualFixtureHook } from "./pixel_world_visual_fixture.js";

afterEach(() => {
  vi.restoreAllMocks();
  window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0");
  delete window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__;
});

describe("pixel world visual fixtures", () => {
  it("installs the deterministic hotspot tooltip fixture only when live websocket connection is disabled", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=hotspot_tooltip");

    expect(installPixelWorldVisualFixtureHook()).toBe("hotspot_tooltip");
    expect(window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.hotspot_tooltip).toEqual(expect.any(Function));
  });

  it("refuses to install the hotspot tooltip fixture when connect is not explicitly disabled", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&pixel_world_visual_fixture=hotspot_tooltip");

    expect(installPixelWorldVisualFixtureHook()).toBeNull();
  });
});
