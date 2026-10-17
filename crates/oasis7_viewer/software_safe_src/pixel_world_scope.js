import { authConnectionGeneration } from "./viewer_auth_session_module.js";
// Display authorization identity only; never include bearer tokens or private keys.
export function pixelWorldScope(state) {
  const auth = state.auth || {};
  return JSON.stringify([authConnectionGeneration(), state.endpoint || state.wsUrl || null, state.worldId ?? null,
    auth.available, auth.playerId, auth.publicKey, auth.deviceSessionId,
    auth.sessionEpoch, auth.bindingEpoch, auth.authorityEpoch, auth.boundAgentId,
    auth.revokeReason]);
}
export function createPixelWorldScopeGuard(state) {
  let scope = pixelWorldScope(state), blocked = false, generation = 0;
  return {
    authorize(expectedScope, expectedGeneration) {
      if (expectedGeneration !== generation || expectedScope !== scope || pixelWorldScope(state) !== scope) return false;
      blocked = false;
      return true;
    },
    check() {
      const next = pixelWorldScope(state);
      const changed = next !== scope;
      if (changed) { scope = next; generation += 1; blocked = true; }
      return {changed, blocked, scope, generation};
    },
  };
}
