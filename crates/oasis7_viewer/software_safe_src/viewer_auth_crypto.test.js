import { afterEach, describe, expect, it, vi } from "vitest";

import { generateEphemeralEd25519Keypair } from "./viewer_auth_crypto.js";

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
});
