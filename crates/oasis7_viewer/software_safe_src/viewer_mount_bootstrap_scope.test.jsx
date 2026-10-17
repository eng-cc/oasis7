import { waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { runtimeMock, sampleSnapshot, useTestRustRenderState } from "./pixel_world_host_test_support.jsx";

const sessionGate = vi.hoisted(() => ({ promise: null }));
vi.mock("./viewer_auth_session_module.js", async () => {
  const actual = await vi.importActual("./viewer_auth_session_module.js");
  return {
    ...actual,
    async installSession(...args) {
      await sessionGate.promise;
      return actual.installSession(...args);
    },
  };
});

let dispose;
let releaseSession;
let rejectSession;
beforeEach(() => {
  vi.resetModules();
  window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en");
  document.body.innerHTML = '<div id="app"></div>';
  sessionGate.promise = new Promise((resolve, reject) => {
    releaseSession = resolve;
    rejectSession = reject;
  });
  useTestRustRenderState();
  const derive = runtimeMock.deriveRenderState;
  runtimeMock.deriveRenderState = input => {
    const state = derive(input);
    state.commercial_surface.next_action.execute_kind = "none";
    return state;
  };
});
afterEach(() => {
  releaseSession();
  dispose?.();
  dispose = null;
  sessionGate.promise = null;
});

it("captures the initial display scope after session bootstrap and fixture installation", async () => {
  const core = await import("./legacy_core.js");
  const { mountViewerApp, configureViewerVisualTest } = await import("./viewer_app.jsx");
  configureViewerVisualTest({
    enabled: () => true,
    fixtureName: () => null,
    install() {
      core.state.auth.authorityEpoch = 7;
      core.injectSnapshot(sampleSnapshot(), { returnState: false });
      return null;
    },
  });
  dispose = mountViewerApp(document.getElementById("app"));
  expect(document.querySelector(".viewer-shell")).not.toBeNull();
  const rendererMountedDuringBootstrap = document.getElementById("pixel-world-embedded-runtime-canvas") !== null;

  releaseSession();
  await core.initializeSoftwareSafeCore();
  expect(core.state.pixelWorldFatal).toBeNull();
  expect(rendererMountedDuringBootstrap).toBe(false);
  await waitFor(() => expect(core.state.pixelWorldRuntimeStatus).toBe("ready"));
  expect(runtimeMock.mountCalls).toBeGreaterThan(0);
  expect(core.state.pixelWorldFatal).toBeNull();
  expect(core.state.wsUrl).toBeTruthy();
  expect(core.state.auth.authorityEpoch).toBe(7);

  // Subsequent identity changes still clear the display; ordinary snapshots
  // never substitute for the correlated scope recovery read.
  core.state.auth.authorityEpoch = 8;
  core.requestRender();
  await waitFor(() => expect(core.state.pixelWorldFatal?.code).toBe("pixel_world_scope_changed"));
  core.injectSnapshot(sampleSnapshot(), { returnState: false });
  expect(core.state.pixelWorldFatal?.code).toBe("pixel_world_scope_changed");
});

it("keeps the shell usable and the world renderer unmounted when session bootstrap fails", async () => {
  const core = await import("./legacy_core.js");
  const { mountViewerApp, configureViewerVisualTest } = await import("./viewer_app.jsx");
  const installFixture = vi.fn();
  configureViewerVisualTest({ enabled: () => true, fixtureName: () => null, install: installFixture });
  dispose = mountViewerApp(document.getElementById("app"));
  const initialized = core.initializeSoftwareSafeCore();
  rejectSession(new Error("session bootstrap failed"));
  await expect(initialized).rejects.toThrow("session bootstrap failed");
  expect(document.querySelector(".viewer-shell")).not.toBeNull();
  expect(core.state.lastError).toContain("session bootstrap failed");
  expect(document.getElementById("pixel-world-embedded-runtime-canvas")).toBeNull();
  expect(runtimeMock.mountCalls).toBe(0);
  expect(installFixture).not.toHaveBeenCalled();
});
