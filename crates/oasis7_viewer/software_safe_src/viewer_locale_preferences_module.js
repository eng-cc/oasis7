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
  function uiLocaleStorageKey() {
    return `${uiLocaleStoragePrefix}:${windowRef.location.pathname || "viewer.html"}`;
  }

  function persistUiLocale(locale) {
    try {
      windowRef.localStorage?.setItem(uiLocaleStorageKey(), locale);
    } catch (_) {
    }
  }

  function resolveStoredUiLocale() {
    try {
      return normalizeUiLocale(windowRef.localStorage?.getItem(uiLocaleStorageKey()));
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

  function promptOverridesVisibilityStorageKey() {
    return `${promptOverridesVisibilityStoragePrefix}:${windowRef.location.pathname || "viewer.html"}`;
  }

  function persistPromptOverridesVisibility(visible) {
    try {
      windowRef.localStorage?.setItem(promptOverridesVisibilityStorageKey(), visible ? "1" : "0");
    } catch (_) {
    }
  }

  function resolveStoredPromptOverridesVisibility() {
    try {
      return windowRef.localStorage?.getItem(promptOverridesVisibilityStorageKey()) === "1";
    } catch (_) {
      return false;
    }
  }

  function applyUiLocaleToDocument(locale) {
    documentRef.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
  }

  function updateUiLocaleQuery(locale) {
    const url = new URL(windowRef.location.href);
    url.searchParams.set("locale", locale);
    url.searchParams.delete("language");
    windowRef.history.replaceState({}, "", url.toString());
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
