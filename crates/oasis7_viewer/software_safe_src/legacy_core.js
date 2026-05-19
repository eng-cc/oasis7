const TEST_API_GLOBAL_NAME = "__AW_TEST__";
const RENDER_META_GLOBAL_NAME = "__AW_VIEWER_RENDER_META__";
const VIEWER_RENDER_MODE = "viewer";
const SOFTWARE_SAFE_RENDER_MODE_ALIAS = "software_safe";
const VIEWER_AUTH_BOOTSTRAP_OBJECT = "__OASIS7_VIEWER_AUTH_ENV";
const VIEWER_PLAYER_ID_KEY = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
const VIEWER_AUTH_SIGNATURE_PREFIX = "awviewauth:v1:";
const HOSTED_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.hosted_player_session.v1";
const UI_LOCALE_STORAGE_PREFIX = "oasis7.viewer.locale.v1";
const PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX = "oasis7.viewer.prompt_overrides_visible.v1";
const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE = "/api/public/player-session/admission";
const HOSTED_PLAYER_SESSION_REFRESH_ROUTE = "/api/public/player-session/refresh";
const HOSTED_PLAYER_SESSION_ISSUE_ROUTE = "/api/public/player-session/issue";
const HOSTED_PLAYER_SESSION_RELEASE_ROUTE = "/api/public/player-session/release";
const HOSTED_ACCOUNT_LOGIN_START_ROUTE = "/api/public/hosted-account/login/start";
const HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE = "/api/public/hosted-account/login/complete";
const HOSTED_STRONG_AUTH_GRANT_ROUTE = "/api/public/strong-auth/grant";
const HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS = 30000;
const DEFAULT_WS_ADDR = "ws://127.0.0.1:5011";
const MAX_EVENTS = 24;
const MAX_DECISION_TRACES = 12;
const SOFTWARE_RENDERER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "software rasterizer",
  "basic render driver",
  "softpipe",
  "lavapipe",
];
const ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
const textEncoder = new TextEncoder();

export const state = {
  uiLocale: "en",
  promptOverridesVisible: false,
  connectionStatus: "connecting",
  logicalTime: 0,
  eventSeq: 0,
  tick: 0,
  selectedKind: null,
  selectedId: null,
  errorCount: 0,
  lastError: null,
  eventCount: 0,
  traceCount: 0,
  cameraMode: "viewer",
  cameraRadius: 0,
  cameraOrthoScale: 0,
  renderMode: VIEWER_RENDER_MODE,
  rendererClass: "none",
  viewerReason: null,
  renderer: null,
  vendor: null,
  webglVersion: null,
  pixelWorldRuntimeStatus: "detached",
  pixelWorldRuntimeSource: "detached",
  pixelWorldRuntimeModuleUrl: null,
  pixelWorldCamera: null,
  pixelWorldFatal: null,
  controlProfile: "playback",
  debugViewerMode: "debug_viewer",
  debugViewerStatus: "detached",
  worldId: null,
  server: null,
  wsUrl: null,
  lastControlFeedback: null,
  lastPromptFeedback: null,
  lastChatFeedback: null,
  lastGameplayActionFeedback: null,
  snapshot: null,
  metrics: null,
  hostedAccess: null,
  hostedAdmission: null,
  recentEvents: [],
  recentDecisionTraces: [],
  chatHistory: [],
  selectedObject: null,
  auth: {
    available: false,
    hostedAccountId: null,
    playerId: null,
    loginChannel: null,
    maskedLoginHint: null,
    deviceSessionId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "guest_only",
    registrationStatus: "guest",
    sessionEpoch: null,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null,
  },
  hostedLogin: {
    channel: "email",
    handle: "",
    challengeId: null,
    maskedLoginHint: null,
    deliveryMode: null,
    previewCode: null,
    code: "",
    expiresAtUnixMs: null,
    accountExists: false,
    startInFlight: false,
    completeInFlight: false,
    error: null,
  },
  promptDraft: {
    agentId: null,
    currentVersion: 0,
    rollbackTargetVersion: 0,
    updatedBy: "",
    updatedAtTick: 0,
    systemPrompt: "",
    shortTermGoal: "",
    longTermGoal: "",
    dirty: false,
  },
  chatDraft: {
    agentId: null,
    message: "",
    dirty: false,
  },
  strongAuth: {
    approvalCode: "",
    lastGrantActionId: null,
    lastGrantExpiresAtUnixMs: null,
    lastGrantError: null,
  },
};

let socket = null;
let reconnectTimer = null;
let hostedSessionRefreshTimer = null;
let requestId = 0;
let authNonceCounter = 0;
let selectedSearch = "";
let semanticSendLoop = null;
const pendingControlFeedback = new Map();
const pendingSemanticCommands = [];
const authKeyCache = new Map();
let pendingSessionRegisterWaiter = null;

const elements = {};
let renderHook = () => {};
let bootstrapped = false;

function normalizeUiLocale(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (["zh", "zh-cn", "zh_cn", "cn", "chinese"].includes(value)) {
    return "zh";
  }
  if (["en", "en-us", "en_us", "english"].includes(value)) {
    return "en";
  }
  return null;
}

export function isLocaleZh(locale = state.uiLocale) {
  return normalizeUiLocale(locale) === "zh";
}

function uiLocaleStorageKey() {
  return `${UI_LOCALE_STORAGE_PREFIX}:${window.location.pathname || "viewer.html"}`;
}

function persistUiLocale(locale) {
  try {
    window.localStorage?.setItem(uiLocaleStorageKey(), locale);
  } catch (_) {
  }
}

function resolveStoredUiLocale() {
  try {
    return normalizeUiLocale(window.localStorage?.getItem(uiLocaleStorageKey()));
  } catch (_) {
    return null;
  }
}

function resolveInitialUiLocale() {
  const params = getSearchParams();
  return normalizeUiLocale(params.get("locale") || params.get("language"))
    || resolveStoredUiLocale()
    || "en";
}

function localeText(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}

function promptOverridesVisibilityStorageKey() {
  return `${PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX}:${window.location.pathname || "viewer.html"}`;
}

function persistPromptOverridesVisibility(visible) {
  try {
    window.localStorage?.setItem(promptOverridesVisibilityStorageKey(), visible ? "1" : "0");
  } catch (_) {
  }
}

function resolveStoredPromptOverridesVisibility() {
  try {
    const raw = window.localStorage?.getItem(promptOverridesVisibilityStorageKey());
    return raw === "1";
  } catch (_) {
    return false;
  }
}

function applyUiLocaleToDocument(locale) {
  document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
}

function updateUiLocaleQuery(locale) {
  const url = new URL(window.location.href);
  url.searchParams.set("locale", locale);
  url.searchParams.delete("language");
  window.history.replaceState({}, "", url.toString());
}

export function setViewerLocale(locale) {
  const normalized = normalizeUiLocale(locale);
  if (!normalized) {
    return state.uiLocale;
  }
  state.uiLocale = normalized;
  persistUiLocale(normalized);
  applyUiLocaleToDocument(normalized);
  updateUiLocaleQuery(normalized);
  render();
  return state.uiLocale;
}

export function toggleViewerLocale() {
  return setViewerLocale(state.uiLocale === "zh" ? "en" : "zh");
}

export const setSoftwareSafeLocale = setViewerLocale;
export const toggleSoftwareSafeLocale = toggleViewerLocale;

export function setPromptOverridesVisible(visible) {
  state.promptOverridesVisible = !!visible;
  persistPromptOverridesVisibility(state.promptOverridesVisible);
  render();
  return state.promptOverridesVisible;
}

export function togglePromptOverridesVisible() {
  return setPromptOverridesVisible(!state.promptOverridesVisible);
}

export function getSelectedSearch() {
  return selectedSearch;
}

export function setSelectedSearch(value) {
  selectedSearch = String(value || "");
  render();
}

export function setRenderHook(nextHook) {
  renderHook = typeof nextHook === "function" ? nextHook : () => {};
}

function getSearchParams() {
  return new URLSearchParams(window.location.search || "");
}

function isTestApiEnabled() {
  const value = String(getSearchParams().get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}

function normalizeWsAddr(raw) {
  const value = String(raw || "").trim();
  if (!value) return DEFAULT_WS_ADDR;
  if (value.startsWith("ws://") || value.startsWith("wss://")) return value;
  if (value.startsWith("http://")) return `ws://${value.slice("http://".length)}`;
  if (value.startsWith("https://")) return `wss://${value.slice("https://".length)}`;
  return `ws://${value}`;
}

function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}

function normalizeU64Display(value) {
  if (value == null) {
    return null;
  }
  if (typeof value === "bigint") {
    return value >= 0n ? value.toString() : `invalid_u64(${value.toString()})`;
  }
  if (typeof value === "number") {
    if (Number.isSafeInteger(value) && value >= 0) {
      return String(value);
    }
    return `unsafe_u64(${String(value)})`;
  }
  const text = String(value).trim();
  if (!text) {
    return null;
  }
  return /^\d+$/.test(text) ? text : `invalid_u64(${text})`;
}

function normalizeFiniteNumber(value) {
  if (value == null) {
    return null;
  }
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function finitePositionComponents(pos) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = normalizeFiniteNumber(pos.x_cm);
  const y = normalizeFiniteNumber(pos.y_cm);
  const z = normalizeFiniteNumber(pos.z_cm);
  if (x == null || y == null || z == null) {
    return null;
  }
  return { x, y, z };
}

function trimFixed(value, digits) {
  if (!Number.isFinite(value)) {
    return null;
  }
  const fixed = value.toFixed(digits);
  return fixed.replace(/\.0+$/, "").replace(/(\.\d*[1-9])0+$/, "$1");
}

function formatPhysicalDistanceCm(value, locale = state.uiLocale) {
  const numeric = normalizeFiniteNumber(value);
  if (numeric == null) {
    return null;
  }
  const absolute = Math.abs(numeric);
  if (absolute >= 100_000) {
    const km = numeric / 100_000;
    const label = trimFixed(km, Math.abs(km) >= 100 ? 0 : Math.abs(km) >= 10 ? 1 : 2);
    return `${label} km`;
  }
  if (absolute >= 100) {
    const meters = numeric / 100;
    const label = trimFixed(
      meters,
      Math.abs(meters) >= 100 ? 0 : Math.abs(meters) >= 10 ? 1 : 2,
    );
    return `${label} m`;
  }
  return `${trimFixed(numeric, 0)} cm`;
}

function formatWorldPositionCm(pos, locale = state.uiLocale) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = formatPhysicalDistanceCm(pos.x_cm, locale);
  const y = formatPhysicalDistanceCm(pos.y_cm, locale);
  const z = formatPhysicalDistanceCm(pos.z_cm, locale);
  if (!x || !y || !z) {
    return null;
  }
  return `x=${x} · y=${y} · z=${z}`;
}

function distanceCmBetweenPositions(a, b) {
  const left = finitePositionComponents(a);
  const right = finitePositionComponents(b);
  if (!left || !right) {
    return null;
  }
  const dx = left.x - right.x;
  const dy = left.y - right.y;
  const dz = left.z - right.z;
  return Math.max(0, Math.round(Math.sqrt((dx * dx) + (dy * dy) + (dz * dz))));
}

function locationRadiusCm(location) {
  return normalizeFiniteNumber(location?.profile?.radius_cm);
}

function snapshotSpaceConfig() {
  const space = state.snapshot?.config?.space;
  return space && typeof space === "object" ? space : null;
}

function selectedWorldAnchor() {
  const selected = state.selectedObject;
  if (selected && selected.pos) {
    return {
      kind: state.selectedKind || "location",
      id: state.selectedId || selected.id || selected.name || "selected",
      pos: selected.pos,
      radiusCm: locationRadiusCm(selected),
      locationId: selected.location_id || selected.id || null,
    };
  }

  const locations = Object.values(state.snapshot?.model?.locations || {});
  const fallback = locations.find((location) => location?.pos);
  if (!fallback) {
    return null;
  }
  return {
    kind: "location",
    id: fallback.id || fallback.name || "location",
    pos: fallback.pos,
    radiusCm: locationRadiusCm(fallback),
    locationId: fallback.id || null,
  };
}

function buildWorldScaleSurface(locale = state.uiLocale) {
  const isZh = isLocaleZh(locale);
  const space = snapshotSpaceConfig();
  const anchor = selectedWorldAnchor();
  const locations = Object.values(state.snapshot?.model?.locations || {})
    .filter((location) => location?.id && location?.pos);

  const nearestLocations = anchor
    ? locations
      .filter((location) => location.id !== anchor.locationId)
      .map((location) => {
        const distanceCm = distanceCmBetweenPositions(anchor.pos, location.pos);
        return {
          id: location.id,
          name: location.name || location.id,
          distanceCm,
          distanceLabel: formatPhysicalDistanceCm(distanceCm, locale),
          radiusCm: locationRadiusCm(location),
          radiusLabel: formatPhysicalDistanceCm(locationRadiusCm(location), locale),
        };
      })
      .filter((location) => location.distanceCm != null)
      .sort((left, right) => left.distanceCm - right.distanceCm)
      .slice(0, 3)
    : [];

  const physicalTruth = {
    canonicalUnitLabel: formatPhysicalDistanceCm(1, locale),
    canonicalUnitDetail: isZh
      ? "世界位置、距离、半径和尺寸的正式真值都按整数厘米存储。"
      : "World positions, distances, radii, and sizes are stored as integer centimeters.",
    worldBoundsLabel: space
      ? `${formatPhysicalDistanceCm(space.width_cm, locale)} × ${formatPhysicalDistanceCm(space.depth_cm, locale)} × ${formatPhysicalDistanceCm(space.height_cm, locale)}`
      : null,
    worldBoundsDetail: space
      ? isZh
        ? "来自 snapshot.config.space 的真实世界边界。"
        : "Physical world bounds derived from snapshot.config.space."
      : isZh
        ? "当前快照没有发布 world bounds。"
        : "The current snapshot does not publish world bounds yet.",
    anchor: anchor
      ? {
          kind: anchor.kind,
          id: anchor.id,
          label: anchor.kind === "agent"
            ? (isZh ? "当前选中 Agent 锚点" : "Selected agent anchor")
            : (isZh ? "当前选中地点锚点" : "Selected location anchor"),
          positionLabel: formatWorldPositionCm(anchor.pos, locale),
          radiusCm: anchor.radiusCm,
          radiusLabel: anchor.radiusCm == null ? null : formatPhysicalDistanceCm(anchor.radiusCm, locale),
          locationId: anchor.locationId,
        }
      : null,
    nearestLocations,
  };

  const presentationScale = {
    markerTruthNote: isZh
      ? "3D marker、2D overview map 和 halo 允许为了可读性被放大；请把距离/半径标签当成真值，不要把屏幕上的直径当成真实几何尺寸。"
      : "3D markers, the 2D overview map, and halos may be enlarged for readability. Treat the distance/radius labels as truth; do not read on-screen diameter as real geometry size.",
    zoomTruthNote: isZh
      ? "overview/detail 的 zoom tier 只切换表现语义，不会改写世界的厘米真值。"
      : "Overview/detail zoom tiers only switch presentation semantics; they do not rewrite centimeter truth in the world model.",
    softwareSafeNote: isZh
      ? "viewer 主入口优先给出文字和数值真值；更底层的 visual QA viewer 可以更夸张，但不应覆盖这里的物理标签。"
      : "The viewer entry prioritizes textual and numeric truth. Lower-level visual QA surfaces may exaggerate more aggressively, but they should not override the physical labels here.",
  };

  return {
    physicalTruth,
    presentationScale,
  };
}

function detectRendererMeta() {
  const params = getSearchParams();
  const reasonFromQuery = params.get("viewer_reason") || params.get("software_safe_reason");
  const requestedRenderMode = String(params.get("render_mode") || "").trim().toLowerCase();
  const meta = {
    renderMode:
      requestedRenderMode === SOFTWARE_SAFE_RENDER_MODE_ALIAS || requestedRenderMode === VIEWER_RENDER_MODE
        ? VIEWER_RENDER_MODE
        : VIEWER_RENDER_MODE,
    rendererClass: "none",
    viewerReason: reasonFromQuery || "direct_viewer_entry",
    renderer: null,
    vendor: null,
    webglVersion: null,
  };

  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (!gl) {
      meta.rendererClass = "none";
      meta.viewerReason = reasonFromQuery || "webgl_unavailable";
      return meta;
    }
    meta.webglVersion = gl.getParameter(gl.VERSION) || null;
    const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
    if (debugInfo) {
      meta.renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || null;
      meta.vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) || null;
    }
    const rendererText = String(meta.renderer || "").toLowerCase();
    if (SOFTWARE_RENDERER_MARKERS.some((marker) => rendererText.includes(marker))) {
      meta.rendererClass = "software";
    } else {
      meta.rendererClass = "unknown";
    }
  } catch (error) {
    meta.rendererClass = "none";
    meta.renderer = String(error);
  }
  return meta;
}

function resolveAuthBootstrap() {
  const raw = window[VIEWER_AUTH_BOOTSTRAP_OBJECT];
  if (!raw || typeof raw !== "object") {
    return {
      available: false,
      hostedAccountId: null,
      playerId: null,
      loginChannel: null,
      maskedLoginHint: null,
      deviceSessionId: null,
      publicKey: null,
      privateKey: null,
      releaseToken: null,
      error: "viewer auth bootstrap is unavailable",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null,
    };
  }
  const playerId = String(raw[VIEWER_PLAYER_ID_KEY] || "").trim();
  const publicKey = String(raw[VIEWER_AUTH_PUBLIC_KEY] || "")
    .trim()
    .toLowerCase();
  const privateKey = String(raw[VIEWER_AUTH_PRIVATE_KEY] || "")
    .trim()
    .toLowerCase();
  if (!playerId || !publicKey || !privateKey) {
    return {
      available: false,
      hostedAccountId: null,
      playerId: playerId || null,
      loginChannel: null,
      maskedLoginHint: null,
      deviceSessionId: null,
      publicKey: publicKey || null,
      privateKey: privateKey || null,
      releaseToken: null,
      error: "viewer auth bootstrap is incomplete",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null,
    };
  }
  return {
    available: true,
    hostedAccountId: null,
    playerId,
    loginChannel: null,
    maskedLoginHint: null,
    deviceSessionId: null,
    publicKey,
    privateKey,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "legacy_viewer_auth_bootstrap",
    registrationStatus: "registered",
    sessionEpoch: 1,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "legacy_preview",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null,
  };
}

function initialWsUrl() {
  const params = getSearchParams();
  return normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
}

function shouldConnectViewerWs() {
  const mode = String(getSearchParams().get("connect") || "").trim().toLowerCase();
  return mode !== "0" && mode !== "false" && mode !== "off";
}

function hostedPlayerSessionStorageKey() {
  return `${HOSTED_PLAYER_SESSION_STORAGE_PREFIX}:${initialWsUrl()}`;
}

