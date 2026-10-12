import assert from "node:assert/strict";
import { test } from "vitest";
import {
  locationFrameCropGeometry,
  measureLocationCornerFrame,
  readScreenshotBitmap,
} from "../scripts/pixel-world-location-frame-pixels.mjs";

function bitmapBytes(width, height, pixelAt, { topDown = false, bitCount = 24 } = {}) {
  const stride = Math.ceil((width * bitCount) / 32) * 4;
  const bytes = Buffer.alloc(54 + stride * height);
  bytes.write("BM");
  bytes.writeUInt32LE(bytes.length, 2);
  bytes.writeUInt32LE(54, 10);
  bytes.writeUInt32LE(40, 14);
  bytes.writeInt32LE(width, 18);
  bytes.writeInt32LE(topDown ? -height : height, 22);
  bytes.writeUInt16LE(1, 26);
  bytes.writeUInt16LE(bitCount, 28);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [red, green, blue] = pixelAt(x, y);
      const row = topDown ? y : height - 1 - y;
      const index = 54 + row * stride + x * (bitCount / 8);
      bytes[index] = blue;
      bytes[index + 1] = green;
      bytes[index + 2] = red;
    }
  }
  return bytes;
}

// Independently drawn 18px marker with 3px padding and two 5.4px L corners.
// The marker is itself mint: only the separate outside arms can prove a frame.
const cornerRects = {
  upperLeftHorizontal: [20, 19, 5.4, 2],
  upperLeftVertical: [19, 20, 2, 5.4],
  lowerRightHorizontal: [38.6, 43, 5.4, 2],
  lowerRightVertical: [43, 38.6, 2, 5.4],
};

function cornerFixture({
  scale = 1, missing = [], shift = [0, 0], color = [106, 155, 135], fill = null,
  extraCorners = false, replacementRects = null,
} = {}) {
  const rects = replacementRects
    ? replacementRects.map((rect, index) => [`replacement-${index}`, rect])
    : Object.entries(cornerRects).filter(([name]) => !missing.includes(name));
  if (extraCorners) rects.push(
    ["upperRightHorizontal", [38.6, 19, 5.4, 2]],
    ["upperRightVertical", [43, 20, 2, 5.4]],
    ["lowerLeftHorizontal", [20, 43, 5.4, 2]],
    ["lowerLeftVertical", [19, 38.6, 2, 5.4]],
  );
  const inside = (x, y, [left, top, width, height]) => (
    x >= left && x < left + width && y >= top && y < top + height
  );
  const bytes = bitmapBytes(64 * scale, 64 * scale, (px, py) => {
    if (fill) return fill;
    const x = (px + 0.5) / scale;
    const y = (py + 0.5) / scale;
    if (rects.some(([, rect]) => inside(x - shift[0], y - shift[1], rect))) return color;
    if (inside(x, y, [23, 25.5, 18, 13])) return [80, 169, 135];
    return [8, 12, 20];
  });
  return {
    bitmap: readScreenshotBitmap(bytes),
    crop: { center: { x: 32 * scale, y: 32 * scale }, scaleX: scale, scaleY: scale },
  };
}

function measure(options) {
  const { bitmap, crop } = cornerFixture(options);
  return measureLocationCornerFrame(bitmap, crop, 18);
}

for (const topDown of [false, true]) {
  for (const bitCount of [24, 32]) {
    test(`BMP ${bitCount}-bit ${topDown ? "top-down" : "bottom-up"} preserves RGB and row order`, () => {
      const bitmap = readScreenshotBitmap(bitmapBytes(3, 2, (x, y) => (
        [40 + x * 30, 60 + y * 70, 90 + x + y]
      ), { topDown, bitCount }));
      assert.deepEqual(bitmap.pixelAt(0, 0), [40, 60, 90]);
      assert.deepEqual(bitmap.pixelAt(2, 1), [100, 130, 93]);
      assert.throws(() => bitmap.pixelAt(3, 0), /out of bounds/);
    });
  }
}

