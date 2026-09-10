const CLEARANCE = 8;
const MIN_PANEL_HEIGHT = 160;

export function rendererDecisionMaxHeight(panel, viewportHeight, hotspots) {
  let height = Math.max(MIN_PANEL_HEIGHT, viewportHeight / 2 - 76);
  for (const hotspot of hotspots) {
    if (hotspot.right <= panel.left || hotspot.left >= panel.right || hotspot.bottom <= 0 || hotspot.top >= viewportHeight) continue;
    height = Math.min(height, panel.bottom - hotspot.bottom - CLEARANCE);
  }
  // An edge event must not make primary actions or receipts unreachable.
  // Keep a scrollable decision surface even when complete clearance is impossible.
  return Math.max(MIN_PANEL_HEIGHT, Math.floor(height));
}

export function applyRendererDecisionClearance(canvasRoot) {
  if (canvasRoot?.dataset.rendererProjection !== 'true') return;
  const host = canvasRoot.closest('.pixel-world-host');
  const panel = host?.querySelector('.pixel-world-decision-area');
  if (!panel) return;
  const hotspots = [...canvasRoot.querySelectorAll('.pixel-world-hotspot[data-renderer-target="true"]')]
    .filter(node => getComputedStyle(node).display !== 'none')
    .map(node => node.getBoundingClientRect());
  const height = rendererDecisionMaxHeight(panel.getBoundingClientRect(),window.innerHeight,hotspots);
  panel.style.setProperty('--renderer-decision-max-height',`${height}px`);
}
