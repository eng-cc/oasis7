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

  it("installs a recommendation-only fixture with an enabled rendered Agent target and no receipt inputs", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=recommended_target");

    expect(installPixelWorldVisualFixtureHook()).toBe("recommended_target");
    expect(window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.recommended_target()).toMatchObject({
      model: { agents: { "agent-0": { id: "agent-0" } } },
      player_gameplay: {
        available_actions: [{ target_agent_id: "agent-0", disabled_reason: null }],
        recent_feedback: null,
      },
    });
    const gameplay = window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.recommended_target().player_gameplay;
    expect(gameplay.accepted_intent_id).toBeUndefined();
    expect(gameplay.intent_target).toBeUndefined();
    expect(gameplay.last_world_change).toBeUndefined();
  });

  it("publishes an active Micro Depot at a known location for the selected-blocker renderer fixture", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=selected_blocker");

    expect(installPixelWorldVisualFixtureHook()).toBe("selected_blocker");
    expect(window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.selected_blocker()).toMatchObject({
      model: { locations: { "loc-0": { id: "loc-0" } } },
      player_gameplay: {
        micro_depot_facilities: [{
          facility_id: "depot-fixture-loc-0",
          status: "active",
          location_id: "loc-0",
        }],
      },
    });
    expect(
      window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.selected_blocker().player_gameplay
        .micro_depot_facilities[0].service_radius_cm,
    ).toBe(240_000);
  });

  it("registers a spatially distinct healthy/low/zero Micro Depot stock-runway fixture", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=micro_depot_stock_runway");

    expect(installPixelWorldVisualFixtureHook()).toBe("micro_depot_stock_runway");
    const fixture = window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.micro_depot_stock_runway();
    const facilities = fixture.player_gameplay.micro_depot_facilities;

    expect(facilities).toHaveLength(3);
    expect(new Set(facilities.map((facility) => facility.facility_id)).size).toBe(3);
    expect(facilities.map((facility) => ({
      inventoryRevision: facility.inventory_revision,
      remaining: facility.available_units_by_kind?.data,
      epoch: facility.throughput_epoch,
      throughputRemaining: facility.throughput_remaining_units,
      throughputLimit: facility.throughput_limit_units_per_epoch,
    }))).toEqual(expect.arrayContaining([
      expect.objectContaining({
        inventoryRevision: expect.any(Number),
        remaining: 8,
        epoch: expect.any(Number),
        throughputRemaining: 8,
        throughputLimit: 8,
      }),
      expect.objectContaining({
        inventoryRevision: expect.any(Number),
        remaining: 2,
        epoch: expect.any(Number),
        throughputRemaining: 2,
        throughputLimit: 8,
      }),
      expect.objectContaining({
        inventoryRevision: expect.any(Number),
        remaining: 0,
        epoch: expect.any(Number),
        throughputRemaining: 0,
        throughputLimit: 8,
      }),
    ]));

    const anchors = facilities.map((facility) => {
      const location = fixture.model.locations[facility.location_id];
      expect(location).toEqual(expect.objectContaining({
        pos: expect.objectContaining({ x_cm: expect.any(Number), y_cm: expect.any(Number) }),
      }));
      return `${location.pos.x_cm}:${location.pos.y_cm}`;
    });
    expect(new Set(anchors).size).toBe(3);
  });

  it("injects the two player-visible recent-event kinds only for the glyph visual fixture", () => {
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&pixel_world_visual_fixture=recent_event_glyphs");

    expect(installPixelWorldVisualFixtureHook()).toBe("recent_event_glyphs");
    expect(core.state.recentEvents).toEqual([
      expect.objectContaining({ event_id: "resource-transfer-fixture", kind: "resource_transfer" }),
      expect.objectContaining({ event_id: "build-queue-fixture", kind: "build_queue" }),
    ]);
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
