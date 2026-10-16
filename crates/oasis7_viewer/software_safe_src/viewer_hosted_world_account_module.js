import { createPasskeyAccountClient, encodePasskeyBytes } from "./viewer_passkey_account_module.js";
import { installSession, updateRegistrationGrant, clearSession, authCredentials } from "./viewer_auth_session_module.js";

export function createHostedWorldAccount({ state, generateKeypair, register, render, clearLegacy }) {
  const client = createPasskeyAccountClient();
  let pending = null;
  let active = null;
  let inFlight = null;
  let timer = null;
  let renewal = null;
  let generation=0;
  function disableLegacy() {
    clearLegacy();
    if (state.auth.source === "hosted_browser_storage") clearSession(state, { source: "guest_only", registrationStatus: "guest", runtimeStatus: "guest" });
    if (state.auth.source !== "hosted_account_cookie") { state.auth.error = null; state.auth.rebindNotice = null; render(); }
  }
  function join(account, { takeover = false, expectedEpoch } = {}) {
    if(state.auth.hostedAccountId&&state.auth.hostedAccountId!==account.account_id)invalidate();
    if(inFlight)return inFlight;
    const revision=generation;
    const task=performJoin(account,takeover,expectedEpoch,revision).finally(()=>{if(inFlight===task)inFlight=null;});
    inFlight=task;return task;
  }
  async function performJoin(account, takeover, expectedEpoch, revision) {
    const check=()=>{if(generation!==revision)throw new Error("state_changed");};
    disableLegacy();
    if(!pending||pending.accountId!==account.account_id){const keypair=await generateKeypair();check();pending={accountId:account.account_id,attempt:encodePasskeyBytes(crypto.getRandomValues(new Uint8Array(32))),keypair};}
    const candidate = pending;
    await client.capabilities();check();
    const payload = await client.joinWorld({ public_key: candidate.keypair.publicKey, join_attempt_id: candidate.attempt, takeover, expected_epoch: expectedEpoch });
    check();
    if (pending !== candidate) throw new Error("state_changed");
    const grant = payload.grant;
    if (!grant?.registration_grant || !grant.player_id) throw new Error("world_admission_unavailable");
    if(payload.account&&payload.account.account_id!==account.account_id)throw new Error("state_changed");
    const previous = state.auth;
    const previousLogin = {...previous,...authCredentials(previous)};
    const auth = await installSession(state, {
      ...previous, available: true, hostedAccountId: account.account_id, playerId: grant.player_id,
      ...candidate.keypair, registrationGrant: grant.registration_grant, releaseToken: null,
      deviceSessionId: grant.device_session_id, source: "hosted_account_cookie", registrationStatus: "issued",
      runtimeStatus: "issued", sessionEpoch: grant.world_session_epoch, error: null,
    });
    check();
    try {
      await register(null, { forceRebind: false });
      check();
      if (state.auth !== auth) throw new Error("state_changed");
      active = { auth, publicKey: candidate.keypair.publicKey, deviceSessionId: grant.device_session_id, epoch: grant.world_session_epoch };
      pending = null;
      startRenewal();
      render();
      return payload;
    } catch (error) {
      if(generation!==revision)throw error;
      try {
        await client.capabilities();
        const status=await client.refreshWorld({public_key:candidate.keypair.publicKey,device_session_id:grant.device_session_id,expected_epoch:grant.world_session_epoch});
        check();
        if(status.world?.state!=="bound"&&previousLogin.available&&previousLogin.privateKey&&state.auth===auth){const restored=await installSession(state,previousLogin,auth);if(active?.auth===previous){active.auth=restored;startRenewal();}}
      }catch(_){ /* Preserve the candidate until its original outcome can be checked. */ }
      check();state.auth.error=String(error);render();throw error;
    }
  }
  function startRenewal() {
    if (timer) clearInterval(timer);
    timer = setInterval(() => { void renew().catch(error => { if (active?.auth === state.auth) { state.auth.error = String(error); render(); } }); }, 10_000);
  }
  function renew() {
    if (renewal) return renewal;
    renewal=performRenew().finally(()=>{renewal=null;});return renewal;
  }
  async function performRenew() {
    const current = active;
    if (!current || state.auth !== current.auth) return null;
    await client.capabilities();
    const result = await client.refreshWorld({ public_key: current.publicKey, device_session_id: current.deviceSessionId, expected_epoch: current.epoch });
    if (active !== current || state.auth !== current.auth) return null;
    updateRegistrationGrant(current.auth, result.grant.registration_grant, result.grant.device_session_id);
    await register(null, { forceRebind: false, renewAuthorization: true });
    return result;
  }
  async function release() {
    const revision=++generation;inFlight=null;
    const current = active;
    if (timer) clearInterval(timer); timer = null;
    if (current) { await client.capabilities(); await client.releaseWorld({ public_key: current.publicKey, device_session_id: current.deviceSessionId, expected_epoch: current.epoch }); }
    if(generation!==revision)return;
    active = pending = null;
    clearSession(state, { source: "guest_only", registrationStatus: "guest", runtimeStatus: "guest" }); render();
  }
  function invalidate() {
    generation++;inFlight=null;
    if (timer) clearInterval(timer);timer=null;active=pending=null;
    clearSession(state,{source:"guest_only",registrationStatus:"guest",runtimeStatus:"guest"});render();
  }
  return { join, renew, release, disableLegacy, invalidate };
}
