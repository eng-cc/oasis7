import { afterEach, describe, expect, it, vi } from "vitest";

import {
  buildAuthEnvelope,
  cborCanonicalEncode,
  buildPromptControlSigningPayload,
  generateEphemeralEd25519Keypair,
  promptFieldPatchV1,
} from "./viewer_auth_crypto.js";

const ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);

const originalCryptoDescriptor = Object.getOwnPropertyDescriptor(window, "crypto");

afterEach(() => {
  vi.restoreAllMocks();
  if (originalCryptoDescriptor) {
    Object.defineProperty(window, "crypto", originalCryptoDescriptor);
  }
});

describe("viewer auth crypto", () => {
  it("matches the Rust WorldService canonical CBOR signing golden without changing legacy envelope order", () => {
    const request = {contract_version:1,world:{world_id:"world",genesis_digest:"genesis"},scope_id:"agent:owner-agent",min_commit:null,fixed_commit:null,deadline_unix_ms:null};
    const bytes = cborCanonicalEncode(["oasis7.world-service.v1","/v1/world/view",request]);
    expect(Array.from(bytes,x=>x.toString(16).padStart(2,"0")).join("")).toBe("83776f61736973372e776f726c642d736572766963652e76316e2f76312f776f726c642f76696577a665776f726c64a268776f726c645f696465776f726c646e67656e657369735f6469676573746767656e657369736873636f70655f6964716167656e743a6f776e65722d6167656e746a6d696e5f636f6d6d6974f66c66697865645f636f6d6d6974f670636f6e74726163745f76657273696f6e0170646561646c696e655f756e69785f6d73f6");
  });
  it("serializes generated Ed25519 key bytes as zero-padded lowercase hex", async () => {
    const privateKey = new Uint8Array(32);
    const publicKey = new Uint8Array(32);
    for (let index = 0; index < 32; index += 1) {
      privateKey[index] = index;
      publicKey[index] = 255 - index;
    }

    Object.defineProperty(window, "crypto", {
      configurable: true,
      value: {
        subtle: {
          generateKey: vi.fn(async () => ({ privateKey: "private-key", publicKey: "public-key" })),
          exportKey: vi.fn(async (format, key) => {
            if (format === "pkcs8" && key === "private-key") {
              return new Uint8Array([...ED25519_PKCS8_PREFIX, ...privateKey]).buffer;
            }
            if (format === "raw" && key === "public-key") {
              return publicKey.buffer;
            }
            throw new Error(`unexpected exportKey call: ${format}/${key}`);
          }),
        },
      },
    });

    await expect(generateEphemeralEd25519Keypair()).resolves.toEqual({
      privateKey: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
      publicKey: "fffefdfcfbfaf9f8f7f6f5f4f3f2f1f0efeeedecebeae9e8e7e6e5e4e3e2e1e0",
    });
  });

  it("matches the enhanced prompt-control CBOR payload contract", () => {
    const request = {
      request_id: "pc-00000001",
      agent_id: "agent-0",
      player_id: "player-a",
      public_key: "AB".repeat(32),
      nonce: 41,
      session_epoch: 7,
      binding_epoch: 3,
      expected_authority_epoch: "authority-1",
      expected_version: 5,
      system_prompt_override: { mode: "set", value: "  Stay concise.  " },
      short_term_goal_override: { mode: "unchanged" },
      long_term_goal_override: { mode: "clear" },
      updated_by: "player-a",
    };
    const payload = buildPromptControlSigningPayload("apply", request, {
      playerId: request.player_id,
      publicKey: request.public_key,
    });

    expect(Object.keys(payload)).toEqual([
      "operation",
      "preview",
      "request_id",
      "agent_id",
      "player_id",
      "public_key",
      "nonce",
      "session_epoch",
      "binding_epoch",
      "expected_authority_epoch",
      "expected_version",
      "system_prompt",
      "short_term_goal",
      "long_term_goal",
      "rollback_target",
      "updated_by",
    ]);
    expect(payload).toEqual({
      operation: "prompt_control_apply",
      preview: false,
      request_id: "pc-00000001",
      agent_id: "agent-0",
      player_id: "player-a",
      public_key: "ab".repeat(32),
      nonce: 41,
      session_epoch: 7,
      binding_epoch: 3,
      expected_authority_epoch: "authority-1",
      expected_version: 5,
      system_prompt: { set: "Stay concise." },
      short_term_goal: "unchanged",
      long_term_goal: "clear",
      rollback_target: null,
      updated_by: "player-a",
    });
    const envelope = buildAuthEnvelope(payload);
    expect(envelope[0]).toBe(0xa2);
    expect(envelope[1]).toBe(0x67);
    expect(promptFieldPatchV1({ mode: "set", value: "   " })).toBe("clear");
  });

  it("uses the rollback operation identity and target with unchanged patches", () => {
    const payload = buildPromptControlSigningPayload(
      "rollback",
      {
        request_id: "pc-00000002",
        agent_id: "agent-0",
        player_id: "player-a",
        public_key: "cd".repeat(32),
        nonce: 42,
        session_epoch: 8,
        binding_epoch: 4,
        expected_authority_epoch: "authority-2",
        expected_version: 6,
        to_version: 4,
        updated_by: "player-a",
      },
      { playerId: "player-a", publicKey: "cd".repeat(32) },
    );

    expect(payload).toEqual(expect.objectContaining({
      operation: "prompt_control_rollback",
      preview: false,
      rollback_target: 4,
      system_prompt: "unchanged",
      short_term_goal: "unchanged",
      long_term_goal: "unchanged",
    }));
    expect(payload.actor).toBeUndefined();
  });
});
