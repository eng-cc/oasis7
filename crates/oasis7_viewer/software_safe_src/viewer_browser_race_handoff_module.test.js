import { beforeEach, describe, expect, it, vi } from "vitest";
import { createViewerBrowserRaceHandoffModule } from "./viewer_browser_race_handoff_module.js";

class FakeBroadcastChannel {
  static channels = new Map();
  static messages = [];
  static closeCounts = new Map();

  static reset() {
    FakeBroadcastChannel.channels = new Map();
    FakeBroadcastChannel.messages = [];
    FakeBroadcastChannel.closeCounts = new Map();
  }

  static closeCount(name) {
    return FakeBroadcastChannel.closeCounts.get(name) || 0;
  }

  constructor(name) {
    this.name = name;
    this.closed = false;
    this.onmessage = null;
    const peers = FakeBroadcastChannel.channels.get(name) || new Set();
    peers.add(this);
    FakeBroadcastChannel.channels.set(name, peers);
  }

  postMessage(data) {
    if (this.closed) {
      throw new Error("channel is closed");
    }
    FakeBroadcastChannel.messages.push({ channel: this.name, data });
    const peers = FakeBroadcastChannel.channels.get(this.name) || new Set();
    for (const peer of peers) {
      if (peer !== this && !peer.closed && typeof peer.onmessage === "function") {
        queueMicrotask(() => peer.onmessage({ data }));
      }
    }
  }

  close() {
    this.closed = true;
    FakeBroadcastChannel.closeCounts.set(
      this.name,
      FakeBroadcastChannel.closeCount(this.name) + 1,
    );
  }
}

function createHarness({ origin = "http://127.0.0.1:4310", search = "?test_api=1&hosted_test_login=1" } = {}) {
  let clock = 1_000;
  let nonceCounter = 0;
  const cryptoRef = {
    randomUUID: vi.fn(() => `run-token-${++nonceCounter}-unguessable`),
    getRandomValues: vi.fn((bytes) => {
      bytes.fill(7);
      return bytes;
    }),
  };
  const locationRef = {
    hostname: new URL(origin).hostname,
    origin,
    search,
  };
  const dependencies = {
    BroadcastChannelImpl: FakeBroadcastChannel,
    clearTimeoutImpl: clearTimeout,
    cryptoRef,
    locationRef,
    now: () => clock,
    setTimeoutImpl: setTimeout,
  };
  return {
    advance(ms) {
      clock += ms;
    },
    create() {
      return createViewerBrowserRaceHandoffModule(dependencies);
    },
    cryptoRef,
    locationRef,
  };
}

beforeEach(() => {
  FakeBroadcastChannel.reset();
});

