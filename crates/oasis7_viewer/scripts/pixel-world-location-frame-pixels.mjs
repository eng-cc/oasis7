// Pixel assertions shared by the real browser smoke and synthetic negative controls.
// Coordinates exposed by the hit-target probe are CSS pixels; screenshots may not be.
export function readScreenshotBitmap(bytes) {
  if (bytes.length < 54 || bytes.toString("ascii", 0, 2) !== "BM") {
    throw new Error("screenshot is not a Windows bitmap");
  }
  const offset = bytes.readUInt32LE(10);
  const headerSize = bytes.readUInt32LE(14);
  const width = bytes.readInt32LE(18);
  const signedHeight = bytes.readInt32LE(22);
  const height = Math.abs(signedHeight);
  const bitCount = bytes.readUInt16LE(28);
  const compression = bytes.readUInt32LE(30);
  const rgbMasks = compression === 3 && bitCount === 32 && bytes.length >= 66
    && bytes.readUInt32LE(54) === 0xff0000
    && bytes.readUInt32LE(58) === 0xff00
    && bytes.readUInt32LE(62) === 0xff;
  const stride = Math.ceil((width * bitCount) / 32) * 4;
  if (headerSize < 40 || width <= 0 || height === 0 || ![24, 32].includes(bitCount)
    || bytes.readUInt16LE(26) !== 1 || (compression !== 0 && !rgbMasks)
    || (rgbMasks && offset < 66)
    || offset < 14 + headerSize || offset + stride * height > bytes.length) {
    throw new Error("unsupported or truncated screenshot bitmap");
  }
  return {
    width,
    height,
    bitCount,
    topDown: signedHeight < 0,
    pixelAt(x, y) {
      if (!Number.isInteger(x) || !Number.isInteger(y)
        || x < 0 || y < 0 || x >= width || y >= height) {
        throw new RangeError(`screenshot pixel out of bounds: ${x},${y}`);
      }
      const row = signedHeight < 0 ? y : height - 1 - y;
      const index = offset + row * stride + x * (bitCount / 8);
      return [bytes[index + 2], bytes[index + 1], bytes[index]];
    },
  };
}

export function locationFrameCropGeometry(bitmap, frame, sideCss = 64) {
  const scaleX = bitmap.width / frame.viewport.width;
  const scaleY = bitmap.height / frame.viewport.height;
  const center = {
    x: (frame.canvas.left + Number(frame.after.canvas_x)) * scaleX,
    y: (frame.canvas.top + Number(frame.after.canvas_y)) * scaleY,
  };
  const width = Math.round(sideCss * scaleX);
  const height = Math.round(sideCss * scaleY);
  const left = Math.round(center.x - width / 2);
  const top = Math.round(center.y - height / 2);
  if (![scaleX, scaleY, center.x, center.y].every(Number.isFinite)
    || scaleX <= 0 || scaleY <= 0 || width < 1 || height < 1
    || left < 0 || top < 0 || left + width > bitmap.width || top + height > bitmap.height) {
    throw new Error("location-frame crop is outside the viewport screenshot");
  }
  return {
    sideCss, scaleX, scaleY, left, top, width, height,
    center: { x: center.x - left, y: center.y - top },
  };
}

