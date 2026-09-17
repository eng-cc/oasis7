import { beforeEach, describe, expect, it, vi } from "vitest";

class TestRaceBroadcastChannel {
  static channels = new Map();

  static reset() {
    TestRaceBroadcastChannel.channels = new Map();
  }

  constructor(name) {
    this.name = name;
    this.closed = false;
    this.onmessage = null;
    const peers = TestRaceBroadcastChannel.channels.get(name) || new Set();
    peers.add(this);
    TestRaceBroadcastChannel.channels.set(name, peers);
  }

  postMessage(data) {
    if (this.closed) {
      throw new Error("channel is closed");
    }
    const peers = TestRaceBroadcastChannel.channels.get(this.name) || new Set();
    for (const peer of peers) {
      if (peer !== this && !peer.closed && typeof peer.onmessage === "function") {
        queueMicrotask(() => peer.onmessage({ data }));
      }
    }
  }

  close() {
    this.closed = true;
  }
}

function createTestCrypto() {
  return {
    randomUUID: vi.fn(() => `race-token-${Math.random()}`),
    getRandomValues(bytes) {
      bytes.fill(7);
      return bytes;
    },
  };
}

beforeEach(() => {
  TestRaceBroadcastChannel.reset();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&hosted_test_login=1");
  window.localStorage.clear();
  document.body.innerHTML = "";
});

describe("viewer browser race identity test API", () => {
  it("keeps a claimed browser race identity pending until runtime acknowledges reconnect", async () => {
    const previousBroadcastChannel = window.BroadcastChannel;
    const previousCrypto = window.crypto;
    Object.defineProperty(window, "BroadcastChannel", {
      configurable: true,
      value: TestRaceBroadcastChannel,
    });
    Object.defineProperty(window, "crypto", {
      configurable: true,
      value: createTestCrypto(),
    });
    try {
      vi.resetModules();
      const core = await import("./legacy_core.js");
      core.initializeSoftwareSafeCore();
      core.state.auth = {
        ...core.state.auth,
        available: true,
        playerId: "hosted-player-race-1",
        publicKey: "race-public-key",
        privateKey: "race-private-key",
        releaseToken: "race-release-token",
        source: "hosted_test_login",
        registrationStatus: "registered",
        runtimeStatus: "registered",
        sessionEpoch: 7,
      };

      const descriptor = window.__AW_TEST__.offerBrowserRaceIdentityForTest();
      core.state.auth = {
        ...core.state.auth,
        publicKey: null,
        privateKey: null,
        source: "hosted_browser_storage",
        registrationStatus: "issued",
        runtimeStatus: "issued",
      };

      await expect(window.__AW_TEST__.claimBrowserRaceIdentityForTest(descriptor)).resolves.toMatchObject({
        ok: true,
        playerId: "hosted-player-race-1",
      });
      expect(core.state.auth.registrationStatus).toBe("issued");
      expect(core.state.auth.runtimeStatus).toBe("issued");
    } finally {
      Object.defineProperty(window, "BroadcastChannel", {
        configurable: true,
        value: previousBroadcastChannel,
      });
      Object.defineProperty(window, "crypto", {
        configurable: true,
        value: previousCrypto,
      });
    }
  });
});
