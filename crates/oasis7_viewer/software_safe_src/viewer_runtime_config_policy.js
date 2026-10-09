const CONFIG_ID = "oasis7-viewer-runtime-config";
const MODES = new Set(["trusted_local_only", "hosted_public_join"]);
export function normalizeTrustedWsEndpoint(value) {
  if (typeof value !== "string" || !value) throw new Error("viewer endpoint must be a URL string");
  const url = new URL(value);
  if (!["ws:", "wss:"].includes(url.protocol) || url.username || url.password || value.includes("#") || /^wss?:\/\/[^/?#]*@/i.test(value)) {
    throw new Error("viewer endpoint contains a forbidden URL component");
  }
  return url.href;
}
export function parseViewerRuntimeConfig(documentRef) {
  const nodes = documentRef.querySelectorAll(`[id="${CONFIG_ID}"]`);
  if (nodes.length !== 1 || nodes[0].type !== "application/json") throw new Error("viewer requires exactly one trusted runtime configuration");
  const raw = JSON.parse(nodes[0].textContent);
  if (!raw || !MODES.has(raw.deploymentMode)) throw new Error("invalid viewer deployment mode");
  const viewerWsEndpoint = normalizeTrustedWsEndpoint(raw.viewerWsEndpoint);
  if (raw.deploymentMode === "hosted_public_join" && !viewerWsEndpoint.startsWith("wss:") && !["127.0.0.1", "localhost", "[::1]"].includes(new URL(viewerWsEndpoint).hostname)) {
    throw new Error("hosted viewer requires a secure WebSocket endpoint");
  }
  return Object.freeze({ deploymentMode: raw.deploymentMode, viewerWsEndpoint, endpointId: viewerWsEndpoint });
}
export function resolveViewerEndpoint(config, params) {
  for (const key of ["ws", "addr"]) {
    if (params.has(key) && normalizeTrustedWsEndpoint(params.get(key)) !== config.viewerWsEndpoint) {
      throw new Error("viewer URL endpoint does not match trusted configuration");
    }
  }
  return config.viewerWsEndpoint;
}
