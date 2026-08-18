import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createDirectorCapabilityApiAdapter,
  createDirectorCapabilityController,
  validateDirectorCapabilityResponse,
} from "./director_capability_controller.js";

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

afterEach(() => {
  vi.useRealTimers();
});

describe("director capability controller", () => {
  it("starts in Player and does not consult browser persistence", () => {
    const controller = createDirectorCapabilityController({ now: () => 1_500 });

    expect(controller.getState()).toMatchObject({ mode: "player", status: "idle" });
    expect(controller.getState().capability).toBeNull();
    expect(controller.getState().reason).toBeNull();
    expect(controller.getState()).not.toHaveProperty("token");
  });

  it("keeps Player while an adapter request is pending, then fails closed on denial", async () => {
    let resolveRequest;
    const requestCapability = vi.fn(() => new Promise((resolve) => {
      resolveRequest = resolve;
    }));
    const controller = createDirectorCapabilityController({
      adapter: { requestCapability },
      now: () => 1_500,
    });

    const request = controller.request({ playerSession: true, localFlag: true });
    expect(controller.getState()).toMatchObject({ mode: "player", status: "pending" });
    resolveRequest({ ok: false, error_code: "director_not_authorized" });
    await request;

    expect(requestCapability).toHaveBeenCalledWith({ playerSession: true, localFlag: true });
    expect(controller.getState()).toMatchObject({
      mode: "player",
      status: "denied",
      reason: "not_authorized",
    });
  });

  it("rejects player-session, local-flag, and prompt-grant shaped responses", () => {
    for (const response of [
      { ok: true, player_session: true },
      { ok: true, local_flag: true },
      { ok: true, strong_auth_grant: { action_id: "prompt_control_apply" } },
      { ok: true, server_validated: false, grant: VALID_GRANT.grant },
      { ...VALID_GRANT, grant: { ...VALID_GRANT.grant, signature: "" } },
      { ...VALID_GRANT, grant: { ...VALID_GRANT.grant, expires_at_unix_ms: 62_000 } },
    ]) {
      expect(validateDirectorCapabilityResponse(response, 1_500)).toMatchObject({
        ok: false,
      });
    }
  });

  it("enters ephemeral Director only after a validated server response", async () => {
    const controller = createDirectorCapabilityController({
      adapter: { requestCapability: vi.fn(async () => VALID_GRANT) },
      now: () => 1_500,
    });

    await controller.request({});

    expect(controller.getState()).toMatchObject({
      mode: "director",
      status: "active",
      capability: {
        name: "director",
        visibility: "dense",
        issuer: "runtime-authority",
        expiresAtUnixMs: 2_000,
      },
    });
    expect(controller.getState().capability).not.toHaveProperty("token");
  });

  it("downgrades and sanitizes on expiry, revocation, or reconnect loss", async () => {
    vi.useFakeTimers();
    let now = 1_500;
    const controller = createDirectorCapabilityController({
      adapter: { requestCapability: vi.fn(async () => VALID_GRANT) },
      now: () => now,
    });
    await controller.request({});
    expect(controller.getState().mode).toBe("director");

    now = 2_001;
    vi.advanceTimersByTime(501);
    expect(controller.getState()).toMatchObject({ mode: "player", status: "expired" });
    expect(controller.getState().capability).toBeNull();

    now = 1_500;
    await controller.request({});
    expect(controller.getState().mode).toBe("director");
    controller.observeRuntime({ connectionStatus: "disconnected" });
    expect(controller.getState()).toMatchObject({ mode: "player", status: "reconnect_required" });
    expect(controller.getState().capability).toBeNull();

    await controller.request({});
    controller.observeRuntime({ authRevokeReason: "operator_revoked" });
    expect(controller.getState()).toMatchObject({ mode: "player", status: "revoked" });
    expect(controller.getState().capability).toBeNull();
  });

  it("uses a real fetch adapter without exposing raw failure details", async () => {
    const fetchImpl = vi.fn(async () => new Response(JSON.stringify({
      ok: false,
      error: "private operator details must not reach the viewer",
    }), { status: 403, headers: { "Content-Type": "application/json" } }));
    const adapter = createDirectorCapabilityApiAdapter({ fetchImpl, endpoint: "/api/public/director/capability" });

    await expect(adapter.requestCapability({ worldId: "world-1" })).resolves.toMatchObject({
      ok: false,
      error_code: "director_not_authorized",
    });
    expect(fetchImpl).toHaveBeenCalledWith("/api/public/director/capability", expect.objectContaining({
      method: "GET",
      headers: { Accept: "application/json" },
    }));
  });
});
