const MARGIN = 10;
const MIN_HEIGHT = 56;

export function hotspotTooltipSafeBand(summary, command, viewportHeight, preferredTop = 52) {
  const obstacles = [summary, command].filter(Boolean).map((rect) => ({
    top: Math.max(MARGIN, rect.top - 4),
    bottom: Math.min(viewportHeight - MARGIN, (rect.bottom ?? viewportHeight) + 4),
  })).sort((a, b) => a.top - b.top);
  const bands = [];
  let cursor = MARGIN;
  for (const obstacle of obstacles) {
    if (obstacle.top - cursor >= MIN_HEIGHT) bands.push({ top: cursor, bottom: obstacle.top });
    cursor = Math.max(cursor, obstacle.bottom);
  }
  if (viewportHeight - MARGIN - cursor >= MIN_HEIGHT) bands.push({ top: cursor, bottom: viewportHeight - MARGIN });
  // Keep the close reachable even on smaller viewports with no unoccupied HUD band.
  if (!bands.length) bands.push({ top: MARGIN, bottom: Math.max(MARGIN + MIN_HEIGHT, viewportHeight - MARGIN) });
  const distance = (band) => Math.abs(Math.max(band.top, Math.min(band.bottom - MIN_HEIGHT, preferredTop)) - preferredTop);
  bands.sort((a, b) => distance(a) - distance(b));
  const band = bands[0];
  return { top: band.top, maxHeight: band.bottom - band.top };
}

export function installHotspotTooltipPlacement(tooltip) {
  const feed = document.querySelector('[data-viewer-overlay="feed"]');
  const command = document.querySelector('[data-viewer-overlay="next-move"]');
  const summary = feed?.querySelector("summary");
  const visibleRect = (node) => node && getComputedStyle(node).display !== "none" ? node.getBoundingClientRect() : null;
  const update = () => {
    tooltip.style.removeProperty("top");
    tooltip.style.removeProperty("bottom");
    const preferredTop = tooltip.getBoundingClientRect().top;
    const band = hotspotTooltipSafeBand(visibleRect(summary), visibleRect(command), window.innerHeight, preferredTop);
    tooltip.style.top = `${band.top}px`;
    tooltip.style.bottom = "auto";
    tooltip.style.setProperty("--hotspot-tooltip-max-height", `${band.maxHeight}px`);
  };
  update();
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
  for (const node of [summary, command]) if (node) observer?.observe(node);
  window.addEventListener("resize", update);
  feed?.addEventListener("toggle", update);
  return () => { observer?.disconnect(); window.removeEventListener("resize", update); feed?.removeEventListener("toggle", update); };
}
