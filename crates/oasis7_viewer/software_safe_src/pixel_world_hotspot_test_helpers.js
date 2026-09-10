import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { expect } from "vitest";

export async function verifyHotspotCameraAndFocus(marker, onCameraEvent) {
  expect(marker).toHaveAttribute("data-hotspot-hit-target", "44");
  expect(marker.querySelector(".pixel-world-hotspot__glyph")).toHaveStyle({ width: "20px", height: "20px" });
  const initialLeft = marker.style.getPropertyValue("--hotspot-projected-x");
  onCameraEvent({ type: "camera_state_changed", camera: { zoom: 2, pan_x_px: 40, pan_y_px: -20 } });
  await waitFor(() => expect(marker.style.getPropertyValue("--hotspot-projected-x")).not.toBe(initialLeft));
  fireEvent.mouseEnter(marker);
  const hoverClose = screen.getByRole("button", { name: "关闭热点说明" });
  fireEvent.mouseLeave(marker, { relatedTarget: hoverClose });
  expect(hoverClose).toBeInTheDocument();
  fireEvent.click(hoverClose);
  await waitFor(() => expect(document.activeElement).toBe(marker));
  fireEvent.mouseEnter(marker);
  expect(screen.queryByRole("button", { name: "关闭热点说明" })).not.toBeInTheDocument();
  for (const method of ["click", "Escape"]) {
    fireEvent.click(marker);
    expect(screen.getByRole("status")).toHaveTextContent("阻塞: 缺料阻塞");
    const close = screen.getByRole("button", { name: "关闭热点说明" });
    marker.focus();
    fireEvent.keyDown(marker, { key: "Tab" });
    expect(document.activeElement).toBe(close);
    if (method === "click") fireEvent.click(close);
    else fireEvent.keyDown(close, { key: "Escape" });
    await waitFor(() => expect(document.activeElement).toBe(marker));
    fireEvent.mouseEnter(marker);
    expect(screen.queryByRole("button", { name: "关闭热点说明" })).not.toBeInTheDocument();
    onCameraEvent({ type: "hover_entity", selection: { kind: "hotspot", id: "hotspot-goal" } });
    expect(screen.queryByRole("button", { name: "关闭热点说明" })).not.toBeInTheDocument();
  }
  fireEvent.mouseMove(screen.getByRole("button", { name: /目标热点：稳定生产/ }));
  expect(screen.getByRole("status")).toHaveTextContent("目标: 稳定生产");
}
