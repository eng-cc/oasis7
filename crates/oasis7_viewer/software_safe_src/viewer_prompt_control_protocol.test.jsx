import { waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";

const HEAVY_UI_TEST_TIMEOUT_MS = 60000;
const TEST_ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
let activeCleanup = null;

function createTestCrypto() {
  const privateBytes = new Uint8Array(32).fill(7);
  const publicBytes = new Uint8Array(32).fill(9);
  return {
    subtle: {
      async generateKey() {
        return { privateKey: { kind: "test-private" }, publicKey: { kind: "test-public" } };
      },
      async exportKey(format) {
        if (format === "pkcs8") {
          const out = new Uint8Array(TEST_ED25519_PKCS8_PREFIX.length + privateBytes.length);
          out.set(TEST_ED25519_PKCS8_PREFIX, 0);
          out.set(privateBytes, TEST_ED25519_PKCS8_PREFIX.length);
          return out.buffer;
        }
        if (format === "raw") return publicBytes.buffer.slice(0);
        throw new Error(`unsupported test key export: ${format}`);
      },
      async importKey() {
        return { kind: "test-imported" };
      },
      async sign() {
        return new Uint8Array(64).fill(11).buffer;
      },
    },
  };
}

function installMockWebSocket() {
  const sentMessages = [];
  const sockets = [];
  class MockWebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;
    constructor(url) {
      this.url = url;
      this.readyState = MockWebSocket.CONNECTING;
      this.listeners = new Map();
      sockets.push(this);
    }
    addEventListener(type, listener) {
      this.listeners.set(type, [...(this.listeners.get(type) || []), listener]);
    }
    send(payload) {
      sentMessages.push(JSON.parse(payload));
    }
    close() {
      this.readyState = MockWebSocket.CLOSED;
      this.emit("close", {});
    }
    open() {
      this.readyState = MockWebSocket.OPEN;
      this.emit("open", {});
    }
    receive(message) {
      this.emit("message", { data: JSON.stringify(message) });
    }
    emit(type, event) {
      for (const listener of this.listeners.get(type) || []) listener(event);
    }
  }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket });
  return { sockets, sentMessages };
}

function sampleSnapshot(overrides = {}) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    ...overrides,
    config: { ...base.config, ...(overrides.config || {}) },
    model: {
      ...base.model,
      agent_player_bindings: { "agent-0": "local-test-player-bound" },
      agent_player_public_key_bindings: { "agent-0": "abcdef0123456789abcdef0123456789" },
      ...(overrides.model || {}),
    },
    player_gameplay: { ...base.player_gameplay, ...(overrides.player_gameplay || {}) },
  };
}

function bindLocalTestAgent(core, agentId = "agent-0") {
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "local-test-player-bound",
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: agentId,
  };
}

async function setupConnectedSemanticCore({
  snapshot = sampleSnapshot(),
  agentId = "agent-0",
  avoidLocalAutoAuth = false,
} = {}) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState(
    {},
    "",
    `/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0&locale=en&ws=${avoidLocalAutoAuth ? "ws://198.51.100.1:5011" : "ws://127.0.0.1:5011"}`,
  );
  window.localStorage.clear();
  document.body.innerHTML = "";
  const { sockets, sentMessages } = installMockWebSocket();
  const core = await import("./legacy_core.js");
  core.initializeSoftwareSafeCore();
  sockets[0].open();
  sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
  core.injectSnapshot(snapshot);
  core.applySelection({ kind: "agent", id: agentId });
  bindLocalTestAgent(core, agentId);
  activeCleanup = () => {
    for (const socket of sockets) {
      if (socket.readyState !== socket.CLOSED) socket.close();
    }
    activeCleanup = null;
  };
  return { core, sockets, sentMessages };
}

beforeEach(() => {
  vi.restoreAllMocks();
  Object.defineProperty(window, "crypto", { configurable: true, value: createTestCrypto() });
  window.localStorage.clear();
  document.body.innerHTML = "";
});

afterEach(() => {
  vi.restoreAllMocks();
  activeCleanup?.();
  activeCleanup = null;
  document.body.innerHTML = "";
});

