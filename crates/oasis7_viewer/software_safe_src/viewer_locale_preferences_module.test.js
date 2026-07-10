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

  it("uses the legacy language query when locale is invalid", () => {
    setViewerPath("/software_safe.html?locale=fr&language=zh&test_api=1&connect=0");

    expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");
  });

  it("migrates legacy alias-scoped UI locale preferences into the shared viewer key", () => {
    window.localStorage.setItem("oasis7:viewer:ui-locale:/software_safe.html", "zh");

    setViewerPath("/software_safe.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");
    expect(window.localStorage.getItem("oasis7:viewer:ui-locale:viewer")).toBe("zh");

    setViewerPath("/viewer.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");
  });

  it("keeps legacy UI locale readable when shared-key migration cannot be written", () => {
    window.localStorage.setItem("oasis7:viewer:ui-locale:/software_safe.html", "zh");
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage quota exceeded");
    });

    try {
      setViewerPath("/software_safe.html?test_api=1&connect=0");
      expect(createPreferencesModule().resolveInitialUiLocale()).toBe("zh");
    } finally {
      setItemSpy.mockRestore();
    }
  });

  it("keeps UI locale updates usable when localStorage is unavailable", () => {
    const localStorageDescriptor = Object.getOwnPropertyDescriptor(window, "localStorage");
    const module = createPreferencesModule();
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("localStorage unavailable");
      },
    });

    try {
      expect(module.setViewerLocale("zh")).toBe("zh");
      expect(document.documentElement.lang).toBe("zh-CN");
    } finally {
      Object.defineProperty(window, "localStorage", localStorageDescriptor);
    }
  });

  it("keeps UI locale updates usable when history state cannot be rewritten", () => {
    const module = createPreferencesModule();
    const replaceStateSpy = vi.spyOn(window.history, "replaceState").mockImplementation(() => {
      throw new Error("history state unavailable");
    });

    try {
      expect(module.setViewerLocale("zh")).toBe("zh");
      expect(document.documentElement.lang).toBe("zh-CN");
      expect(window.localStorage.getItem("oasis7:viewer:ui-locale:viewer")).toBe("zh");
    } finally {
      replaceStateSpy.mockRestore();
    }
  });

  it("shares prompt override visibility across viewer entrypoint aliases", () => {
    setViewerPath("/viewer.html?test_api=1&connect=0");
    createPreferencesModule().setPromptOverridesVisible(true);

    setViewerPath("/software_safe.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);

    setViewerPath("/?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);
  });

  it("migrates legacy alias-scoped prompt override visibility into the shared viewer key", () => {
    window.localStorage.setItem("oasis7:viewer:prompt-overrides-visible:/viewer.html", "1");

    setViewerPath("/viewer.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);
    expect(window.localStorage.getItem("oasis7:viewer:prompt-overrides-visible:viewer")).toBe("1");

    setViewerPath("/software_safe.html?test_api=1&connect=0");
    expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);
  });

  it("keeps legacy prompt override visibility readable when shared-key migration cannot be written", () => {
    window.localStorage.setItem("oasis7:viewer:prompt-overrides-visible:/viewer.html", "1");
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage quota exceeded");
    });

    try {
      setViewerPath("/viewer.html?test_api=1&connect=0");
      expect(createPreferencesModule().resolveStoredPromptOverridesVisibility()).toBe(true);
    } finally {
      setItemSpy.mockRestore();
    }
  });

  it("keeps prompt override visibility updates usable when localStorage is unavailable", () => {
    const localStorageDescriptor = Object.getOwnPropertyDescriptor(window, "localStorage");
    const module = createPreferencesModule();
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("localStorage unavailable");
      },
    });

    try {
      expect(module.setPromptOverridesVisible(true)).toBe(true);
    } finally {
      Object.defineProperty(window, "localStorage", localStorageDescriptor);
    }
  });

  it("keeps non-alias viewer paths isolated from alias legacy preferences", () => {
    window.localStorage.setItem("oasis7:viewer:ui-locale:/viewer.html", "zh");
    window.localStorage.setItem("oasis7:viewer:prompt-overrides-visible:/viewer.html", "1");

    setViewerPath("/tools/viewer.html?test_api=1&connect=0");
    const module = createPreferencesModule();

    expect(module.resolveInitialUiLocale()).toBe("en");
    expect(module.resolveStoredPromptOverridesVisibility()).toBe(false);
    expect(window.localStorage.getItem("oasis7:viewer:ui-locale:/tools/viewer.html")).toBe(null);
    expect(window.localStorage.getItem("oasis7:viewer:prompt-overrides-visible:/tools/viewer.html")).toBe(null);
  });
});
