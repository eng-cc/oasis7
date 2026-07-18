import * as core from "./legacy_core.js";
import { pixelWorldSelectedBlockerVisualFixture } from "./pixel_world_visual_fixture_data.js";

const PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL = "__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__";

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

export function installPixelWorldVisualFixtureHook() {
  if (typeof window === "undefined" || !pixelWorldTestApiEnabled()) {
    return null;
  }
  const fixtures = {
    selected_blocker: () => core.clone(pixelWorldSelectedBlockerVisualFixture()),
  };
  window[PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL] = fixtures;

  const fixtureName = requestedVisualFixtureName();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  const fixture = fixtures[fixtureName]();
  core.injectSnapshot(fixture, { returnState: false });
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "player-one",
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
  };
  core.applySelection({ kind: "agent", id: "agent-0" });
  return fixtureName;
}
