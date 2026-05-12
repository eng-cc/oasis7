import initPixelWorldBridgeModule, { PixelWorldBridge } from "./pixel_world_bridge_bindgen.js";

export const PIXEL_WORLD_RUNTIME_SOURCE = "wasm_bindgen_runtime";

let runtimeInitPromise = null;

function ensurePixelWorldBridgeModule() {
  if (!runtimeInitPromise) {
    runtimeInitPromise = initPixelWorldBridgeModule(
      new URL("./pixel_world_bridge_bindgen_bg.wasm", import.meta.url),
    );
  }
  return runtimeInitPromise;
}

function toCanvasPoint(canvas, event) {
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

export async function createPixelWorldBridge({ onEvent, onFatal } = {}) {
  await ensurePixelWorldBridgeModule();

  let mountedCanvas = null;
  let animationFrameId = null;
  let removeCanvasListeners = () => {};
  let dragState = null;

  const runtime = new PixelWorldBridge(
    (event) => {
      if (mountedCanvas && event?.type === "hover_entity") {
        mountedCanvas.style.cursor = event.selection ? "pointer" : "grab";
      }
      onEvent?.(event);
    },
    (fatal) => {
      onFatal?.(fatal);
    },
  );

  function stopAnimationLoop() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }

  function startAnimationLoop() {
    stopAnimationLoop();
    const tick = (animationMs) => {
      animationFrameId = requestAnimationFrame(tick);
      try {
        runtime.tick(animationMs);
      } catch (error) {
        onFatal?.({
          code: "pixel_world_renderer_fatal",
          message: error instanceof Error ? error.message : String(error || "renderer fatal"),
        });
      }
    };
    animationFrameId = requestAnimationFrame(tick);
  }

  function cleanupCanvasListeners() {
    removeCanvasListeners();
    removeCanvasListeners = () => {};
    dragState = null;
    if (mountedCanvas) {
      mountedCanvas.style.cursor = "default";
    }
  }

  function attachCanvasListeners(canvas) {
    cleanupCanvasListeners();
    const disposers = [];

    const onPointerDown = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      dragState = {
        pointerId: event.pointerId,
        moved: false,
      };
      canvas.style.cursor = "grabbing";
      canvas.setPointerCapture?.(event.pointerId);
      runtime.pointer_down(point.x, point.y, event.pointerId);
    };

    const onPointerMove = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      if (dragState && dragState.pointerId === event.pointerId) {
        dragState.moved = true;
      }
      runtime.pointer_move(point.x, point.y, false, event.pointerId);
    };

    const onPointerLeave = (event) => {
      canvas.style.cursor = "default";
      runtime.pointer_move(0, 0, true, event.pointerId ?? -1);
    };

    const onPointerUp = (event) => {
      runtime.pointer_up(event.pointerId);
      canvas.releasePointerCapture?.(event.pointerId);
      canvas.style.cursor = "grab";
      dragState = null;
    };

    const onWheel = (event) => {
      event.preventDefault();
      runtime.wheel(event.deltaY);
    };

    const onClick = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      if (dragState?.moved) {
        return;
      }
      runtime.click(point.x, point.y);
    };

    const bind = (name, handler, options) => {
      canvas.addEventListener(name, handler, options);
      disposers.push(() => canvas.removeEventListener(name, handler, options));
    };

    bind("pointerdown", onPointerDown);
    bind("pointermove", onPointerMove);
    bind("pointerleave", onPointerLeave);
    bind("pointerup", onPointerUp);
    bind("pointercancel", onPointerUp);
    bind("wheel", onWheel, { passive: false });
    bind("click", onClick);

    canvas.style.cursor = "grab";
    removeCanvasListeners = () => {
      for (const dispose of disposers.splice(0)) {
        dispose();
      }
    };
  }

  return {
    mount(canvas, renderState) {
      mountedCanvas = canvas;
      const result = runtime.mount(canvas, renderState);
      attachCanvasListeners(canvas);
      startAnimationLoop();
      return result;
    },
    update(renderState) {
      return runtime.update(renderState);
    },
    unmount() {
      stopAnimationLoop();
      cleanupCanvasListeners();
      const result = runtime.unmount();
      mountedCanvas = null;
      return result;
    },
  };
}