function persistHostedPlayerSession(auth) {
  if (!auth?.available || !auth?.playerId || auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  try {
    window.localStorage?.setItem(
      hostedPlayerSessionStorageKey(),
      JSON.stringify({
        hostedAccountId: auth.hostedAccountId || null,
        playerId: auth.playerId,
        loginChannel: auth.loginChannel || null,
        maskedLoginHint: auth.maskedLoginHint || null,
        deviceSessionId: auth.deviceSessionId || auth.releaseToken || null,
        releaseToken: auth.releaseToken || null,
        issuedAtUnixMs: auth.issuedAtUnixMs || null,
        sessionEpoch: auth.sessionEpoch || null,
      }),
    );
  } catch (_) {
  }
}

function clearHostedPlayerSession() {
  try {
    window.localStorage?.removeItem(hostedPlayerSessionStorageKey());
  } catch (_) {
  }
}

function resolveStoredHostedPlayerSession() {
  try {
    const raw = window.localStorage?.getItem(hostedPlayerSessionStorageKey());
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    const hostedAccountId = String(parsed?.hostedAccountId || parsed?.hosted_account_id || "").trim();
    const playerId = String(parsed?.playerId || "").trim();
    const loginChannel = String(parsed?.loginChannel || parsed?.login_channel || "").trim();
    const maskedLoginHint = String(parsed?.maskedLoginHint || parsed?.masked_login_hint || "").trim();
    const releaseToken = String(parsed?.releaseToken || "").trim();
    const deviceSessionId = String(parsed?.deviceSessionId || parsed?.device_session_id || parsed?.releaseToken || "").trim();
    if (!playerId || !releaseToken) {
      clearHostedPlayerSession();
      return null;
    }
    window.localStorage?.setItem(
      hostedPlayerSessionStorageKey(),
      JSON.stringify({
        hostedAccountId: hostedAccountId || null,
        playerId,
        loginChannel: loginChannel || null,
        maskedLoginHint: maskedLoginHint || null,
        deviceSessionId: deviceSessionId || releaseToken,
        releaseToken,
        issuedAtUnixMs: parsed?.issuedAtUnixMs ?? null,
        sessionEpoch: parsed?.sessionEpoch ?? null,
      }),
    );
    return {
      available: true,
      hostedAccountId: hostedAccountId || null,
      playerId,
      loginChannel: loginChannel || null,
      maskedLoginHint: maskedLoginHint || null,
      deviceSessionId: deviceSessionId || releaseToken,
      publicKey: null,
      privateKey: null,
      releaseToken,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: parsed?.sessionEpoch == null ? null : Number(parsed.sessionEpoch),
      issuedAtUnixMs: parsed?.issuedAtUnixMs == null ? null : Number(parsed.issuedAtUnixMs),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null,
    };
  } catch (_) {
    clearHostedPlayerSession();
    return null;
  }
}

function resolveViewerAuthState() {
  const bootstrap = resolveAuthBootstrap();
  if (bootstrap.available) {
    return bootstrap;
  }
  return resolveStoredHostedPlayerSession() || bootstrap;
}

function resetHostedLoginChallenge() {
  state.hostedLogin.channel = "email";
  state.hostedLogin.challengeId = null;
  state.hostedLogin.maskedLoginHint = null;
  state.hostedLogin.deliveryMode = null;
  state.hostedLogin.previewCode = null;
  state.hostedLogin.code = "";
  state.hostedLogin.expiresAtUnixMs = null;
  state.hostedLogin.accountExists = false;
  state.hostedLogin.completeInFlight = false;
}

function authHasSigningKeyMaterial(auth) {
  return !!String(auth?.publicKey || "").trim() && !!String(auth?.privateKey || "").trim();
}

async function ensureHostedAuthSigningKey(auth = state.auth) {
  if (!auth?.available || auth.source === "legacy_viewer_auth_bootstrap") {
    return auth;
  }
  if (authHasSigningKeyMaterial(auth)) {
    return auth;
  }
  const keypair = await generateEphemeralEd25519Keypair();
  auth.publicKey = keypair.publicKey;
  auth.privateKey = keypair.privateKey;
  auth.registrationStatus = "issued";
  auth.runtimeStatus = "recovery_pending_key";
  auth.syncInFlight = false;
  auth.recoveryErrorCode = null;
  auth.recoveryErrorMessage = null;
  persistHostedPlayerSession(auth);
  return auth;
}

async function refreshHostedAdmissionState() {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    state.hostedAdmission = null;
    return null;
  }
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ADMISSION_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" },
    });
    const payload = await response.json();
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : null;
    return state.hostedAdmission;
  } catch (_) {
    return state.hostedAdmission;
  }
}

async function refreshHostedPlayerLease() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return null;
  }
  try {
    const response = await fetch(
      `${HOSTED_PLAYER_SESSION_REFRESH_ROUTE}?player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`,
      {
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json" },
      },
    );
    const payload = await response.json();
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
    }
    return payload;
  } catch (error) {
    state.auth.error = String(error);
    return null;
  }
}

function stopHostedSessionRefreshLoop() {
  if (hostedSessionRefreshTimer) {
    window.clearInterval(hostedSessionRefreshTimer);
    hostedSessionRefreshTimer = null;
  }
}

function syncHostedSessionRefreshLoop() {
  const shouldRun = state.connectionStatus === "connected"
    && state.auth.available
    && state.auth.source !== "legacy_viewer_auth_bootstrap"
    && state.auth.registrationStatus === "registered"
    && !!state.auth.releaseToken;
  if (!shouldRun) {
    stopHostedSessionRefreshLoop();
    return;
  }
  if (hostedSessionRefreshTimer) {
    return;
  }
  hostedSessionRefreshTimer = window.setInterval(() => {
    probeHostedRuntimeSession();
    void refreshHostedPlayerLease().then(() => render());
  }, HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS);
}

function resolveHostedAccessHint() {
  const raw = getSearchParams().get("hosted_access");
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch (_) {
    return null;
  }
}

function hostnameFromUrl(raw) {
  const value = String(raw || "").trim();
  if (!value) return null;
  try {
    return new URL(value, window.location.href).hostname || null;
  } catch (_) {
    return null;
  }
}

function isLoopbackHostname(raw) {
  const value = String(raw || "").trim().toLowerCase();
  return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]";
}

function authDeploymentHint(auth) {
  const hostedMode = String(state.hostedAccess?.deployment_mode || "").trim();
  if (hostedMode === "hosted_public_join") {
    if (auth.available && auth.source === "legacy_viewer_auth_bootstrap") {
      return "hosted_public_join_contract_with_legacy_bootstrap";
    }
    return auth.available
      ? "hosted_public_join_contract_with_browser_session"
      : "hosted_public_join_contract";
  }
  if (hostedMode === "trusted_local_only") {
    return auth.available ? "trusted_local_contract" : "trusted_local_contract_guest";
  }
  const params = getSearchParams();
  const wsHost = hostnameFromUrl(state.wsUrl || params.get("ws") || params.get("addr") || "");
  const pageHost = String(window.location.hostname || "").trim();
  const remoteOriginLikely = [pageHost, wsHost].filter(Boolean).some((host) => !isLoopbackHostname(host));
  if (auth.available) {
    return remoteOriginLikely ? "remote_origin_legacy_bootstrap" : "trusted_local_preview";
  }
  return remoteOriginLikely ? "hosted_public_join_likely" : "guest_only_or_missing_bootstrap";
}

function isHostedPublicJoinHint(deploymentHint) {
  return [
    "hosted_public_join_contract",
    "hosted_public_join_contract_with_browser_session",
    "hosted_public_join_contract_with_legacy_bootstrap",
    "hosted_public_join_likely",
  ].includes(deploymentHint);
}

function hostedActionPolicy(actionId) {
  const normalizedActionId = actionId === "prompt_control"
    ? "prompt_control_apply"
    : actionId;
  return state.hostedAccess?.action_matrix?.find((policy) => policy?.action_id === normalizedActionId) || null;
}

function guestSessionReason(auth, deploymentHint) {
  if (auth.available) {
    return auth.source === "legacy_viewer_auth_bootstrap"
      ? "guest session has already been superseded by the current preview player auth lane"
      : "guest session has already been superseded by a hosted-issued player identity";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "this browser is still guest-only; hosted public join must complete hosted account login before low-risk interaction unlocks";
  }
  return auth.error || "viewer auth bootstrap is unavailable, so the browser cannot leave guest session";
}

function playerSessionReason(auth, deploymentHint) {
  if (auth.available) {
    if (auth.source === "legacy_viewer_auth_bootstrap") {
      return "player interaction is currently unlocked through legacy viewer auth bootstrap in trusted preview mode";
    }
    if (auth.registrationStatus === "registered") {
      return "player interaction is unlocked through hosted-issued player_id + browser device session plus an in-memory browser-generated Ed25519 session key";
    }
    if (auth.registrationStatus === "registering" || auth.registrationStatus === "issued") {
      return "browser device session is ready; runtime player-session registration is still in progress";
    }
    return auth.error || "hosted player identity exists, but runtime registration still needs recovery";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "player session upgrade/login is still pending hosted account verification";
  }
  return auth.error || "viewer auth bootstrap is missing or incomplete";
}

function strongAuthReason() {
  return "strong auth remains a separate upgrade plane; viewer already supports hosted player-session issue/reconnect/release, but backend reauth stays preview-only for prompt_control and still does not unlock hosted-ready asset/governance proofs";
}

function buildStrongAuthTier(promptCapability) {
  const promptPolicy = hostedActionPolicy("prompt_control");
  if (!promptPolicy || promptPolicy.required_auth !== "strong_auth") {
    return {
      status: "separate_upgrade_plane",
      reason: strongAuthReason(),
    };
  }
  if (promptPolicy.availability === "public_player_plane_with_backend_reauth_preview") {
    if (!state.auth.available) {
      return {
        status: "upgrade_after_player_session",
        reason:
          "hosted preview backend reauth is available on this join lane after the browser acquires a player_session",
      };
    }
    if (state.auth.registrationStatus === "registered") {
      return {
        status: "preview_backend_reauth_available",
        reason:
          "hosted preview backend reauth is available after the browser device-session-backed player_session has completed runtime registration for prompt_control",
      };
    }
    return {
      status: "issued_pending_register",
      reason:
        "hosted preview backend reauth stays pending until the browser device-session-backed player_session finishes runtime registration",
    };
  }
  if (promptPolicy.availability === "trusted_local_preview_only") {
    return {
      status: state.auth.available ? "active_legacy_preview" : "trusted_local_only",
      reason:
        "trusted_local_preview keeps prompt_control on the legacy local preview lane; hosted/public strong_auth still remains outside this window",
    };
  }
  return {
    status: "blocked_until_strong_auth",
    reason: promptPolicy.reason || strongAuthReason(),
  };
}

function isStrongAuthSensitiveAction(actionId) {
  const policy = hostedActionPolicy(actionId);
  if (policy) {
    return policy.required_auth === "strong_auth";
  }
  return actionId === "prompt_control" || actionId === "main_token_transfer";
}

function buildSemanticCapability(actionId) {
  const observerOnly = selectedAgentInteractionMode() === "observer_only";
  const deploymentHint = authDeploymentHint(state.auth);
  const strongAuthSensitive = isStrongAuthSensitiveAction(actionId);
  const policy = hostedActionPolicy(actionId);
  if (observerOnly) {
    return {
      actionId,
      enabled: false,
      code: "observer_only",
      reason:
        "selected agent runs through the provider-backed loopback bridge; viewer stays observer-only for prompt/chat on this lane",
    };
  }
  if (policy) {
    if (policy.required_auth === "strong_auth") {
      const isLocalPreviewOnly = policy.availability === "trusted_local_preview_only";
      const isBackendGrantPreview = policy.availability === "public_player_plane_with_backend_reauth_preview";
      if (isLocalPreviewOnly && state.auth.available && !isHostedPublicJoinHint(deploymentHint)) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || "trusted local preview currently allows this strong-auth-marked action through preview bootstrap",
        };
      }
      if (isBackendGrantPreview && state.auth.available) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || `${actionId} is available through browser-local player auth plus backend re-authorization`,
        };
      }
      if (isBackendGrantPreview && !state.auth.available) {
        return {
          actionId,
          enabled: false,
          code: "auth_level_insufficient",
          reason: `${actionId} requires player_session before backend re-authorization can upgrade it to strong_auth`,
        };
      }
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: policy.reason || strongAuthReason(),
      };
    }
    if (!state.auth.available) {
      return {
        actionId,
        enabled: false,
        code: "auth_level_insufficient",
        reason: `${actionId} requires ${policy.required_auth}; current browser remains guest_session only`,
      };
    }
    return {
      actionId,
      enabled: true,
      code: null,
      reason: policy.reason || `${actionId} is allowed on the ${policy.required_auth} lane`,
    };
  }
  if (strongAuthSensitive && isHostedPublicJoinHint(deploymentHint)) {
    const hostedStrongAuthReason = state.auth.available
      ? `${actionId} still requires strong_auth on the hosted public join path; this browser only has a legacy preview player_session, so backend re-authorization or a private operator plane must take over`
      : `${actionId} requires strong_auth on the hosted public join path; acquire a player_session first, then complete the hosted re-authorization step for this action`;
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: hostedStrongAuthReason,
    };
  }
  if (strongAuthSensitive && state.auth.available && deploymentHint === "remote_origin_legacy_bootstrap") {
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: `${actionId} is blocked on remote-origin legacy bootstrap; hosted/public prompt control must move to strong_auth or private operator plane`,
    };
  }
  if (!state.auth.available) {
    const reason = isHostedPublicJoinHint(deploymentHint)
      ? `${actionId} requires player_session; this browser is still guest_session only on the hosted public join path`
      : `${actionId} requires viewer auth bootstrap; current status: ${state.auth.error || "missing"}`;
    return {
      actionId,
      enabled: false,
      code: "auth_level_insufficient",
      reason,
    };
  }
  return {
    actionId,
    enabled: true,
    code: null,
    reason: strongAuthSensitive
      ? "prompt_control stays enabled only in trusted_local_preview via legacy viewer auth bootstrap; hosted/public strong_auth remains pending"
      : "player_session is active via legacy viewer auth bootstrap preview",
  };
}

function buildAuthSurfaceModel() {
  const deploymentHint = authDeploymentHint(state.auth);
  const promptCapability = buildSemanticCapability("prompt_control");
  const chatCapability = buildSemanticCapability("agent_chat");
  const mainTokenTransferCapability = buildSemanticCapability("main_token_transfer");
  const strongAuthTier = buildStrongAuthTier(promptCapability);
  const currentTier = state.auth.available ? "player_session" : "guest_session";
  const source = state.hostedAccess
    ? state.auth.available
      ? state.auth.source === "legacy_viewer_auth_bootstrap"
        ? "legacy_viewer_auth_bootstrap+hosted_access_hint"
        : "hosted_player_issue+browser_local_device_session"
      : "hosted_access_hint"
    : state.auth.available
      ? state.auth.source
      : "guest_only";
  return {
    deploymentHint,
    source,
    currentTier,
    currentTierReason:
      currentTier === "player_session"
        ? playerSessionReason(state.auth, deploymentHint)
        : guestSessionReason(state.auth, deploymentHint),
    tiers: [
      {
        id: "guest_session",
        label: "guest_session",
        status: state.auth.available ? "superseded" : "active",
        reason: guestSessionReason(state.auth, deploymentHint),
      },
      {
        id: "player_session",
        label: "player_session",
        status: state.auth.available
          ? state.auth.source === "legacy_viewer_auth_bootstrap"
            ? "active_legacy_preview"
            : state.auth.registrationStatus === "registered"
              ? "active_hosted_session"
              : "issued_pending_register"
          : "not_issued",
        reason: playerSessionReason(state.auth, deploymentHint),
      },
      {
        id: "strong_auth",
        label: "strong_auth",
        status: strongAuthTier.status,
        reason: strongAuthTier.reason,
      },
    ],
    capabilities: {
      prompt_control: promptCapability,
      agent_chat: chatCapability,
      main_token_transfer: mainTokenTransferCapability,
      strong_auth_actions: mainTokenTransferCapability,
    },
    reconnect: state.auth.available
      ? state.auth.source === "legacy_viewer_auth_bootstrap"
        ? "reconnect still depends on the current preview bootstrap; hosted player-session reconnect/release is available only after switching away from legacy bootstrap"
        : state.auth.registrationStatus === "registered"
          ? "page reload will reuse the hosted device session, mint a fresh browser session key, and attempt reconnect_sync first"
          : "hosted device session is persisted locally, but runtime player-session restore is still pending this page load"
      : isHostedPublicJoinHint(deploymentHint)
        ? buildHostedRecoveryHint("en")?.detail
          || "hosted public join recovers by verifying the hosted account, acquiring a player_session, then re-registering it through reconnect_sync"
        : "page reload is possible once viewer auth bootstrap or hosted account login succeeds",
  };
}

function buildHostedActionMatrixView() {
  const matrix = Array.isArray(state.hostedAccess?.action_matrix)
    ? state.hostedAccess.action_matrix
    : [];
  return matrix.map((policy) => {
    const actionId = String(policy?.action_id || "").trim();
    const capability = buildSemanticCapability(actionId);
    return {
      actionId,
      requiredAuth: String(policy?.required_auth || "").trim() || "unknown",
      availability: String(policy?.availability || "").trim() || "unknown",
      reason: String(policy?.reason || capability.reason || "").trim(),
      enabled: capability.enabled === true,
      code: capability.code || null,
      capabilityReason: capability.reason || null,
    };
  });
}

function buildHostedRecoveryHint(locale = state.uiLocale) {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    return null;
  }
  if (state.auth.available) {
    return null;
  }
  const errorText = String(state.auth.error || "").trim();
  const revokeReason = String(state.auth.revokeReason || "").trim();
  const revokedBy = String(state.auth.revokedBy || "").trim();
  if (!errorText) {
    return null;
  }
  if (errorText.includes("released locally")) {
    return {
      kind: "released",
      title: isLocaleZh(locale) ? "托管玩家会话已释放" : "Hosted player session released",
      detail: isLocaleZh(locale)
        ? "当前浏览器已经在本地释放托管玩家席位。若要继续试玩，需要重新完成托管账户登录并获取新的玩家会话。"
        : "This browser returned its hosted player slot locally. Re-login to the hosted account and acquire a fresh player session if you want to resume gameplay.",
      cta: isLocaleZh(locale) ? "重新登录托管账户" : "Re-login to Hosted Account",
    };
  }
  if (errorText.includes("revoked") || revokeReason || revokedBy) {
    const actorText = revokedBy ? ` by ${revokedBy}` : "";
    const reasonText = revokeReason
      ? ` Reason: ${revokeReason}.`
      : "";
    return {
      kind: "revoked",
      title: isLocaleZh(locale) ? "托管玩家会话已被撤销" : "Hosted player session was revoked",
      detail: isLocaleZh(locale)
        ? `运行时或操作者撤销了这个浏览器会话${actorText}.${reasonText} 继续进行玩法、聊天或 prompt 操作前，需要重新登录托管账户并获取新的玩家会话。`
        : `The runtime or operator revoked this browser session${actorText}.${reasonText} You need to re-login to the hosted account and acquire a fresh player session before gameplay, chat, or prompt actions can continue.`,
      cta: isLocaleZh(locale) ? "重新登录托管账户" : "Re-login to Hosted Account",
    };
  }
  if (errorText.includes("session_not_found") || errorText.includes("not found")) {
    return {
      kind: "missing",
      title: isLocaleZh(locale) ? "运行时中找不到托管玩家会话" : "Hosted player session is missing from runtime",
      detail: isLocaleZh(locale)
        ? "浏览器本地只保留了 device session，但运行时已经不再识别这个会话。请重新登录托管账户，获取新的玩家会话并重新注册。"
        : "The browser only retained the local device-session handle, but the runtime no longer recognizes this session. Re-login to the hosted account, acquire a fresh player session, and register again.",
      cta: isLocaleZh(locale) ? "重新登录托管账户" : "Re-login to Hosted Account",
    };
  }
  return {
    kind: "guest",
    title: isLocaleZh(locale) ? "托管玩家会话不可用" : "Hosted player session is unavailable",
    detail: errorText,
    cta: isLocaleZh(locale) ? "登录托管账户" : "Login to Hosted Account",
  };
}

function nextRequestId() {
  requestId += 1;
  return requestId;
}

function nextAuthNonce() {
  authNonceCounter += 1;
  return Date.now() + authNonceCounter;
}

function snapshotControlFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    action: feedback.action,
    accepted: feedback.accepted,
    stage: feedback.stage,
    reason: feedback.reason,
    hint: feedback.hint,
    effect: feedback.effect,
    deltaLogicalTime: feedback.deltaLogicalTime || 0,
    deltaEventSeq: feedback.deltaEventSeq || 0,
    deltaTraceCount: feedback.deltaTraceCount || 0,
  };
}

function snapshotSemanticFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    kind: feedback.kind,
    action: feedback.action,
    agentId: feedback.agentId || null,
    accepted: feedback.accepted,
    stage: feedback.stage,
    ok: feedback.ok,
    reason: feedback.reason || null,
    effect: feedback.effect || null,
    response: clone(feedback.response) || null,
  };
}

function semanticFeedbackCode(feedback) {
  if (feedback?.stage !== "error") {
    return null;
  }
  const responseCode = String(feedback?.response?.code || "").trim();
  if (responseCode) {
    return responseCode;
  }
  const effectCode = String(feedback?.effect || "").trim();
  return effectCode || null;
}

function semanticFeedbackMessage(feedback) {
  const responseMessage = String(feedback?.response?.message || "").trim();
  if (responseMessage) {
    return responseMessage;
  }
  const reason = String(feedback?.reason || "").trim();
  return reason || null;
}

function formatPromptVersionLabel(value) {
  return `v${Math.max(0, Math.floor(Number(value || 0)))}`;
}

function humanizePromptField(field) {
  return String(field || "")
    .trim()
    .replaceAll("_", " ");
}

function summarizeAppliedFields(feedback) {
  const fields = Array.isArray(feedback?.response?.applied_fields)
    ? feedback.response.applied_fields
        .map(humanizePromptField)
        .filter(Boolean)
    : [];
  if (!fields.length) {
    return null;
  }
  return fields.join(", ");
}

