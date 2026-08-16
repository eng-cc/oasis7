import {
  createDirectorCapabilityApiAdapter,
  createDirectorCapabilityController,
} from "./director_capability_controller.js";

export function createViewerDirectorSession({ core, onChange, fetchImpl } = {}) {
  const controller = createDirectorCapabilityController({
    adapter: createDirectorCapabilityApiAdapter({ fetchImpl }),
    onChange,
  });
  return {
    controller,
    request: async () => {
      window.location.hash = "#viewer-director-entry";
      const result = await controller.request({ worldId: core?.state?.worldId });
      if (result.mode === "director") {
        window.location.hash = "#viewer-director-panel";
        queueMicrotask(() => document.getElementById("viewer-director-panel")?.focus());
      }
      return result;
    },
    observeRuntime: () => controller.observeRuntime({
      connectionStatus: core?.state?.connectionStatus,
      authAvailable: core?.state?.auth?.available,
      authRuntimeStatus: core?.state?.auth?.runtimeStatus,
      authRevokeReason: core?.state?.auth?.revokeReason,
      authRevokedBy: core?.state?.auth?.revokedBy,
    }),
  };
}
