const DIRECTOR_CAPABILITY_ENDPOINT = "/api/public/director/capability";
const DIRECTOR_CAPABILITY_MAX_CLOCK_SKEW_MS = 30_000;
const DIRECTOR_CAPABILITY_MAX_TTL_MS = 60_000;
const DIRECTOR_GRANT_VERSION = 1;
const DIRECTOR_GRANT_ACTION = "director_open";
const DIRECTOR_GRANT_AUDIENCE = "viewer_director";
const DIRECTOR_GRANT_SCOPE = "diagnostics_read";
const DIRECTOR_GRANT_SIGNATURE_PREFIX = "awdirectorgrant:v1:";

const SAFE_REASONS = Object.freeze({
  denied: "not_authorized",
  unavailable: "unavailable",
  invalid: "invalid_server_response",
  expired: "expired",
  revoked: "revoked",
  reconnect: "reconnect_required",
  exited: "player_exit",
});

function finiteUnixMs(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) && numeric > 0 ? numeric : null;
}

function safeErrorCode(payload) {
  const value = String(payload?.error_code || "").trim().toLowerCase();
  return value === "director_not_authorized" || value === "director_denied"
    ? SAFE_REASONS.denied
    : SAFE_REASONS.unavailable;
}

function clonePublicCapability(capability) {
  if (!capability) {
    return null;
  }
  return {
    name: "director",
    visibility: "dense",
    issuer: capability.issuer,
    issuedAtUnixMs: capability.issuedAtUnixMs,
    expiresAtUnixMs: capability.expiresAtUnixMs,
  };
}

export function validateDirectorCapabilityResponse(payload, now = Date.now()) {
  if (!payload || typeof payload !== "object" || payload.ok !== true || payload.server_validated !== true) {
    return { ok: false, reason: safeErrorCode(payload) };
  }
  const raw = payload.grant;
  if (!raw || typeof raw !== "object"
    || raw.version !== DIRECTOR_GRANT_VERSION
    || raw.action !== DIRECTOR_GRANT_ACTION
    || raw.audience !== DIRECTOR_GRANT_AUDIENCE
    || raw.scope !== DIRECTOR_GRANT_SCOPE) {
    return { ok: false, reason: SAFE_REASONS.invalid };
  }
  const issuer = String(raw.server || "").trim();
  const token = String(raw.signature || "").trim();
  const playerId = String(raw.player_id || "").trim();
  const playerPublicKey = String(raw.player_public_key || "").trim();
  const nonce = String(raw.nonce || "").trim();
  const signerPublicKey = String(raw.signer_public_key || "").trim();
  const issuedAtUnixMs = finiteUnixMs(raw.issued_at_unix_ms);
  const expiresAtUnixMs = finiteUnixMs(raw.expires_at_unix_ms);
  const currentUnixMs = finiteUnixMs(now) || Date.now();
  const sessionEpoch = Number(raw.session_epoch);
  if (!issuer || !token.startsWith(DIRECTOR_GRANT_SIGNATURE_PREFIX) || !playerId
    || !playerPublicKey || !nonce || !signerPublicKey || !Number.isSafeInteger(sessionEpoch)
    || sessionEpoch <= 0 || !issuedAtUnixMs || !expiresAtUnixMs) {
    return { ok: false, reason: SAFE_REASONS.invalid };
  }
  if (issuedAtUnixMs > currentUnixMs + DIRECTOR_CAPABILITY_MAX_CLOCK_SKEW_MS || expiresAtUnixMs <= currentUnixMs) {
    return { ok: false, reason: expiresAtUnixMs <= currentUnixMs ? SAFE_REASONS.expired : SAFE_REASONS.invalid };
  }
  if (expiresAtUnixMs <= issuedAtUnixMs || expiresAtUnixMs - issuedAtUnixMs > DIRECTOR_CAPABILITY_MAX_TTL_MS) {
    return { ok: false, reason: SAFE_REASONS.invalid };
  }
  return {
    ok: true,
    capability: {
      ...clonePublicCapability({ issuer, issuedAtUnixMs, expiresAtUnixMs }),
      token,
    },
  };
}

