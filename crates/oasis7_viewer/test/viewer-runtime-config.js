export { parseViewerRuntimeConfig, normalizeTrustedWsEndpoint, resolveViewerEndpoint } from "../software_safe_src/viewer_runtime_config_policy.js";
export function viewerRuntimeConfig() {
  const params = new URLSearchParams(window.location.search);
  let hint; try { hint = JSON.parse(params.get("hosted_access")); } catch {}
  return Object.freeze({ deploymentMode: hint?.deployment_mode || "trusted_local_only", viewerWsEndpoint: new URL(params.get("ws") || params.get("addr") || "ws://127.0.0.1:9001").href });
}
