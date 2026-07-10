export function createViewerLocalePreferencesModule({
  documentRef,
  getSearchParams,
  normalizeUiLocale,
  promptOverridesVisibilityStoragePrefix,
  renderViewer,
  state,
  uiLocaleStoragePrefix,
  windowRef,
}) {
  const viewerEntryAliasSegments = ["/viewer.html", "/software_safe.html", "/"];

  function viewerEntryStorageSegment() {
    const pathname = windowRef.location.pathname || "/viewer.html";
    if (viewerEntryAliasSegments.includes(pathname)) {
      return "viewer";
    }
    return pathname;
  }

  function legacyViewerEntryStorageSegments() {
    const pathname = windowRef.location.pathname || "/viewer.html";
    if (viewerEntryStorageSegment() !== "viewer") {
      return [pathname];
    }
    return [pathname, ...viewerEntryAliasSegments].filter((segment, index, segments) => (
      segment !== "viewer" && segments.indexOf(segment) === index
    ));
  }

  function uiLocaleStorageKey() {
    return `${uiLocaleStoragePrefix}:${viewerEntryStorageSegment()}`;
  }

  function legacyUiLocaleStorageKeys() {
    return legacyViewerEntryStorageSegments().map((segment) => `${uiLocaleStoragePrefix}:${segment}`);
  }

  function trySetStorageItem(getStorage, key, value) {
    try {
      getStorage()?.setItem(key, value);
    } catch (_) {
    }
  }

  function persistUiLocale(locale) {
    trySetStorageItem(() => windowRef.localStorage, uiLocaleStorageKey(), locale);
  }

  function resolveStoredUiLocale() {
    try {
      const storage = windowRef.localStorage;
      const storedLocale = normalizeUiLocale(storage?.getItem(uiLocaleStorageKey()));
      if (storedLocale) {
        return storedLocale;
      }
      for (const legacyKey of legacyUiLocaleStorageKeys()) {
        const legacyLocale = normalizeUiLocale(storage?.getItem(legacyKey));
        if (legacyLocale) {
          trySetStorageItem(() => storage, uiLocaleStorageKey(), legacyLocale);
          return legacyLocale;
        }
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  function resolveInitialUiLocale() {
    const params = getSearchParams();
    return normalizeUiLocale(params.get("locale"))
      || normalizeUiLocale(params.get("language"))
      || resolveStoredUiLocale()
      || "en";
  }

  function promptOverridesVisibilityStorageKey() {
    return `${promptOverridesVisibilityStoragePrefix}:${viewerEntryStorageSegment()}`;
  }

  function legacyPromptOverridesVisibilityStorageKeys() {
    return legacyViewerEntryStorageSegments().map((segment) => (
      `${promptOverridesVisibilityStoragePrefix}:${segment}`
    ));
  }

  function persistPromptOverridesVisibility(visible) {
    trySetStorageItem(() => windowRef.localStorage, promptOverridesVisibilityStorageKey(), visible ? "1" : "0");
  }

  function resolveStoredPromptOverridesVisibility() {
    try {
      const storage = windowRef.localStorage;
      const storedValue = storage?.getItem(promptOverridesVisibilityStorageKey());
      if (storedValue !== null && storedValue !== undefined) {
        return storedValue === "1";
      }
      for (const legacyKey of legacyPromptOverridesVisibilityStorageKeys()) {
        const legacyValue = storage?.getItem(legacyKey);
        if (legacyValue !== null && legacyValue !== undefined) {
          trySetStorageItem(() => storage, promptOverridesVisibilityStorageKey(), legacyValue);
          return legacyValue === "1";
        }
      }
      return false;
    } catch (_) {
      return false;
    }
  }

  function applyUiLocaleToDocument(locale) {
    documentRef.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
  }

  function updateUiLocaleQuery(locale) {
    try {
      const url = new URL(windowRef.location.href);
      url.searchParams.set("locale", locale);
      url.searchParams.delete("language");
      windowRef.history.replaceState({}, "", url.toString());
    } catch (_) {
    }
  }

  function setViewerLocale(locale) {
    const normalized = normalizeUiLocale(locale);
    if (!normalized) {
      return state.uiLocale;
    }
    state.uiLocale = normalized;
    persistUiLocale(normalized);
    applyUiLocaleToDocument(normalized);
    updateUiLocaleQuery(normalized);
    renderViewer();
    return state.uiLocale;
  }

  function toggleViewerLocale() {
    return setViewerLocale(state.uiLocale === "zh" ? "en" : "zh");
  }

  function setPromptOverridesVisible(visible) {
    state.promptOverridesVisible = !!visible;
    persistPromptOverridesVisibility(state.promptOverridesVisible);
    renderViewer();
    return state.promptOverridesVisible;
  }

  function togglePromptOverridesVisible() {
    return setPromptOverridesVisible(!state.promptOverridesVisible);
  }

  return {
    applyUiLocaleToDocument,
    resolveInitialUiLocale,
    resolveStoredPromptOverridesVisibility,
    setPromptOverridesVisible,
    setViewerLocale,
    togglePromptOverridesVisible,
    toggleViewerLocale,
  };
}