export function createDirectorCapabilityApiAdapter({
  fetchImpl = globalThis.fetch,
  endpoint = DIRECTOR_CAPABILITY_ENDPOINT,
} = {}) {
  return {
    async requestCapability(context = {}) {
      if (typeof fetchImpl !== "function") {
        return { ok: false, error_code: "director_capability_unavailable" };
      }
      try {
        const response = await fetchImpl(endpoint, {
          method: "GET",
          cache: "no-store",
          headers: { Accept: "application/json" },
        });
        const payload = await response.json();
        if (!response.ok) {
          return { ok: false, error_code: response.status === 401 || response.status === 403 ? "director_not_authorized" : "director_capability_unavailable" };
        }
        return payload && typeof payload === "object"
          ? payload
          : { ok: false, error_code: "director_capability_unavailable" };
      } catch (_) {
        return { ok: false, error_code: "director_capability_unavailable" };
      }
    },
  };
}

function initialState() {
  return { mode: "player", status: "idle", capability: null, reason: null };
}

export function createDirectorCapabilityController({ adapter, now = Date.now, onChange = () => {} } = {}) {
  const capabilityAdapter = adapter || createDirectorCapabilityApiAdapter();
  let current = initialState();
  let sequence = 0;
  let expiryTimer = null;
  const listeners = new Set();

  function clearExpiryTimer() {
    if (expiryTimer != null && typeof window !== "undefined" && typeof window.clearTimeout === "function") {
      window.clearTimeout(expiryTimer);
    }
    expiryTimer = null;
  }

  function publish(next) {
    current = next;
    onChange(current);
    listeners.forEach((listener) => listener(current));
    return current;
  }

  function downgrade(status, reason = status) {
    sequence += 1;
    clearExpiryTimer();
    return publish({ mode: "player", status, capability: null, reason });
  }

  function scheduleExpiry(capability, requestSequence) {
    clearExpiryTimer();
    const delay = Math.max(0, capability.expiresAtUnixMs - Number(now()));
    if (typeof window === "undefined" || typeof window.setTimeout !== "function") {
      return;
    }
    expiryTimer = window.setTimeout(() => {
      if (requestSequence === sequence && current.mode === "director") {
        downgrade("expired", SAFE_REASONS.expired);
      }
    }, delay);
  }

  async function request(context = {}) {
    if (current.status === "pending") {
      return current;
    }
    const requestSequence = ++sequence;
    clearExpiryTimer();
    publish({ mode: "player", status: "pending", capability: null, reason: null });
    try {
      const response = await capabilityAdapter.requestCapability(context);
      if (requestSequence !== sequence) {
        return current;
      }
      const validated = validateDirectorCapabilityResponse(response, Number(now()));
      if (!validated.ok) {
        return publish({ mode: "player", status: validated.reason === SAFE_REASONS.denied ? "denied" : "unavailable", capability: null, reason: validated.reason });
      }
      const capability = validated.capability;
      const next = publish({ mode: "director", status: "active", capability: clonePublicCapability(capability), reason: null });
      scheduleExpiry(capability, requestSequence);
      return next;
    } catch (_) {
      if (requestSequence !== sequence) {
        return current;
      }
      return publish({ mode: "player", status: "unavailable", capability: null, reason: SAFE_REASONS.unavailable });
    }
  }

  function observeRuntime(runtime = {}) {
    if (current.mode !== "director") {
      return current;
    }
    const disconnected = ["disconnected", "error"].includes(String(runtime.connectionStatus || "").trim().toLowerCase());
    const unavailableAuth = runtime.authAvailable === false;
    const runtimeStatus = String(runtime.authRuntimeStatus || "").trim().toLowerCase();
    if (runtime.authRevokeReason || runtime.authRevokedBy || ["revoked", "disconnected", "error"].includes(runtimeStatus)) {
      return downgrade("revoked", SAFE_REASONS.revoked);
    }
    if (disconnected || unavailableAuth || runtimeStatus === "probing") {
      return downgrade("reconnect_required", SAFE_REASONS.reconnect);
    }
    return current;
  }

  function exit() {
    return downgrade("exited", SAFE_REASONS.exited);
  }

  return {
    getState: () => ({
      mode: current.mode,
      status: current.status,
      capability: clonePublicCapability(current.capability),
      reason: current.reason,
    }),
    request,
    observeRuntime,
    exit,
    subscribe(listener) {
      if (typeof listener !== "function") return () => {};
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export { DIRECTOR_CAPABILITY_ENDPOINT };
