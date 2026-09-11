import { render, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import { PixelWorldCanvasRenderer } from "./pixel_world_host.jsx";
import { rendererEntityTargetStyle } from "./pixel_world_renderer_targets.jsx";

describe("pixel world canvas renderer", () => {
  it("refreshes renderer targets after a late backing-size update during async mount", async () => {
    const [rendererStatus, setRendererStatus] = createSignal("booting");
    const [cameraState, setCameraState] = createSignal(null);
    const renderState = () => ({
      world_bounds: { width_cm: 10_000_000, depth_cm: 5_000_000 },
      agents: [{ id: "agent-0", pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 } }],
      locations: [],
      module_visual_entities: [],
    });
    const rectSpy = vi.spyOn(HTMLCanvasElement.prototype, "getBoundingClientRect").mockReturnValue({
      width: 1440,
      height: 1000,
      top: 0,
      left: 0,
      right: 1440,
      bottom: 1000,
    });
    let mountedCanvas;
    let view;
    try {
      view = render(() => <PixelWorldCanvasRenderer
        locale={() => "en"}
        rendererStatus={rendererStatus}
        rendererProjection={() => rendererStatus() === "ready" && cameraState() !== null}
        cameraState={cameraState}
        renderInput={() => null}
        renderState={renderState}
        selection={() => null}
        hoveredEntity={() => null}
        visualOverlayEnabled={() => false}
        onSelect={() => {}}
        onHover={() => {}}
        hoveredHotspot={() => null}
        onCanvasMount={(canvas) => { mountedCanvas = canvas; }}
        onCanvasUpdate={() => {}}
      />);
      const camera = { zoom: 1.563, pan_x_px: 932, pan_y_px: -523 };
      setRendererStatus("ready");
      setCameraState(camera);
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      mountedCanvas.width = 2880;
      mountedCanvas.height = 2000;

      const entity = { id: "agent-0", pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 } };
      const bounds = { width_cm: 10_000_000, depth_cm: 5_000_000 };
      const expected = rendererEntityTargetStyle(entity, bounds, { width: 1440, height: 1000 }, camera, undefined, { width: 2880, height: 2000 });
      await waitFor(() => expect(view.container.querySelector('[data-renderer-target="true"]')).toHaveStyle({ left: expected.left, top: expected.top }));
    } finally {
      view?.unmount();
      rectSpy.mockRestore();
    }
  });
});