function describeSemanticFeedback(feedback, locale = state.uiLocale) {
  if (!feedback) {
    return null;
  }
  const code = semanticFeedbackCode(feedback);
  const diagnostics = semanticFeedbackMessage(feedback);
  const description = {
    label: feedback.stage || "idle",
    summary: feedback.effect || diagnostics || (isLocaleZh(locale) ? "反馈已更新。" : "Feedback updated."),
    detail: null,
    code,
    diagnostics,
    badgeClass: feedbackBadgeClass(feedback),
  };

  if (feedback.stage === "error") {
    if (code === "llm_init_failed") {
      description.label = isLocaleZh(locale) ? "LLM 不可用" : "LLM unavailable";
      description.summary = isLocaleZh(locale)
        ? "当前栈没有可用的 LLM 配置，因此无法开始聊天。"
        : "Chat cannot start because this stack has no usable LLM configuration.";
      description.detail =
        isLocaleZh(locale)
          ? "请把 model、base URL 和 API key 写入当前 config.toml 或 OASIS7_LLM_* 环境变量，然后重启 launcher 栈。"
          : "Add model, base URL, and API key to the active config.toml or OASIS7_LLM_* env, then restart the launcher stack.";
      return description;
    }
    if (code === "target_version_not_found") {
      description.label = isLocaleZh(locale) ? "找不到回滚目标" : "Rollback target missing";
      description.summary = isLocaleZh(locale)
        ? "当前 Agent 没有这个可回滚版本。"
        : "The selected rollback version is not available for this agent.";
      description.detail = isLocaleZh(locale)
        ? "请先刷新 prompt 状态，或改选一个真实存在的保存版本后再重试。"
        : "Refresh prompt state or choose an existing saved version before retrying.";
      return description;
    }
    if (code === "rollback_noop") {
      description.label = isLocaleZh(locale) ? "回滚无变化" : "Rollback noop";
      description.summary = isLocaleZh(locale)
        ? "这个回滚目标不会改变当前 prompt。"
        : "That rollback target would not change the current prompt.";
      description.detail = isLocaleZh(locale)
        ? "只有在你确实要恢复不同 prompt 内容时，才需要选择更旧的版本。"
        : "Pick an older version only when you need to restore different prompt content.";
      return description;
    }
    if (feedback.kind === "prompt") {
      description.label = isLocaleZh(locale) ? "Prompt 失败" : "Prompt failed";
      description.summary = isLocaleZh(locale)
        ? "Prompt 控制没有完成。"
        : "Prompt control did not complete.";
      description.detail = isLocaleZh(locale)
        ? "展开诊断可查看后端拒绝的具体原因。"
        : "Open diagnostics for the exact backend rejection.";
      return description;
    }
    if (feedback.kind === "chat") {
      description.label = isLocaleZh(locale) ? "聊天失败" : "Chat failed";
      description.summary = isLocaleZh(locale)
        ? "Agent 聊天没有完成。"
        : "Agent chat did not complete.";
      description.detail = isLocaleZh(locale)
        ? "展开诊断可查看后端拒绝的具体原因。"
        : "Open diagnostics for the exact backend rejection.";
      return description;
    }
    if (feedback.kind === "gameplay_action") {
      description.label = isLocaleZh(locale) ? "玩法动作失败" : "Gameplay action failed";
      description.summary = isLocaleZh(locale)
        ? "正式玩法动作没有完成。"
        : "The gameplay action did not complete.";
      description.detail = isLocaleZh(locale)
        ? "展开诊断可查看 runtime 返回的拒绝原因。"
        : "Open diagnostics for the runtime rejection details.";
      return description;
    }
    description.label = code || "Request failed";
    description.summary = diagnostics || (isLocaleZh(locale) ? "请求失败。" : "The request failed.");
    description.detail = isLocaleZh(locale)
      ? "展开诊断可查看后端原始载荷。"
      : "Open diagnostics for the raw backend payload.";
    return description;
  }

  if (feedback.kind === "prompt") {
    const version = Number(feedback?.response?.version || 0);
    const appliedFields = summarizeAppliedFields(feedback);
    if (feedback.stage === "preview_ack") {
      description.label = isLocaleZh(locale) ? "预览已就绪" : "Preview ready";
      description.summary = isLocaleZh(locale)
        ? `Prompt 预览已基于 ${formatPromptVersionLabel(version)} 准备完成。`
        : `Prompt preview is ready from ${formatPromptVersionLabel(version)}.`;
      description.detail = isLocaleZh(locale)
        ? "应用前请先检查返回的摘要或 prompt 字段。"
        : "Review the returned digest or prompt fields before applying.";
      return description;
    }
    if (feedback.stage === "apply_ack") {
      description.label = isLocaleZh(locale) ? "Prompt 已保存" : "Prompt saved";
      description.summary = isLocaleZh(locale)
        ? `Prompt 改动已保存为 ${formatPromptVersionLabel(version)}。`
        : `Prompt changes are now saved as ${formatPromptVersionLabel(version)}.`;
      description.detail = appliedFields
        ? (isLocaleZh(locale) ? `已应用字段：${appliedFields}。` : `Applied fields: ${appliedFields}.`)
        : (isLocaleZh(locale) ? "Prompt 改动已被接受并持久化。" : "Prompt changes were accepted and persisted.");
      return description;
    }
    if (feedback.stage === "rollback_ack") {
      const restoredVersion = Number(feedback?.response?.rolled_back_to_version || 0);
      description.label = isLocaleZh(locale) ? "回滚已应用" : "Rollback applied";
      description.summary =
        isLocaleZh(locale)
          ? `当前生效 prompt 已保存为 ${formatPromptVersionLabel(version)}，其内容恢复自 ${formatPromptVersionLabel(restoredVersion)}。`
          : `Active prompt is now saved as ${formatPromptVersionLabel(version)} after restoring content from ${formatPromptVersionLabel(restoredVersion)}.`;
      description.detail =
        isLocaleZh(locale)
          ? "回滚会生成一个新的保存版本；下面输入框指向的是下一次回滚目标，不是刚刚恢复出来的版本。"
          : "Rollback creates a new saved version; the rollback input below points to the next target, not the version that was just restored.";
      return description;
    }
    description.label = isLocaleZh(locale) ? "Prompt 进行中" : "Prompt in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "Prompt 请求正在处理。" : "Prompt request is in flight.");
    description.detail = isLocaleZh(locale)
      ? "请等待 ack/error 返回后再发起下一次 prompt 操作。"
      : "Wait for ack/error before issuing another prompt action.";
    return description;
  }

  if (feedback.kind === "chat") {
    if (feedback.stage === "ack") {
      const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
      description.label = isLocaleZh(locale) ? "聊天已受理" : "Chat accepted";
      description.summary = isLocaleZh(locale)
        ? `消息已在 tick ${acceptedAtTick} 进入 runtime 队列。`
        : `Message entered the runtime queue at tick ${acceptedAtTick}.`;
      description.detail = isLocaleZh(locale)
        ? "请查看 Message Flow，确认玩家出站消息和后续 Agent 回应。"
        : "Watch Message Flow for the outbound player message and any inbound agent reply.";
      return description;
    }
    description.label = isLocaleZh(locale) ? "聊天进行中" : "Chat in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "聊天请求正在处理。" : "Chat request is in flight.");
    description.detail = isLocaleZh(locale)
      ? "请等待 ack/error 返回后再发送下一条消息。"
      : "Wait for ack/error before sending another message.";
    return description;
  }

  if (feedback.kind === "gameplay_action") {
    if (feedback.stage === "ack") {
      const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
      description.label = isLocaleZh(locale) ? "玩法动作已受理" : "Gameplay action accepted";
      description.summary = isLocaleZh(locale)
        ? `动作已在 tick ${acceptedAtTick} 进入 runtime 队列。`
        : `The action entered the runtime queue at tick ${acceptedAtTick}.`;
      description.detail = feedback?.response?.message
        || (isLocaleZh(locale)
          ? "请继续观察 gameplay feedback 或刷新后的快照。"
          : "Watch gameplay feedback or the refreshed snapshot for the next world-state change.");
      return description;
    }
    description.label = isLocaleZh(locale) ? "玩法动作进行中" : "Gameplay action in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "玩法动作请求正在处理。" : "Gameplay action request is in flight.");
    description.detail = isLocaleZh(locale)
      ? "请等待 ack/error 或新的 gameplay 快照反馈。"
      : "Wait for ack/error or a new gameplay snapshot update.";
    return description;
  }

  return description;
}

function describePromptVersionState(feedback = state.lastPromptFeedback, locale = state.uiLocale) {
  const currentVersion = Math.max(0, Math.floor(Number(state.promptDraft.currentVersion || 0)));
  const nextRollbackTargetVersion = Math.max(
    0,
    Math.floor(Number(state.promptDraft.rollbackTargetVersion || 0)),
  );
  const responseVersion = Number(feedback?.response?.version);
  const ackVersion = Number.isFinite(responseVersion) ? Math.max(0, Math.floor(responseVersion)) : currentVersion;
  const responseRollbackVersion = Number(feedback?.response?.rolled_back_to_version);
  const restoredFromVersion =
    feedback?.stage === "rollback_ack" && Number.isFinite(responseRollbackVersion)
      ? Math.max(0, Math.floor(responseRollbackVersion))
      : null;
  const summary = restoredFromVersion == null
    ? (isLocaleZh(locale)
        ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}。`
        : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}.`)
    : (isLocaleZh(locale)
        ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}；内容恢复自 ${formatPromptVersionLabel(restoredFromVersion)}。`
        : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}; content was restored from ${formatPromptVersionLabel(restoredFromVersion)}.`);
  const detail = restoredFromVersion == null
    ? (isLocaleZh(locale)
        ? `回滚输入框默认指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}。`
        : `The rollback input defaults to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}.`)
    : (isLocaleZh(locale)
        ? `这次回滚生成了新的保存版本 ${formatPromptVersionLabel(ackVersion)}。下面输入框现在指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}，不是刚恢复的版本。`
        : `The rollback created a new saved version ${formatPromptVersionLabel(ackVersion)}. The input below now points to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}, not the restored version.`);
  return {
    currentVersion,
    nextRollbackTargetVersion,
    ackVersion,
    restoredFromVersion,
    summary,
    detail,
  };
}

function buildGameplaySummary(locale = state.uiLocale) {
  const gameplay = state.snapshot?.player_gameplay;
  if (!gameplay || typeof gameplay !== "object") {
    return null;
  }

  const agents = Object.keys(state.snapshot?.model?.agents || {});
  const locations = Object.keys(state.snapshot?.model?.locations || {});
  const missingAgents = agents.length === 0;
  const missingLocations = locations.length === 0;
  const emptyEntityBlocker = missingAgents || missingLocations
    ? (() => {
        const missing = [];
        if (missingAgents) {
          missing.push(isLocaleZh(locale) ? "Agent" : "agents");
        }
        if (missingLocations) {
          missing.push(isLocaleZh(locale) ? "地点" : "locations");
        }
        const missingLabel = missing.join(isLocaleZh(locale) ? " / " : "/");
        return {
          blockerKind: "runtime_snapshot_empty_entities",
          blockerDetail: isLocaleZh(locale)
            ? `runtime 已发布玩法进度，但当前快照没有 ${missingLabel}，formal web entry 暂时无法继续。`
            : `Runtime published gameplay progress, but the current snapshot has no ${missingLabel}; the formal web entry cannot continue yet.`,
          nextStepHint: isLocaleZh(locale)
            ? "先刷新快照；如果实体仍然为空，请修复或重启 runtime world bootstrap 后再继续。"
            : "Request a fresh snapshot first. If entities stay empty, repair or restart the runtime world bootstrap before continuing.",
          disabledReason: isLocaleZh(locale)
            ? `当前快照缺少 ${missingLabel}；刷新快照或修复 runtime bootstrap 后再试。`
            : `Current snapshot is missing ${missingLabel}; refresh the snapshot or repair runtime bootstrap before retrying.`,
        };
      })()
    : null;

  const progressRaw = Number(gameplay.progress_percent);
  const progressPercent = Number.isFinite(progressRaw)
    ? Math.max(0, Math.min(100, Math.floor(progressRaw)))
    : null;
  const acceptedIntentId = gameplay.accepted_intent_id || null;
  const intentSummary = gameplay.intent_summary || null;
  const intentScope = gameplay.intent_scope || null;
  const intentTarget = gameplay.intent_target || null;
  const statusReason = gameplay.status_reason || null;
  const lastWorldChange = gameplay.last_world_change || null;
  const resumeAnchor = gameplay.resume_anchor || null;
  const resumeNextStep = gameplay.resume_next_step || null;
  const availableActions = Array.isArray(gameplay.available_actions)
    ? gameplay.available_actions
      .map((action) => ({
        actionId: action?.action_id || null,
        label: action?.label || null,
        protocolAction: action?.protocol_action || null,
        targetAgentId: action?.target_agent_id || null,
        disabledReason:
          action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot"
            ? action?.disabled_reason || null
            : action?.disabled_reason || emptyEntityBlocker?.disabledReason || null,
        executeKind:
          action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot"
            ? "request_snapshot"
            : action?.protocol_action === "live_control.step"
              ? "step"
              : action?.protocol_action === "live_control.play"
                ? "play"
                : action?.protocol_action === "gameplay_action.submit"
                  ? "gameplay_action"
                  : action?.protocol_action === "agent_chat"
                    ? "agent_chat"
                    : "unsupported",
      }))
    : [];
  const recentFeedback = gameplay.recent_feedback && typeof gameplay.recent_feedback === "object"
    ? {
        action: gameplay.recent_feedback.action || null,
        stage: gameplay.recent_feedback.stage || null,
        effect: gameplay.recent_feedback.effect || null,
        reason: gameplay.recent_feedback.reason || null,
        hint: gameplay.recent_feedback.hint || null,
        deltaLogicalTime: Number(gameplay.recent_feedback.delta_logical_time || 0),
        deltaEventSeq: Number(gameplay.recent_feedback.delta_event_seq || 0),
      }
    : null;
  const runtimeBlockerKind = gameplay.blocker_kind || null;
  const runtimeBlockerDetail = gameplay.blocker_detail || null;
  const runtimeAlreadyPublishedEmptyEntityBlocker =
    runtimeBlockerKind === "runtime_snapshot_empty_entities";
  const resolvedStageStatus = emptyEntityBlocker ? "blocked" : gameplay.stage_status || null;
  const resolvedBlockerKind = runtimeAlreadyPublishedEmptyEntityBlocker
    ? runtimeBlockerKind
    : emptyEntityBlocker
      ? emptyEntityBlocker.blockerKind
      : runtimeBlockerKind;
  const resolvedBlockerDetail = runtimeAlreadyPublishedEmptyEntityBlocker
    ? runtimeBlockerDetail || emptyEntityBlocker?.blockerDetail || null
    : emptyEntityBlocker
      ? emptyEntityBlocker.blockerDetail
      : runtimeBlockerDetail;
  const executionState = emptyEntityBlocker
    ? "blocked"
    : gameplay.execution_state
    || (() => {
      const recentStage = String(recentFeedback?.stage || "").trim().toLowerCase();
      if (["accepted", "submitted", "queued", "ack"].includes(recentStage)) {
        return "accepted";
      }
      if (recentStage === "rejected") {
        return "rejected";
      }
      if (["blocked", "completed_no_progress"].includes(recentStage)) {
        return "blocked";
      }
      if (recentStage === "completed_advanced") {
        return "completed";
      }
      if (resolvedStageStatus === "blocked") {
        return "blocked";
      }
      if (resolvedStageStatus === "branch_ready") {
        return "completed";
      }
      return "executing";
    })();
  const executionStateLabel = (() => {
    switch (executionState) {
      case "accepted":
        return localeText(locale, "已接受", "Accepted");
      case "blocked":
        return localeText(locale, "已阻塞", "Blocked");
      case "completed":
        return localeText(locale, "已完成", "Completed");
      case "rejected":
        return localeText(locale, "已拒绝", "Rejected");
      default:
        return localeText(locale, "执行中", "Executing");
    }
  })();
  const executionStateMachine = [
    { id: "accepted", label: localeText(locale, "已接受", "Accepted") },
    { id: "executing", label: localeText(locale, "执行中", "Executing") },
    { id: "blocked", label: localeText(locale, "已阻塞", "Blocked") },
    { id: "completed", label: localeText(locale, "已完成", "Completed") },
    { id: "rejected", label: localeText(locale, "已拒绝", "Rejected") },
  ];
  const executionCauseKind = emptyEntityBlocker
    ? "world_constraint"
    : gameplay.causality_kind
    || (() => {
      if (executionState === "accepted") return "queued_for_execution";
      if (executionState === "rejected") return "request_rejected";
      if (executionState === "blocked") return "world_constraint";
      if (executionState === "completed") return "goal_progressed";
      return null;
    })();
  const executionCauseLabel = (() => {
    switch (executionCauseKind) {
      case "queued_for_execution":
        return localeText(locale, "等待执行", "Queued for Execution");
      case "world_constraint":
        return localeText(locale, "世界约束", "World Constraint");
      case "agent_override":
        return localeText(locale, "Agent 改走了别的允许路径", "Agent Chose Differently");
      case "goal_progressed":
        return localeText(locale, "世界已推进", "World Progressed");
      case "request_rejected":
        return localeText(locale, "请求被拒绝", "Request Rejected");
      default:
        return null;
    }
  })();
  const executionCauseDetail = emptyEntityBlocker
    ? resolvedBlockerDetail || emptyEntityBlocker.blockerDetail || null
    : gameplay.causality_detail
    || (() => {
      if (executionState === "blocked") {
        return resolvedBlockerDetail || recentFeedback?.reason || null;
      }
      if (executionState === "accepted") {
        return recentFeedback?.hint || recentFeedback?.effect || null;
      }
      if (executionState === "completed") {
        return recentFeedback?.effect || gameplay.progress_detail || null;
      }
      if (executionState === "rejected") {
        return recentFeedback?.reason || null;
      }
      return null;
    })();
  const executionSummary = (() => {
    if (executionCauseKind === "agent_override") {
      return localeText(
        locale,
        "本次目标已推动世界继续前进，但执行它的 Agent 最终采用了另一条被允许的计划。",
        "This goal still advanced the world, but the acting agent finished it through a different allowed plan.",
      );
    }
    switch (executionState) {
      case "accepted":
        return localeText(
          locale,
          "最新一条目标相关指令已经入队，正在等待 committed world delta 或后续回执。",
          "The latest goal-affecting command is queued and waiting for committed world delta or follow-up feedback.",
        );
      case "blocked":
        return localeText(
          locale,
          "当前目标没有继续推进，主要原因已经被归入可修复的 blocker taxonomy。",
          "The current goal is not moving forward; the primary reason is now grouped into a repairable blocker taxonomy.",
        );
      case "completed":
        return localeText(
          locale,
          "当前目标最近一次执行已经产生世界级结果，可以决定是继续放大、恢复，还是切到下一条主线。",
          "The current goal's latest execution already produced a world-level result; you can now amplify it, recover it, or pivot to the next line.",
        );
      case "rejected":
        return localeText(
          locale,
          "最新请求在执行前被拒绝，需要先修正请求本身或权限/模式前提。",
          "The latest request was rejected before execution; fix the request itself or its permission/mode prerequisites first.",
        );
      default:
        return localeText(
          locale,
          "当前目标正在执行中，先盯住状态机、主因果和下一步，再决定是否继续推进。",
          "The current goal is executing; read the state machine, primary causality, and next step before pushing again.",
        );
    }
  })();
  const blockerLabel = (() => {
    switch (resolvedBlockerKind) {
      case "material_shortage":
        return localeText(locale, "缺料", "Missing Material");
      case "power_shortage":
        return localeText(locale, "缺电", "Missing Power");
      case "governance_gate":
        return localeText(locale, "治理限制", "Governance Restriction");
      case "no_progress":
        return localeText(locale, "没有前进", "No Forward Progress");
      case "llm_required":
        return localeText(locale, "缺少玩法能力", "Missing Gameplay Capability");
      case "runtime_sync_unavailable":
        return localeText(locale, "运行时同步不可用", "Runtime Sync Unavailable");
      case "execution_world_not_ready":
        return localeText(locale, "执行世界未就绪", "Execution World Not Ready");
      case "runtime_snapshot_empty_entities":
        return localeText(locale, "空快照", "Empty Snapshot");
      default:
        return resolvedBlockerKind || null;
    }
  })();
  const recommendedAction = availableActions
    .filter((action) => !action.disabledReason)
    .sort((left, right) => {
      const priority = (action) => {
        switch (action.executeKind) {
          case "gameplay_action":
            return 0;
          case "step":
            return 1;
          case "play":
            return 2;
          case "request_snapshot":
            return 3;
          case "agent_chat":
            return 4;
          default:
            return 5;
        }
      };
      return priority(left) - priority(right);
    })[0] || null;
  const acceptedIntentSummary = intentSummary
    || acceptedIntentId
    || localeText(
      locale,
      "还没有一条被正式接受的玩家意图",
      "No player-facing accepted intent yet",
    );
  const acceptedIntentDetail = (() => {
    if (lastWorldChange) {
      return lastWorldChange;
    }
    if (statusReason) {
      return statusReason;
    }
    if (recentFeedback?.hint) {
      return recentFeedback.hint;
    }
    return localeText(
      locale,
      "先提交一个玩法动作，再看系统如何确认、推进或阻塞它。",
      "Submit one gameplay action first, then read how the system confirms, advances, or blocks it.",
    );
  })();
  const narrativeNextStep = emptyEntityBlocker
    ? emptyEntityBlocker.nextStepHint
    : gameplay.next_step_hint || resumeNextStep || null;
  const narrativeBlockerDetail = resolvedBlockerDetail || statusReason || recentFeedback?.reason || null;

  return {
    stageId: gameplay.stage_id || null,
    stageStatus: resolvedStageStatus,
    acceptedIntentId,
    acceptedIntentSummary,
    acceptedIntentScope: intentScope,
    acceptedIntentTarget: intentTarget,
    acceptedIntentDetail,
    statusReason,
    lastWorldChange,
    resumeAnchor,
    resumeNextStep,
    executionState,
    executionStateLabel,
    executionStateMachine,
    executionSummary,
    executionCauseKind,
    executionCauseLabel,
    executionCauseDetail,
    goalId: gameplay.goal_id || null,
    goalKind: gameplay.goal_kind || null,
    goalTitle: gameplay.goal_title || null,
    objective: gameplay.objective || null,
    progressDetail: gameplay.progress_detail || null,
    progressPercent,
    blockerKind: resolvedBlockerKind,
    blockerLabel,
    blockerDetail: resolvedBlockerDetail,
    blockerSupplementalDetail: emptyEntityBlocker && runtimeBlockerDetail && !runtimeAlreadyPublishedEmptyEntityBlocker
      ? runtimeBlockerDetail
      : null,
    nextStepHint: runtimeAlreadyPublishedEmptyEntityBlocker
      ? gameplay.next_step_hint || emptyEntityBlocker?.nextStepHint || resumeNextStep || null
      : emptyEntityBlocker
        ? emptyEntityBlocker.nextStepHint
        : gameplay.next_step_hint || resumeNextStep || null,
    branchHint: gameplay.branch_hint || null,
    narrativeBlockerDetail,
    narrativeNextStep,
    entityCounts: {
      agents: agents.length,
      locations: locations.length,
    },
    availableActions,
    recommendedAction,
    recentFeedback,
    agentClaim: clone(gameplay.agent_claim),
    assetGovernanceHandoff: isLocaleZh(locale)
      ? "资产 / 治理动作仍在单独 lane 处理；viewer 这里不会直接暴露主代币转账表单。"
      : "Asset/governance actions remain a separate lane. viewer exposes no main token transfer form here.",
  };
}

