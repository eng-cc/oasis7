const GAP = 8;
const PANELS = '[data-viewer-overlay="feed"], [data-viewer-overlay="next-move"], [data-viewer-overlay="receipt"], [data-viewer-overlay="navigation"], [data-viewer-overlay="cinematic-entry"], [data-focus-hud="true"], .pixel-world-canvas__selection, .pixel-world-canvas__sparse-guidance, .pixel-world-canvas__legend';

// Find the nearest free presentation position; entity/world coordinates stay intact.
export function pixelWorldMarkerClearance(marker, panels, bounds, fallback = { x: 0, y: 0 }) {
  const width = marker.right - marker.left;
  const height = marker.bottom - marker.top;
  const xs = [marker.left, bounds.left + GAP, bounds.right - width - GAP];
  const ys = [marker.top, bounds.top + GAP, bounds.bottom - height - GAP];
  for (const panel of panels) {
    xs.push(panel.left - width - GAP, panel.right + GAP);
    ys.push(panel.top - height - GAP, panel.bottom + GAP);
  }
  let best = null;
  let distance = Infinity;
  for (const left of xs) for (const top of ys) {
    const right = left + width;
    const bottom = top + height;
    if (left < bounds.left + GAP || right > bounds.right - GAP || top < bounds.top + GAP || bottom > bounds.bottom - GAP) continue;
    if (panels.some((panel) => left < panel.right + GAP && right > panel.left - GAP && top < panel.bottom + GAP && bottom > panel.top - GAP)) continue;
    const nextDistance = (left - marker.left) ** 2 + (top - marker.top) ** 2;
    if (nextDistance < distance) {
      distance = nextDistance;
      best = { x: left - marker.left, y: top - marker.top };
    }
  }
  return best || fallback;
}

export function applyPixelWorldMarkerClearance(canvasRoot) {
  if (!canvasRoot) return;
  if (canvasRoot.dataset.rendererProjection === 'true') return;
  const markers = [...canvasRoot.querySelectorAll('button.pixel-world-entity, button.pixel-world-hotspot')];
  const fallbacks = new Map();
  for (const marker of markers) {
    // The mobile pass already prioritizes the selected command edge when the
    // panels leave no free hit box. Retain that fallback, never stale offsets.
    const selected = marker.matches(".pixel-world-entity--canvas-hit-target[data-selected='true']");
    const previous = selected ? marker.getBoundingClientRect() : null;
    marker.style.translate = '';
    if (previous) {
      const reset = marker.getBoundingClientRect();
      fallbacks.set(marker, { x: previous.left - reset.left, y: previous.top - reset.top });
    }
  }
  const visibleRect = (node) => {
    if (getComputedStyle(node).visibility === 'hidden' || getComputedStyle(node).display === 'none') return null;
    const rect = node.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 ? rect : null;
  };
  const panels = [...document.querySelectorAll(PANELS)].map(visibleRect).filter(Boolean);
  const canvas = canvasRoot.getBoundingClientRect();
  const bounds = { left: Math.max(0, canvas.left), top: Math.max(0, canvas.top), right: Math.min(window.innerWidth, canvas.right), bottom: Math.min(window.innerHeight, canvas.bottom) };
  const positions = new Map();
  for (const marker of markers) {
    const rect = visibleRect(marker);
    if (!rect) continue;
    const key = marker.dataset.agentId ? `agent:${marker.dataset.agentId}` : marker;
    const offset = positions.get(key) || pixelWorldMarkerClearance(rect, panels, bounds, fallbacks.get(marker));
    marker.style.translate = `${offset.x}px ${offset.y}px`;
    if (!positions.has(key)) panels.push({ left: rect.left + offset.x, right: rect.right + offset.x, top: rect.top + offset.y, bottom: rect.bottom + offset.y });
    positions.set(key, offset);
  }
}

export function observePixelWorldMarkerPanels(sync) {
  if (typeof ResizeObserver === 'undefined') return () => {};
  const observer = new ResizeObserver(sync);
  for (const panel of document.querySelectorAll(PANELS)) observer.observe(panel);
  return () => observer.disconnect();
}
