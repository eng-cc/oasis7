import * as core from "./legacy_core.js";
import {
  pixelWorldMicroDepotStockRunwayVisualFixture,
  pixelWorldRecommendedTargetVisualFixture,
  pixelWorldSelectedBlockerVisualFixture,
  pixelWorldModuleVisualEntitiesFixture,
  pixelWorldRoutesAndEventsVisualFixture,
} from "./pixel_world_visual_fixture_data.js";

const PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL = "__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__";
const PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT_GLOBAL = "__OASIS7_PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT__";

export function pixelWorldTestApiEnabled() {
  if (typeof window === "undefined" || !window.location) {
    return false;
  }
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}

function requestedVisualFixtureName() {
  if (typeof window === "undefined" || !window.location) {
    return null;
  }
  return String(new URLSearchParams(window.location.search || "").get("pixel_world_visual_fixture") || "").trim();
}

function liveConnectionDisabledForFixture() {
  if (typeof window === "undefined" || !window.location) {
    return false;
  }
  return String(new URLSearchParams(window.location.search || "").get("connect") || "").trim() === "0";
}

export function installPixelWorldVisualFixtureHook() {
  if (typeof window === "undefined" || !pixelWorldTestApiEnabled() || !liveConnectionDisabledForFixture()) {
    return null;
  }
  const fixtures = {
    selected_blocker: () => core.clone(pixelWorldSelectedBlockerVisualFixture()),
    hotspot_tooltip: () => core.clone(pixelWorldSelectedBlockerVisualFixture()),
    recent_event_glyphs: () => core.clone(pixelWorldSelectedBlockerVisualFixture()),
    routes_and_events: () => core.clone(pixelWorldRoutesAndEventsVisualFixture()),
    recommended_target: () => core.clone(pixelWorldRecommendedTargetVisualFixture()),
    module_visual_entities: () => core.clone(pixelWorldModuleVisualEntitiesFixture()),
    micro_depot_stock_runway: () => core.clone(pixelWorldMicroDepotStockRunwayVisualFixture()),
  };
  window[PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL] = fixtures;

  const fixtureName = requestedVisualFixtureName();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  const fixture = fixtures[fixtureName]();
  core.injectSnapshot(fixture, { returnState: false });
  if (fixtureName === "module_visual_entities") {
    const moduleFixtureEvents = [
      {
        event_seq: 101,
        kind: "ModuleVisualEntityUpserted",
        summary: "Relay marker published",
        detail: "The relay marker is available on the world map.",
        receipt_ref: null,
        module_visual_entity_id: "module-relay",
      },
      {
        event_seq: 100,
        kind: "ModuleVisualEntityRemoved",
        summary: "Removed marker reference",
        detail: "The referenced marker is no longer in the current snapshot.",
        receipt_ref: null,
        module_visual_entity_id: "module-deleted",
      },
    ];
    core.state.worldFeed = {
      status: "ready",
      schemaVersion: "world_feed/v1",
      worldId: "fixture-world",
      reorgEpoch: "0",
      cursor: "101",
      events: moduleFixtureEvents,
      stale: false,
      gapReason: null,
      unavailableReason: null,
      snapshotReloadRequired: false,
      requestInFlight: false,
      requestCursor: null,
      requestLimit: 50,
      dedupedCount: 0,
      lastError: null,
    };
    // This is test-api-only and uses whole snapshots so it cannot expose a
    // production action path. It lets a browser smoke prove update/removal.
    window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__ = {
      update(entities) {
        const next = core.clone(fixture);
        next.model.module_visual_entities = core.clone(entities || {});
        core.injectSnapshot(next, { returnState: false });
        core.requestRender();
        return true;
      },
      publishEvent(event) {
        core.state.worldFeed.events = [core.clone(event)];
        core.state.worldFeed.status = "ready";
        core.state.worldFeed.stale = false;
        core.requestRender();
        return true;
      },
      publishStaleEvent(event) {
        core.state.worldFeed.events = [core.clone(event)];
        core.state.worldFeed.status = "gap";
        core.state.worldFeed.stale = true;
        core.requestRender();
        return true;
      },
      clear() {
        return this.update({});
      },
    };
  }
  if (["recent_event_glyphs", "routes_and_events"].includes(fixtureName)) {
    // Test-only input for the real WASM renderer smoke. These event kinds are
    // projected by the bridge into two independent, hoverable visual hotspots.
    core.state.recentEvents = [
      { event_id: "resource-transfer-fixture", title: "Resource transfer completed", kind: "resource_transfer" },
      { event_id: "build-queue-fixture", title: "Build queue updated", kind: "build_queue" },
    ];
    core.state.eventCount = core.state.recentEvents.length;
  }
  const alignFixtureAuth = () => {
    const playerId = String(core.state.auth.playerId || "player-one").trim() || "player-one";
    const publicKey = String(core.state.auth.publicKey || "abcdef0123456789abcdef0123456789").trim();
    const model = core.state.snapshot?.model || {};
    const alignedAgentIds = fixtureName === "routes_and_events"
      ? Object.keys(model.agents || {})
      : ["agent-0"];
    model.agent_player_bindings = {
      ...(model.agent_player_bindings || {}),
    };
    model.agent_player_public_key_bindings = {
      ...(model.agent_player_public_key_bindings || {}),
    };
    for (const agentId of alignedAgentIds) {
      model.agent_player_bindings[agentId] = playerId;
      model.agent_player_public_key_bindings[agentId] = publicKey;
    }
    core.state.auth = {
    ...core.state.auth,
    available: true,
      playerId,
      publicKey,
      privateKey: core.state.auth.privateKey || "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
    };
    core.applySelection({ kind: "agent", id: "agent-0" });
    core.requestRender();
    return true;
  };
  window[PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT_GLOBAL] = alignFixtureAuth;
  alignFixtureAuth();
  return fixtureName;
}

export function installPixelWorldRenderDtoProbe(fixtureName, getRenderState, onCleanup) {
  if (!fixtureName || !pixelWorldTestApiEnabled()) {
    return;
  }
  window.__OASIS7_PIXEL_WORLD_RENDER_DTO__ = () => core.clone(getRenderState());
  onCleanup(() => {
    delete window.__OASIS7_PIXEL_WORLD_RENDER_DTO__;
  });
}
