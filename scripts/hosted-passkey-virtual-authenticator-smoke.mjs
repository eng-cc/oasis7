#!/usr/bin/env node
// Software-authenticator evidence only; this does not certify real device support.
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { join } from "node:path";
import assert from "node:assert/strict";

const runtimeModules = process.env.OASIS7_PLAYWRIGHT_NODE_MODULES || "/Users/scc/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules";
const require = existsSync(join(runtimeModules, "playwright")) ? createRequire(join(runtimeModules, "oasis7-passkey-smoke.cjs")) : createRequire(import.meta.url);
const { chromium } = require("playwright");
const url = process.argv[process.argv.indexOf("--url") + 1];
if (!url || !url.startsWith("https://")) throw new Error("Usage: node scripts/hosted-passkey-virtual-authenticator-smoke.mjs --url https://localhost:8449/");
const moduleUrl = new URL("/viewer_passkey_account_module.js", url).toString();
const browser = await chromium.launch({ channel: "chrome", headless: true });
try {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  await cdp.send("WebAuthn.enable");
  const { authenticatorId } = await cdp.send("WebAuthn.addVirtualAuthenticator", { options: {
    protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true,
    isUserVerified: true, automaticPresenceSimulation: true,
  } });
  let registrations = 0;
  let secondAuthenticatorId = null;
  await page.exposeFunction("smokeBeforeCreate", async () => {
    if (registrations++ === 0) return;
    // A second physical authenticator is modelled separately. Registering the
    // same resident user twice on one authenticator correctly raises InvalidState.
    await cdp.send("WebAuthn.setAutomaticPresenceSimulation", { authenticatorId, enabled: false });
    const second = await cdp.send("WebAuthn.addVirtualAuthenticator", { options: {
      protocol: "ctap2", transport: "usb", hasResidentKey: true, hasUserVerification: true,
      isUserVerified: true, automaticPresenceSimulation: true,
    } });
    secondAuthenticatorId = second.authenticatorId;
  });
  await page.goto(url);
  await page.evaluate(async (moduleUrl) => {
    const module = await import(moduleUrl);
    window.smokeClient = module.createPasskeyAccountClient({ credentials: {
      create: async (options) => { await window.smokeBeforeCreate(); return navigator.credentials.create(options); },
      get: (options) => navigator.credentials.get(options),
    }, fetch: async (url, options) => {
      if (url.endsWith("/passkeys/registration/complete")) window.smokeRegisterReplay = { url, body: options.body };
      return fetch(url, options);
    } });
    window.smokeOperations = {};
    window.smokeResult = null;
    const button = document.createElement("button"); button.id = "virtual-authenticator-action"; button.textContent = "Run software-authenticator test";
    button.onclick = async () => {
      try { window.smokeResult = { ok: true, value: await window.smokeOperations[window.smokeOperation]() }; }
      catch (error) { window.smokeResult = { ok: false, code: error.code || error.message, status: error.status }; }
    };
    document.body.append(button);
  }, moduleUrl);
  async function action(name, fn) {
    // CDP installs the test callback; the page needs no eval permission.
    await page.evaluate(`window.smokeOperations[${JSON.stringify(name)}] = ${fn.toString()}; window.smokeOperation = ${JSON.stringify(name)}; window.smokeResult = null;`);
    await page.locator("#virtual-authenticator-action").click({ force: true });
    await page.waitForFunction(() => window.smokeResult !== null);
    return page.evaluate(() => window.smokeResult);
  }
  const capabilities = await page.evaluate(() => window.smokeClient.capabilities());
  assert.equal(capabilities.passkey.available, true);
  assert.equal(capabilities.registration_enabled, true);
  const duplicate = await page.evaluate(async () => {
    const capabilities = await window.smokeClient.capabilities();
    const response = await fetch("/api/public/hosted-account/v2/passkeys/registration/options", {
      method: "POST", headers: { "Content-Type": "application/json", "X-Oasis7-CSRF": capabilities.csrf_token },
      body: '{"intent":"new_account","intent":"bind"}' });
    return response.status;
  });
  assert.equal(duplicate, 400);
  let result = await action("register", () => window.smokeClient.register());
  assert.equal(result.ok, true, JSON.stringify(result));
  const accountId = result.value.account.account_id;
  assert.equal(result.value.world.state, "not_requested");
  const generated = await cdp.send("WebAuthn.getCredentials", { authenticatorId });
  assert.equal(generated.credentials.length, 1);
  assert.equal(generated.credentials[0].isResidentCredential, true);
  let session = await page.evaluate(() => window.smokeClient.session());
  assert.equal(session.account.account_id, accountId);
  const cookieBeforeReplay = await context.cookies();
  const replay = await page.evaluate(async () => {
    const capabilities = await window.smokeClient.capabilities();
    const response = await fetch(window.smokeRegisterReplay.url, { method: "POST", credentials: "same-origin",
      headers: { "Content-Type": "application/json", "X-Oasis7-CSRF": capabilities.csrf_token }, body: window.smokeRegisterReplay.body });
    return { status: response.status, result: await response.json() };
  });
  assert.ok(replay.status >= 400 || replay.result.account?.account_id === accountId
    || (replay.result.phase === "operation_committed" && replay.result.account_retained === true));
  assert.ok(JSON.stringify(await context.cookies()) === JSON.stringify(cookieBeforeReplay), "replayed registration must not issue a fresh account cookie");
  await context.clearCookies();
  await page.evaluate(() => window.smokeClient.capabilities());
  result = await action("login-after-browser-reset", () => window.smokeClient.login());
  assert.equal(result.ok, true, JSON.stringify(result));
  assert.equal(result.value.account.account_id, accountId);
  await page.evaluate(() => window.smokeClient.capabilities());
  const originalKeys = await page.evaluate(() => window.smokeClient.credentials());
  const firstKey = originalKeys.credentials.find((key) => key.state === "active");
  result = await action("add-second-credential", () => window.smokeClient.addCredential());
  assert.equal(result.ok, true, JSON.stringify(result));
  let keys = await page.evaluate(() => window.smokeClient.credentials());
  assert.equal(keys.credentials.filter((key) => key.state === "active").length, 2);
  // Keep both records active on the server but explicitly prove the retained
  // credential for revocation. The test authenticator otherwise selects its first key.
  await cdp.send("WebAuthn.removeCredential", { authenticatorId, credentialId: generated.credentials[0].credentialId });
  await page.evaluate((key) => { window.smokeRevokeKey = key; }, firstKey);
  result = await action("revoke-one-credential", () => window.smokeClient.revokeCredential(window.smokeRevokeKey));
  assert.equal(result.ok, true, JSON.stringify(result));
  // Restore the revoked test key from its in-memory CDP snapshot. Its real
  // signature must still fail against the server's credential tombstone.
  await cdp.send("WebAuthn.addCredential", { authenticatorId, credential: generated.credentials[0] });
  await cdp.send("WebAuthn.setAutomaticPresenceSimulation", { authenticatorId, enabled: true });
  await cdp.send("WebAuthn.setAutomaticPresenceSimulation", { authenticatorId: secondAuthenticatorId, enabled: false });
  await context.clearCookies();
  result = await action("revoked-credential-login", () => window.smokeClient.login());
  assert.equal(result.ok, false);
  assert.equal(result.status, 401);
  await cdp.send("WebAuthn.setAutomaticPresenceSimulation", { authenticatorId, enabled: false });
  await cdp.send("WebAuthn.setAutomaticPresenceSimulation", { authenticatorId: secondAuthenticatorId, enabled: true });
  await context.clearCookies();
  await page.evaluate(() => window.smokeClient.capabilities());
  result = await action("login-retained-credential", () => window.smokeClient.login());
  assert.equal(result.ok, true, JSON.stringify(result));
  assert.equal(result.value.account.account_id, accountId);
  await page.evaluate(() => window.smokeClient.capabilities());
  keys = await page.evaluate(() => window.smokeClient.credentials());
  assert.equal(keys.credentials.filter((key) => key.state === "active").length, 1);
  const csrfFailure = await page.evaluate(async () => {
    const response = await fetch("/api/public/hosted-account/v2/logout", { method: "POST", credentials: "same-origin", headers: { "Content-Type": "application/json", "X-Oasis7-CSRF": "wrong" }, body: "{}" });
    return response.status;
  });
  assert.equal(csrfFailure, 403);
  session = await page.evaluate(() => window.smokeClient.session());
  assert.equal(session.account.account_id, accountId);
  result = await action("logout", () => window.smokeClient.logout());
  assert.equal(result.ok, true);
  result = await action("session-after-logout", () => window.smokeClient.session());
  assert.equal(result.ok, false);
  assert.equal(result.status, 401);
  console.log(JSON.stringify({ ok: true, evidence: "chromium_virtual_authenticator", resident_required: true,
    user_verification_required: true, registration: "passed", discoverable_login_after_cookie_reset: "passed",
    stable_account: "passed", registration_replay_no_cookie: "passed", second_credential: "passed", credential_revocation: "passed", revoked_credential_signature_rejected: "passed", csrf: "passed", logout: "passed",
    real_device_support: "not_tested" }));
} finally { await browser.close(); }
