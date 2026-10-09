import { VIEWER_AUTH_SIGNATURE_PREFIX } from "./software_safe_constants.js";

const ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
const textEncoder = new TextEncoder();
const authKeyCache = new Map();
const signingIdentityCache = new Map();
let nextIdentityId = 0;
const HEX_BYTE_LOOKUP = Array.from({ length: 256 }, (_, value) => value.toString(16).padStart(2, "0"));

function cborHeader(majorType, length) {
  if (!Number.isInteger(length) || length < 0) {
    throw new Error(`invalid CBOR length: ${length}`);
  }
  if (length < 24) {
    return Uint8Array.of((majorType << 5) | length);
  }
  if (length < 0x100) {
    return Uint8Array.of((majorType << 5) | 24, length);
  }
  if (length < 0x10000) {
    return Uint8Array.of((majorType << 5) | 25, (length >> 8) & 0xff, length & 0xff);
  }
  if (length <= 0xffffffff) {
    return Uint8Array.of(
      (majorType << 5) | 26,
      (length >>> 24) & 0xff,
      (length >>> 16) & 0xff,
      (length >>> 8) & 0xff,
      length & 0xff,
    );
  }
  if (length <= Number.MAX_SAFE_INTEGER) {
    const value = BigInt(length);
    return Uint8Array.of(
      (majorType << 5) | 27,
      Number((value >> 56n) & 0xffn),
      Number((value >> 48n) & 0xffn),
      Number((value >> 40n) & 0xffn),
      Number((value >> 32n) & 0xffn),
      Number((value >> 24n) & 0xffn),
      Number((value >> 16n) & 0xffn),
      Number((value >> 8n) & 0xffn),
      Number(value & 0xffn),
    );
  }
  throw new Error("CBOR length exceeds Number.MAX_SAFE_INTEGER");
}

function concatBytes(...parts) {
  const totalLength = parts.reduce((sum, bytes) => sum + bytes.length, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const bytes of parts) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}

export function cborEncode(value) {
  if (value === null) {
    return Uint8Array.of(0xf6);
  }
  if (value === false) {
    return Uint8Array.of(0xf4);
  }
  if (value === true) {
    return Uint8Array.of(0xf5);
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`unsupported CBOR number: ${value}`);
    }
    return cborHeader(0, value);
  }
  if (typeof value === "string") {
    const bytes = textEncoder.encode(value);
    return concatBytes(cborHeader(3, bytes.length), bytes);
  }
  if (Array.isArray(value)) {
    return concatBytes(cborHeader(4, value.length), ...value.map((entry) => cborEncode(entry)));
  }
  if (value instanceof Uint8Array) {
    return concatBytes(cborHeader(2, value.length), value);
  }
  if (typeof value === "object") {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined);
    const encoded = [cborHeader(5, entries.length)];
    for (const [key, entryValue] of entries) {
      encoded.push(cborEncode(String(key)));
      encoded.push(cborEncode(entryValue));
    }
    return concatBytes(...encoded);
  }
  throw new Error(`unsupported CBOR type: ${typeof value}`);
}