describe("viewer browser race handoff module", () => {
  it("only enables the in-memory handoff on loopback test_api hosted-test-login pages", () => {
    expect(createHarness().create().enabled).toBe(true);
    expect(createHarness({ origin: "https://viewer.example.test" }).create().enabled).toBe(false);
    expect(createHarness({ search: "?test_api=1" }).create().enabled).toBe(false);
    expect(createHarness({ search: "?hosted_test_login=1" }).create().enabled).toBe(false);
  });

  it("requires a player binding for every offered identity", () => {
    const owner = createHarness().create();

    expect(() => owner.offerKeyMaterial({
      publicKey: "public-key",
      privateKey: "private-key",
      releaseToken: "release-token",
      sessionEpoch: 7,
    })).toThrowError(expect.objectContaining({ code: "key_material_missing" }));
  });

  it("offers the same browser key once through a run-scoped channel without exposing it in metadata", async () => {
    const owner = createHarness().create();
    const recipient = createHarness().create();
    const offer = owner.offerKeyMaterial({ publicKey: "public-key", privateKey: "private-key", releaseToken: "release-token", playerId: "player-1", sessionEpoch: 7 });

    expect(offer.descriptor).toMatchObject({
      origin: "http://127.0.0.1:4310",
      schema: "oasis7.viewer.race-handoff/v1",
    });
    expect(offer.descriptor.channelName).toContain("oasis7.viewer.race-handoff.v1");
    expect(offer.descriptor.privateKey).toBeUndefined();
    expect(offer.descriptor.publicKey).toBeUndefined();
    expect(offer.descriptor.releaseToken).toBeUndefined();

    const claimed = await recipient.claimOffer(offer.descriptor);

    expect(claimed).toEqual({
      authorityEpoch: null,
      bindingEpoch: null,
      boundAgentId: null,
      privateKey: "private-key",
      publicKey: "public-key",
      releaseToken: "release-token",
      playerId: "player-1",
      sessionEpoch: 7,
    });
    expect(FakeBroadcastChannel.closeCount(offer.descriptor.channelName)).toBe(2);
    owner.dispose();
    recipient.dispose();
    expect(FakeBroadcastChannel.closeCount(offer.descriptor.channelName)).toBe(2);
    expect(FakeBroadcastChannel.messages.some(({ data }) => data.keyMaterial?.privateKey === "private-key")).toBe(true);
    expect(FakeBroadcastChannel.messages.some(({ data }) => data.keyMaterial?.publicKey === "public-key")).toBe(true);
    expect(FakeBroadcastChannel.messages.some(({ data }) => data.keyMaterial?.releaseToken === "release-token")).toBe(true);
  });

  it("rejects a second claim and a tampered nonce as replay or nonce mismatch", async () => {
    const owner = createHarness().create();
    const recipient = createHarness().create();
    const offer = owner.offerKeyMaterial({ publicKey: "public-key", privateKey: "private-key", releaseToken: "release-token", playerId: "player-1", sessionEpoch: 7 });

    await recipient.claimOffer(offer.descriptor);
    await expect(recipient.claimOffer(offer.descriptor)).rejects.toMatchObject({ code: "replay" });

    const secondOwner = createHarness().create();
    const secondRecipient = createHarness().create();
    const secondOffer = secondOwner.offerKeyMaterial({ publicKey: "public-key-2", privateKey: "private-key-2", releaseToken: "release-token-2", playerId: "player-2", sessionEpoch: 8 });
    void secondOffer.waitForClaim().catch(() => {});
    await expect(secondRecipient.claimOffer({
      ...secondOffer.descriptor,
      nonce: "tampered-nonce",
    })).rejects.toMatchObject({ code: "nonce_mismatch" });
  });

  it("rejects a cross-origin claim and an expired offer before key delivery", async () => {
    const ownerHarness = createHarness();
    const owner = ownerHarness.create();
    const crossOriginRecipient = createHarness({ origin: "http://localhost:4310" }).create();
    const offer = owner.offerKeyMaterial({
      publicKey: "public-key",
      privateKey: "private-key",
      releaseToken: "release-token", playerId: "player-1", sessionEpoch: 7,
      ttlMs: 20,
    });

    await expect(crossOriginRecipient.claimOffer(offer.descriptor)).rejects.toMatchObject({ code: "origin_mismatch" });

    ownerHarness.advance(21);
    const expiredRecipientHarness = createHarness();
    expiredRecipientHarness.advance(21);
    const expiredRecipient = expiredRecipientHarness.create();
    await expect(expiredRecipient.claimOffer(offer.descriptor)).rejects.toMatchObject({ code: "expired" });
    expect(FakeBroadcastChannel.messages.some(({ data }) => data.privateKey === "private-key")).toBe(false);
  });

  it("closes the channel and rejects pending work on dispose", async () => {
    const owner = createHarness().create();
    const recipient = createHarness().create();
    const offer = owner.offerKeyMaterial({ publicKey: "public-key", privateKey: "private-key", releaseToken: "release-token", playerId: "player-1", sessionEpoch: 7 });
    void offer.waitForClaim().catch(() => {});
    const pendingClaim = recipient.claimOffer(offer.descriptor);

    owner.dispose();
    recipient.dispose();

    await expect(pendingClaim).rejects.toMatchObject({ code: "disposed" });
    expect(offer.disposed).toBe(true);
  });
});
