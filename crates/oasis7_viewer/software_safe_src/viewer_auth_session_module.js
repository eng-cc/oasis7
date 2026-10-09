import { unwrap } from "solid-js/store";
import { importSigningIdentity, setSigningIdentityLookup } from "./viewer_auth_crypto.js";

const sessions = new WeakMap();
const generations = new WeakMap();
const credentialFields = ["privateKey", "releaseToken", "registrationGrant"];
export function authCredentials(auth) {
  const held = sessions.get(unwrap(auth));
  return held ? Object.freeze({ privateKey: held.privateKey, releaseToken: held.releaseToken, registrationGrant: held.grantGeneration === connectionGeneration ? held.registrationGrant : null }) : {};
}
let connectionGeneration = 0;
export function invalidateAuthConnection() { connectionGeneration += 1; }
export function authConnectionGeneration() { return connectionGeneration; }
export function authSigningIdentity(auth) { return sessions.get(unwrap(auth))?.identity || null; }
export function hasSigningIdentity(auth) { return Boolean(authSigningIdentity(auth)); }
export function clearSession(state, projection = {}) {
  if (state.auth) sessions.delete(unwrap(state.auth));
  invalidateAuthConnection();
  generations.set(state, (generations.get(state) || 0) + 1);
  state.auth = { ...projection, available: false, playerId: null, publicKey: null };
  for (const key of credentialFields) delete state.auth[key];
  return state.auth;
}
export async function installSession(state, verifiedLogin, expectedAuth = state.auth) {
  if (verifiedLogin?.source === "visual_fixture_projection") throw new Error("visual fixture projection cannot install authentication");
  const generation = generations.get(state) || 0;
  const login = { ...verifiedLogin };
  const credentials = sessions.get(unwrap(verifiedLogin)) || Object.fromEntries(credentialFields.map((key) => [key, verifiedLogin[key]]));
  const identity = credentials.privateKey && login.publicKey
    ? await importSigningIdentity(login.publicKey, credentials.privateKey) : null;
  if (state.auth !== expectedAuth || (generations.get(state) || 0) !== generation) {
    throw new Error("authentication context changed during identity installation");
  }
  if (credentials.privateKey && !identity) throw new Error("signing identity validation failed");
  const projection = login;
  for (const key of credentialFields) delete projection[key];
  Object.defineProperties(projection, {
    playerId: { value: login.playerId || null, enumerable: true, writable: false, configurable: false },
    publicKey: { value: identity?.publicKey || login.publicKey || null, enumerable: true, writable: false, configurable: false },
  });
  if (state.auth) sessions.delete(unwrap(state.auth));
  invalidateAuthConnection();
  sessions.set(projection, { identity, privateKey: credentials.privateKey || null, releaseToken: credentials.releaseToken || null, registrationGrant: credentials.registrationGrant || null, grantGeneration: connectionGeneration });
  generations.set(state, generation + 1);
  state.auth = projection;
  return state.auth;
}
export function updateRegistrationGrant(auth, grant, deviceSessionId = null) {
  const held = sessions.get(unwrap(auth));
  if (!held) throw new Error("registration grant requires an installed session");
  held.registrationGrant = grant;
  held.grantGeneration = connectionGeneration;
  if (deviceSessionId != null) auth.deviceSessionId = deviceSessionId;
}
export function captureSessionContext(state, socketGeneration, endpointId) {
  return Object.freeze({ authGeneration: generations.get(state) || 0, socketGeneration, endpointId,
    identityId: authSigningIdentity(state.auth)?.identityId || null, playerId: state.auth.playerId, publicKey: state.auth.publicKey });
}
export function isSessionContextCurrent(state, context, socketGeneration, endpointId) {
  const current = captureSessionContext(state, socketGeneration, endpointId);
  return Object.keys(current).every((key) => current[key] === context[key]);
}

setSigningIdentityLookup(authSigningIdentity, authConnectionGeneration);