test("BMP inspection rejects truncated and unsupported screenshots", () => {
  const bytes = bitmapBytes(2, 2, () => [0, 0, 0]);
  assert.throws(() => readScreenshotBitmap(bytes.subarray(0, 55)), /truncated/);
  bytes.writeUInt32LE(1, 30);
  assert.throws(() => readScreenshotBitmap(bytes), /unsupported/);
});

test("BMP inspection accepts explicit RGB masks but rejects overlapping mask and pixel data", () => {
  const source = bitmapBytes(2, 2, () => [110, 160, 130], { bitCount: 32 });
  const bytes = Buffer.alloc(source.length + 12);
  source.copy(bytes, 0, 0, 54);
  source.copy(bytes, 66, 54);
  bytes.writeUInt32LE(bytes.length, 2);
  bytes.writeUInt32LE(66, 10);
  bytes.writeUInt32LE(3, 30);
  bytes.writeUInt32LE(0xff0000, 54);
  bytes.writeUInt32LE(0xff00, 58);
  bytes.writeUInt32LE(0xff, 62);
  assert.deepEqual(readScreenshotBitmap(bytes).pixelAt(0, 0), [110, 160, 130]);
  bytes.writeUInt32LE(54, 10);
  assert.throws(() => readScreenshotBitmap(bytes), /unsupported/);
});

for (const scale of [1, 1.5, 2]) {
  test(`viewport crop converts CSS target coordinates at screenshot scale ${scale}`, () => {
    const crop = locationFrameCropGeometry({ width: 390 * scale, height: 844 * scale }, {
      viewport: { width: 390, height: 844 },
      canvas: { left: 10, top: 120 },
      after: { canvas_x: 185, canvas_y: 160 },
      scrollY: 999, // A viewport screenshot must not include the full-page scroll offset.
    });
    assert.equal(crop.width, 64 * scale);
    assert.equal(crop.height, 64 * scale);
    assert.equal(crop.left + crop.center.x, 195 * scale);
    assert.equal(crop.top + crop.center.y, 280 * scale);
    assert.equal(measure({ scale }).passed, true);
  });
}

test("crop coordinates outside the screenshot fail explicitly", () => {
  assert.throws(() => locationFrameCropGeometry({ width: 390, height: 844 }, {
    viewport: { width: 390, height: 844 },
    canvas: { left: 0, top: 0 },
    after: { canvas_x: 4, canvas_y: 4 },
  }), /outside/);
});

test("subpixel positioning and dimmer mint colors stay within the recorded tolerance", () => {
  const result = measure({ scale: 2, shift: [0.6, -0.35], color: [82, 140, 112] });
  assert.equal(result.passed, true, JSON.stringify(result));
  assert.equal(result.tolerances.positionCss, 1);
});

for (const [name, options] of [
  ["mint marker without any frame", { missing: Object.keys(cornerRects) }],
  ["missing upper-left corner", { missing: ["upperLeftHorizontal", "upperLeftVertical"] }],
  ["missing horizontal arm", { missing: ["upperLeftHorizontal"] }],
  ["missing vertical arm", { missing: ["lowerRightVertical"] }],
  ["frame displaced by several pixels", { shift: [5, -3] }],
  ["amber instead of mint frame", { color: [251, 191, 36] }],
  ["bright white crop", { fill: [230, 230, 230] }],
  ["uniform mint crop", { fill: [106, 155, 135] }],
  ["four corners instead of two", { extraCorners: true }],
  ["disconnected short strokes", { replacementRects: [
    [22.5, 19.5, 2, 1], [19.5, 22.5, 1, 2],
    [39.5, 43.5, 2, 1], [43.5, 39.5, 1, 2],
  ] }],
  ["short strokes plus isolated junction dots", { replacementRects: [
    [22.5, 19.5, 2, 1], [19.5, 22.5, 1, 2], [20, 20, 1, 1],
    [39.5, 43.5, 2, 1], [43.5, 39.5, 1, 2], [43, 43, 1, 1],
  ] }],
  ["filled corner blocks", { replacementRects: [[19, 19, 6, 6], [39, 39, 6, 6]] }],
]) {
  test(`shape assertion rejects ${name}`, () => {
    const result = measure(options);
    assert.equal(result.passed, false, JSON.stringify(result));
  });
}
