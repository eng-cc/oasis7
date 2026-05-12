function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function createInitialCameraState() {
  return {
    zoom: 1,
    pan_x_px: 0,
    pan_y_px: 0,
  };
}

function toCanvasPoint(position, worldBounds, width, height, cameraState) {
  if (!position || !worldBounds) {
    return null;
  }
  const safeWidth = Math.max(1, Number(worldBounds.width_cm) || 1);
  const safeDepth = Math.max(1, Number(worldBounds.depth_cm) || 1);
  const normalizedX = clamp(position.x_cm / safeWidth, 0, 1);
  const normalizedY = clamp(position.y_cm / safeDepth, 0, 1);
  const baseX = 20 + (normalizedX * Math.max(1, width - 40));
  const baseY = 20 + (normalizedY * Math.max(1, height - 40));
  const zoom = Math.max(0.5, Number(cameraState?.zoom) || 1);
  const panX = Number(cameraState?.pan_x_px) || 0;
  const panY = Number(cameraState?.pan_y_px) || 0;
  const centeredX = baseX - (width / 2);
  const centeredY = baseY - (height / 2);
  return {
    x: (width / 2) + (centeredX * zoom) + panX,
    y: (height / 2) + (centeredY * zoom) + panY,
  };
}

function fallbackPointForEntity(id, width, height, cameraState) {
  const baseX = 36 + ((Math.abs(id.length * 29) % Math.max(40, width - 72)));
  const baseY = 44 + ((Math.abs(id.length * 17) % Math.max(48, height - 88)));
  return toCanvasPoint(
    { x_cm: baseX, y_cm: baseY, z_cm: 0 },
    { width_cm: width, depth_cm: height },
    width,
    height,
    cameraState,
  );
}

function drawGrid(context, width, height, cameraState) {
  const zoom = Math.max(0.5, Number(cameraState?.zoom) || 1);
  const panX = Number(cameraState?.pan_x_px) || 0;
  const panY = Number(cameraState?.pan_y_px) || 0;
  const gridStep = clamp(24 * zoom, 12, 72);
  const offsetX = ((panX % gridStep) + gridStep) % gridStep;
  const offsetY = ((panY % gridStep) + gridStep) % gridStep;

  context.strokeStyle = "rgba(99, 179, 255, 0.10)";
  context.lineWidth = 1;
  for (let x = offsetX; x <= width; x += gridStep) {
    context.beginPath();
    context.moveTo(x + 0.5, 0);
    context.lineTo(x + 0.5, height);
    context.stroke();
  }
  for (let y = offsetY; y <= height; y += gridStep) {
    context.beginPath();
    context.moveTo(0, y + 0.5);
    context.lineTo(width, y + 0.5);
    context.stroke();
  }
}

function drawBridgeFrame(canvas, renderState, cameraState, animationMs) {
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("2d canvas context unavailable");
  }

  const width = canvas.width;
  const height = canvas.height;
  context.clearRect(0, 0, width, height);

  context.fillStyle = "#0a121a";
  context.fillRect(0, 0, width, height);

  drawGrid(context, width, height, cameraState);

  for (const location of renderState.locations || []) {
    const point = toCanvasPoint(location.pos, renderState.world_bounds, width, height, cameraState);
    if (!point) {
      continue;
    }
    const pulse = 1 + (0.08 * Math.sin((animationMs / 360) + location.id.length));
    const size = 16 * pulse;
    context.fillStyle = "rgba(110, 231, 183, 0.72)";
    context.fillRect(point.x - (size / 2), point.y - (size / 2), size, size);
    context.strokeStyle = "rgba(110, 231, 183, 0.95)";
    context.strokeRect(point.x - (size / 2), point.y - (size / 2), size, size);
  }

  for (const [index, agent] of (renderState.agents || []).entries()) {
    const point = toCanvasPoint(agent.pos, renderState.world_bounds, width, height, cameraState)
      || fallbackPointForEntity(agent.id, width, height, cameraState);
    const isSelected = renderState.selection?.kind === "agent" && renderState.selection?.id === agent.id;
    const pulse = 1 + (0.12 * Math.sin((animationMs / 240) + index));
    const size = (isSelected ? 15 : 12) * pulse;
    context.fillStyle = isSelected ? "#fbbf24" : "#63b3ff";
    context.fillRect(point.x - (size / 2), point.y - (size / 2), size, size);
    context.strokeStyle = isSelected ? "#fde68a" : "#c6e4ff";
    context.lineWidth = 2;
    context.strokeRect(point.x - (size / 2), point.y - (size / 2), size, size);
  }
}