function getState() {
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  const gameplaySummary = buildGameplaySummary();
  return {
    connectionStatus: state.connectionStatus,
    logicalTime: state.logicalTime,
    eventSeq: state.eventSeq,
    tick: state.tick,
    selectedKind: state.selectedKind,
    selectedId: state.selectedId,
    errorCount: state.errorCount,
    lastError: state.lastError,
    eventCount: state.eventCount,
    traceCount: state.traceCount,
    cameraMode: state.cameraMode,
    cameraRadius: state.cameraRadius,
    cameraOrthoScale: state.cameraOrthoScale,
    lastControlFeedback: snapshotControlFeedback(state.lastControlFeedback),
    lastPromptFeedback: snapshotSemanticFeedback(state.lastPromptFeedback),
    lastChatFeedback: snapshotSemanticFeedback(state.lastChatFeedback),
    lastGameplayActionFeedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback),
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion,
    pixelWorldRuntimeStatus: state.pixelWorldRuntimeStatus,
    pixelWorldRuntimeSource: state.pixelWorldRuntimeSource,
    pixelWorldRuntimeModuleUrl: state.pixelWorldRuntimeModuleUrl,
    pixelWorldCamera: clone(state.pixelWorldCamera),
    pixelWorldFatal: clone(state.pixelWorldFatal),
    uiLocale: state.uiLocale,
    promptOverridesVisible: state.promptOverridesVisible,
    controlProfile: state.controlProfile,
    debugViewerMode: state.debugViewerMode,
    debugViewerStatus: state.debugViewerStatus,
    worldId: state.worldId,
    server: state.server,
    wsUrl: state.wsUrl,
    authReady: state.auth.available,
    authPlayerId: state.auth.playerId,
    authPublicKey: state.auth.publicKey,
    authError: state.auth.error,
    authRevokeReason: state.auth.revokeReason,
    authRevokedBy: state.auth.revokedBy,
    authRegistrationStatus: state.auth.registrationStatus,
    authSessionEpoch: state.auth.sessionEpoch,
    authRecoveryErrorCode: state.auth.recoveryErrorCode,
    authRecoveryErrorMessage: state.auth.recoveryErrorMessage,
    authRuntimeStatus: state.auth.runtimeStatus,
    authBoundAgentId: state.auth.boundAgentId,
    authPendingRequestedAgentId: state.auth.pendingRequestedAgentId,
    authPendingForceRebind: state.auth.pendingForceRebind,
    authRebindNotice: state.auth.rebindNotice,
    authTier: authSurface.currentTier,
    authSource: authSurface.source,
    authDeploymentHint: authSurface.deploymentHint,
    authSurface: clone(authSurface),
    hostedRecoveryHint: clone(hostedRecoveryHint),
    hostedAccess: clone(state.hostedAccess),
    hostedActionMatrix: clone(hostedActionMatrixView),
    hostedAdmission: clone(state.hostedAdmission),
    gameplaySummary: clone(gameplaySummary),
    lastDecisionTrace: snapshotDecisionTrace(state.recentDecisionTraces[0] || null),
    recentDecisionTracesCount: state.recentDecisionTraces.length,
    recentDecisionTraces: state.recentDecisionTraces
      .slice(0, 4)
      .map((trace) => snapshotDecisionTrace(trace)),
    strongAuthApprovalCodeConfigured: !!String(state.strongAuth.approvalCode || "").trim(),
    strongAuthLastGrantActionId: state.strongAuth.lastGrantActionId,
    strongAuthLastGrantExpiresAtUnixMs: state.strongAuth.lastGrantExpiresAtUnixMs,
    strongAuthLastGrantError: state.strongAuth.lastGrantError,
    selectedAgentInteractionMode: selectedAgentInteractionMode(),
    selectedAgentDebug: clone(selectedAgentExecutionDebugContext()),
    selectedPromptVersion: state.promptDraft.currentVersion || 0,
    promptRollbackTargetVersion: state.promptDraft.rollbackTargetVersion || 0,
    chatHistoryCount: state.chatHistory.length,
    chatHistory: clone(state.chatHistory),
  };
}

function reportFatalError(message, source = "runtime") {
  const text = `${source}: ${String(message || "unknown runtime error")}`.trim();
  if (state.lastError !== text) {
    state.errorCount += 1;
  }
  state.connectionStatus = "error";
  state.debugViewerStatus = "error";
  state.lastError = text;
  render();
}

function parseSelectionPayload(payload) {
  if (payload == null) {
    return null;
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed) return null;
    const parts = trimmed.split(":");
    if (parts.length >= 2) {
      return { kind: parts[0], id: parts.slice(1).join(":") };
    }
    return { kind: "agent", id: trimmed };
  }
  if (typeof payload === "object") {
    const kind = payload.kind || payload.targetKind || payload.type;
    const id = payload.id || payload.targetId || payload.value;
    if (!kind || !id) return null;
    return { kind: String(kind), id: String(id) };
  }
  return null;
}

function entityCollections() {
  const model = state.snapshot?.model || {};
  return {
    agents: Object.values(model.agents || {}),
    locations: Object.values(model.locations || {}),
  };
}

function selectedAgentId() {
  return state.selectedKind === "agent" ? state.selectedId : null;
}

function selectedAgentPromptProfile() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {
    agent_id: agentId,
    version: 0,
    updated_at_tick: 0,
    updated_by: "",
    system_prompt_override: null,
    short_term_goal_override: null,
    long_term_goal_override: null,
  };
}

function selectedAgentBindingInfo() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return {
    playerId: state.snapshot?.model?.agent_player_bindings?.[agentId] || null,
    publicKey: state.snapshot?.model?.agent_player_public_key_bindings?.[agentId] || null,
  };
}

function selectedAgentExecutionDebugContext() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_execution_debug_contexts?.[agentId] || null;
}

function selectedAgentInteractionMode() {
  const debugContext = selectedAgentExecutionDebugContext();
  if (debugContext?.provider_mode === "provider_loopback_http") {
    return "observer_only";
  }
  return "interactive";
}

function syncAgentInteractionDrafts(force = false) {
  const agentId = selectedAgentId();
  const profile = selectedAgentPromptProfile();
  if (force || state.promptDraft.agentId !== agentId || (!state.promptDraft.dirty && agentId)) {
    const currentVersion = Number(profile?.version || 0);
    state.promptDraft = {
      agentId,
      currentVersion,
      rollbackTargetVersion: Math.max(0, currentVersion - 1),
      updatedBy: String(profile?.updated_by || ""),
      updatedAtTick: Number(profile?.updated_at_tick || 0),
      systemPrompt: String(profile?.system_prompt_override || ""),
      shortTermGoal: String(profile?.short_term_goal_override || ""),
      longTermGoal: String(profile?.long_term_goal_override || ""),
      dirty: false,
    };
  }
  if (force || state.chatDraft.agentId !== agentId) {
    state.chatDraft = {
      agentId,
      message: agentId === state.chatDraft.agentId ? state.chatDraft.message : "",
      dirty: false,
    };
  }
}

function applySelection(selection) {
  if (!selection) return null;
  const kind = String(selection.kind || "").toLowerCase();
  const id = String(selection.id || "");
  const { agents, locations } = entityCollections();
  let object = null;
  if (kind === "agent") {
    object = agents.find((entry) => entry.id === id) || null;
  } else if (kind === "location") {
    object = locations.find((entry) => entry.id === id) || null;
  }
  if (!object) {
    return null;
  }
  state.selectedKind = kind;
  state.selectedId = id;
  state.selectedObject = object;
  syncAgentInteractionDrafts(true);
  render();
  return { kind, id };
}

function select(payload) {
  const parsed = parseSelectionPayload(payload);
  if (!parsed) {
    return { ok: false, reason: "invalid selection payload" };
  }
  const applied = applySelection(parsed);
  if (!applied) {
    return { ok: false, reason: `target not found: ${parsed.kind}:${parsed.id}` };
  }
  return { ok: true, ...applied };
}

function focus(payload) {
  return select(payload);
}

function parseStepCount(payload) {
  if (payload == null) return 1;
  if (typeof payload === "number" && Number.isFinite(payload) && payload >= 1) {
    return Math.floor(payload);
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed || trimmed === "step") return 1;
    const numeric = Number(trimmed);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
    const matched = trimmed.match(/step\s*[:=]\s*(\d+)/i);
    if (matched) {
      return Number(matched[1]);
    }
    return null;
  }
  if (typeof payload === "object") {
    const numeric = Number(payload.count);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
  }
  return null;
}

function controlActions() {
  return [
    {
      action: "play",
      description: "Start continuous world advancement",
      descriptionZh: "开始连续推进世界",
      examplePayload: null,
    },
    {
      action: "pause",
      description: "Pause continuous advancement",
      descriptionZh: "暂停连续推进",
      examplePayload: null,
    },
    {
      action: "step",
      description: "Advance fixed steps (payload.count)",
      descriptionZh: "推进固定步数（payload.count）",
      examplePayload: { count: 5 },
    },
  ];
}

function describeControls() {
  return {
    controls: controlActions(),
    semanticActions: [
      {
        action: "sendAgentChat",
        description: "Send a player-authenticated chat message to an agent",
      },
      {
        action: "sendPromptControl",
        description: "Preview, apply, or rollback prompt overrides for an agent",
      },
    ],
    usage: "Use fillControlExample(action), sendControl(action), sendGameplayAction(actionIdOrPayload), sendAgentChat(agentId, message), sendPromptControl(mode, payload).",
    notes: [
      "viewer acts as a debug_viewer lane: it subscribes to runtime snapshots/events and does not own world authority",
      "when selectedAgentDebug.provider_mode=provider_loopback_http, prompt/chat stay observer-only in runtime live",
      "without viewer auth bootstrap the browser stays guest_session only; hosted public join player-session issuance is still pending",
    ],
  };
}

function fillControlExample(action) {
  const normalized = String(action || "").trim().toLowerCase();
  return controlActions().find((entry) => entry.action === normalized)?.examplePayload ?? null;
}

function sendJson(payload) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    throw new Error("viewer websocket is not connected");
  }
  socket.send(JSON.stringify(payload));
}

function gameplayActionByProtocolAction(protocolAction) {
  const actions = state.snapshot?.player_gameplay?.available_actions;
  if (!Array.isArray(actions)) {
    return null;
  }
  return actions.find((action) => action?.protocol_action === protocolAction) || null;
}

function viewerControlGate(normalizedAction) {
  const protocolAction =
    state.controlProfile === "live"
      ? normalizedAction === "play"
        ? "live_control.play"
        : normalizedAction === "step"
          ? "live_control.step"
          : null
      : null;
  if (!protocolAction) {
    return null;
  }
  const gameplayAction = gameplayActionByProtocolAction(protocolAction);
  const disabledReason = String(gameplayAction?.disabled_reason || "").trim();
  if (!disabledReason) {
    return null;
  }
  return {
    reason: disabledReason,
    effect: `control blocked by gameplay gate: ${disabledReason}`,
    hint: state.snapshot?.player_gameplay?.next_step_hint || null,
  };
}

function sendViewerControl(action, payload) {
  const normalized = String(action || "").trim().toLowerCase();
  const currentRequestId = nextRequestId();
  const feedback = {
    id: currentRequestId,
    action: normalized,
    accepted: false,
    stage: "rejected",
    reason: null,
    hint: null,
    effect: null,
    baselineLogicalTime: state.logicalTime,
    baselineEventSeq: state.eventSeq,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    deltaTraceCount: 0,
    requestId: currentRequestId,
  };

  let mode = null;
  if (normalized === "play") {
    mode = { mode: "play" };
  } else if (normalized === "pause") {
    mode = { mode: "pause" };
  } else if (normalized === "step") {
    const count = parseStepCount(payload);
    if (!count) {
      feedback.reason = "step requires numeric payload.count >= 1";
      feedback.effect = "request rejected before send";
      state.lastControlFeedback = feedback;
      render();
      return snapshotControlFeedback(feedback);
    }
    mode = { mode: "step", count };
  } else {
    feedback.reason = `unsupported action: ${normalized}`;
    feedback.effect = "request rejected before send";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }

  const gate = viewerControlGate(normalized);
  if (gate) {
    feedback.stage = "blocked";
    feedback.reason = gate.reason;
    feedback.hint = gate.hint;
    feedback.effect = gate.effect;
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }

  try {
    if (state.controlProfile === "live") {
      sendJson({ type: "live_control", mode, request_id: currentRequestId });
    } else if (state.controlProfile === "playback") {
      sendJson({ type: "playback_control", mode, request_id: currentRequestId });
    } else {
      sendJson({ type: "control", mode, request_id: currentRequestId });
    }
    feedback.accepted = true;
    feedback.stage = "queued";
    feedback.effect = "queued, check getState().lastControlFeedback for world delta";
    pendingControlFeedback.set(currentRequestId, feedback);
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  } catch (error) {
    feedback.reason = String(error);
    feedback.effect = "request send failed";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
}

function sendControl(action, payload = null) {
  return sendViewerControl(action, payload);
}

function runSteps(payload) {
  const count = parseStepCount(payload);
  if (!count) {
    return { ok: false, reason: "payload must be non-empty step string or count" };
  }
  const feedback = sendControl("step", { count });
  return { ok: Boolean(feedback?.accepted), count, feedback };
}

function setMode() {
  return {
    ok: false,
    reason: "viewer does not expose 2d/3d camera modes",
  };
}

function updateControlFeedbackFromProgress() {
  const feedback = state.lastControlFeedback;
  if (!feedback || !feedback.accepted) return;
  const deltaLogicalTime = Math.max(0, state.logicalTime - feedback.baselineLogicalTime);
  const deltaEventSeq = Math.max(0, state.eventSeq - feedback.baselineEventSeq);
  feedback.deltaLogicalTime = deltaLogicalTime;
  feedback.deltaEventSeq = deltaEventSeq;
  if (deltaLogicalTime > 0 || deltaEventSeq > 0) {
    feedback.stage = "completed_advanced";
    feedback.effect = `world advanced: logicalTime +${deltaLogicalTime}, eventSeq +${deltaEventSeq}`;
  }
}

function summarizeEventTitle(event) {
  const kind = event?.kind?.type || "unknown";
  return kind.replace(/_/g, " ");
}

function addRecentEvent(event) {
  state.recentEvents.unshift(event);
  state.recentEvents = state.recentEvents.slice(0, MAX_EVENTS);
  state.eventCount = state.recentEvents.length;
  state.eventSeq = Math.max(state.eventSeq, Number(event?.id || 0));
}

function handleSnapshot(snapshot) {
  state.snapshot = snapshot;
  state.logicalTime = Math.max(state.logicalTime, Number(snapshot?.time || 0));
  state.tick = state.logicalTime;
  const { agents, locations } = entityCollections();
  if (!state.selectedObject) {
    if (agents[0]) {
      applySelection({ kind: "agent", id: agents[0].id });
    } else if (locations[0]) {
      applySelection({ kind: "location", id: locations[0].id });
    }
  } else if (state.selectedKind && state.selectedId) {
    applySelection({ kind: state.selectedKind, id: state.selectedId });
  }
  syncAgentInteractionDrafts(false);
}

function injectSnapshot(snapshot) {
  if (!isTestApiEnabled()) {
    throw new Error("injectSnapshot requires test_api=1");
  }
  handleSnapshot(clone(snapshot));
  render();
  return getState();
}

function handleMetrics(time, metrics) {
  state.metrics = metrics || null;
  state.traceCount = Number(metrics?.decision_trace_count || 0);
  state.logicalTime = Math.max(state.logicalTime, Number(time || 0), Number(metrics?.total_ticks || 0));
  state.tick = state.logicalTime;
}

function clipTraceText(value, limit = 480) {
  const text = String(value || "").trim();
  if (!text) {
    return null;
  }
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, limit)}…`;
}

function snapshotDecisionTrace(trace) {
  if (!trace || typeof trace !== "object") {
    return null;
  }
  return {
    agent_id: trace.agent_id || null,
    time: Number(trace.time || 0),
    decision: clone(trace.decision || null),
    llm_error: trace.llm_error || null,
    parse_error: trace.parse_error || null,
    llm_input_excerpt: clipTraceText(trace.llm_input),
    llm_output_excerpt: clipTraceText(trace.llm_output),
    llm_diagnostics: clone(trace.llm_diagnostics || null),
  };
}

function handleDecisionTrace(trace) {
  if (!trace || typeof trace !== "object") {
    return;
  }
  state.recentDecisionTraces.unshift(clone(trace));
  state.recentDecisionTraces = state.recentDecisionTraces.slice(0, MAX_DECISION_TRACES);
  state.traceCount = Math.max(state.traceCount, state.recentDecisionTraces.length);
  state.logicalTime = Math.max(state.logicalTime, Number(trace?.time || 0));
  state.tick = state.logicalTime;
}

function handleControlCompletionAck(ack) {
  const feedback = pendingControlFeedback.get(ack?.request_id) || state.lastControlFeedback;
  if (!feedback) return;
  feedback.deltaLogicalTime = Number(ack?.delta_logical_time || 0);
  feedback.deltaEventSeq = Number(ack?.delta_event_seq || 0);
  if (ack?.status === "advanced") {
    feedback.stage = "completed_advanced";
    feedback.effect = `control ack advanced: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
    feedback.reason = null;
  } else if (ack?.status === "blocked") {
    feedback.stage = "blocked";
    feedback.reason =
      ack?.error_message || ack?.error_code || "control was blocked before runtime advance";
    feedback.hint = state.snapshot?.player_gameplay?.next_step_hint || feedback.hint;
    feedback.effect = `gameplay blocked before requested advance completed: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  } else {
    feedback.stage = "completed_no_progress";
    feedback.reason = "timeout_no_progress";
    feedback.effect = `no visible world delta: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  }
  state.lastControlFeedback = feedback;
  pendingControlFeedback.delete(feedback.requestId);
}

