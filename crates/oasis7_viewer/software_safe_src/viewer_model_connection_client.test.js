import { describe, expect, it, vi } from "vitest";
import { acceptsModelCredentials, createViewerModelConnectionClient } from "./viewer_model_connection_client.js";
const snapshot = { id: "connection-1", version: 1, preset_id: "deepseek-v1", state: "ready", run_state: "idle", expires_at_ms: 200000, absolute_expires_at_ms: 900000 };
const response = (value, status = 200) => ({ ok: status < 400, status, json: async () => value });
describe("model connection client", () => {
 it("uses same-origin no-store requests and fresh identity CSRF for every write", async () => {
  const fetch = vi.fn(async (url) => response(url.endsWith("/hosted-account/v2/capabilities") ? { csrf_token: "identity-csrf" } : { ok: true, connection: snapshot }));
  const client = createViewerModelConnectionClient({ fetch, operationId: () => "operation-1" });
  await client.create({ preset_id: "deepseek-v1", raw_key: "private-canary-key", consent: true });
  await client.probe(snapshot); await client.activate(snapshot, { target_ref: { agent_id: "a" }, limits: { max_calls: 1 } });
  await client.lease(snapshot); await client.stop(snapshot); await client.forget(snapshot);
  expect(fetch.mock.calls.filter(([, init]) => init.method === "GET")).toHaveLength(6);
  const writes = fetch.mock.calls.filter(([, init]) => init.method !== "GET");
  expect(writes).toHaveLength(6);
  expect(writes[0][0]).toBe("/v1/agent-client/model-connections");
  expect(JSON.parse(writes[0][1].body)).toEqual({ preset_id: "deepseek-v1", raw_key: "private-canary-key", consent: true });
  for (const [url, init] of fetch.mock.calls) {
   expect(url.startsWith("/")).toBe(true); expect(url).not.toContain("private-canary");
   expect(init).toMatchObject({ credentials: "same-origin", mode: "same-origin", cache: "no-store", redirect: "error", referrerPolicy: "no-referrer" });
   if (init.method !== "GET") expect(init.headers["X-Oasis7-CSRF"]).toBe("identity-csrf");
  }
  expect(writes.slice(1).every(([, init]) => JSON.parse(init.body).expected_version === 1)).toBe(true);
  expect(writes.at(-1)[1].method).toBe("DELETE");
 });
 it("refuses writes without CSRF before sending a key and never exposes upstream errors", async () => {
  const fetch = vi.fn(async () => response({ ok: true }));
  const client = createViewerModelConnectionClient({ fetch });
  await expect(client.create({ preset_id: "p", raw_key: "secret", consent: true })).rejects.toMatchObject({ code: "authentication_required" });
  expect(fetch).toHaveBeenCalledTimes(1);
  const failing = createViewerModelConnectionClient({ fetch: async () => { throw new Error("private-secret error body"); } });
  await expect(failing.capabilities()).rejects.toMatchObject({ message: "transport_failed" });
 });
 it("retains the same probe operation identity after an unknown transport result", async () => {
  let attempts = 0;
  const fetch = vi.fn(async (url) => {
   if (url.endsWith("/capabilities")) return response({ csrf_token: "csrf" });
   if (++attempts === 1) throw new Error("lost response");
   return response({ ok: true, connection: snapshot });
  });
  const operationId = vi.fn(() => "probe-original");
  const client = createViewerModelConnectionClient({ fetch, operationId });
  await expect(client.probe(snapshot)).rejects.toMatchObject({ code: "transport_failed" });
  await client.probe(snapshot);
  const sent = fetch.mock.calls.filter(([url]) => url.endsWith("/probe")).map(([, init]) => JSON.parse(init.body));
  expect(sent).toHaveLength(2); expect(sent[0]).toEqual(sent[1]); expect(operationId).toHaveBeenCalledTimes(1);
 });
 it("gates credentials on authenticated same-origin trust and complete bounded limits", () => {
  const cap = { ok: true, enabled: true, authenticated: true, trusted_executor: { trusted: true, display_name: "Executor", location: "node", origin: "https://game.example" }, presets: [{}], limits: { max_calls: 1, max_input_tokens: 10, max_output_tokens: 10, max_concurrent: 1, lease_ms: 120000, absolute_ms: 7200000, renew_interval_ms: 30000 } };
  expect(acceptsModelCredentials(cap, "https://game.example")).toBe(true);
  for (const changed of [{ ...cap, authenticated: false }, { ...cap, enabled: false }, { ...cap, trusted_executor: { ...cap.trusted_executor, trusted: false } }, { ...cap, limits: {} }]) expect(acceptsModelCredentials(changed, "https://game.example")).toBe(false);
  expect(acceptsModelCredentials(cap, "https://other.example")).toBe(false);
 });
 it("maps conflicts safely and drops unsolicited secret-bearing response fields", async () => {
  const client = createViewerModelConnectionClient({ fetch: async (url) => response(url.endsWith("/capabilities") ? { csrf_token: "csrf" } : { ok: true, connection: { ...snapshot, raw_key: "private-canary", error_code: "private-canary" } }) });
  const result = await client.probe(snapshot);
  expect(result.raw_key).toBeUndefined(); expect(result.error_code).toBe("request_failed");
  expect(JSON.stringify(result)).not.toContain("private-canary");
  const conflict = createViewerModelConnectionClient({ fetch: async (url) => url.endsWith("/capabilities") ? response({ csrf_token: "csrf" }) : response({ error: "private-canary" }, 409) });
  await expect(conflict.stop(snapshot)).rejects.toMatchObject({ code: "version_conflict", status: 409 });
 });
 it("preserves safe model and budget errors instead of turning all conflicts into CAS errors", async () => {
  for (const code of ["budget_exhausted", "model_unavailable", "game_executor_unavailable"]) {
   const client = createViewerModelConnectionClient({ fetch: async (url) => url.endsWith("/capabilities") ? response({ csrf_token: "csrf" }) : response({ error_code: code, error: "private-canary" }, 409) });
   await expect(client.stop(snapshot)).rejects.toMatchObject({ code, status: 409 });
  }
 });

});
