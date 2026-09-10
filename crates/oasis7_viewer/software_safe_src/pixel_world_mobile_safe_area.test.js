import { afterEach, describe, expect, it, vi } from "vitest";
import { applyPixelWorldMobileSelectionSafeArea } from './pixel_world_mobile_safe_area.js';
import { pixelWorldMobileFocusSelectionOffset, pixelWorldMobileSelectionChipOffset, pixelWorldMobileSelectionOffset } from "./pixel_world_mobile_safe_area.js";

describe("pixel world mobile selection safe area", () => {
  afterEach(() => { document.body.innerHTML = ''; vi.unstubAllGlobals(); });
  it('retains selected command-edge fallback through the complete cramped mobile clearance pipeline', () => {
    vi.stubGlobal('innerWidth', 390);
    vi.stubGlobal('innerHeight', 844);
    document.body.innerHTML = `<div id="canvas"><button class="pixel-world-entity pixel-world-entity--canvas-hit-target" data-selected="true"></button></div><div data-viewer-overlay="feed"></div><div data-viewer-overlay="next-move"></div>`;
    const canvas = document.querySelector('#canvas');
    const marker = canvas.querySelector('button');
    const rect = (left, top, width, height) => ({left,top,right:left+width,bottom:top+height,width,height});
    canvas.getBoundingClientRect = () => rect(0,0,390,844);
    marker.getBoundingClientRect = () => rect(170,319 + (parseFloat(marker.style.translate.split(' ')[1]) || 0),44,44);
    document.querySelector('[data-viewer-overlay="feed"]').getBoundingClientRect = () => rect(0,0,390,260);
    document.querySelector('[data-viewer-overlay="next-move"]').getBoundingClientRect = () => rect(0,266,390,578);
    applyPixelWorldMobileSelectionSafeArea(canvas);
    expect(marker.getBoundingClientRect().bottom).toBe(258);
    applyPixelWorldMobileSelectionSafeArea(canvas);
    expect(marker.getBoundingClientRect().bottom).toBe(258);
  });
  it("clears the command band while preserving the Feed gap", () => {
    expect(pixelWorldMobileSelectionOffset({ markerTop: 520, markerBottom: 566, commandTop: 420, feedBottom: 146 })).toBe(-154);
    expect(pixelWorldMobileSelectionOffset({ markerTop: 267, markerBottom: 313, commandTop: 208, feedBottom: 146 })).toBe(-113);
    expect(pixelWorldMobileSelectionOffset({ markerTop: 319, markerBottom: 363, commandTop: 266, feedBottom: 260 })).toBe(-105);
  });

  it("moves the selected marker beside an expanded Focus HUD", () => {
    expect(pixelWorldMobileFocusSelectionOffset({ markerLeft: 178, hudRight: 300 })).toBe(130);
  });

  it("derives collapsed Feed chip clearance from the rendered bottom for every status height", () => {
    const statusHeights = [
      ["ready", 165],
      ["replay", 165],
      ["empty", 173],
      ["gap", 181],
      ["unavailable", 189],
    ];
    for (const [, feedBottom] of statusHeights) {
      expect(pixelWorldMobileSelectionChipOffset({ chipTop: 160, feedBottom })).toBe(feedBottom - 152);
    }
    expect(pixelWorldMobileSelectionChipOffset({ chipTop: 104, feedBottom: 400, feedOpen: true })).toBe(0);
  });
});