function cborHeader(majorType, length) {
  if (!Number.isInteger(length) || length < 0) {
    throw new Error(`invalid CBOR length: ${length}`);
  }
  if (length < 24) {
    return Uint8Array.of((majorType << 5) | length);
  }
  if (length < 0x100) {
    return Uint8Array.of((majorType << 5) | 24, length);
  }
  if (length < 0x10000) {
    return Uint8Array.of((majorType << 5) | 25, (length >> 8) & 0xff, length & 0xff);
  }
  if (length <= 0xffffffff) {
    return Uint8Array.of(
      (majorType << 5) | 26,
      (length >>> 24) & 0xff,
      (length >>> 16) & 0xff,
      (length >>> 8) & 0xff,
      length & 0xff,
    );
  }
  if (length <= Number.MAX_SAFE_INTEGER) {
    const value = BigInt(length);
    return Uint8Array.of(
      (majorType << 5) | 27,
      Number((value >> 56n) & 0xffn),
      Number((value >> 48n) & 0xffn),
      Number((value >> 40n) & 0xffn),
      Number((value >> 32n) & 0xffn),
      Number((value >> 24n) & 0xffn),
      Number((value >> 16n) & 0xffn),
      Number((value >> 8n) & 0xffn),
      Number(value & 0xffn),
    );
  }
  throw new Error("CBOR length exceeds Number.MAX_SAFE_INTEGER");
}

function concatBytes(...parts) {
  const totalLength = parts.reduce((sum, bytes) => sum + bytes.length, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const bytes of parts) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}

function cborEncode(value) {
  if (value === null) {
    return Uint8Array.of(0xf6);
  }
  if (value === false) {
    return Uint8Array.of(0xf4);
  }
  if (value === true) {
    return Uint8Array.of(0xf5);
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`unsupported CBOR number: ${value}`);
    }
    return cborHeader(0, value);
  }
  if (typeof value === "string") {
    const bytes = textEncoder.encode(value);
    return concatBytes(cborHeader(3, bytes.length), bytes);
  }
  if (Array.isArray(value)) {
    return concatBytes(cborHeader(4, value.length), ...value.map((entry) => cborEncode(entry)));
  }
  if (value instanceof Uint8Array) {
    return concatBytes(cborHeader(2, value.length), value);
  }
  if (typeof value === "object") {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined);
    const encoded = [cborHeader(5, entries.length)];
    for (const [key, entryValue] of entries) {
      encoded.push(cborEncode(String(key)));
      encoded.push(cborEncode(entryValue));
    }
    return concatBytes(...encoded);
  }
  throw new Error(`unsupported CBOR type: ${typeof value}`);
}

function hexToBytes(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (!value || value.length % 2 !== 0 || /[^0-9a-f]/.test(value)) {
    throw new Error("invalid hex payload");
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes) {
  return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
}

function bytesStartWith(bytes, prefix) {
  if (bytes.length < prefix.length) {
    return false;
  }
  for (let index = 0; index < prefix.length; index += 1) {
    if (bytes[index] !== prefix[index]) {
      return false;
    }
  }
  return true;
}

async function importEd25519SigningKey(privateKeyHex) {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  if (!authKeyCache.has(privateKeyHex)) {
    const rawPrivateKey = hexToBytes(privateKeyHex);
    if (rawPrivateKey.length !== 32) {
      throw new Error(`viewer auth private key length mismatch: expected 32 bytes, got ${rawPrivateKey.length}`);
    }
    const pkcs8 = concatBytes(ED25519_PKCS8_PREFIX, rawPrivateKey);
    authKeyCache.set(
      privateKeyHex,
      window.crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"]),
    );
  }
  return authKeyCache.get(privateKeyHex);
}

async function signAuthPayload(signingPayloadBytes, auth) {
  const key = await importEd25519SigningKey(auth.privateKey);
  const signature = await window.crypto.subtle.sign({ name: "Ed25519" }, key, signingPayloadBytes);
  return `${VIEWER_AUTH_SIGNATURE_PREFIX}${bytesToHex(new Uint8Array(signature))}`;
}

async function generateEphemeralEd25519Keypair() {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  const keyPair = await window.crypto.subtle.generateKey(
    { name: "Ed25519" },
    true,
    ["sign", "verify"],
  );
  const pkcs8 = new Uint8Array(await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey));
  if (!bytesStartWith(pkcs8, ED25519_PKCS8_PREFIX) || pkcs8.length !== ED25519_PKCS8_PREFIX.length + 32) {
    throw new Error("unexpected Ed25519 pkcs8 encoding from Web Crypto");
  }
  const rawPublicKey = new Uint8Array(await window.crypto.subtle.exportKey("raw", keyPair.publicKey));
  if (rawPublicKey.length !== 32) {
    throw new Error(`unexpected Ed25519 public key length: ${rawPublicKey.length}`);
  }
  return {
    publicKey: bytesToHex(rawPublicKey),
    privateKey: bytesToHex(pkcs8.slice(ED25519_PKCS8_PREFIX.length)),
  };
}

function buildAuthEnvelope(payload) {
  return cborEncode({
    version: 1,
    payload,
  });
}

async function buildAgentChatAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "agent_chat",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    message: request.message,
  };
  if (request.intent_tick != null) {
    payload.intent_tick = request.intent_tick;
  }
  if (request.intent_seq != null) {
    payload.intent_seq = request.intent_seq;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth),
  };
}

function promptPatchFromDraft(currentValue, draftValue) {
  const current = currentValue == null ? "" : String(currentValue);
  const draft = String(draftValue ?? "");
  if (draft === current) {
    return { mode: "unchanged" };
  }
  if (draft.length === 0) {
    return currentValue == null ? { mode: "unchanged" } : { mode: "clear" };
  }
  return { mode: "set", value: draft };
}

async function buildPromptControlAuthProof(mode, request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: mode === "preview" ? "prompt_control_preview" : "prompt_control_apply",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null,
    system_prompt_override: request.system_prompt_override,
    short_term_goal_override: request.short_term_goal_override,
    long_term_goal_override: request.long_term_goal_override,
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth),
  };
}

async function buildPromptRollbackAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "prompt_control_rollback",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    to_version: request.to_version,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null,
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth),
  };
}

async function buildSessionRegisterAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "session_register",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
  };
  if (request.requested_agent_id != null) {
    payload.requested_agent_id = request.requested_agent_id;
  }
  payload.force_rebind = request.force_rebind === true;
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth),
  };
}

async function buildGameplayActionAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "gameplay_action",
    action_id: request.action_id,
    target_agent_id: request.target_agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
  };
  if (request.actor_agent_id != null) {
    payload.actor_agent_id = request.actor_agent_id;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth),
  };
}

function canAutoIssueHostedPlayerSession() {
  return String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"
    && state.auth.source !== "legacy_viewer_auth_bootstrap";
}

async function startHostedAccountLogin() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted account login is unavailable on this lane" };
  }
  const channel = "email";
  state.hostedLogin.channel = channel;
  const handle = String(state.hostedLogin.handle || "").trim();
  if (!handle) {
    state.hostedLogin.error = "email is required before login can start";
    render();
    return { ok: false, reason: state.hostedLogin.error };
  }
  state.hostedLogin.startInFlight = true;
  state.hostedLogin.error = null;
  render();
  try {
    const query = new URLSearchParams({
      channel,
      handle,
    });
    const response = await fetch(`${HOSTED_ACCOUNT_LOGIN_START_ROUTE}?${query.toString()}`, {
      method: "POST",
      cache: "no-store",
      headers: { Accept: "application/json" },
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.challenge?.challenge_id) {
      throw new Error(payload?.error || payload?.error_code || `hosted account login start failed with HTTP ${response.status}`);
    }
    state.hostedLogin.challengeId = String(payload.challenge.challenge_id || "").trim() || null;
    state.hostedLogin.maskedLoginHint = String(payload.challenge.masked_login_hint || "").trim() || null;
    state.hostedLogin.deliveryMode = String(payload.challenge.delivery_mode || "").trim() || null;
    state.hostedLogin.previewCode = String(payload.challenge.preview_code || "").trim() || null;
    state.hostedLogin.code = state.hostedLogin.previewCode || "";
    state.hostedLogin.expiresAtUnixMs = payload?.challenge?.expires_at_unix_ms == null ? null : Number(payload.challenge.expires_at_unix_ms);
    state.hostedLogin.accountExists = payload?.challenge?.account_exists === true;
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.completeInFlight = false;
    state.hostedLogin.error = null;
    render();
    return { ok: true, challengeId: state.hostedLogin.challengeId };
  } catch (error) {
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.error = String(error);
    render();
    return { ok: false, reason: state.hostedLogin.error };
  }
}

async function ensureHostedPlayerAuthAvailable() {
  return state.auth;
}

async function completeHostedAccountLogin() {
  if (!canAutoIssueHostedPlayerSession()) {
    return state.auth;
  }
  if (state.auth.available) {
    return state.auth;
  }
  const challengeId = String(state.hostedLogin.challengeId || "").trim();
  const otpCode = String(state.hostedLogin.code || "").trim();
  if (!challengeId || !otpCode) {
    state.hostedLogin.error = "verification code is required before hosted login can complete";
    render();
    return state.auth;
  }
  state.auth.issueInFlight = true;
  state.hostedLogin.completeInFlight = true;
  state.hostedLogin.error = null;
  state.auth.error = null;
  render();
  try {
    const query = new URLSearchParams({
      challenge_id: challengeId,
      otp_code: otpCode,
    });
    const response = await fetch(`${HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE}?${query.toString()}`, {
      method: "POST",
      cache: "no-store",
      headers: { Accept: "application/json" },
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.grant?.player_id || !payload?.account?.hosted_account_id) {
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      throw new Error(payload?.error || payload?.error_code || `hosted account login complete failed with HTTP ${response.status}`);
    }
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
    const keypair = await generateEphemeralEd25519Keypair();
    state.auth = {
      available: true,
      hostedAccountId: String(payload.account.hosted_account_id || "").trim() || null,
      playerId: String(payload.grant.player_id || "").trim(),
      loginChannel: String(payload.account.login_channel || "").trim() || null,
      maskedLoginHint: String(payload.account.masked_login_hint || "").trim() || null,
      deviceSessionId: String(payload.grant.device_session_id || "").trim()
        || String(payload.grant.release_token || "").trim()
        || null,
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      releaseToken: String(payload.grant.release_token || "").trim() || null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      issuedAtUnixMs: payload?.grant?.issued_at_unix_ms == null ? Date.now() : Number(payload.grant.issued_at_unix_ms),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null,
    };
    persistHostedPlayerSession(state.auth);
    resetHostedLoginChallenge();
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.error = null;
    render();
    return state.auth;
  } catch (error) {
    state.auth.issueInFlight = false;
    state.hostedLogin.completeInFlight = false;
    state.hostedLogin.error = String(error);
    state.auth.error = String(error);
    render();
    return state.auth;
  }
}

async function issueHostedPlayerIdentity() {
  return completeHostedAccountLogin();
}

async function retryHostedPlayerIdentityIssue() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted account login is unavailable on this lane" };
  }
  const auth = state.hostedLogin.challengeId
    ? await completeHostedAccountLogin()
    : await startHostedAccountLogin();
  render();
  return {
    ok: auth?.available === true || auth?.ok === true,
    playerId: auth?.playerId || null,
    error: auth?.error || state.hostedLogin.error,
  };
}

async function requestHostedStrongAuthGrant(actionId, agentId) {
  const auth = await ensureHostedAuthSigningKey(state.auth);
  const playerId = String(auth.playerId || "").trim();
  const publicKey = String(auth.publicKey || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  const approvalCode = String(state.strongAuth.approvalCode || "").trim();
  if (!playerId || !publicKey || !releaseToken) {
    throw new Error("hosted strong-auth grant requires an active player_session with release token");
  }
  if (!approvalCode) {
    throw new Error("backend approval code is required before hosted strong auth can be granted");
  }
  const query = new URLSearchParams({
    player_id: playerId,
    public_key: publicKey,
    release_token: releaseToken,
    agent_id: String(agentId || "").trim(),
    action_id: String(actionId || "").trim(),
    approval_code: approvalCode,
  });
  const response = await fetch(`${HOSTED_STRONG_AUTH_GRANT_ROUTE}?${query.toString()}`, {
    method: "GET",
    cache: "no-store",
    headers: { Accept: "application/json" },
  });
  const payload = await response.json();
  if (payload?.admission) {
    state.hostedAdmission = clone(payload.admission);
  }
  if (!response.ok || !payload?.ok || !payload?.grant) {
    state.strongAuth.lastGrantError = payload?.error || payload?.error_code || `hosted strong-auth grant failed with HTTP ${response.status}`;
    throw new Error(state.strongAuth.lastGrantError);
  }
  state.strongAuth.lastGrantActionId = String(payload.grant.action_id || "").trim() || actionId;
  state.strongAuth.lastGrantExpiresAtUnixMs = payload?.grant?.expires_at_unix_ms == null
    ? null
    : Number(payload.grant.expires_at_unix_ms);
  state.strongAuth.lastGrantError = null;
  return payload.grant;
}

function sendReconnectSync() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.registrationStatus = "registering";
  state.auth.runtimeStatus = "probing";
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey,
      },
    },
  });
}

function probeHostedRuntimeSession() {
  if (
    !state.auth.available
    || state.auth.source === "legacy_viewer_auth_bootstrap"
    || state.connectionStatus !== "connected"
    || state.auth.registrationStatus !== "registered"
  ) {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.runtimeStatus = "probing";
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey,
      },
    },
  });
}

async function releaseHostedPlayerSlot() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, skipped: true };
  }
  const query = `player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`;
  const response = await fetch(`${HOSTED_PLAYER_SESSION_RELEASE_ROUTE}?${query}`, {
    method: "POST",
    cache: "no-store",
    headers: { Accept: "application/json" },
  });
  const payload = await response.json();
  if (!response.ok || !payload?.ok) {
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    throw new Error(payload?.error || payload?.error_code || `hosted player-session release failed with HTTP ${response.status}`);
  }
  state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
  return payload;
}

function resetHostedPlayerAuthState(errorMessage = null, revocationMeta = null) {
  stopHostedSessionRefreshLoop();
  clearHostedPlayerSession();
  resetHostedLoginChallenge();
  const bootstrap = resolveAuthBootstrap();
  const revokeReason = String(revocationMeta?.revokeReason || "").trim() || null;
  const revokedBy = String(revocationMeta?.revokedBy || "").trim() || null;
  state.auth = bootstrap.available
      ? bootstrap
      : {
        ...bootstrap,
        source: "guest_only",
        registrationStatus: "guest",
        error: errorMessage,
        revokeReason,
        revokedBy,
        hostedAccountId: null,
        loginChannel: null,
        maskedLoginHint: null,
        deviceSessionId: null,
        sessionEpoch: null,
        issuedAtUnixMs: null,
        releaseToken: null,
        recoveryErrorCode: null,
        recoveryErrorMessage: null,
        issueInFlight: false,
        syncInFlight: false,
        runtimeStatus: "guest",
        boundAgentId: null,
        pendingRequestedAgentId: null,
        pendingForceRebind: false,
        rebindNotice: null,
      };
  void refreshHostedAdmissionState().then(() => render());
}

async function logoutHostedPlayerSession() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, reason: "hosted browser session is unavailable" };
  }
  const revokeRequest = {
    player_id: state.auth.playerId,
    session_pubkey: state.auth.publicKey,
    revoke_reason: "player_logout",
    revoked_by: state.auth.playerId,
  };
  try {
    if (state.connectionStatus === "connected") {
      sendJson({
        type: "authoritative_recovery",
        command: {
          mode: "revoke_session",
          request: revokeRequest,
        },
      });
    }
  } catch (_) {
  }
  try {
    await releaseHostedPlayerSlot();
  } finally {
    resetHostedPlayerAuthState("hosted player session released locally");
    render();
  }
  return { ok: true };
}

function syncHostedPlayerSessionOnConnect() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap" || state.auth.syncInFlight) {
    return;
  }
  sendReconnectSync();
}

function clearPendingSessionRegisterWaiter(error = null) {
  if (!pendingSessionRegisterWaiter) {
    return;
  }
  const waiter = pendingSessionRegisterWaiter;
  pendingSessionRegisterWaiter = null;
  if (error != null) {
    waiter.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

async function dispatchSessionRegisterRequest(requestedAgentId, forceRebind) {
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  if (state.auth.source !== "legacy_viewer_auth_bootstrap") {
    state.auth.registrationStatus = "registering";
    state.auth.syncInFlight = true;
    state.auth.recoveryErrorCode = null;
    state.auth.recoveryErrorMessage = null;
    state.auth.runtimeStatus = forceRebind === true ? "rebind_registering" : "registering";
  }
  if (forceRebind === true) {
    state.auth.rebindNotice = `Switching player session to ${normalizedRequestedAgentId || "requested agent"}...`;
  }
  state.auth.pendingRequestedAgentId = normalizedRequestedAgentId;
  state.auth.pendingForceRebind = forceRebind === true;
  const auth = await ensureHostedAuthSigningKey(state.auth);
  const request = {
    player_id: auth.playerId,
    public_key: auth.publicKey,
  };
  if (normalizedRequestedAgentId) {
    request.requested_agent_id = normalizedRequestedAgentId;
  }
  if (forceRebind === true) {
    request.force_rebind = true;
  }
  request.auth = await buildSessionRegisterAuthProof(request, auth);
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "register_session",
      request,
    },
  });
  render();
}

