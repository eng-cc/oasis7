const PIXEL_WORLD_CANVAS_WIDTH = 960;
const PIXEL_WORLD_CANVAS_HEIGHT = 540;
export const PIXEL_WORLD_HOTSPOT_TOUCH_TARGET_PX = 44;

export function pixelWorldHotspotIntersectsStage(style, stageSize, glyphSize = 20) {
  const x = parseFloat(style?.left || "50") * stageSize.width / 100;
  const y = parseFloat(style?.top || "50") * stageSize.height / 100;
  const radius = glyphSize / 2;
  return x + radius > 0 && y + radius > 0
    && x - radius < stageSize.width && y - radius < stageSize.height;
}

function safeNumber(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function clampRatio(value) {
  return Math.min(1, Math.max(0, safeNumber(value)));
}

export function toCanvasPoint(position, worldBounds, width, height, cameraState) {
  if (!position || !worldBounds) return null;
  const x = clampRatio(safeNumber(position.x_cm ?? position.xCm) / Math.max(1, safeNumber(worldBounds.width_cm ?? worldBounds.widthCm, 1)));
  const y = clampRatio(safeNumber(position.y_cm ?? position.yCm) / Math.max(1, safeNumber(worldBounds.depth_cm ?? worldBounds.depthCm, 1)));
  const zoom = Math.max(0.5, Number(cameraState?.zoom) || 1);
  return {
    x: width / 2 + (20 + x * Math.max(1, width - 40) - width / 2) * zoom + (Number(cameraState?.pan_x_px ?? cameraState?.panX) || 0),
    y: height / 2 + (20 + y * Math.max(1, height - 40) - height / 2) * zoom + (Number(cameraState?.pan_y_px ?? cameraState?.panY) || 0),
  };
}

export function pixelWorldHotspotGlyphSize(hotspot) {
  return Math.max(
    14,
    Math.min(32, safeNumber(hotspot?.size_hint_px ?? hotspot?.sizeHintPx, 16)),
  );
}

export function pixelWorldHotspotStyle(
  hotspot,
  worldBounds,
  index = 0,
  cameraState,
  stageSize = { width: PIXEL_WORLD_CANVAS_WIDTH, height: PIXEL_WORLD_CANVAS_HEIGHT },
) {
  const fallback = {
    left: `${20 + ((index % 4) * 16)}%`,
    top: `${22 + (Math.floor(index / 4) * 16)}%`,
  };
  const position = hotspot?.pos;
  if (!position || !worldBounds) {
    return {
      ...fallback,
      width: `${PIXEL_WORLD_HOTSPOT_TOUCH_TARGET_PX}px`,
      height: `${PIXEL_WORLD_HOTSPOT_TOUCH_TARGET_PX}px`,
      transform: "translate(-50%, -50%)",
    };
  }

  const point = toCanvasPoint(position, worldBounds, stageSize.width, stageSize.height, cameraState);

  return {
    left: `${(point.x / stageSize.width) * 100}%`,
    top: `${(point.y / stageSize.height) * 100}%`,
    width: `${PIXEL_WORLD_HOTSPOT_TOUCH_TARGET_PX}px`,
    height: `${PIXEL_WORLD_HOTSPOT_TOUCH_TARGET_PX}px`,
    transform: "translate(-50%, -50%)",
  };
}
