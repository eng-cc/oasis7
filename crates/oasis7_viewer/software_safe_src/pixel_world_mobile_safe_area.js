import { applyPixelWorldMarkerClearance, observePixelWorldMarkerPanels } from './pixel_world_marker_clearance.js';
import { applyRendererDecisionClearance } from './pixel_world_renderer_decision_area.js';
const MOBILE_SHELL_MAX_WIDTH = 640;
const SAFE_AREA_GAP_PX = 8;

export function pixelWorldMobileSelectionOffset({ markerTop, markerBottom, commandTop, feedBottom = 0 }) {
  const clearCommandOffset = Math.min(0, commandTop - SAFE_AREA_GAP_PX - markerBottom);
  const clearFeedOffset = feedBottom + SAFE_AREA_GAP_PX - markerTop;
  // The marker must satisfy both edges: above the bottom decision band and
  // below the top Feed band. When the bands leave no room for its hit box,
  // prioritize the command edge so the selected target remains reachable.
  if (clearCommandOffset < 0) return clearCommandOffset;
  if (clearFeedOffset > 0) return clearFeedOffset;
  return 0;
}

export function pixelWorldMobileSelectionChipOffset({ chipTop, feedBottom = 0, feedOpen = false }) {
  if (feedOpen || !Number.isFinite(chipTop) || !Number.isFinite(feedBottom) || feedBottom <= 0) return 0;
  return Math.max(0, feedBottom + SAFE_AREA_GAP_PX - chipTop);
}

export function pixelWorldMobileFocusSelectionOffset({ markerLeft, hudRight }) {
  return hudRight + SAFE_AREA_GAP_PX - markerLeft;
}

function applyMobileSelectionSafeArea(canvasRoot) {
  if (canvasRoot?.dataset.rendererProjection === 'true') return;
  const marker = canvasRoot?.querySelector(".pixel-world-entity--canvas-hit-target[data-selected='true']");
  const selectionChip = canvasRoot?.querySelector(".pixel-world-canvas__selection");
  const feed = document.querySelector('[data-viewer-overlay="feed"]');
  const feedVisible = feed && getComputedStyle(feed).display !== "none";
  const feedRect = feedVisible ? feed.getBoundingClientRect() : null;
  selectionChip?.style.setProperty("translate", "");
  marker?.style.setProperty("translate", "");
  if (window.innerWidth > MOBILE_SHELL_MAX_WIDTH) return;
  if (selectionChip && feedRect && !feed.open) {
    const chipRect = selectionChip.getBoundingClientRect();
    const chipOffset = pixelWorldMobileSelectionChipOffset({
      chipTop: chipRect.top,
      feedBottom: feedRect.bottom,
      feedOpen: feed.open,
    });
    selectionChip.style.translate = `0 ${Math.ceil(chipOffset)}px`;
  }
  if (!marker) return;
  const focusHost = canvasRoot.closest(".pixel-world-host--focus");
  if (focusHost) {
    const focusHud = focusHost.querySelector("[data-focus-hud='true']");
    if (!focusHud) return;
    const markerRect = marker.getBoundingClientRect();
    const focusHudRect = focusHud.getBoundingClientRect();
    const offset = pixelWorldMobileFocusSelectionOffset({ markerLeft: markerRect.left, hudRight: focusHudRect.right });
    marker.style.translate = `${Math.ceil(offset)}px 0`;
    return;
  }
  const command = document.querySelector('[data-viewer-overlay="next-move"]');
  if (!command || getComputedStyle(command).display === "none") return;
  const markerRect = marker.getBoundingClientRect();
  const commandRect = command.getBoundingClientRect();
  const offset = pixelWorldMobileSelectionOffset({
    markerTop: markerRect.top,
    markerBottom: markerRect.bottom,
    commandTop: commandRect.top,
    feedBottom: feedRect?.bottom || 0,
  });
  marker.style.translate = `0 ${Math.floor(offset)}px`;
}

export function applyPixelWorldMobileSelectionSafeArea(canvasRoot) {
  applyRendererDecisionClearance(canvasRoot);
  applyMobileSelectionSafeArea(canvasRoot);
  applyPixelWorldMarkerClearance(canvasRoot);
}

export function installPixelWorldMobileSelectionSafeArea(canvasRoot) {
  const sync = () => applyPixelWorldMobileSelectionSafeArea(canvasRoot());
  window.addEventListener("resize", sync);
  const focusStateObserver = new MutationObserver(() => requestAnimationFrame(sync));
  focusStateObserver.observe(document.body, { attributes: true, attributeFilter: ["class"] });
  const feed = document.querySelector('[data-viewer-overlay="feed"]');
  feed?.addEventListener("toggle", sync, true);
  requestAnimationFrame(sync);
  const stopObservingPanels = observePixelWorldMarkerPanels(() => requestAnimationFrame(sync));
  return () => {
    stopObservingPanels();
    window.removeEventListener("resize", sync);
    focusStateObserver.disconnect();
    feed?.removeEventListener("toggle", sync, true);
  };
}
