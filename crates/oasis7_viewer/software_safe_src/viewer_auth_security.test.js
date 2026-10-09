import { createMutable } from "solid-js/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { webcrypto } from "node:crypto";
import { generateEphemeralEd25519Keypair, importSigningIdentity } from "./viewer_auth_crypto.js";
import { installSession, clearSession, authCredentials, hasSigningIdentity } from "./viewer_auth_session_module.js";
import { validateRuntimeAckIdentity } from "./viewer_runtime_ack_identity.js";
import { parseViewerRuntimeConfig, resolveViewerEndpoint } from "./viewer_runtime_config_policy.js";

beforeEach(() => { Object.defineProperty(window, "crypto", { configurable: true, value: webcrypto }); });
describe("viewer authentication security boundary", () => {
  it("build flag", () => { expect(__OASIS7_VISUAL_TEST__).toBe(true); });
  it("proves pairing, reuses the exact identity, and refuses mismatched keys", async () => {
    const a = await generateEphemeralEd25519Keypair();
    const b = await generateEphemeralEd25519Keypair();
    const identity = await importSigningIdentity(a.publicKey, a.privateKey);
    expect(await importSigningIdentity(a.publicKey, a.privateKey)).toBe(identity);
    expect(await identity.sign(new Uint8Array([1, 2]))).toHaveLength(64);
    await expect(importSigningIdentity(a.publicKey, b.privateKey)).rejects.toThrow("do not match");
    expect(() => importSigningIdentity("zz".repeat(32), a.privateKey)).toThrow("hex");
    expect(() => importSigningIdentity("aa", a.privateKey)).toThrow("32 bytes");
  });
  it("keeps credentials outside UI serialization and identity fields immutable", async () => {
    const pair = await generateEphemeralEd25519Keypair();
    const state = { auth: {} };
    await installSession(state, { available: true, playerId: "alice", ...pair, releaseToken: "secret", registrationGrant: "grant" });
    expect(JSON.stringify(state.auth)).not.toContain("secret");
    expect(state.auth.privateKey).toBeUndefined();
    expect(authCredentials(state.auth).releaseToken).toBe("secret");
    expect(() => { state.auth.playerId = "mallory"; }).toThrow();
    const old = state.auth;
    clearSession(state);
    expect(authCredentials(old).releaseToken).toBeUndefined();
  });
  it("rejects identity and requested-target conflicts without accepting a registration with missing identity", () => {
    const auth = { playerId: "alice", publicKey: "ab" };
    expect(validateRuntimeAckIdentity({ player_id: "mallory" }, auth)).toContain("conflict");
    expect(validateRuntimeAckIdentity({}, auth, true)).toContain("missing");
    expect(validateRuntimeAckIdentity({ player_id: "alice", session_pubkey: "ab", agent_id: "wrong" }, auth, true, "wanted")).toContain("target");
    expect(validateRuntimeAckIdentity({}, auth)).toBeNull();
  });
  it("retains one verified capability through the reactive UI proxy", async () => {
    const pair = await generateEphemeralEd25519Keypair();
    const state = createMutable({ auth: {} });
    const installed = await installSession(state, { available: true, playerId: "alice", ...pair });
    expect(installed).toBe(state.auth); expect(hasSigningIdentity(state.auth)).toBe(true);
    expect(authCredentials({ ...state.auth })).toEqual({});
    expect(JSON.stringify(state.auth)).not.toContain(pair.privateKey);
  });
  it("cannot turn a visual projection into signing or issuer authority", async () => {
    const state = { auth: {} };
    await expect(installSession(state, { source: "visual_fixture_projection", available: true, playerId: "fixture" })).rejects.toThrow("cannot install");
    expect(authCredentials({ privateKey: "seed", releaseToken: "token", registrationGrant: "grant" })).toEqual({});
  });
  it("requires unique trusted configuration and compares the whole normalized URL", () => {
    const doc = document.implementation.createHTMLDocument();
    expect(() => parseViewerRuntimeConfig(doc)).toThrow("exactly one");
    const node = doc.createElement("script"); node.id = "oasis7-viewer-runtime-config"; node.type = "application/json";
    node.textContent = JSON.stringify({ deploymentMode: "hosted_public_join", viewerWsEndpoint: "wss://trusted.example/runtime?lane=1" }); doc.head.append(node);
    const config = parseViewerRuntimeConfig(doc);
    expect(resolveViewerEndpoint(config, new URLSearchParams("ws=wss://trusted.example:443/runtime?lane=1"))).toBe(config.viewerWsEndpoint);
    for (const url of ["ws://trusted.example/runtime?lane=1", "wss://trusted.example:444/runtime?lane=1", "wss://trusted.example/other", "wss://user@trusted.example/runtime", "wss://trusted.example/runtime#fragment"]) {
      expect(() => resolveViewerEndpoint(config, new URLSearchParams({ ws: url }))).toThrow();
    }
    doc.head.append(node.cloneNode(true)); expect(() => parseViewerRuntimeConfig(doc)).toThrow("exactly one");
  });
});
