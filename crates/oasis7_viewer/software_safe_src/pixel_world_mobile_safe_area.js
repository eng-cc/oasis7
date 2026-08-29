const MOBILE_SHELL_MAX_WIDTH = 640;
const SAFE_AREA_GAP_PX = 8;

export function pixelWorldMobileSelectionOffset({ markerTop, markerBottom, commandTop, feedBottom = 0 }) {
  const clearCommandOffset = Math.min(0, commandTop - SAFE_AREA_GAP_PX - markerBottom);
  const clearFeedOffset = feedBottom + SAFE_AREA_GAP_PX - markerTop;
  return Math.max(clearCommandOffset, clearFeedOffset);
}

export function pixelWorldMobileFocusSelectionOffset({ markerLeft, hudRight }) {
  return hudRight + SAFE_AREA_GAP_PX - markerLeft;
}

export function applyPixelWorldMobileSelectionSafeArea(canvasRoot) {
  const marker = canvasRoot?.querySelector(".pixel-world-entity--canvas-hit-target[data-selected='true']");
  if (!marker) return;
  marker.style.translate = "";
  if (window.innerWidth > MOBILE_SHELL_MAX_WIDTH) return;
  const focusHost = canvasRoot.closest(".pixel-world-host--focus");
  if (focusHost) {
    const focusHud = focusHost.querySelector(".pixel-world-hud");
    if (!focusHud) return;
    const markerRect = marker.getBoundingClientRect();
    const focusHudRect = focusHud.getBoundingClientRect();
    const offset = pixelWorldMobileFocusSelectionOffset({ markerLeft: markerRect.left, hudRight: focusHudRect.right });
    marker.style.translate = `${Math.ceil(offset)}px 0`;
    return;
  }
  const command = document.querySelector('[data-viewer-overlay="next-move"]');
  if (!command || getComputedStyle(command).display === "none") return;
  const feed = document.querySelector('[data-viewer-overlay="feed"]');
  const markerRect = marker.getBoundingClientRect();
  const commandRect = command.getBoundingClientRect();
  const feedRect = feed && getComputedStyle(feed).display !== "none" ? feed.getBoundingClientRect() : null;
  const offset = pixelWorldMobileSelectionOffset({
    markerTop: markerRect.top,
    markerBottom: markerRect.bottom,
    commandTop: commandRect.top,
    feedBottom: feedRect?.bottom || 0,
  });
  marker.style.translate = `0 ${Math.floor(offset)}px`;
}

export function installPixelWorldMobileSelectionSafeArea(canvasRoot) {
  const sync = () => applyPixelWorldMobileSelectionSafeArea(canvasRoot());
  window.addEventListener("resize", sync);
  const focusStateObserver = new MutationObserver(() => requestAnimationFrame(sync));
  focusStateObserver.observe(document.body, { attributes: true, attributeFilter: ["class"] });
  requestAnimationFrame(sync);
  return () => {
    window.removeEventListener("resize", sync);
    focusStateObserver.disconnect();
  };
}
