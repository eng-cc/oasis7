import { fireEvent, render, screen } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import { PixelWorldHotspot, PixelWorldHotspotTooltip } from "./pixel_world_hotspot.jsx";

const hotspot = { id: "hotspot-blocker", kind: "blocker", label: "Blocked route" };

describe("pixel world hotspot controls", () => {
  it("removes fully offstage glyph controls and restores partially visible edge controls", () => {
    const [left, setLeft] = createSignal("-10%");
    const { container } = render(() => <PixelWorldHotspot locale="en" hotspot={{ id: "edge", kind: "goal", label: "Edge" }} glyphSize={20} style={{ left: left(), top: "50%", width: "44px", height: "44px" }} />);
    const marker = container.querySelector("button");
    expect(marker).toBeDisabled();
    expect(marker).toHaveAttribute("tabindex", "-1");
    expect(marker).not.toBeVisible();
    setLeft("-0.5%");
    expect(marker).not.toBeDisabled();
    expect(marker).toHaveAttribute("tabindex", "0");
    expect(marker).toBeVisible();
    expect(marker).toHaveStyle({ width: "44px", height: "44px" });
    setLeft("110%");
    expect(marker).not.toBeVisible();
    expect(marker).toBeDisabled();
  });
  it("continues both directions from a portaled close control", () => {
    render(() => <><button class="pixel-world-hotspot" aria-describedby="pixel-world-hotspot-tooltip-test">Origin</button><button>Next control</button><PixelWorldHotspotTooltip locale="en" hotspot={{ id: "test", kind: "goal", label: "Goal" }} /></>);
    const close = screen.getByRole("button", { name: "Close hotspot explanation" });
    close.focus();
    fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Origin" }));
    close.focus();
    fireEvent.keyDown(close, { key: "Tab" });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Next control" }));
  });
  it("is touch and keyboard inspectable without selecting or executing gameplay", () => {
    const onHover = vi.fn();
    const onHotspotInspect = vi.fn();
    const onHotspotClear = vi.fn();
    const onHotspotRestoreFocus = vi.fn();
    render(() => (
      <PixelWorldHotspot
        locale="en"
        hotspot={hotspot}
        style={{ left: "20%", top: "20%" }}
        onHover={onHover}
        onHotspotInspect={onHotspotInspect}
        onHotspotClear={onHotspotClear}
        onHotspotRestoreFocus={onHotspotRestoreFocus}
      />
    ));
    const marker = screen.getByRole("button", { name: /Blocker hotspot: Blocked route/i });
    expect(marker).toHaveAttribute("type", "button");
    expect(marker).toHaveAttribute("data-hotspot-hit-target", "44");
    expect(marker.querySelector(".pixel-world-hotspot__glyph")).not.toBeNull();
    fireEvent.focus(marker);
    fireEvent.click(marker);
    expect(onHotspotInspect).toHaveBeenCalledWith({ kind: "hotspot", id: hotspot.id });
    expect(onHover).toHaveBeenCalledWith({ kind: "hotspot", id: hotspot.id });
    expect(onHover).not.toHaveBeenCalledWith({ kind: "agent", id: hotspot.id });
    marker.focus();
    fireEvent.keyDown(marker, { key: "Escape" });
    expect(onHotspotClear).toHaveBeenCalledTimes(1);
    expect(onHotspotRestoreFocus).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(marker);
  });

  it("keeps the explanation close control keyboard reachable and read-only", () => {
    const onClose = vi.fn();
    render(() => <PixelWorldHotspotTooltip locale="en" hotspot={hotspot} onClose={onClose} />);
    expect(screen.getByRole("status")).toHaveTextContent("Blocker: Blocked route");
    const close = screen.getByRole("button", { name: "Close hotspot explanation" });
    fireEvent.click(close);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes the explanation when Escape is pressed from the close control", () => {
    const onClose = vi.fn();
    render(() => <PixelWorldHotspotTooltip locale="en" hotspot={hotspot} onClose={onClose} />);
    const close = screen.getByRole("button", { name: "Close hotspot explanation" });
    fireEvent.keyDown(close, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

});
