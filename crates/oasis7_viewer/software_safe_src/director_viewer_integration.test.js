import { afterEach, describe, expect, it, vi } from "vitest";
import { createViewerDirectorSession } from "./director_viewer_integration.js";

const VALID_GRANT = {
  ok: true,
  server_validated: true,
  grant: {
    version: 1,
    action: "director_open",
    audience: "viewer_director",
    scope: "diagnostics_read",
    player_id: "player-1",
    player_public_key: "abcdef0123456789abcdef0123456789",
    server: "runtime-authority",
    session_epoch: 7,
    nonce: "nonce-1",
    issued_at_unix_ms: 1_000,
    expires_at_unix_ms: 2_000,
    signer_public_key: "fedcba9876543210fedcba9876543210",
    signature: "awdirectorgrant:v1:opaque-signature",
  },
};

function installRouteTargets() {
  document.body.innerHTML = `
    <section id="viewer-stage-panel" tabindex="-1"></section>
    <section id="viewer-director-panel" tabindex="-1"></section>
  `;
  window.history.replaceState({}, "", "/software_safe.html#viewer-stage-panel");
}

function createCore() {
  return {
    state: {
      worldId: "world-1",
      connectionStatus: "connected",
      auth: { available: true, runtimeStatus: "ready" },
    },
  };
}

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

describe("viewer Director route recovery", () => {
  it.each([
    ["reconnect", (core) => { core.state.connectionStatus = "disconnected"; }],
    ["revoke", (core) => { core.state.auth.revokeReason = "operator_revoked"; }],
  ])("downgrades on %s and returns the route to the focused stage", async (_name, mutate) => {
    vi.useFakeTimers();
    vi.setSystemTime(1_500);
    installRouteTargets();
    const core = createCore();
    const session = createViewerDirectorSession({
      core,
      fetchImpl: vi.fn(async () => ({ ok: true, json: async () => VALID_GRANT })),
    });

    await session.request();
    await Promise.resolve();
    expect(window.location.hash).toBe("#viewer-director-panel");
    mutate(core);
    session.observeRuntime();

    expect(session.controller.getState().mode).toBe("player");
    expect(window.location.hash).toBe("#viewer-stage-panel");
    expect(document.activeElement).toBe(document.querySelector("#viewer-stage-panel"));
  });

  it("downgrades on expiry and restores the focused stage route", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_500);
    installRouteTargets();
    const core = createCore();
    const session = createViewerDirectorSession({
      core,
      fetchImpl: vi.fn(async () => ({ ok: true, json: async () => VALID_GRANT })),
    });

    await session.request();
    await Promise.resolve();
    expect(window.location.hash).toBe("#viewer-director-panel");
    vi.setSystemTime(2_001);
    vi.advanceTimersByTime(501);

    expect(session.controller.getState()).toMatchObject({ mode: "player", status: "expired" });
    expect(window.location.hash).toBe("#viewer-stage-panel");
    expect(document.activeElement).toBe(document.querySelector("#viewer-stage-panel"));
  });
});
