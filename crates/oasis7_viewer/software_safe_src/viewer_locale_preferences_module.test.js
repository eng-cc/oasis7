import { beforeEach, describe, expect, it, vi } from "vitest";
import { createViewerLocalePreferencesModule } from "./viewer_locale_preferences_module.js";

function normalizeUiLocale(locale) {
  return locale === "zh" || locale === "en" ? locale : null;
}

function setViewerPath(path) {
  window.history.replaceState({}, "", `http://127.0.0.1:4173${path}`);
}

function createPreferencesModule(stateOverrides = {}) {
  return createViewerLocalePreferencesModule({
    documentRef: document,
    getSearchParams: () => new URL(window.location.href).searchParams,
    normalizeUiLocale,
    promptOverridesVisibilityStoragePrefix: "oasis7:viewer:prompt-overrides-visible",
    renderViewer: vi.fn(),
    state: {
      promptOverridesVisible: false,
      uiLocale: "en",
      ...stateOverrides,
    },
    uiLocaleStoragePrefix: "oasis7:viewer:ui-locale",
    windowRef: window,
  });
}

describe("viewer locale preferences module", () => {
  beforeEach(() => {
    window.localStorage.clear();
    setViewerPath("/software_safe.html?test_api=1&connect=0");
  });

  it("shares stored UI locale across viewer entrypoint aliases", () => {
    setViewerPath("/viewer.html?test_api=1&connect=0");
    createPreferencesModule().setViewerLocale("zh");

    setViewerPath("/software_safe.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");

    setViewerPath("/?test_api=1&connect=0");
    expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");
  });

  it("shares prompt override visibility across viewer entrypoint aliases", () => {
    setViewerPath("/viewer.html?test_api=1&connect=0");
    createPreferencesModule().setPromptOverridesVisible(true);

    setViewerPath("/software_safe.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);

    setViewerPath("/?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);
  });
});
