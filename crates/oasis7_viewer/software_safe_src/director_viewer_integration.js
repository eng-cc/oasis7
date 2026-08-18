import {
  createDirectorCapabilityApiAdapter,
  createDirectorCapabilityController,
} from "./director_capability_controller.js";

function restoreStageRoute() {
  const hash = String(window.location.hash || "");
  if (hash === "#viewer-director-entry" || hash === "#viewer-director-panel") {
    window.location.hash = "#viewer-stage-panel";
  }
  document.getElementById("viewer-stage-panel")?.focus();
}

export function createViewerDirectorSession({ core, onChange, fetchImpl } = {}) {
  let previousState = null;
  const controller = createDirectorCapabilityController({
    adapter: createDirectorCapabilityApiAdapter({ fetchImpl }),
    onChange: (nextState) => {
      const wasDirector = previousState?.mode === "director";
      previousState = nextState;
      onChange?.(nextState);
      if (wasDirector && nextState?.mode === "player") {
        restoreStageRoute();
      }
    },
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