function hexToBytes(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (!value || value.length % 2 !== 0 || /[^0-9a-f]/.test(value)) {
    throw new Error("invalid hex payload");
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes) {
  let out = "";
  for (let index = 0; index < bytes.length; index += 1) {
    out += HEX_BYTE_LOOKUP[bytes[index]];
  }
  return out;
}

function bytesStartWith(bytes, prefix) {
  if (bytes.length < prefix.length) {
    return false;
  }
  for (let index = 0; index < prefix.length; index += 1) {
    if (bytes[index] !== prefix[index]) {
      return false;
    }
  }
  return true;
}

async function importEd25519SigningKey(privateKeyHex) {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  if (!authKeyCache.has(privateKeyHex)) {
    const rawPrivateKey = hexToBytes(privateKeyHex);
    if (rawPrivateKey.length !== 32) {
      throw new Error(`viewer auth private key length mismatch: expected 32 bytes, got ${rawPrivateKey.length}`);
    }
    const pkcs8 = concatBytes(ED25519_PKCS8_PREFIX, rawPrivateKey);
    authKeyCache.set(
      privateKeyHex,
      window.crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"]),
    );
  }
  return authKeyCache.get(privateKeyHex);
}

export function importSigningIdentity(publicKeyHex, privateKeyHex) {
  if (typeof publicKeyHex !== "string" || typeof privateKeyHex !== "string" || !/^[0-9a-f]{64}$/i.test(publicKeyHex) || !/^[0-9a-f]{64}$/i.test(privateKeyHex)) {
    throw new Error("viewer auth keys require strict hex encoding of exactly 32 bytes");
  }
  // Cache the exact pair, never just the public key: mismatched material must
  // prove possession independently. Failed imports are evicted for retry.
  const publicBytes = hexToBytes(publicKeyHex);
  const privateBytes = hexToBytes(privateKeyHex);
  if (publicBytes.length !== 32 || privateBytes.length !== 32) {
    throw new Error("viewer auth keys must each contain exactly 32 bytes");
  }
  const publicKey = bytesToHex(publicBytes);
  const cacheKey = `${publicKey}:${bytesToHex(privateBytes)}`;
  if (!signingIdentityCache.has(cacheKey)) {
    const pending = (async () => {
      if (!window.crypto?.subtle) throw new Error("Ed25519 Web Crypto is unavailable");
      const privateKey = await importEd25519SigningKey(bytesToHex(privateBytes));
      const verifyKey = await window.crypto.subtle.importKey("raw", publicBytes, { name: "Ed25519" }, false, ["verify"]);
      const challenge = window.crypto.getRandomValues(new Uint8Array(32));
      const proof = await window.crypto.subtle.sign("Ed25519", privateKey, challenge);
      if (!await window.crypto.subtle.verify("Ed25519", verifyKey, proof, challenge)) {
        throw new Error("viewer auth public and private keys do not match");
      }
      return Object.freeze({
        identityId: ++nextIdentityId,
        publicKey,
        sign: async (bytes) => new Uint8Array(await window.crypto.subtle.sign("Ed25519", privateKey, bytes)),
      });
    })();
    signingIdentityCache.set(cacheKey, pending);
    pending.catch(() => signingIdentityCache.delete(cacheKey));
  }
  return signingIdentityCache.get(cacheKey);
}

let identityLookup = () => null;
let signingGeneration = () => 0;
export function setSigningIdentityLookup(lookup, generation) { identityLookup = lookup; signingGeneration = generation; }
export async function signAuthPayload(signingPayloadBytes, auth) {
  const generation = signingGeneration();
  const installed = identityLookup(auth);
  if (!installed) throw new Error("authentication requires a verified installed signing identity");
  const identity = installed;
  const signature = await identity.sign(signingPayloadBytes);
  if (signingGeneration() !== generation || (installed && identityLookup(auth) !== installed)) throw new Error("authentication context changed during signing");
  return `${VIEWER_AUTH_SIGNATURE_PREFIX}${bytesToHex(signature)}`;
}

export async function generateEphemeralEd25519Keypair() {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  const keyPair = await window.crypto.subtle.generateKey(
    { name: "Ed25519" },
    true,
    ["sign", "verify"],
  );
  const pkcs8 = new Uint8Array(await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey));
  if (!bytesStartWith(pkcs8, ED25519_PKCS8_PREFIX) || pkcs8.length !== ED25519_PKCS8_PREFIX.length + 32) {
    throw new Error("unexpected Ed25519 pkcs8 encoding from Web Crypto");
  }
  const rawPublicKey = new Uint8Array(await window.crypto.subtle.exportKey("raw", keyPair.publicKey));
  if (rawPublicKey.length !== 32) {
    throw new Error(`unexpected Ed25519 public key length: ${rawPublicKey.length}`);
  }
  return {
    publicKey: bytesToHex(rawPublicKey),
    privateKey: bytesToHex(pkcs8.slice(ED25519_PKCS8_PREFIX.length)),
  };
}

export function buildAuthEnvelope(payload) {
  return cborEncode({
    version: 1,
    payload,
  });
}

// Keep this property order aligned with PromptControlEnhancedSigningPayload in
// crates/oasis7/src/viewer/auth.rs. The runtime signs the CBOR envelope, so
// insertion order is part of the protocol contract.
export function promptFieldPatchV1(patch) {
  if (!patch || patch.mode === "unchanged") {
    return "unchanged";
  }
  if (patch.mode === "clear") {
    return "clear";
  }
  const value = String(patch.value ?? "").trim();
  return value ? { set: value } : "clear";
}

export function buildPromptControlSigningPayload(mode, request, auth) {
  const normalizedMode = String(mode || "").trim().toLowerCase();
  const rollback = normalizedMode === "rollback";
  const preview = normalizedMode === "preview";
  return {
    operation: rollback
      ? "prompt_control_rollback"
      : preview
        ? "prompt_control_preview"
        : "prompt_control_apply",
    preview,
    request_id: String(request?.request_id || "").trim(),
    agent_id: String(request?.agent_id || "").trim(),
    player_id: String(auth?.playerId || request?.player_id || "").trim(),
    public_key: String(auth?.publicKey || request?.public_key || "").trim().toLowerCase(),
    nonce: request?.nonce,
    session_epoch: Number(request?.session_epoch),
    binding_epoch: Number(request?.binding_epoch),
    expected_authority_epoch: String(request?.expected_authority_epoch || "").trim(),
    expected_version: Number(request?.expected_version),
    system_prompt: rollback ? "unchanged" : promptFieldPatchV1(request?.system_prompt_override),
    short_term_goal: rollback ? "unchanged" : promptFieldPatchV1(request?.short_term_goal_override),
    long_term_goal: rollback ? "unchanged" : promptFieldPatchV1(request?.long_term_goal_override),
    rollback_target: rollback ? Number(request?.to_version) : null,
    updated_by: String(request?.updated_by || "").trim() || undefined,
  };
}