async function retryPendingSessionRegisterWaiterWithForceRebind() {
  const waiter = pendingSessionRegisterWaiter;
  if (!waiter) {
    return;
  }
  waiter.forceRebind = true;
  try {
    await dispatchSessionRegisterRequest(waiter.requestedAgentId, true);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
}

function latestRequestedAgentId(fallbackAgentId = null) {
  const agentId = String(
    fallbackAgentId
      || state.auth.pendingRequestedAgentId
      || state.auth.boundAgentId
      || "",
  ).trim();
  return agentId || null;
}

function recoveryErrorRequiresExplicitRebind(error) {
  return String(error?.code || "").trim() === "player_bind_failed"
    && String(error?.message || "").includes("explicit rebind required");
}

async function ensureRegisteredPlayerSession(requestedAgentId = null, options = {}) {
  await ensureHostedPlayerAuthAvailable();
  if (!state.auth.available) {
    throw new Error(state.auth.error || "player session auth is unavailable");
  }
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  const forceRebind = options?.forceRebind === true;
  if (
    state.auth.registrationStatus === "registered"
    && (state.auth.runtimeStatus === "registered" || state.auth.runtimeStatus === "registered_unbound")
    && !forceRebind
    && (
      normalizedRequestedAgentId == null
      || normalizedRequestedAgentId === state.auth.boundAgentId
    )
  ) {
    return state.auth;
  }
  if (pendingSessionRegisterWaiter) {
    const sameRequest = pendingSessionRegisterWaiter.requestedAgentId === normalizedRequestedAgentId
      && pendingSessionRegisterWaiter.forceRebind === forceRebind;
    if (!sameRequest) {
      throw new Error("another player session registration is already in flight");
    }
    return pendingSessionRegisterWaiter.promise;
  }
  let resolveWaiter;
  let rejectWaiter;
  const promise = new Promise((resolve, reject) => {
    resolveWaiter = resolve;
    rejectWaiter = reject;
  });
  pendingSessionRegisterWaiter = {
    requestedAgentId: normalizedRequestedAgentId,
    forceRebind,
    promise,
    resolve: resolveWaiter,
    reject: rejectWaiter,
  };
  try {
    await dispatchSessionRegisterRequest(normalizedRequestedAgentId, forceRebind);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
  return promise;
}

function buildPromptRequestFromDraft(agentId, draftOverrides) {
  const currentProfile = selectedAgentPromptProfile();
  if (!agentId || !currentProfile) {
    throw new Error("select an agent before editing prompt overrides");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    expected_version: Number(currentProfile.version || 0),
    updated_by: state.auth.playerId,
    system_prompt_override: promptPatchFromDraft(currentProfile.system_prompt_override, draftOverrides.systemPrompt),
    short_term_goal_override: promptPatchFromDraft(currentProfile.short_term_goal_override, draftOverrides.shortTermGoal),
    long_term_goal_override: promptPatchFromDraft(currentProfile.long_term_goal_override, draftOverrides.longTermGoal),
  };
}

function encodePromptRequestForJson(request) {
  const encodePatch = (patch) => {
    if (!patch || patch.mode === "unchanged") {
      return undefined;
    }
    if (patch.mode === "clear") {
      return null;
    }
    return patch.value;
  };
  return {
    agent_id: request.agent_id,
    player_id: request.player_id,
    public_key: request.public_key,
    expected_version: request.expected_version,
    updated_by: request.updated_by,
    system_prompt_override: encodePatch(request.system_prompt_override),
    short_term_goal_override: encodePatch(request.short_term_goal_override),
    long_term_goal_override: encodePatch(request.long_term_goal_override),
  };
}

function buildPromptRollbackRequest(agentId, toVersion) {
  const profile = selectedAgentPromptProfile();
  const targetVersion = Number(toVersion);
  if (!agentId || !profile) {
    throw new Error("select an agent before rolling back prompt overrides");
  }
  if (!Number.isInteger(targetVersion) || targetVersion < 0) {
    throw new Error("prompt rollback requires integer toVersion >= 0");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    to_version: targetVersion,
    expected_version: Number(profile.version || 0),
    updated_by: state.auth.playerId,
  };
}

function pushChatHistory(entry) {
  if (!entry) {
    return;
  }
  state.chatHistory.unshift({
    id: entry.id || `${entry.source || "chat"}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    source: entry.source || "event",
    agentId: entry.agentId || null,
    locationId: entry.locationId || null,
    message: String(entry.message || ""),
    tick: Number(entry.tick || 0),
    speaker: entry.speaker || null,
    playerId: entry.playerId || null,
    targetAgentId: entry.targetAgentId || null,
    intentSeq: entry.intentSeq || null,
  });
  state.chatHistory = state.chatHistory.slice(0, 40);
}

function extractAgentSpokeEntry(event) {
  const kind = event?.kind;
  const kindType = String(kind?.type || "");
  if (!["agent_spoke", "AgentSpoke"].includes(kindType)) {
    return null;
  }
  const data = kind.data || {};
  return {
    id: `event-${event.id}`,
    source: "event",
    agentId: data.agent_id || null,
    locationId: data.location_id || null,
    message: data.message || "",
    tick: Number(event.time || 0),
    speaker: data.agent_id || null,
    targetAgentId: data.target_agent_id || null,
  };
}

function requestSnapshotSafe() {
  try {
    sendJson({ type: "request_snapshot" });
  } catch (_) {
  }
}

function createSemanticFeedback(kind, action, agentId, extra = {}) {
  return {
    id: nextRequestId(),
    kind,
    action,
    agentId,
    accepted: true,
    ok: false,
    stage: "queued",
    reason: null,
    effect: null,
    response: null,
    ...extra,
  };
}

function markPendingSemanticRebind(message) {
  const text = String(message || "explicit rebind required; retrying player session registration").trim();
  for (const feedback of [state.lastChatFeedback, state.lastPromptFeedback]) {
    if (!feedback || feedback.stage !== "registering") {
      continue;
    }
    feedback.effect = text;
    feedback.reason = null;
  }
}

function enqueueSemanticCommand(command) {
  pendingSemanticCommands.push(command);
  if (!semanticSendLoop) {
    semanticSendLoop = processSemanticCommands();
  }
}

async function processSemanticCommands() {
  try {
    while (pendingSemanticCommands.length > 0) {
      const command = pendingSemanticCommands.shift();
      try {
        await command.execute();
      } catch (error) {
        command.feedback.stage = "error";
        command.feedback.ok = false;
        command.feedback.reason = String(error);
        command.feedback.effect = "request build/send failed";
        if (command.kind === "chat") {
          state.lastChatFeedback = command.feedback;
        } else {
          state.lastPromptFeedback = command.feedback;
        }
        render();
      }
    }
  } finally {
    semanticSendLoop = null;
    if (pendingSemanticCommands.length > 0) {
      semanticSendLoop = processSemanticCommands();
    }
  }
}

function assertSemanticCapability(actionId) {
  const capability = buildSemanticCapability(actionId);
  if (!capability.enabled) {
    throw new Error(capability.reason || state.auth.error || `${actionId} is unavailable`);
  }
}

function sendAgentChat(agentIdOrPayload, maybeMessage) {
  let agentId = null;
  let message = null;
  if (typeof agentIdOrPayload === "object" && agentIdOrPayload !== null) {
    agentId = String(agentIdOrPayload.agentId || agentIdOrPayload.agent_id || selectedAgentId() || "");
    message = String(agentIdOrPayload.message || "");
  } else {
    agentId = String(agentIdOrPayload || selectedAgentId() || "");
    message = String(maybeMessage || "");
  }
  if (!agentId) {
    return { ok: false, reason: "agent chat requires a selected agent or explicit agentId" };
  }
  if (!message.trim()) {
    return { ok: false, reason: "agent chat message cannot be empty" };
  }
  const feedback = createSemanticFeedback("chat", "agent_chat", agentId, {
    effect: "queued for signing and send",
    pendingMessage: message,
    pendingPlayerId: state.auth.playerId || null,
  });
  state.lastChatFeedback = feedback;
  enqueueSemanticCommand({
    kind: "chat",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("agent_chat");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        agent_id: agentId,
        message,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey,
      };
      request.auth = await buildAgentChatAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "agent_chat request sent; waiting for ack";
      state.lastChatFeedback = feedback;
      sendJson({ type: "agent_chat", request });
      state.chatDraft.message = "";
      state.chatDraft.dirty = false;
      render();
    },
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}

function sendPromptControl(mode, payload = null) {
  const normalizedMode = String(mode || "").trim().toLowerCase();
  if (!["preview", "apply", "rollback"].includes(normalizedMode)) {
    return { ok: false, reason: "prompt control mode must be preview, apply, or rollback" };
  }
  const selectedId = selectedAgentId();
  const agentId = String(payload?.agentId || payload?.agent_id || selectedId || "");
  if (!agentId) {
    return { ok: false, reason: "prompt control requires a selected agent or explicit agentId" };
  }
  let request;
  try {
    if (normalizedMode === "rollback") {
      const currentVersion = Number(state.promptDraft.currentVersion || selectedAgentPromptProfile()?.version || 0);
      const fallbackVersion = Math.max(0, currentVersion - 1);
      const toVersion = payload?.toVersion ?? payload?.to_version ?? fallbackVersion;
      request = buildPromptRollbackRequest(agentId, toVersion);
    } else {
      request = buildPromptRequestFromDraft(agentId, {
        systemPrompt: payload?.systemPrompt ?? payload?.system_prompt_override ?? state.promptDraft.systemPrompt,
        shortTermGoal: payload?.shortTermGoal ?? payload?.short_term_goal_override ?? state.promptDraft.shortTermGoal,
        longTermGoal: payload?.longTermGoal ?? payload?.long_term_goal_override ?? state.promptDraft.longTermGoal,
      });
    }
  } catch (error) {
    return { ok: false, reason: String(error) };
  }

  const feedback = createSemanticFeedback("prompt", `prompt_${normalizedMode}`, agentId, {
    effect: "queued for signing and send",
    toVersion: request.to_version ?? null,
  });
  state.lastPromptFeedback = feedback;
  enqueueSemanticCommand({
    kind: "prompt",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("prompt_control");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      request.player_id = state.auth.playerId;
      request.public_key = state.auth.publicKey;
      request.updated_by = state.auth.playerId;
      let strongAuthGrant = null;
      if (String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join") {
        feedback.stage = "authorizing";
        feedback.effect = "requesting backend strong-auth grant";
        render();
        strongAuthGrant = await requestHostedStrongAuthGrant(
          normalizedMode === "rollback" ? "prompt_control_rollback" : `prompt_control_${normalizedMode}`,
          agentId,
        );
      }
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      let commandRequest;
      if (normalizedMode === "rollback") {
        commandRequest = {
          ...request,
          auth: await buildPromptRollbackAuthProof(request, state.auth),
        };
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      } else {
        commandRequest = encodePromptRequestForJson(request);
        commandRequest.auth = await buildPromptControlAuthProof(normalizedMode, request, state.auth);
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      }
      feedback.stage = "sent";
      feedback.effect = `prompt ${normalizedMode} request sent; waiting for ack`;
      state.lastPromptFeedback = feedback;
      sendJson({
        type: "prompt_control",
        command: {
          mode: normalizedMode,
          request: commandRequest,
        },
      });
      render();
    },
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}

function gameplayActionRequiresActorAgent(actionId) {
  return actionId === "claim_agent" || actionId === "release_agent_claim";
}

function normalizeGameplayActionRequest(action) {
  if (!action || typeof action !== "object") {
    return null;
  }
  const normalized = {
    ...action,
    protocol_action: action.protocol_action || action.protocolAction || null,
    action_id: action.action_id || action.actionId || null,
    target_agent_id: action.target_agent_id || action.targetAgentId || null,
    disabled_reason: action.disabled_reason || action.disabledReason || null,
  };
  return normalized;
}

function resolveGameplayActionRequest(actionOrId) {
  if (typeof actionOrId === "string") {
    const actions = Array.isArray(state.snapshot?.player_gameplay?.available_actions)
      ? state.snapshot.player_gameplay.available_actions
      : [];
    return actions.find((action) => action?.action_id === actionOrId) || null;
  }
  if (!actionOrId || typeof actionOrId !== "object") {
    return null;
  }
  if (typeof actionOrId.actionId === "string" && actionOrId.actionId.trim()) {
    const resolved = resolveGameplayActionRequest(actionOrId.actionId.trim());
    if (resolved) {
      return resolved;
    }
  }
  return normalizeGameplayActionRequest(actionOrId);
}

function sendGameplayAction(actionOrId) {
  const action = resolveGameplayActionRequest(actionOrId);
  if (!action) {
    return { ok: false, reason: "gameplay action is unavailable in the current snapshot" };
  }

  const protocolAction = String(action.protocol_action || "").trim();
  if (protocolAction === "request_snapshot" || protocolAction === "world.request_snapshot") {
    requestSnapshotSafe();
    state.lastGameplayActionFeedback = {
      id: nextRequestId(),
      kind: "gameplay_action",
      action: action.action_id || "request_snapshot",
      agentId: action.target_agent_id || null,
      accepted: true,
      ok: true,
      stage: "ack",
      reason: null,
      effect: "snapshot refresh requested",
      response: {
        action_id: action.action_id || "request_snapshot",
        target_agent_id: action.target_agent_id || "",
        accepted_at_tick: state.logicalTime,
        message: "snapshot refresh requested",
      },
    };
    render();
    return { ok: true, feedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback) };
  }
  if (protocolAction === "live_control.step") {
    return { ok: true, feedback: sendControl("step", { count: 1 }) };
  }
  if (protocolAction === "live_control.play") {
    return { ok: true, feedback: sendControl("play", null) };
  }
  if (protocolAction !== "gameplay_action.submit") {
    return { ok: false, reason: `unsupported gameplay action protocol: ${protocolAction || "(empty)"}` };
  }

  const actionId = String(action.action_id || "").trim();
  const targetAgentId = String(action.target_agent_id || "").trim();
  if (!actionId || !targetAgentId) {
    return { ok: false, reason: "gameplay_action.submit requires action_id and target_agent_id" };
  }
  const disabledReason = String(action.disabled_reason || "").trim();
  if (disabledReason) {
    return { ok: false, reason: disabledReason };
  }

  const feedback = createSemanticFeedback("gameplay_action", actionId, targetAgentId, {
    effect: "queued for signing and send",
    targetAgentId,
    protocolAction,
  });
  state.lastGameplayActionFeedback = feedback;
  render();

  void (async () => {
    try {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability(actionId);
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(targetAgentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        action_id: actionId,
        target_agent_id: targetAgentId,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey,
      };
      if (gameplayActionRequiresActorAgent(actionId)) {
        request.actor_agent_id = state.auth.boundAgentId || targetAgentId;
      }
      request.auth = await buildGameplayActionAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "gameplay action sent; waiting for ack";
      state.lastGameplayActionFeedback = feedback;
      sendJson({
        type: "gameplay_action",
        request,
      });
      render();
    } catch (error) {
      feedback.stage = "error";
      feedback.ok = false;
      feedback.accepted = false;
      feedback.reason = String(error);
      feedback.effect = "gameplay action send failed";
      state.lastGameplayActionFeedback = feedback;
      render();
    }
  })();

  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}

function handleGameplayActionAck(ack) {
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    ack?.action_id || "gameplay_action",
    ack?.target_agent_id || null,
  );
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.message || `gameplay action accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastGameplayActionFeedback = feedback;
  requestSnapshotSafe();
}

function handleGameplayActionError(error) {
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    error?.action_id || "gameplay_action",
    error?.target_agent_id || null,
  );
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "gameplay action failed";
  feedback.effect = error?.code || "gameplay action error";
  feedback.response = clone(error);
  state.lastGameplayActionFeedback = feedback;
}

function applyPromptAckLocally(ack) {
  const agentId = ack?.agent_id;
  if (!agentId || !state.snapshot?.model) {
    return;
  }
  if (!state.snapshot.model.agent_prompt_profiles) {
    state.snapshot.model.agent_prompt_profiles = {};
  }
  const current = state.snapshot.model.agent_prompt_profiles[agentId] || { agent_id: agentId };
  const nextProfile = {
    ...current,
    agent_id: agentId,
    version: Number(ack.version || current.version || 0),
    updated_at_tick: Number(ack.updated_at_tick || state.logicalTime),
    updated_by: state.auth.playerId || current.updated_by || "",
  };
  if (!ack.preview) {
    nextProfile.system_prompt_override = state.promptDraft.systemPrompt || null;
    nextProfile.short_term_goal_override = state.promptDraft.shortTermGoal || null;
    nextProfile.long_term_goal_override = state.promptDraft.longTermGoal || null;
  }
  state.snapshot.model.agent_prompt_profiles[agentId] = nextProfile;
  if (selectedAgentId() === agentId) {
    state.promptDraft = {
      agentId,
      currentVersion: nextProfile.version,
      rollbackTargetVersion: Math.max(0, Number(nextProfile.version || 0) - 1),
      updatedBy: nextProfile.updated_by,
      updatedAtTick: nextProfile.updated_at_tick,
      systemPrompt: String(nextProfile.system_prompt_override || ""),
      shortTermGoal: String(nextProfile.short_term_goal_override || ""),
      longTermGoal: String(nextProfile.long_term_goal_override || ""),
      dirty: false,
    };
  }
}

function handlePromptControlAck(ack) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_ack", ack?.agent_id || null);
  const operation = String(ack?.operation || (ack?.preview ? "preview" : "apply"));
  feedback.stage = ack?.preview ? "preview_ack" : operation === "rollback" ? "rollback_ack" : "apply_ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.preview
    ? `prompt preview ready: version=${ack.version}`
    : operation === "rollback"
      ? `prompt rolled back via version=${ack.version} → target=${Number(ack?.rolled_back_to_version || 0)}`
      : `prompt applied: version=${ack.version}`;
  feedback.response = clone(ack);
  state.lastPromptFeedback = feedback;
  if (ack?.preview) {
    return;
  }
  if (operation === "rollback") {
    state.promptDraft.currentVersion = Number(ack?.version || state.promptDraft.currentVersion || 0);
    state.promptDraft.rollbackTargetVersion = Math.max(0, state.promptDraft.currentVersion - 1);
    state.promptDraft.dirty = false;
    requestSnapshotSafe();
    return;
  }
  applyPromptAckLocally(ack);
}

function handlePromptControlError(error) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_error", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "prompt control failed";
  feedback.effect = error?.code || "prompt control error";
  feedback.response = clone(error);
  state.lastPromptFeedback = feedback;
}

function handleAgentChatAck(ack) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", ack?.agent_id || null);
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = `chat accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastChatFeedback = feedback;
  pushChatHistory({
    id: `chat-ack-${feedback.id}`,
    source: "player",
    agentId: ack?.agent_id || feedback.agentId || null,
    message: feedback.pendingMessage || "",
    tick: Number(ack?.accepted_at_tick || state.logicalTime || 0),
    speaker: feedback.pendingPlayerId || state.auth.playerId || null,
    playerId: feedback.pendingPlayerId || state.auth.playerId || null,
    targetAgentId: ack?.agent_id || feedback.agentId || null,
    intentSeq: ack?.intent_seq || null,
  });
}

function handleAgentChatError(error) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "agent chat failed";
  feedback.effect = error?.code || "agent chat error";
  feedback.response = clone(error);
  state.lastChatFeedback = feedback;
}

function adoptHostedRecoveryAck(ack) {
  if (!ack || !state.auth.available) {
    return;
  }
  const usesLegacyPreviewBootstrap = state.auth.source === "legacy_viewer_auth_bootstrap";
  const hadPendingForceRebind = state.auth.pendingForceRebind === true;
  const previousRequestedAgentId = state.auth.pendingRequestedAgentId;
  const nextBoundAgentId = ack.agent_id || state.auth.boundAgentId || null;
  const nextRequestedAgentId = ack.agent_id || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || null;
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  state.auth.error = null;
  state.auth.revokeReason = null;
  state.auth.revokedBy = null;
  if (ack.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack.session_pubkey) {
    state.auth.publicKey = ack.session_pubkey;
  }
  if (ack.session_epoch != null) {
    state.auth.sessionEpoch = Number(ack.session_epoch);
  }
  state.auth.boundAgentId = nextBoundAgentId;
  state.auth.pendingRequestedAgentId = nextRequestedAgentId;
  state.auth.pendingForceRebind = false;
  if (ack.status === "session_registered" && hadPendingForceRebind) {
    state.auth.rebindNotice = `Player session switched to ${ack.agent_id || previousRequestedAgentId || "requested agent"}.`;
  }
  state.auth.registrationStatus = ack.status === "session_registered" || ack.status === "catch_up_ready"
    ? "registered"
    : ack.status === "session_revoked"
      ? "guest"
      : "issued";
  state.auth.runtimeStatus = ack.status === "session_revoked"
    ? "revoked"
    : nextBoundAgentId
      ? "registered"
      : "registered_unbound";
  if (ack.status === "session_revoked") {
    if (usesLegacyPreviewBootstrap) {
      state.auth.registrationStatus = "issued";
      state.auth.runtimeStatus = "revoked";
      state.auth.error = ack.message || "legacy preview player session was revoked";
      state.auth.pendingRequestedAgentId = null;
      state.auth.pendingForceRebind = false;
    } else {
      void releaseHostedPlayerSlot().catch(() => {});
      resetHostedPlayerAuthState(
        ack.message || "hosted player session was revoked",
        {
          revokeReason: ack.revoke_reason || ack.message || null,
          revokedBy: ack.revoked_by || null,
        },
      );
    }
  } else {
    if (!usesLegacyPreviewBootstrap) {
      persistHostedPlayerSession(state.auth);
      void refreshHostedPlayerLease();
      syncHostedSessionRefreshLoop();
    }
  }
  if (pendingSessionRegisterWaiter && ack.status === "session_registered") {
    const waiter = pendingSessionRegisterWaiter;
    pendingSessionRegisterWaiter = null;
    waiter.resolve(ack);
  }
}

async function recoverHostedSessionFromError(error) {
  if (!canAutoIssueHostedPlayerSession() || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  const code = String(error?.code || "").trim();
  if (recoveryErrorRequiresExplicitRebind(error) && state.auth.pendingRequestedAgentId && !state.auth.pendingForceRebind) {
    await ensureRegisteredPlayerSession(state.auth.pendingRequestedAgentId, { forceRebind: true });
    return;
  }
  if (code === "session_not_found") {
    await ensureRegisteredPlayerSession(latestRequestedAgentId());
    return;
  }
  if (code === "session_revoked") {
    void releaseHostedPlayerSlot().catch(() => {});
    resetHostedPlayerAuthState(
      error?.message || code || "hosted player session failed",
      {
        revokeReason: error?.revoke_reason || error?.message || null,
        revokedBy: error?.revoked_by || null,
      },
    );
    render();
    return;
  }
  if (["session_key_mismatch", "session_player_id_invalid"].includes(code)) {
    void releaseHostedPlayerSlot().catch(() => {});
    resetHostedPlayerAuthState(error?.message || code || "hosted player session failed");
    render();
  }
}

function handleAuthoritativeRecoveryAck(ack) {
  adoptHostedRecoveryAck(ack);
}

function handleAuthoritativeRecoveryError(error) {
  if (
    pendingSessionRegisterWaiter
    && recoveryErrorRequiresExplicitRebind(error)
    && pendingSessionRegisterWaiter.requestedAgentId
    && !pendingSessionRegisterWaiter.forceRebind
  ) {
    state.auth.recoveryErrorCode = error?.code || null;
    state.auth.recoveryErrorMessage = error?.message || null;
    state.auth.error = error?.message || error?.code || "authoritative recovery failed";
    state.auth.registrationStatus = "registering";
    state.auth.runtimeStatus = "rebind_retrying";
    state.auth.pendingForceRebind = true;
    state.auth.rebindNotice = `Requested agent ${state.auth.pendingRequestedAgentId || "-"} needs explicit rebind; retrying now.`;
    markPendingSemanticRebind("explicit rebind required; retrying registration for the requested agent");
    render();
    void retryPendingSessionRegisterWaiterWithForceRebind().catch((retryError) => {
      handleAuthoritativeRecoveryError({
        code: "player_bind_failed",
        message: String(retryError),
      });
    });
    return;
  }
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
    return;
  }
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = error?.code || null;
  state.auth.recoveryErrorMessage = error?.message || null;
  state.auth.error = error?.message || error?.code || "authoritative recovery failed";
  state.auth.revokeReason = error?.revoke_reason || null;
  state.auth.revokedBy = error?.revoked_by || null;
  state.auth.registrationStatus = "issued";
  state.auth.runtimeStatus = error?.code === "session_revoked"
    ? "revoked"
    : error?.code === "session_not_found"
      ? "missing"
      : "error";
  if (!recoveryErrorRequiresExplicitRebind(error)) {
    state.auth.boundAgentId = null;
  }
  clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
  syncHostedSessionRefreshLoop();
  void recoverHostedSessionFromError(error);
}

function handleViewerMessage(message) {
  switch (message?.type) {
    case "hello_ack":
      state.server = message.server || null;
      state.worldId = message.world_id || null;
      state.controlProfile = message.control_profile || "playback";
      state.debugViewerStatus = "subscribed";
      void ensureHostedPlayerAuthAvailable().then(() => {
        syncHostedPlayerSessionOnConnect();
        render();
      });
      break;
    case "snapshot":
      handleSnapshot(message.snapshot);
      break;
    case "event": {
      addRecentEvent(message.event);
      const chatEntry = extractAgentSpokeEntry(message.event);
      if (chatEntry) {
        pushChatHistory(chatEntry);
      }
      state.logicalTime = Math.max(state.logicalTime, Number(message.event?.time || 0));
      state.tick = state.logicalTime;
      break;
    }
    case "metrics":
      handleMetrics(message.time, message.metrics);
      break;
    case "decision_trace":
      handleDecisionTrace(message.trace);
      break;
    case "control_completion_ack":
      handleControlCompletionAck(message.ack);
      break;
    case "prompt_control_ack":
      handlePromptControlAck(message.ack);
      break;
    case "prompt_control_error":
      handlePromptControlError(message.error);
      break;
    case "agent_chat_ack":
      handleAgentChatAck(message.ack);
      break;
    case "agent_chat_error":
      handleAgentChatError(message.error);
      break;
    case "gameplay_action_ack":
      handleGameplayActionAck(message.ack);
      break;
    case "gameplay_action_error":
      handleGameplayActionError(message.error);
      break;
    case "authoritative_recovery_ack":
      handleAuthoritativeRecoveryAck(message.ack);
      break;
    case "authoritative_recovery_error":
      handleAuthoritativeRecoveryError(message.error);
      break;
    case "error":
      reportFatalError(message.message, "viewer");
      break;
    default:
      break;
  }
  updateControlFeedbackFromProgress();
  render();
}

function attachSocket(ws) {
  ws.addEventListener("open", () => {
    state.connectionStatus = "connected";
    state.debugViewerStatus = "detached";
    state.lastError = null;
    sendJson({ type: "hello", client: "viewer", version: 1 });
    sendJson({ type: "subscribe", streams: ["snapshot", "events", "metrics"], event_kinds: [] });
    sendJson({ type: "request_snapshot" });
    syncHostedSessionRefreshLoop();
    render();
  });

  ws.addEventListener("message", (event) => {
    try {
      const message = JSON.parse(String(event.data || "null"));
      handleViewerMessage(message);
    } catch (error) {
      reportFatalError(String(error), "viewer.parse");
    }
  });

  ws.addEventListener("error", () => {
    reportFatalError("websocket error", "viewer.ws");
  });

  ws.addEventListener("close", () => {
    state.connectionStatus = "connecting";
    state.debugViewerStatus = "detached";
    if (state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap") {
      state.auth.syncInFlight = false;
      state.auth.runtimeStatus = "disconnected";
    }
    clearPendingSessionRegisterWaiter("websocket disconnected during player session registration");
    stopHostedSessionRefreshLoop();
    render();
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
    }
    reconnectTimer = window.setTimeout(connect, 1200);
  });
}

function connect() {
  if (socket) {
    try {
      socket.close();
    } catch (_) {
    }
  }
  const params = getSearchParams();
  state.wsUrl = normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
  state.connectionStatus = "connecting";
  render();
  socket = new WebSocket(state.wsUrl);
  attachSocket(socket);
}

function resourceSummary(resources) {
  if (!resources || typeof resources !== "object") {
    return "-";
  }
  return Object.entries(resources)
    .map(([key, value]) => {
      if (value && typeof value === "object") {
        return `${key}:${JSON.stringify(value)}`;
      }
      return `${key}:${value}`;
    })
    .join(" · ") || "-";
}

function modelLists() {
  const { agents, locations } = entityCollections();
  const keyword = selectedSearch.trim().toLowerCase();
  const filter = (entry, label) => {
    if (!keyword) return true;
    return String(label).toLowerCase().includes(keyword);
  };
  return {
    agents: agents
      .filter((agent) => filter(agent, `${agent.id} ${agent.location_id}`))
      .sort((a, b) => String(a.id).localeCompare(String(b.id))),
    locations: locations
      .filter((location) => filter(location, `${location.id} ${location.name}`))
      .sort((a, b) => String(a.id).localeCompare(String(b.id))),
  };
}

function connectionBadgeClass() {
  if (state.connectionStatus === "connected") return "badge badge--good";
  if (state.connectionStatus === "error") return "badge badge--bad";
  return "badge badge--warn";
}

function feedbackBadgeClass(feedback) {
  if (!feedback) return "badge";
  if (feedback.stage === "error") return "badge badge--bad";
  if (feedback.ok) return "badge badge--good";
  return "badge badge--warn";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function renderLists() {
  const { agents, locations } = modelLists();
  const renderItem = (kind, entry, title, meta) => {
    const selected = state.selectedKind === kind && state.selectedId === entry.id;
    return `
      <button class="list-item" data-select-kind="${kind}" data-select-id="${escapeHtml(entry.id)}" data-selected="${selected}">
        <div class="list-item__title">${escapeHtml(title)}</div>
        <div class="list-item__meta">${escapeHtml(meta)}</div>
      </button>
    `;
  };

  elements.leftPanel.innerHTML = `
    <div class="stack">
      <div class="field">
        <label for="entity-search">Filter targets</label>
        <input id="entity-search" type="search" placeholder="Search agents or locations" value="${escapeHtml(selectedSearch)}" />
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Agents</div>
        <div class="list">
          ${agents.length
            ? agents
                .map((agent) =>
                  renderItem(
                    "agent",
                    agent,
                    agent.id,
                    `location=${agent.location_id} · resources=${resourceSummary(agent.resources)}`,
                  ),
                )
                .join("")
            : '<div class="empty">No agents in current snapshot.</div>'}
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Locations</div>
        <div class="list">
          ${locations.length
            ? locations
                .map((location) =>
                  renderItem(
                    "location",
                    location,
                    location.name || location.id,
                    `id=${location.id} · resources=${resourceSummary(location.resources)}`,
                  ),
                )
                .join("")
            : '<div class="empty">No locations in current snapshot.</div>'}
        </div>
      </div>
    </div>
  `;
}

function renderSummary() {
  const controlFeedback = snapshotControlFeedback(state.lastControlFeedback);
  const promptFeedback = snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = snapshotSemanticFeedback(state.lastChatFeedback);
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  const authBadgeClass = state.auth.available ? "badge badge--good" : "badge badge--warn";
  const selectedDebug = selectedAgentExecutionDebugContext();
  const tierBadgeClass = (status) =>
    status === "active" || status === "active_legacy_preview"
      ? "badge badge--good"
      : status === "superseded"
        ? "badge"
        : "badge badge--warn";
  const showRebindNotice = !!state.auth.pendingRequestedAgentId
    && (state.auth.pendingForceRebind
      || state.auth.runtimeStatus === "rebind_retrying"
      || state.auth.runtimeStatus === "rebind_registering");
  const showHostedLoginForm = !state.auth.available
    && String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join";
  elements.centerPanel.innerHTML = `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">viewer</span>
        <span class="${connectionBadgeClass()}">${escapeHtml(state.connectionStatus)}</span>
        <span class="badge">debugViewer=${escapeHtml(`${state.debugViewerMode}:${state.debugViewerStatus}`)}</span>
        <span class="badge">rendererClass=${escapeHtml(state.rendererClass)}</span>
        <span class="badge">controlProfile=${escapeHtml(state.controlProfile)}</span>
      </div>
      <div class="summary-grid">
        <div class="metric"><div class="metric__label">Logical Time</div><div class="metric__value">${state.logicalTime}</div></div>
        <div class="metric"><div class="metric__label">Event Seq</div><div class="metric__value">${state.eventSeq}</div></div>
        <div class="metric"><div class="metric__label">World</div><div class="metric__value">${escapeHtml(state.worldId || "-")}</div></div>
        <div class="metric"><div class="metric__label">Viewer Server</div><div class="metric__value">${escapeHtml(state.server || "-")}</div></div>
      </div>
      <div class="badge-row">
        <span class="badge">ws=${escapeHtml(state.wsUrl || "-")}</span>
        <span class="badge">entryReason=${escapeHtml(state.viewerReason || "-")}</span>
        <span class="badge">renderer=${escapeHtml(state.renderer || "n/a")}</span>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Execution Lanes</div></div>
        <div class="panel__body stack">
          <div class="badge-row">
            <span class="badge badge--accent">debug_viewer</span>
            <span class="badge">status=${escapeHtml(state.debugViewerStatus)}</span>
            <span class="badge">renderMode=${escapeHtml(state.renderMode)}</span>
            <span class="badge">entryReason=${escapeHtml(state.viewerReason || "-")}</span>
          </div>
          <div class="empty" style="margin-top:-2px;">debug_viewer is a read-only subscription lane for runtime snapshots/events; closing the viewer does not stop the agent lane.</div>
          ${selectedDebug
            ? `<div class="badge-row">
                <span class="badge badge--accent">selected agent lane</span>
                <span class="badge">provider=${escapeHtml(selectedDebug.provider_mode || "-")}</span>
                <span class="badge">mode=${escapeHtml(selectedDebug.execution_mode || "-")}</span>
                <span class="badge">env=${escapeHtml(selectedDebug.environment_class || "-")}</span>
              </div>
              <div class="badge-row">
                <span class="badge">obs=${escapeHtml(selectedDebug.observation_schema_version || "-")}</span>
                <span class="badge">act=${escapeHtml(selectedDebug.action_schema_version || "-")}</span>
                <span class="badge">agentProfile=${escapeHtml(selectedDebug.agent_profile || "-")}</span>
                <span class="badge">providerFallback=${escapeHtml(selectedDebug.fallback_reason || "-")}</span>
              </div>
              <pre class="json">${escapeHtml(JSON.stringify(selectedDebug, null, 2))}</pre>`
            : '<div class="empty">Select an agent to compare the headless execution lane against this debug_viewer observer lane.</div>'}
        </div>
      </div>
      <div class="badge-row">
        <span class="${authBadgeClass}">auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}</span>
        <span class="badge badge--accent">tier=${escapeHtml(authSurface.currentTier)}</span>
        <span class="badge">source=${escapeHtml(authSurface.source)}</span>
        <span class="badge">deploymentHint=${escapeHtml(authSurface.deploymentHint)}</span>
        <span class="badge">player=${escapeHtml(state.auth.playerId || "-")}</span>
        <span class="badge">pubkey=${escapeHtml(state.auth.publicKey ? `${state.auth.publicKey.slice(0, 10)}…` : "-")}</span>
        <span class="badge">epoch=${escapeHtml(state.auth.sessionEpoch == null ? "-" : state.auth.sessionEpoch)}</span>
        <span class="badge">runtime=${escapeHtml(state.auth.runtimeStatus || "-")}</span>
        <span class="badge">boundAgent=${escapeHtml(state.auth.boundAgentId || "-")}</span>
        <span class="badge">requestedAgent=${escapeHtml(state.auth.pendingRequestedAgentId || "-")}</span>
        <span class="badge">${escapeHtml(state.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle")}</span>
      </div>
      ${state.auth.recoveryErrorCode || state.auth.recoveryErrorMessage
        ? `<div class="badge-row">
            <span class="badge badge--warn">recoveryError=${escapeHtml(state.auth.recoveryErrorCode || "-")}</span>
            <span class="badge">${escapeHtml(state.auth.recoveryErrorMessage || "-")}</span>
          </div>`
        : ""}
      ${showRebindNotice
        ? `<div class="badge-row">
            <span class="badge badge--accent">rebind</span>
            <span class="badge">target=${escapeHtml(state.auth.pendingRequestedAgentId || "-")}</span>
            <span class="badge">${escapeHtml(state.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry")}</span>
          </div>
          <div class="empty">Player session is switching to the requested agent and the current action will continue after registration succeeds.</div>`
        : ""}
      ${state.auth.rebindNotice
        ? `<div class="empty">${escapeHtml(state.auth.rebindNotice)}</div>`
        : ""}
      ${state.hostedAdmission
        ? `<div class="badge-row">
            <span class="badge">activeSlots=${escapeHtml(`${state.hostedAdmission.active_player_sessions}/${state.hostedAdmission.max_player_sessions}`)}</span>
            <span class="badge">effectiveSlots=${escapeHtml(state.hostedAdmission.effective_player_sessions == null ? "-" : `${state.hostedAdmission.effective_player_sessions}/${state.hostedAdmission.max_player_sessions}`)}</span>
            <span class="badge">runtimeBound=${escapeHtml(state.hostedAdmission.runtime_bound_player_sessions == null ? "-" : state.hostedAdmission.runtime_bound_player_sessions)}</span>
            <span class="badge">runtimeOnly=${escapeHtml(state.hostedAdmission.runtime_only_player_sessions == null ? "-" : state.hostedAdmission.runtime_only_player_sessions)}</span>
            <span class="badge">runtimeProbe=${escapeHtml(state.hostedAdmission.runtime_probe_status || "-")}</span>
            <span class="badge">issueBudget=${escapeHtml(state.hostedAdmission.remaining_issue_budget)}</span>
            <span class="badge">leaseTTL=${escapeHtml(state.hostedAdmission.slot_lease_ttl_ms)}</span>
            <span class="badge">issued=${escapeHtml(state.hostedAdmission.issued_players_total)}</span>
            <span class="badge">released=${escapeHtml(state.hostedAdmission.released_players_total)}</span>
          </div>`
        : ""}
      ${state.hostedAdmission?.runtime_probe_error
        ? `<div class="badge-row">
            <span class="badge badge--warn">runtimeProbeError=${escapeHtml(state.hostedAdmission.runtime_probe_error)}</span>
          </div>`
        : ""}
      ${hostedRecoveryHint
        ? `<div class="panel panel--nested" style="background:rgba(255,255,255,0.02); border-color:rgba(255,184,77,0.35);">
            <div class="panel__header"><div class="panel__title">Hosted Recovery</div></div>
            <div class="panel__body stack">
              <div class="badge-row">
                <span class="badge badge--warn">${escapeHtml(hostedRecoveryHint.kind)}</span>
                <span class="badge">${escapeHtml(hostedRecoveryHint.title)}</span>
              </div>
              <div class="empty">${escapeHtml(hostedRecoveryHint.detail)}</div>
            </div>
          </div>`
        : ""}
      ${showHostedLoginForm
        ? `<div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
            <div class="panel__header"><div class="panel__title">Hosted Account Login</div></div>
            <div class="panel__body stack">
              <div class="empty">Hosted public join now upgrades guest access through a centralized email login before acquiring a player session.</div>
              <div class="control-grid">
                <div class="field">
                  <label for="hosted-login-handle">Email</label>
                  <input id="hosted-login-handle" type="email" autocomplete="email" value="${escapeHtml(state.hostedLogin.handle || "")}" />
                </div>
              </div>
              <div class="toolbar"><button data-auth-action="start-login" ${state.hostedLogin.startInFlight ? "disabled" : ""}>Request Login Code</button></div>
              ${state.hostedLogin.challengeId
                ? `<div class="badge-row">
                    <span class="badge">challenge=${escapeHtml(state.hostedLogin.challengeId)}</span>
                    <span class="badge">target=${escapeHtml(state.hostedLogin.maskedLoginHint || "-")}</span>
                    <span class="badge">delivery=${escapeHtml(state.hostedLogin.deliveryMode || "-")}</span>
                    <span class="badge">${escapeHtml(state.hostedLogin.accountExists ? "account=existing" : "account=new")}</span>
                  </div>
                  ${state.hostedLogin.previewCode
                    ? `<div class="badge-row"><span class="badge badge--accent">previewCode=${escapeHtml(state.hostedLogin.previewCode)}</span></div>`
                    : ""}
                  <div class="field">
                    <label for="hosted-login-code">Verification Code</label>
                    <input id="hosted-login-code" type="text" autocomplete="off" value="${escapeHtml(state.hostedLogin.code || "")}" />
                  </div>
                  <div class="toolbar"><button data-auth-action="complete-login" ${state.hostedLogin.completeInFlight || state.auth.issueInFlight ? "disabled" : ""}>Sign In and Acquire Player Session</button></div>`
                : ""}
              ${state.hostedLogin.error
                ? `<div class="empty">${escapeHtml(state.hostedLogin.error)}</div>`
                : ""}
            </div>
          </div>`
        : ""}
      ${state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap"
        ? `<div class="toolbar"><button data-auth-action="logout">Release Hosted Player Session</button></div>`
        : ""}
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Session Ladder</div></div>
        <div class="panel__body stack">
          <div class="empty">${escapeHtml(authSurface.currentTierReason)}</div>
          <div class="event-list">
            ${authSurface.tiers
              .map(
                (tier) => `
                  <div class="event-card">
                    <div class="event-card__title">
                      <span>${escapeHtml(tier.label)}</span>
                      <span class="${tierBadgeClass(tier.status)}">${escapeHtml(tier.status)}</span>
                    </div>
                    <div class="event-card__meta">${escapeHtml(tier.reason)}</div>
                  </div>`,
              )
              .join("")}
          </div>
          <div class="badge-row">
            <span class="${authSurface.capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn"}">prompt=${escapeHtml(authSurface.capabilities.prompt_control.enabled ? "enabled" : authSurface.capabilities.prompt_control.code)}</span>
            <span class="${authSurface.capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn"}">chat=${escapeHtml(authSurface.capabilities.agent_chat.enabled ? "enabled" : authSurface.capabilities.agent_chat.code)}</span>
            <span class="badge badge--warn">mainToken=${escapeHtml(authSurface.capabilities.main_token_transfer.code)}</span>
          </div>
          <div class="empty">${escapeHtml(authSurface.reconnect)}</div>
        </div>
      </div>
      ${hostedActionMatrixView.length
        ? `<div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
            <div class="panel__header"><div class="panel__title">Hosted Action Matrix</div></div>
            <div class="panel__body stack">
              <div class="empty">This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone.</div>
              <div class="event-list">
                ${hostedActionMatrixView
                  .map(
                    (item) => `
                      <div class="event-card">
                        <div class="event-card__title">
                          <span>${escapeHtml(item.actionId)}</span>
                          <span class="${item.enabled ? "badge badge--good" : "badge badge--warn"}">${escapeHtml(item.enabled ? "enabled" : item.code || "blocked")}</span>
                        </div>
                        <div class="event-card__meta">required_auth=${escapeHtml(item.requiredAuth)} · availability=${escapeHtml(item.availability)}</div>
                        <div class="empty">${escapeHtml(item.reason || "-")}</div>
                        ${item.capabilityReason && item.capabilityReason !== item.reason
                          ? `<div class="empty">viewer=${escapeHtml(item.capabilityReason)}</div>`
                          : ""}
                      </div>`,
                  )
                  .join("")}
              </div>
            </div>
          </div>`
        : ""}
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Playback Controls</div></div>
        <div class="panel__body stack">
          <div class="toolbar">
            <button data-action="play">Play</button>
            <button data-action="pause">Pause</button>
            <button data-action="step">Step x1</button>
          </div>
          <div class="control-grid">
            <div class="field">
              <label for="step-count">Step count</label>
              <input id="step-count" type="number" min="1" step="1" value="3" />
            </div>
            <div class="field" style="align-self:end;">
              <button data-action="step-count">Step custom count</button>
            </div>
          </div>
          ${controlFeedback
            ? `<div class="badge-row">
                <span class="badge">action=${escapeHtml(controlFeedback.action)}</span>
                <span class="badge">stage=${escapeHtml(controlFeedback.stage)}</span>
                <span class="badge">Δtick=${controlFeedback.deltaLogicalTime}</span>
                <span class="badge">Δevent=${controlFeedback.deltaEventSeq}</span>
              </div>
              <pre class="json">${escapeHtml(JSON.stringify(controlFeedback, null, 2))}</pre>`
            : '<div class="empty">No control feedback yet.</div>'}
        </div>
      </div>
      <div class="summary-grid">
        <div class="metric">
          <div class="metric__label">Prompt Feedback</div>
          <div class="metric__value">${escapeHtml(promptFeedback?.stage || "idle")}</div>
          ${promptFeedback ? `<div class="badge-row" style="margin-top:8px;"><span class="${feedbackBadgeClass(promptFeedback)}">${escapeHtml(promptFeedback.effect || promptFeedback.reason || "ready")}</span></div>` : ""}
        </div>
        <div class="metric">
          <div class="metric__label">Chat Feedback</div>
          <div class="metric__value">${escapeHtml(chatFeedback?.stage || "idle")}</div>
          ${chatFeedback ? `<div class="badge-row" style="margin-top:8px;"><span class="${feedbackBadgeClass(chatFeedback)}">${escapeHtml(chatFeedback.effect || chatFeedback.reason || "ready")}</span></div>` : ""}
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Recent Events</div>
        <div class="event-list">
          ${state.recentEvents.length
            ? state.recentEvents
                .map(
                  (event) => `
                    <div class="event-card">
                      <div class="event-card__title">
                        <span>${escapeHtml(summarizeEventTitle(event))}</span>
                        <span>#${Number(event.id || 0)}</span>
                      </div>
                      <div class="event-card__meta">time=${Number(event.time || 0)}</div>
                      <pre class="json">${escapeHtml(JSON.stringify(event.kind, null, 2))}</pre>
                    </div>`,
                )
                .join("")
            : '<div class="empty">Waiting for live events…</div>'}
        </div>
      </div>
    </div>
  `;
}

function renderInteractionPanel() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return '<div class="empty">Select an agent to unlock prompt/chat controls.</div>';
  }
  const binding = selectedAgentBindingInfo();
  const debugContext = selectedAgentExecutionDebugContext();
  const promptFeedback = snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = snapshotSemanticFeedback(state.lastChatFeedback);
  const authSurface = buildAuthSurfaceModel();
  const promptCapability = authSurface.capabilities.prompt_control;
  const chatCapability = authSurface.capabilities.agent_chat;
  const mainTokenTransferCapability = authSurface.capabilities.main_token_transfer;
  const mainTokenTransferPolicy = hostedActionPolicy("main_token_transfer");
  const interactionEnabled = promptCapability.enabled;
  const strongAuthGrantHint = authSurface.capabilities.prompt_control.enabled
    && String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"
    ? `<div class="field">
         <label for="strong-auth-approval-code">Backend Approval Code</label>
         <input id="strong-auth-approval-code" type="password" autocomplete="off" value="${escapeHtml(state.strongAuth.approvalCode || "")}" />
       </div>`
    : "";
  const authNotice = debugContext?.provider_mode === "provider_loopback_http"
    ? `<div class="empty">Selected agent currently runs through the provider-backed loopback bridge in ${escapeHtml(debugContext?.execution_mode || "headless_agent")}; viewer stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.</div>`
    : interactionEnabled
      ? `<div class="badge-row"><span class="badge badge--good">${escapeHtml(authSurface.currentTier)}</span><span class="badge">player=${escapeHtml(state.auth.playerId)}</span><span class="badge">source=${escapeHtml(authSurface.source)}</span></div>
         <div class="empty">${escapeHtml(promptCapability.reason)}</div>`
      : `<div class="empty">${escapeHtml(promptCapability.reason)}</div>`;
  const chatHistory = state.chatHistory
    .filter((entry) => entry.agentId === agentId || entry.targetAgentId === agentId)
    .slice(0, 12);
  const assetLaneStatusText = mainTokenTransferCapability.enabled ? "preview_only" : mainTokenTransferCapability.code || "blocked";
  const assetLaneDetail = mainTokenTransferCapability.enabled
    ? "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here."
    : mainTokenTransferCapability.reason;

  return `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">Agent Interaction</span>
        <span class="badge">agent=${escapeHtml(agentId)}</span>
        <span class="badge">promptVersion=${Number(state.promptDraft.currentVersion || 0)}</span>
      </div>
      ${authNotice}
      <div class="badge-row">
        <span class="badge">boundPlayer=${escapeHtml(binding?.playerId || "-")}</span>
        <span class="badge">boundKey=${escapeHtml(binding?.publicKey ? `${binding.publicKey.slice(0, 10)}…` : "-")}</span>
        <span class="${promptCapability.enabled ? "badge badge--good" : "badge badge--warn"}">prompt=${escapeHtml(promptCapability.enabled ? "enabled" : promptCapability.code)}</span>
        <span class="${chatCapability.enabled ? "badge badge--good" : "badge badge--warn"}">chat=${escapeHtml(chatCapability.enabled ? "enabled" : chatCapability.code)}</span>
        <span class="${mainTokenTransferCapability.enabled ? "badge badge--good" : "badge badge--warn"}">mainToken=${escapeHtml(assetLaneStatusText)}</span>
      </div>
      <div class="empty">${escapeHtml(assetLaneDetail)}</div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Prompt Overrides</div></div>
        <div class="panel__body stack">
          ${strongAuthGrantHint}
          <div class="field">
            <label for="prompt-system">System Prompt Override</label>
            <textarea id="prompt-system" rows="4" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.systemPrompt)}</textarea>
          </div>
          <div class="field">
            <label for="prompt-short">Short-Term Goal Override</label>
            <textarea id="prompt-short" rows="3" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.shortTermGoal)}</textarea>
          </div>
          <div class="field">
            <label for="prompt-long">Long-Term Goal Override</label>
            <textarea id="prompt-long" rows="3" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.longTermGoal)}</textarea>
          </div>
          <div class="toolbar">
            <button data-prompt-action="preview" ${promptCapability.enabled ? "" : "disabled"}>Preview Prompt</button>
            <button data-prompt-action="apply" ${promptCapability.enabled ? "" : "disabled"}>Apply Prompt</button>
          </div>
          <div class="toolbar">
            <div class="field" style="margin:0; min-width:180px; flex:1;">
              <label for="prompt-rollback-version">Rollback Target Version</label>
              <input id="prompt-rollback-version" type="number" min="0" step="1" value="${Number(state.promptDraft.rollbackTargetVersion || 0)}" ${promptCapability.enabled ? "" : "disabled"} />
            </div>
            <button data-prompt-action="rollback" ${promptCapability.enabled ? "" : "disabled"}>Rollback Prompt</button>
          </div>
          ${promptFeedback
            ? `<div class="badge-row"><span class="${feedbackBadgeClass(promptFeedback)}">${escapeHtml(promptFeedback.stage)}</span></div>
               <pre class="json">${escapeHtml(JSON.stringify(promptFeedback, null, 2))}</pre>`
            : '<div class="empty">No prompt feedback yet.</div>'}
          ${state.strongAuth.lastGrantActionId
            ? `<div class="empty">lastGrant=${escapeHtml(state.strongAuth.lastGrantActionId)} expiresAt=${escapeHtml(String(state.strongAuth.lastGrantExpiresAtUnixMs || "-"))}</div>`
            : ""}
          ${state.strongAuth.lastGrantError
            ? `<div class="empty" style="color:var(--bad);">${escapeHtml(state.strongAuth.lastGrantError)}</div>`
            : ""}
        </div>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Asset / Governance Lane</div></div>
        <div class="panel__body stack">
          <div class="badge-row">
            <span class="${mainTokenTransferCapability.enabled ? "badge badge--good" : "badge badge--warn"}">main_token_transfer=${escapeHtml(assetLaneStatusText)}</span>
            <span class="badge">required_auth=${escapeHtml(mainTokenTransferPolicy?.required_auth || "-")}</span>
            <span class="badge">availability=${escapeHtml(mainTokenTransferPolicy?.availability || "-")}</span>
          </div>
          <div class="empty">${escapeHtml(assetLaneDetail)}</div>
          <div class="empty">${escapeHtml(mainTokenTransferPolicy?.reason || "No hosted action policy is available for main_token_transfer on this lane.")}</div>
          <div class="toolbar">
            <button disabled>Main Token Transfer (Not Exposed Here Yet)</button>
          </div>
        </div>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Agent Chat</div></div>
        <div class="panel__body stack">
          <div class="field">
            <label for="agent-chat-message">Message</label>
            <textarea id="agent-chat-message" rows="4" placeholder="Send a message to the selected agent" ${chatCapability.enabled ? "" : "disabled"}>${escapeHtml(state.chatDraft.message)}</textarea>
          </div>
          <div class="toolbar">
            <button data-chat-send="1" ${chatCapability.enabled ? "" : "disabled"}>Send Chat</button>
          </div>
          ${chatFeedback
            ? `<div class="badge-row"><span class="${feedbackBadgeClass(chatFeedback)}">${escapeHtml(chatFeedback.stage)}</span></div>
               <pre class="json">${escapeHtml(JSON.stringify(chatFeedback, null, 2))}</pre>`
            : '<div class="empty">No chat feedback yet.</div>'}
          <div>
            <div class="panel__title" style="margin-bottom:10px;">Message Flow</div>
            <div class="event-list">
              ${chatHistory.length
                ? chatHistory
                    .map(
                      (entry) => `
                        <div class="event-card">
                          <div class="event-card__title">
                            <span>${escapeHtml(entry.source === "player" ? `player → ${entry.targetAgentId || entry.agentId || "agent"}` : `${entry.agentId || "agent"} spoke`)}</span>
                            <span>tick=${Number(entry.tick || 0)}</span>
                          </div>
                          <div class="event-card__meta">speaker=${escapeHtml(entry.speaker || entry.playerId || "-")} · location=${escapeHtml(entry.locationId || "-")}</div>
                          <pre class="json">${escapeHtml(JSON.stringify(entry, null, 2))}</pre>
                        </div>`,
                    )
                    .join("")
                : '<div class="empty">No chat history for this agent yet.</div>'}
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
}

