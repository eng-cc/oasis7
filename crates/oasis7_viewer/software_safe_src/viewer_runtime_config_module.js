import { parseViewerRuntimeConfig } from "./viewer_runtime_config_policy.js";
export { parseViewerRuntimeConfig, normalizeTrustedWsEndpoint, resolveViewerEndpoint } from "./viewer_runtime_config_policy.js";
let fixedConfig;
export function viewerRuntimeConfig() {
  if (!fixedConfig) fixedConfig = parseViewerRuntimeConfig(document);
  return fixedConfig;
}