describe("viewer prompt control protocol", () => {
  it("consumes the enhanced prompt-control handshake, epochs, and request identity", async () => {
    const { core, sockets, sentMessages } = await setupConnectedSemanticCore();
    expect(sentMessages[0]).toEqual({
      type: "hello_v2",
      client: "viewer",
      version: 2,
      capabilities: ["prompt_control_result_v1"],
    });
    sockets[0].receive({
      type: "hello_ack",
      server: "test-live",
      world_id: "test-world",
      version: 2,
      capabilities: ["prompt_control_result_v1"],
      authority_epoch: "authority-test-1",
    });
    core.state.auth = {
      ...core.state.auth,
      available: true,
      playerId: "local-test-player-bound",
      publicKey: "abcdef0123456789abcdef0123456789",
      privateKey: "07".repeat(32),
      source: "local_test_api_ephemeral",
      registrationStatus: "issued",
      runtimeStatus: "issued",
      sessionEpoch: null,
      bindingEpoch: null,
      boundAgentId: "agent-0",
      syncInFlight: false,
    };
    const registerPromise = core.registerPlayerSessionForTest("agent-0");
    await waitFor(() => {
      expect(sentMessages).toEqual(expect.arrayContaining([
        expect.objectContaining({
          type: "authoritative_recovery",
          command: expect.objectContaining({
            mode: "register_session",
            request: expect.objectContaining({ requested_agent_id: "agent-0" }),
          }),
        }),
      ]));
    });
    sockets[0].receive({
      type: "authoritative_recovery_ack",
      ack: {
        status: "session_registered",
        player_id: "local-test-player-bound",
        session_pubkey: "abcdef0123456789abcdef0123456789",
        agent_id: "agent-0",
        session_epoch: 12,
        binding_epoch: 9,
      },
    });
    await expect(registerPromise).resolves.toEqual(expect.objectContaining({ status: "session_registered" }));
    expect(core.getState()).toEqual(expect.objectContaining({
      authSessionEpoch: 12,
      authBindingEpoch: 9,
      authAuthorityEpoch: "authority-test-1",
    }));
    expect(core.sendPromptControl("preview", { agentId: "agent-0", systemPrompt: "Stay concise" }))
      .toEqual(expect.objectContaining({ ok: true }));
    await waitFor(() => {
      expect(sentMessages).toEqual(expect.arrayContaining([
        expect.objectContaining({
          type: "prompt_control",
          command: expect.objectContaining({
            mode: "preview",
            request: expect.objectContaining({
              request_id: expect.stringMatching(/^pc-/),
              session_epoch: 12,
              binding_epoch: 9,
              expected_authority_epoch: "authority-test-1",
            }),
          }),
        }),
      ]));
    });
    sockets[0].close();
    expect(core.state.viewerProtocol.authorityEpoch).toBeNull();
    expect(core.state.auth.sessionEpoch).toBeNull();
    expect(core.state.auth.bindingEpoch).toBeNull();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("waits for the authoritative snapshot after an enhanced applied receipt", async () => {
    const { core, sockets, sentMessages } = await setupConnectedSemanticCore({
      snapshot: sampleSnapshot({ model: { agent_player_bindings: {} } }),
      avoidLocalAutoAuth: true,
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    core.state.hostedAccess = { deployment_mode: "trusted_local_only", action_matrix: [] };
    sockets[0].receive({
      type: "hello_ack",
      version: 2,
      capabilities: ["prompt_control_result_v1"],
      authority_epoch: "authority-test-1",
      server: "test-live",
      world_id: "test-world",
    });
    core.state.auth = {
      ...core.state.auth,
      available: true,
      playerId: "local-test-player-bound",
      publicKey: "abcdef0123456789abcdef0123456789",
      privateKey: "07".repeat(32),
      source: "local_test_api_ephemeral",
      registrationStatus: "registered",
      runtimeStatus: "registered",
      sessionEpoch: 12,
      bindingEpoch: 9,
      authorityEpoch: "authority-test-1",
      boundAgentId: "agent-0",
      syncInFlight: false,
    };
    expect(core.sendPromptControl("apply", { agentId: "agent-0", systemPrompt: "authoritative prompt" }))
      .toEqual(expect.objectContaining({ ok: true }));
    await waitFor(() => {
      expect(sentMessages.some((message) => message.type === "prompt_control"
        || (message.type === "authoritative_recovery" && message.command?.mode === "register_session"))).toBe(true);
    });
    const pendingRegistration = [...sentMessages].reverse().find(
      (message) => message.type === "authoritative_recovery"
        && message.command?.mode === "register_session",
    );
    if (pendingRegistration) {
      const registrationRequest = pendingRegistration.command.request;
      sockets[0].receive({
        type: "authoritative_recovery_ack",
        ack: {
          status: "session_registered",
          player_id: registrationRequest.player_id,
          session_pubkey: registrationRequest.public_key,
          agent_id: registrationRequest.requested_agent_id,
          session_epoch: 12,
          binding_epoch: 9,
        },
      });
    }
    await waitFor(() => {
      expect(sentMessages).toEqual(expect.arrayContaining([
        expect.objectContaining({
          type: "prompt_control",
          command: expect.objectContaining({ mode: "apply" }),
        }),
      ]));
    });
    const snapshotRequestsBeforeAck = sentMessages.filter((message) => message.type === "request_snapshot").length;
    sockets[0].receive({
      type: "prompt_control_ack",
      ack: {
        request_id: sentMessages.find((message) => message.type === "prompt_control")?.command?.request?.request_id,
        operation: "apply",
        status: "applied",
        agent_id: "agent-0",
        version: 1,
        updated_at_tick: 14,
        applied_fields: ["system_prompt"],
        applied_scope: "runtime_instance",
        persistence_scope: "none",
        sync_scope: "none",
        value_visibility: "latest_allowed",
      },
    });
    expect(sentMessages.filter((message) => message.type === "request_snapshot").length)
      .toBe(snapshotRequestsBeforeAck + 1);
    expect(core.state.promptDraft.systemPrompt).toBe("");
    expect(core.state.promptDraft.dirty).toBe(false);
    core.injectSnapshot(sampleSnapshot({
      model: {
        agent_prompt_profiles: {
          "agent-0": {
            agent_id: "agent-0",
            version: 1,
            updated_at_tick: 14,
            updated_by: "local-test-player-bound",
            system_prompt_override: "authoritative prompt",
            short_term_goal_override: null,
            long_term_goal_override: null,
          },
        },
      },
    }));
    expect(core.state.promptDraft.systemPrompt).toBe("authoritative prompt");
    expect(core.state.promptDraft.currentVersion).toBe(1);
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