function renderDetails() {
  const selectedLabel = state.selectedKind && state.selectedId
    ? `${state.selectedKind}:${state.selectedId}`
    : "nothing selected";
  elements.rightPanel.innerHTML = `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">Selected</span>
        <span class="badge">${escapeHtml(selectedLabel)}</span>
      </div>
      ${renderInteractionPanel()}
      ${state.selectedObject
        ? `<pre class="json">${escapeHtml(JSON.stringify(clone(state.selectedObject), null, 2))}</pre>`
        : '<div class="empty">Select an agent or location from the left list.</div>'}
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Snapshot Summary</div>
        <pre class="json">${escapeHtml(
          JSON.stringify(
            {
              config: state.snapshot?.config || null,
              counts: {
                agents: Object.keys(state.snapshot?.model?.agents || {}).length,
                locations: Object.keys(state.snapshot?.model?.locations || {}).length,
                promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
                executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length,
              },
    metrics: state.metrics,
    hostedAccess: clone(state.hostedAccess),
            },
            null,
            2,
          ),
        )}</pre>
      </div>
      ${state.lastError
        ? `<div>
            <div class="panel__title" style="margin-bottom:10px; color: var(--bad);">Last Error</div>
            <pre class="json">${escapeHtml(state.lastError)}</pre>
          </div>`
        : ""}
    </div>
  `;
}

