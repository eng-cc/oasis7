import { afterEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { createViewerAgentChatAuthModule } from "./viewer_agent_chat_auth_module.js";

const originalCryptoDescriptor = Object.getOwnPropertyDescriptor(window, "crypto");
const originalWebSocketDescriptor = Object.getOwnPropertyDescriptor(window, "WebSocket");
const ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);

function makeRequest() {
  return {
    agent_id: "agent-0",
    message: "Keep the forge queue moving.",
    player_id: "player-1",
    public_key: "ab".repeat(32),
  };
}

function installMockWebSocket() {
  const sentMessages = [];
  const sockets = [];
  class MockWebSocket {
    static OPEN = 1;
    static CONNECTING = 0;
    static CLOSED = 3;

    constructor() {
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

    open() {
      this.readyState = MockWebSocket.OPEN;
      this.emit("open", {});
    }

    receive(message) {
      this.emit("message", { data: JSON.stringify(message) });
    }

    close() {
      this.readyState = MockWebSocket.CLOSED;
      this.emit("close", {});
    }

    emit(type, event) {
      for (const listener of this.listeners.get(type) || []) listener(event);
    }
  }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket });
  return { sentMessages, sockets };
}

afterEach(() => {
  vi.restoreAllMocks();
  if (originalCryptoDescriptor) {
    Object.defineProperty(window, "crypto", originalCryptoDescriptor);
  }
  if (originalWebSocketDescriptor) {
    Object.defineProperty(window, "WebSocket", originalWebSocketDescriptor);
  }
  document.body.innerHTML = "";
});

describe("viewer agent chat auth", () => {
  it("projects current world-feed authority into the request and exact signing payload", async () => {
    const state = {
      worldFeed: {
        status: "ready",
        stale: false,
        worldId: "runtime-world-7",
        reorgEpoch: 12,
      },
    };
    const buildAuthEnvelope = vi.fn((payload) => payload);
    const signAuthPayload = vi.fn(async () => "awviewauth:v1:signed");
    const module = createViewerAgentChatAuthModule({
      buildAuthEnvelope,
      nextAuthNonce: () => 41,
      signAuthPayload,
      state,
    });
    const request = makeRequest();
    request.intent_tick = 18;
    request.intent_seq = 41;

    await expect(module.buildAuthProof(request, {
      playerId: request.player_id,
      publicKey: request.public_key,
    })).resolves.toEqual({
      scheme: "ed25519",
      player_id: request.player_id,
      public_key: request.public_key,
      nonce: 41,
      signature: "awviewauth:v1:signed",
    });
    expect(request).toMatchObject({
      world_id: "runtime-world-7",
      reorg_epoch: 12,
      authority_scope: "player_agent_chat",
    });
    expect(Object.keys(buildAuthEnvelope.mock.calls[0][0])).toEqual([
      "operation",
      "agent_id",
      "player_id",
      "public_key",
      "nonce",
      "message",
      "intent_tick",
      "intent_seq",
      "world_id",
      "reorg_epoch",
      "authority_scope",
    ]);
    expect(buildAuthEnvelope).toHaveBeenCalledWith(expect.objectContaining({
      operation: "agent_chat",
      agent_id: "agent-0",
      player_id: "player-1",
      public_key: "ab".repeat(32),
      nonce: 41,
      message: "Keep the forge queue moving.",
      intent_tick: 18,
      intent_seq: 41,
      world_id: "runtime-world-7",
      reorg_epoch: 12,
      authority_scope: "player_agent_chat",
    }));
    expect(signAuthPayload).toHaveBeenCalledWith(buildAuthEnvelope.mock.calls[0][0], expect.anything());
  });

  it("fails closed until the current runtime authority identity is available", async () => {
    const signAuthPayload = vi.fn();
    const module = createViewerAgentChatAuthModule({
      buildAuthEnvelope: vi.fn(),
      nextAuthNonce: () => 41,
      signAuthPayload,
      state: {
        worldFeed: {
          status: "loading",
          stale: false,
          worldId: "runtime-world-7",
          reorgEpoch: null,
        },
      },
    });

    await expect(module.buildAuthProof(makeRequest(), {
      playerId: "player-1",
      publicKey: "ab".repeat(32),
    })).rejects.toThrow(/current runtime world authority/i);
    expect(signAuthPayload).not.toHaveBeenCalled();
  });

  it("sends the projected authority fields through the real Viewer chat request path", async () => {
    Object.defineProperty(window, "crypto", {
      configurable: true,
      value: {
        subtle: {
          async generateKey() {
            return { privateKey: "test-private", publicKey: "test-public" };
          },
          async exportKey(format) {
            if (format === "pkcs8") {
              return new Uint8Array([...ED25519_PKCS8_PREFIX, ...new Uint8Array(32).fill(7)]).buffer;
            }
            return new Uint8Array(32).fill(9).buffer;
          },
          async importKey() {
            return { kind: "test-signing-key" };
          },
          async sign() {
            return new Uint8Array(64).fill(11).buffer;
          },
        },
      },
    });
    const { sockets, sentMessages } = installMockWebSocket();
    window.history.replaceState(
      {},
      "",
      "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0&locale=en&ws=ws://127.0.0.1:5011",
    );
    vi.resetModules();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "runtime-world-7" });
    core.injectSnapshot(buildTaskGame076ScenarioSnapshot());
    core.applySelection({ kind: "agent", id: "agent-0" });
    core.state.auth = {
      ...core.state.auth,
      available: true,
      playerId: "player-1",
      publicKey: "ab".repeat(32),
      privateKey: "07".repeat(32),
      source: "local_test_api_ephemeral",
      registrationStatus: "registered",
      runtimeStatus: "registered",
      boundAgentId: "agent-0",
    };
    core.state.worldFeed = {
      status: "ready",
      stale: false,
      schemaVersion: "world_feed/v1",
      worldId: "runtime-world-7",
      reorgEpoch: 12,
      cursor: "runtime-world-7:12:0",
      events: [],
      gapReason: null,
      unavailableReason: null,
      snapshotReloadRequired: false,
      requestInFlight: false,
    };

    expect(core.sendAgentChat("agent-0", "Keep the forge queue moving.")).toEqual(
      expect.objectContaining({ ok: true }),
    );
    for (let attempt = 0; attempt < 20; attempt += 1) {
      if (sentMessages.some((message) => message.type === "agent_chat")) break;
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    const chatMessage = sentMessages.find((message) => message.type === "agent_chat");
    expect(chatMessage?.request).toEqual(expect.objectContaining({
      world_id: "runtime-world-7",
      reorg_epoch: 12,
      authority_scope: "player_agent_chat",
      auth: expect.objectContaining({
        player_id: "player-1",
        public_key: "ab".repeat(32),
        signature: expect.stringMatching(/^awviewauth:v1:/),
      }),
    }));
    sockets[0].receive({
      type: "agent_chat_ack",
      ack: { agent_id: "agent-0", player_id: "player-1", accepted_at_tick: 12 },
    });
    sockets[0].close();
  });
});
