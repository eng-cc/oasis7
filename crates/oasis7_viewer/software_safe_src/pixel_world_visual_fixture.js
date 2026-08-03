import * as core from "./legacy_core.js";
import { pixelWorldSelectedBlockerVisualFixture } from "./pixel_world_visual_fixture_data.js";

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
  };
  window[PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL] = fixtures;

  const fixtureName = requestedVisualFixtureName();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  const fixture = fixtures[fixtureName]();
  core.injectSnapshot(fixture, { returnState: false });
  const alignFixtureAuth = () => {
    const playerId = String(core.state.auth.playerId || "player-one").trim() || "player-one";
    const publicKey = String(core.state.auth.publicKey || "abcdef0123456789abcdef0123456789").trim();
    const model = core.state.snapshot?.model || {};
    model.agent_player_bindings = {
      ...(model.agent_player_bindings || {}),
      "agent-0": playerId,
    };
    model.agent_player_public_key_bindings = {
      ...(model.agent_player_public_key_bindings || {}),
      "agent-0": publicKey,
    };
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