function bindEvents() {
  const searchInput = document.getElementById("entity-search");
  if (searchInput) {
    searchInput.addEventListener("input", (event) => {
      selectedSearch = String(event.target.value || "");
      renderLists();
      bindEvents();
    });
  }

  document.querySelectorAll("[data-select-kind][data-select-id]").forEach((button) => {
    button.addEventListener("click", () => {
      applySelection({
        kind: button.getAttribute("data-select-kind"),
        id: button.getAttribute("data-select-id"),
      });
    });
  });

  document.querySelectorAll("[data-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-action");
      if (action === "step-count") {
        const value = Number(document.getElementById("step-count")?.value || 1);
        sendControl("step", { count: Math.max(1, Math.floor(value || 1)) });
        return;
      }
      sendControl(action, null);
    });
  });

  const promptSystem = document.getElementById("prompt-system");
  if (promptSystem) {
    promptSystem.addEventListener("input", (event) => {
      state.promptDraft.systemPrompt = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptShort = document.getElementById("prompt-short");
  if (promptShort) {
    promptShort.addEventListener("input", (event) => {
      state.promptDraft.shortTermGoal = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptLong = document.getElementById("prompt-long");
  if (promptLong) {
    promptLong.addEventListener("input", (event) => {
      state.promptDraft.longTermGoal = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptRollbackVersion = document.getElementById("prompt-rollback-version");
  if (promptRollbackVersion) {
    promptRollbackVersion.addEventListener("input", (event) => {
      const nextValue = Number(event.target.value || 0);
      state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
    });
  }
  const strongAuthApprovalCode = document.getElementById("strong-auth-approval-code");
  if (strongAuthApprovalCode) {
    strongAuthApprovalCode.addEventListener("input", (event) => {
      state.strongAuth.approvalCode = String(event.target.value || "");
    });
  }
  document.querySelectorAll("[data-prompt-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-prompt-action");
      if (action === "rollback") {
        sendPromptControl("rollback", {
          toVersion: Number(state.promptDraft.rollbackTargetVersion || 0),
        });
        return;
      }
      sendPromptControl(action, null);
    });
  });

  const chatMessage = document.getElementById("agent-chat-message");
  if (chatMessage) {
    chatMessage.addEventListener("input", (event) => {
      state.chatDraft.message = String(event.target.value || "");
      state.chatDraft.dirty = true;
    });
  }
  document.querySelectorAll("[data-chat-send]").forEach((button) => {
    button.addEventListener("click", () => {
      sendAgentChat(selectedAgentId(), state.chatDraft.message);
    });
  });
  document.querySelectorAll("[data-auth-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-auth-action");
      if (action === "logout") {
        void logoutHostedPlayerSession();
        return;
      }
      if (action === "start-login") {
        void startHostedAccountLogin();
        return;
      }
      if (action === "complete-login") {
        void completeHostedAccountLogin();
        return;
      }
      if (action === "retry-issue") {
        void retryHostedPlayerIdentityIssue();
      }
    });
  });
  const hostedLoginHandle = document.getElementById("hosted-login-handle");
  if (hostedLoginHandle) {
    hostedLoginHandle.addEventListener("input", (event) => {
      state.hostedLogin.handle = String(event.target.value || "");
      state.hostedLogin.error = null;
    });
  }
  const hostedLoginCode = document.getElementById("hosted-login-code");
  if (hostedLoginCode) {
    hostedLoginCode.addEventListener("input", (event) => {
      state.hostedLogin.code = String(event.target.value || "");
      state.hostedLogin.error = null;
    });
  }
}

function render() {
  renderHook();
}

function requestRender() {
  render();
}

function setStrongAuthApprovalCode(value) {
  state.strongAuth.approvalCode = String(value || "");
  render();
  return {
    ok: true,
    configured: !!state.strongAuth.approvalCode.trim(),
  };
}

function mountApp() {
  const app = document.getElementById("app");
  app.innerHTML = `
    <section class="panel"><div class="panel__header"><div class="panel__title">Targets</div></div><div id="left-panel" class="panel__body"></div></section>
    <section class="panel"><div class="panel__header"><div class="panel__title">World Summary</div></div><div id="center-panel" class="panel__body"></div></section>
    <section class="panel"><div class="panel__header"><div class="panel__title">Details</div></div><div id="right-panel" class="panel__body"></div></section>
  `;
  elements.leftPanel = document.getElementById("left-panel");
  elements.centerPanel = document.getElementById("center-panel");
  elements.rightPanel = document.getElementById("right-panel");
}

function installTestApi() {
  if (!isTestApiEnabled()) {
    return;
  }
  window[TEST_API_GLOBAL_NAME] = {
    getState,
    describeControls,
    fillControlExample,
    sendControl,
    sendGameplayAction,
    runSteps,
    setMode,
    focus,
    select,
    sendAgentChat,
    sendPromptControl,
    setPromptOverridesVisible,
    togglePromptOverridesVisible,
    setStrongAuthApprovalCode,
    injectSnapshot,
    logoutHostedPlayerSession,
    startHostedAccountLogin,
    completeHostedAccountLogin,
    retryHostedPlayerIdentityIssue,
    reportFatalError,
  };
}

function bootstrap() {
  state.uiLocale = resolveInitialUiLocale();
  state.promptOverridesVisible = resolveStoredPromptOverridesVisibility();
  applyUiLocaleToDocument(state.uiLocale);
  Object.assign(state, detectRendererMeta());
  state.hostedAccess = resolveHostedAccessHint();
  state.auth = resolveViewerAuthState();
  state.wsUrl = initialWsUrl();
  window[RENDER_META_GLOBAL_NAME] = Object.freeze({
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion,
  });
  installTestApi();
  render();
  void refreshHostedAdmissionState().then(() => render());
  void ensureHostedPlayerAuthAvailable().then(() => render());
  if (shouldConnectViewerWs()) {
    connect();
  } else {
    state.connectionStatus = "disconnected";
  }
}

function updatePixelWorldRuntimeMeta(meta = {}) {
  if (!meta || typeof meta !== "object") {
    return getState();
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeStatus")) {
    state.pixelWorldRuntimeStatus = meta.runtimeStatus || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeSource")) {
    state.pixelWorldRuntimeSource = meta.runtimeSource || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeModuleUrl")) {
    state.pixelWorldRuntimeModuleUrl = meta.runtimeModuleUrl || null;
  }
  if (Object.prototype.hasOwnProperty.call(meta, "camera")) {
    state.pixelWorldCamera = clone(meta.camera || null);
  }
  if (Object.prototype.hasOwnProperty.call(meta, "fatal")) {
    state.pixelWorldFatal = clone(meta.fatal || null);
  }
  return getState();
}

export function initializeSoftwareSafeCore() {
  if (bootstrapped) {
    return;
  }
  bootstrapped = true;
  bootstrap();
}

window.addEventListener("error", (event) => {
  const message = event?.message || event?.error?.message || "window error";
  reportFatalError(message, "window.error");
});
window.addEventListener("unhandledrejection", (event) => {
  const message = event?.reason?.message || String(event?.reason || "unhandled rejection");
  reportFatalError(message, "window.unhandledrejection");
});

export {
  applySelection,
  bindEvents,
  buildAuthSurfaceModel,
  buildGameplaySummary,
  buildHostedActionMatrixView,
  buildHostedRecoveryHint,
  buildWorldScaleSurface,
  clone,
  connectionBadgeClass,
  describeControls,
  describePromptVersionState,
  describeSemanticFeedback,
  entityCollections,
  feedbackBadgeClass,
  fillControlExample,
  formatPhysicalDistanceCm,
  formatWorldPositionCm,
  focus,
  getState,
  handleControlCompletionAck,
  hostedActionPolicy,
  injectSnapshot,
  modelLists,
  refreshHostedAdmissionState,
  requestRender,
  renderInteractionPanel,
  renderLists,
  renderSummary,
  renderDetails,
  reportFatalError,
  resourceSummary,
  startHostedAccountLogin,
  completeHostedAccountLogin,
  retryHostedPlayerIdentityIssue,
  runSteps,
  select,
  selectedAgentBindingInfo,
  selectedAgentExecutionDebugContext,
  selectedAgentId,
  sendAgentChat,
  sendControl,
  sendGameplayAction,
  sendPromptControl,
  setMode,
  setStrongAuthApprovalCode,
  updatePixelWorldRuntimeMeta,
  snapshotControlFeedback,
  snapshotSemanticFeedback,
  summarizeEventTitle,
  logoutHostedPlayerSession,
};
