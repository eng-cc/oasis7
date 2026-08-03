const HOTSPOT_POINTER_PROBE_GLOBAL = "__OASIS7_PIXEL_WORLD_HOTSPOT_POINTER_PROBE__";

function canvasClientPoint(canvas, hotspot) {
  const rect = canvas.getBoundingClientRect();
  return {
    clientX: rect.left + Number(hotspot.canvas_x),
    clientY: rect.top + Number(hotspot.canvas_y),
  };
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

export function installPixelWorldHotspotPointerProbe({
  fixtureName,
  getCanvas,
  getRendererStatus,
  getHotspotHitTargets,
  getHoverSelection,
  getHoveredHotspot,
}) {
  if (typeof window === "undefined" || fixtureName !== "hotspot_tooltip") {
    return () => {};
  }

  const receiptBase = () => ({
    fixtureName,
    rendererStatus: getRendererStatus(),
    hoverSelection: getHoverSelection(),
  });
  const targetHotspot = () => getHotspotHitTargets().find((entry) => entry.id === "blocker-highlight") || null;

  window[HOTSPOT_POINTER_PROBE_GLOBAL] = {
    async hover() {
      const canvas = getCanvas();
      const hotspot = targetHotspot();
      if (!canvas || !hotspot || getRendererStatus() !== "ready") {
        return { ...receiptBase(), dispatched: false, eventType: "pointermove", visible: false, reason: "canvas_hotspot_or_renderer_unavailable" };
      }
      const point = canvasClientPoint(canvas, hotspot);
      const dispatchMove = (clientX, clientY) => canvas.dispatchEvent(new PointerEvent("pointermove", {
        bubbles: true, clientX, clientY, pointerId: 2853,
      }));
      dispatchMove(point.clientX, point.clientY);
      await nextFrame();
      const visibleHotspot = getHoveredHotspot();
      return {
        ...receiptBase(),
        dispatched: true,
        eventType: "pointermove",
        hotspot: { id: hotspot.id, label: hotspot.label },
        point,
        visible: visibleHotspot?.id === hotspot.id,
        tooltipLabel: visibleHotspot?.label || null,
      };
    },
    async leave() {
      const canvas = getCanvas();
      if (!canvas) {
        return { ...receiptBase(), dispatched: false, eventType: "pointerleave", cleared: false, reason: "canvas_unavailable" };
      }
      canvas.dispatchEvent(new PointerEvent("pointerleave", { bubbles: true, pointerId: 2853 }));
      await nextFrame();
      return {
        ...receiptBase(),
        dispatched: true,
        eventType: "pointerleave",
        cleared: !getHoveredHotspot(),
      };
    },
  };
  return () => delete window[HOTSPOT_POINTER_PROBE_GLOBAL];
}

export { HOTSPOT_POINTER_PROBE_GLOBAL };
