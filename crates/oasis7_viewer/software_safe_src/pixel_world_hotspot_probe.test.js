import { afterEach, describe, expect, it } from "vitest";

import { installPixelWorldHotspotPointerProbe } from "./pixel_world_hotspot_probe.js";

afterEach(() => {
  delete window.__OASIS7_PIXEL_WORLD_HOTSPOT_POINTER_PROBE__;
  document.body.replaceChildren();
});

describe("pixel world hotspot pointer probe", () => {
  it("dispatches native canvas PointerEvents and records visible then cleared hover receipts", async () => {
    const canvas = document.createElement("canvas");
    canvas.width = 960;
    canvas.height = 540;
    canvas.getBoundingClientRect = () => ({ left: 20, top: 10, width: 480, height: 270 });
    document.body.append(canvas);
    let hover = null;
    const seen = [];
    canvas.addEventListener("pointermove", (event) => {
      seen.push(event);
      hover = { kind: "hotspot", id: "blocker-highlight" };
    });
    canvas.addEventListener("pointerleave", (event) => {
      seen.push(event);
      hover = null;
    });

    installPixelWorldHotspotPointerProbe({
      fixtureName: "hotspot_tooltip",
      getCanvas: () => canvas,
      getRendererStatus: () => "ready",
      getHotspotHitTargets: () => [{ id: "blocker-highlight", kind: "hotspot", canvas_x: 240, canvas_y: 135 }],
      getHoverSelection: () => hover,
      getHoveredHotspot: () => hover ? { id: hover.id, label: "Missing Material" } : null,
    });

    const visible = await window.__OASIS7_PIXEL_WORLD_HOTSPOT_POINTER_PROBE__.hover();
    const cleared = await window.__OASIS7_PIXEL_WORLD_HOTSPOT_POINTER_PROBE__.leave();

    expect(seen).toHaveLength(2);
    expect(seen.every((event) => event instanceof PointerEvent)).toBe(true);
    expect(visible).toMatchObject({ dispatched: true, eventType: "pointermove", visible: true, hotspot: { id: "blocker-highlight" } });
    expect(cleared).toMatchObject({ dispatched: true, eventType: "pointerleave", cleared: true });
  });
});
