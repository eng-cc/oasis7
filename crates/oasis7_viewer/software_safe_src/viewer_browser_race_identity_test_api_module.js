import { createViewerBrowserRaceHandoffModule } from "./viewer_browser_race_handoff_module.js";

export function createViewerBrowserRaceIdentityTestApi({
  authHasSigningKeyMaterial,
  bumpRequestCounters,
  clone,
  connect,
  isTestApiEnabled,
  render,
  state,
  viewerBrowserRaceHandoffModule = createViewerBrowserRaceHandoffModule(),
} = {}) {
  let browserRaceIdentityOffer = null;

  function requireBrowserRaceHandoff() {
    if (!isTestApiEnabled?.() || !viewerBrowserRaceHandoffModule.enabled) {
      throw new Error("browser race identity handoff requires loopback test_api=1&hosted_test_login=1");
    }
  }

  function offerBrowserRaceIdentityForTest() {
    requireBrowserRaceHandoff();
    if (!authHasSigningKeyMaterial?.(state.auth) || state.auth.source !== "hosted_test_login") {
      throw new Error("browser race identity offer requires an active hosted test-login signing identity");
    }
    browserRaceIdentityOffer?.dispose?.();
    browserRaceIdentityOffer = viewerBrowserRaceHandoffModule.offerKeyMaterial({
      publicKey: state.auth.publicKey,
      privateKey: state.auth.privateKey,
      releaseToken: state.auth.releaseToken,
      playerId: state.auth.playerId,
      sessionEpoch: state.auth.sessionEpoch,
      bindingEpoch: state.auth.bindingEpoch,
      boundAgentId: state.auth.boundAgentId,
      authorityEpoch: state.auth.authorityEpoch,
    });
    return clone(browserRaceIdentityOffer.descriptor);
  }

  async function claimBrowserRaceIdentityForTest(descriptor) {
    requireBrowserRaceHandoff();
    if (!state.auth?.available || state.auth.source !== "hosted_browser_storage") {
      throw new Error("browser race identity claim requires the stored hosted test-login session");
    }
    const keyMaterial = await viewerBrowserRaceHandoffModule.claimOffer(descriptor);
    const claimedPlayerId = String(keyMaterial.playerId || "").trim();
    const currentPlayerId = String(state.auth.playerId || "").trim();
    if (!claimedPlayerId || !currentPlayerId || claimedPlayerId !== currentPlayerId) {
      throw new Error("browser race identity claim player binding mismatch");
    }
    state.auth.publicKey = keyMaterial.publicKey;
    state.auth.privateKey = keyMaterial.privateKey;
    state.auth.releaseToken = keyMaterial.releaseToken;
    state.auth.sessionEpoch = keyMaterial.sessionEpoch;
    state.auth.bindingEpoch = keyMaterial.bindingEpoch;
    state.auth.boundAgentId = keyMaterial.boundAgentId;
    state.auth.authorityEpoch = keyMaterial.authorityEpoch;
    state.auth.source = "hosted_test_login";
    state.auth.loginChannel = "test";
    state.auth.registrationStatus = "issued";
    state.auth.runtimeStatus = "issued";
    state.auth.syncInFlight = false;
    state.auth.error = null;
    bumpRequestCounters?.();
    render();
    return {
      ok: true,
      playerId: state.auth.playerId,
      source: state.auth.source,
    };
  }

  function connectBrowserRaceActorForTest() {
    requireBrowserRaceHandoff();
    if (!authHasSigningKeyMaterial?.(state.auth)) {
      throw new Error("browser race actor connect requires claimed signing key material");
    }
    connect();
    return { ok: true };
  }

  return {
    claimBrowserRaceIdentityForTest,
    connectBrowserRaceActorForTest,
    offerBrowserRaceIdentityForTest,
  };
}
