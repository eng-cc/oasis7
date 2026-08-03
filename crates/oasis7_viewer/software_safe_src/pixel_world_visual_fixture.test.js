import { afterEach, describe, expect, it, vi } from "vitest";

import * as core from "./legacy_core.js";
import { installPixelWorldVisualFixtureHook } from "./pixel_world_visual_fixture.js";

afterEach(() => {
  vi.restoreAllMocks();
  window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0");
  delete window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__;
  delete window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT__;
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

  it("rebinds the fixture Agent to a later local test session without changing global visibility rules", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=hotspot_tooltip");
    installPixelWorldVisualFixtureHook();
    core.state.auth = {
      ...core.state.auth,
      available: true,
      playerId: "local-test-player-fixture",
      publicKey: "fixture-public-key",
      source: "local_test_api_ephemeral",
      boundAgentId: null,
    };

    expect(window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT__()).toBe(true);
    expect(core.state.auth.boundAgentId).toBe("agent-0");
    expect(core.modelLists().agents.map((agent) => agent.id)).toContain("agent-0");
    expect(core.state.snapshot.model.agent_player_bindings["agent-0"]).toBe("local-test-player-fixture");
  });
});