export function measureLocationCornerFrame(bitmap, crop, bodySizeCss) {
  const { center, scaleX, scaleY } = crop;
  const armLengthCss = Math.max(3, Math.min(6, bodySizeCss * 0.30));
  const outerOffsetCss = bodySizeCss / 2 + 3;
  const tolerances = {
    positionCss: 1,
    minimumArmCoverage: 0.6,
    maximumEmptyCornerCoverage: 0.2,
    minimumCornerSpanCss: Math.max(3, armLengthCss - 1),
    minimumCornerAreaCss: (4 * armLengthCss - 1) * 0.55,
    color: { minimumGreen: 60, minimumGreenMinusRed: 20, minimumGreenMinusBlue: 8 },
  };
  if (![bodySizeCss, scaleX, scaleY, center.x, center.y].every(Number.isFinite)
    || bodySizeCss <= 0 || scaleX <= 0 || scaleY <= 0
    || center.x - (outerOffsetCss + 2) * scaleX < 0
    || center.y - (outerOffsetCss + 2) * scaleY < 0
    || center.x + (outerOffsetCss + 2) * scaleX >= bitmap.width
    || center.y + (outerOffsetCss + 2) * scaleY >= bitmap.height) {
    throw new Error("location corner-frame expectation leaves the crop");
  }
  function isMint(x, y) {
    const [red, green, blue] = bitmap.pixelAt(x, y);
    return green >= tolerances.color.minimumGreen
      && green - red >= tolerances.color.minimumGreenMinusRed
      && green - blue >= tolerances.color.minimumGreenMinusBlue;
  }
  function hasMintNear(dx, dy) {
    const x = center.x + dx * scaleX;
    const y = center.y + dy * scaleY;
    const radiusX = tolerances.positionCss * scaleX;
    const radiusY = tolerances.positionCss * scaleY;
    for (let py = Math.ceil(y - radiusY - 0.5); py <= Math.floor(y + radiusY - 0.5); py += 1) {
      for (let px = Math.ceil(x - radiusX - 0.5); px <= Math.floor(x + radiusX - 0.5); px += 1) {
        if (isMint(px, py)) return true;
      }
    }
    return false;
  }
  function arm(name, cornerX, cornerY, horizontal) {
    let samples = 0;
    let matched = 0;
    // Sample beyond the shared junction: one surviving arm cannot stand in
    // for a missing perpendicular arm, even with the one-CSS-pixel tolerance.
    for (let distance = 2.5; distance <= armLengthCss - 0.25; distance += 0.75) {
      const dx = cornerX * (outerOffsetCss - (horizontal ? distance : 0));
      const dy = cornerY * (outerOffsetCss - (horizontal ? 0 : distance));
      samples += 1;
      if (hasMintNear(dx, dy)) matched += 1;
    }
    return { name, matched, samples, coverage: matched / samples };
  }
  function corner(name, cornerX, cornerY) {
    const pixels = new Map();
    const inwardPoint = (px, py) => ({
      x: outerOffsetCss - cornerX * ((px + 0.5 - center.x) / scaleX),
      y: outerOffsetCss - cornerY * ((py + 0.5 - center.y) / scaleY),
    });
    // Restrict connectivity to the two arms plus the position tolerance. A
    // neighboring marker body cannot bridge disconnected frame fragments.
    for (let py = 0; py < bitmap.height; py += 1) {
      for (let px = 0; px < bitmap.width; px += 1) {
        const inward = inwardPoint(px, py);
        if (inward.x >= -2 && inward.y >= -2
          && inward.x <= armLengthCss + 1 && inward.y <= armLengthCss + 1
          && (Math.abs(inward.x) <= 2 || Math.abs(inward.y) <= 2)
          && isMint(px, py)) pixels.set(py * bitmap.width + px, { px, py, inward });
      }
    }
    let connected = [];
    while (pixels.size) {
      const first = pixels.keys().next().value;
      const pending = [first];
      const component = [];
      while (pending.length) {
        const key = pending.pop();
        const pixel = pixels.get(key);
        if (!pixel) continue;
        pixels.delete(key);
        component.push(pixel);
        for (let dy = -1; dy <= 1; dy += 1) {
          for (let dx = -1; dx <= 1; dx += 1) {
            const x = pixel.px + dx;
            const y = pixel.py + dy;
            if (x >= 0 && x < bitmap.width && y >= 0 && y < bitmap.height) {
              const neighbor = y * bitmap.width + x;
              if (pixels.has(neighbor)) pending.push(neighbor);
            }
          }
        }
      }
      const touchesJunction = component.some(({ inward }) => (
        Math.abs(inward.x) <= 1.5 && Math.abs(inward.y) <= 1.5
      ));
      if (touchesJunction && component.length > connected.length) connected = component;
    }
    const xs = connected.map(({ px }) => px);
    const ys = connected.map(({ py }) => py);
    const spanXCss = connected.length ? (Math.max(...xs) - Math.min(...xs) + 1) / scaleX : 0;
    const spanYCss = connected.length ? (Math.max(...ys) - Math.min(...ys) + 1) / scaleY : 0;
    const areaCss = connected.length / (scaleX * scaleY);
    const interiorClear = !hasMintNear(
      cornerX * (outerOffsetCss - 3.25), cornerY * (outerOffsetCss - 3.25),
    );
    return {
      name, spanXCss, spanYCss, areaCss, interiorClear,
      passed: spanXCss >= tolerances.minimumCornerSpanCss
        && spanYCss >= tolerances.minimumCornerSpanCss
        && areaCss >= tolerances.minimumCornerAreaCss && interiorClear,
    };
  }
  const arms = [
    arm("upperLeftHorizontal", -1, -1, true),
    arm("upperLeftVertical", -1, -1, false),
    arm("lowerRightHorizontal", 1, 1, true),
    arm("lowerRightVertical", 1, 1, false),
  ];
  const emptyCorners = [
    arm("upperRightHorizontal", 1, -1, true),
    arm("upperRightVertical", 1, -1, false),
    arm("lowerLeftHorizontal", -1, 1, true),
    arm("lowerLeftVertical", -1, 1, false),
  ];
  const corners = [corner("upperLeft", -1, -1), corner("lowerRight", 1, 1)];
  return {
    passed: arms.every(({ coverage }) => coverage >= tolerances.minimumArmCoverage)
      && emptyCorners.every(({ coverage }) => coverage <= tolerances.maximumEmptyCornerCoverage)
      && corners.every(({ passed }) => passed),
    bodySizeCss, armLengthCss, outerOffsetCss, tolerances, arms, emptyCorners, corners,
  };
}
