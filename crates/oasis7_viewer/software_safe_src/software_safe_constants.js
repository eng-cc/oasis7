export const TEST_API_GLOBAL_NAME = "__AW_TEST__";
export const RENDER_META_GLOBAL_NAME = "__AW_VIEWER_RENDER_META__";
export const VIEWER_RENDER_MODE = "viewer";
export const SOFTWARE_SAFE_RENDER_MODE_ALIAS = "software_safe";
export const VIEWER_AUTH_BOOTSTRAP_OBJECT = "__OASIS7_VIEWER_AUTH_ENV";
export const VIEWER_PLAYER_ID_KEY = "OASIS7_VIEWER_PLAYER_ID";
export const VIEWER_AUTH_PUBLIC_KEY = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
export const VIEWER_AUTH_PRIVATE_KEY = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
export const VIEWER_AUTH_SIGNATURE_PREFIX = "awviewauth:v1:";
export const HOSTED_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.hosted_player_session.v1";
export const UI_LOCALE_STORAGE_PREFIX = "oasis7.viewer.locale.v1";
export const PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX = "oasis7.viewer.prompt_overrides_visible.v1";
export const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE = "/api/public/player-session/admission";
export const HOSTED_PLAYER_SESSION_REFRESH_ROUTE = "/api/public/player-session/refresh";
export const HOSTED_PLAYER_SESSION_RELEASE_ROUTE = "/api/public/player-session/release";
export const HOSTED_ACCOUNT_LOGIN_START_ROUTE = "/api/public/hosted-account/login/start";
export const HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE = "/api/public/hosted-account/login/complete";
export const HOSTED_STRONG_AUTH_GRANT_ROUTE = "/api/public/strong-auth/grant";
export const HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS = 30000;
export const DEFAULT_WS_ADDR = "ws://127.0.0.1:5011";
export const MAX_EVENTS = 24;
export const MAX_DECISION_TRACES = 12;
export const SOFTWARE_RENDERER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "software rasterizer",
  "basic render driver",
  "softpipe",
  "lavapipe",
];
