import { describe, expect, it, vi } from "vitest";

import { createPixelWorldFocusController } from "./pixel_world_focus_controller.js";

describe("pixel world focus controller", () => {
  it("ignores Escape with the IME keyCode even when isComposing is false", () => {
    const setFocusMode = vi.fn();
    const setCommandDrawerOpen = vi.fn();
    const setDiagnosticsDrawerOpen = vi.fn();
    const setMaximized = vi.fn();
    const controller = createPixelWorldFocusController({
      focusMode: () => true,
      commandDrawerOpen: () => true,
      diagnosticsDrawerOpen: () => false,
      setFocusMode,
      setCommandDrawerOpen,
      setDiagnosticsDrawerOpen,
      setMaximized,
    });
    const event = {
      key: "Escape",
      keyCode: 229,
      isComposing: false,
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    };

    controller.handleKeyDown(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
    expect(setFocusMode).not.toHaveBeenCalled();
    expect(setCommandDrawerOpen).not.toHaveBeenCalled();
    expect(setDiagnosticsDrawerOpen).not.toHaveBeenCalled();
    expect(setMaximized).not.toHaveBeenCalled();
  });
});