export function createPixelWorldBevyBridge({ onEvent, onFatal } = {}) {
  let mountedCanvas = null;
  let lastRenderState = null;
  let hitRegions = [];
  let cameraState = createInitialCameraState();
  let boundPointerDown = null;
  let boundPointerMove = null;
  let boundPointerUp = null;
  let boundWheel = null;
  let boundClick = null;
  let lastHoverId = null;
  let dragState = null;
  let animationFrameId = null;
  let lastAnimationMs = 0;

  function emit(event) {
    onEvent?.(event);
  }

  function emitCameraState() {
    emit({
      type: "camera_state_changed",
      camera: {
        zoom: Number(cameraState.zoom.toFixed(3)),
        pan_x_px: Math.round(cameraState.pan_x_px),
        pan_y_px: Math.round(cameraState.pan_y_px),
      },
    });
  }

  function fatal(error) {
    const normalized = {
      code: "pixel_world_renderer_fatal",
      message: error instanceof Error ? error.message : String(error || "renderer fatal"),
    };
    onFatal?.(normalized);
    return normalized;
  }

  function rebuildHitRegions(canvas, renderState) {
    const width = canvas.width;
    const height = canvas.height;
    const nextRegions = [];

    for (const location of renderState.locations || []) {
      const point = toCanvasPoint(location.pos, renderState.world_bounds, width, height, cameraState);
      if (!point) {
        continue;
      }
      nextRegions.push({
        kind: "location",
        id: location.id,
        left: point.x - 8,
        top: point.y - 8,
        right: point.x + 8,
        bottom: point.y + 8,
      });
    }

    for (const agent of renderState.agents || []) {
      const point = toCanvasPoint(agent.pos, renderState.world_bounds, width, height, cameraState)
        || fallbackPointForEntity(agent.id, width, height, cameraState);
      nextRegions.push({
        kind: "agent",
        id: agent.id,
        left: point.x - 8,
        top: point.y - 8,
        right: point.x + 8,
        bottom: point.y + 8,
      });
    }

    hitRegions = nextRegions;
  }

  function renderCurrentFrame(animationMs = lastAnimationMs) {
    if (!mountedCanvas || !lastRenderState) {
      return;
    }
    lastAnimationMs = animationMs;
    drawBridgeFrame(mountedCanvas, lastRenderState, cameraState, animationMs);
    rebuildHitRegions(mountedCanvas, lastRenderState);
  }

  function stopAnimationLoop() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }

  function scheduleAnimationLoop() {
    stopAnimationLoop();
    const animate = (animationMs) => {
      animationFrameId = requestAnimationFrame(animate);
      try {
        renderCurrentFrame(animationMs);
      } catch (error) {
        stopAnimationLoop();
        fatal(error);
      }
    };
    animationFrameId = requestAnimationFrame(animate);
  }

  function eventPoint(canvas, event) {
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) {
      return null;
    }
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (event.clientX - rect.left) * scaleX,
      y: (event.clientY - rect.top) * scaleY,
    };
  }

  function hitTest(point) {
    if (!point) {
      return null;
    }
    for (let index = hitRegions.length - 1; index >= 0; index -= 1) {
      const region = hitRegions[index];
      if (
        point.x >= region.left
        && point.x <= region.right
        && point.y >= region.top
        && point.y <= region.bottom
      ) {
        return { kind: region.kind, id: region.id };
      }
    }
    return null;
  }

  function detachCanvasEvents() {
    if (!mountedCanvas) {
      return;
    }
    if (boundPointerDown) {
      mountedCanvas.removeEventListener("pointerdown", boundPointerDown);
    }
    if (boundPointerMove) {
      mountedCanvas.removeEventListener("pointermove", boundPointerMove);
      mountedCanvas.removeEventListener("pointerleave", boundPointerMove);
    }
    if (boundPointerUp) {
      mountedCanvas.removeEventListener("pointerup", boundPointerUp);
      mountedCanvas.removeEventListener("pointercancel", boundPointerUp);
    }
    if (boundWheel) {
      mountedCanvas.removeEventListener("wheel", boundWheel);
    }
    if (boundClick) {
      mountedCanvas.removeEventListener("click", boundClick);
    }
    dragState = null;
    mountedCanvas.style.cursor = "default";
    boundPointerDown = null;
    boundPointerMove = null;
    boundPointerUp = null;
    boundWheel = null;
    boundClick = null;
    lastHoverId = null;
  }

  function attachCanvasEvents(canvas) {
    detachCanvasEvents();
    boundPointerDown = (event) => {
      dragState = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startClientY: event.clientY,
        startPanX: cameraState.pan_x_px,
        startPanY: cameraState.pan_y_px,
        moved: false,
      };
      canvas.style.cursor = "grabbing";
      canvas.setPointerCapture?.(event.pointerId);
    };
    boundPointerMove = (event) => {
      if (dragState && event.pointerId === dragState.pointerId) {
        const deltaX = event.clientX - dragState.startClientX;
        const deltaY = event.clientY - dragState.startClientY;
        if (Math.abs(deltaX) > 1 || Math.abs(deltaY) > 1) {
          dragState.moved = true;
        }
        cameraState = {
          ...cameraState,
          pan_x_px: dragState.startPanX + deltaX,
          pan_y_px: dragState.startPanY + deltaY,
        };
        renderCurrentFrame();
        emitCameraState();
        return;
      }
      if (event.type === "pointerleave") {
        if (lastHoverId !== null) {
          lastHoverId = null;
          emit({ type: "hover_entity", selection: null });
        }
        canvas.style.cursor = "default";
        return;
      }
      const hit = hitTest(eventPoint(canvas, event));
      const hoverKey = hit ? `${hit.kind}/${hit.id}` : null;
      if (hoverKey === lastHoverId) {
        return;
      }
      lastHoverId = hoverKey;
      canvas.style.cursor = hit ? "pointer" : "grab";
      emit({ type: "hover_entity", selection: hit });
    };
    boundPointerUp = (event) => {
      if (dragState && event.pointerId === dragState.pointerId) {
        canvas.releasePointerCapture?.(event.pointerId);
        const moved = dragState.moved;
        dragState = null;
        canvas.style.cursor = moved ? "grab" : (lastHoverId ? "pointer" : "default");
      }
    };
    boundWheel = (event) => {
      event.preventDefault();
      const nextZoom = clamp(
        cameraState.zoom * (event.deltaY < 0 ? 1.12 : 0.89),
        0.6,
        3.5,
      );
      if (Math.abs(nextZoom - cameraState.zoom) < 0.001) {
        return;
      }
      cameraState = {
        ...cameraState,
        zoom: nextZoom,
      };
      renderCurrentFrame();
      emitCameraState();
    };
    boundClick = (event) => {
      if (dragState?.moved) {
        return;
      }
      const hit = hitTest(eventPoint(canvas, event));
      if (!hit) {
        return;
      }
      emit({ type: "select_entity", selection: hit });
    };
    canvas.style.cursor = "grab";
    canvas.addEventListener("pointerdown", boundPointerDown);
    canvas.addEventListener("pointermove", boundPointerMove);
    canvas.addEventListener("pointerleave", boundPointerMove);
    canvas.addEventListener("pointerup", boundPointerUp);
    canvas.addEventListener("pointercancel", boundPointerUp);
    canvas.addEventListener("wheel", boundWheel, { passive: false });
    canvas.addEventListener("click", boundClick);
  }

  return {
    mount(canvas, initialRenderState) {
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("pixel world bridge mount requires a canvas element");
      }
      mountedCanvas = canvas;
      lastRenderState = initialRenderState;
      cameraState = createInitialCameraState();
      try {
        attachCanvasEvents(canvas);
        renderCurrentFrame(0);
        scheduleAnimationLoop();
      } catch (error) {
        return { status: "fallback", fatal: fatal(error) };
      }
      emit({ type: "canvas_ready" });
      emitCameraState();
      return { status: "ready" };
    },
    update(nextRenderState) {
      lastRenderState = nextRenderState;
      if (!mountedCanvas) {
        return { status: "detached" };
      }
      try {
        renderCurrentFrame();
      } catch (error) {
        return { status: "fallback", fatal: fatal(error) };
      }
      return { status: "ready" };
    },
    unmount() {
      stopAnimationLoop();
      detachCanvasEvents();
      mountedCanvas = null;
      lastRenderState = null;
      hitRegions = [];
      cameraState = createInitialCameraState();
      return { status: "detached" };
    },
    getLastRenderState() {
      return lastRenderState;
    },
  };
}
