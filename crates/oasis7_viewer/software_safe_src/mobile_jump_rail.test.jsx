import { fireEvent, screen, within } from "@solidjs/testing-library";
import { readFile } from "node:fs/promises";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./pixel_world_host.jsx", () => ({ PixelWorldHost: () => <div data-testid="pixel-world-host" /> }));

let dispose = null;

beforeEach(async () => {
  vi.resetModules();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en");
  document.body.innerHTML = "";
  const core = await import("./legacy_core.js");
  const { mountViewerApp } = await import("./main.jsx");
  core.initializeSoftwareSafeCore();
  const root = document.createElement("div");
  document.body.appendChild(root);
  dispose = mountViewerApp(root);
});

afterEach(() => {
  dispose?.();
  dispose = null;
  document.body.innerHTML = "";
});

describe("mobile jump rail", () => {
  it("wraps rather than horizontally scrolling its quiet secondary links", async () => {
    const viewerHtml = await readFile("viewer.html", "utf8");
    expect(viewerHtml).toMatch(/\.mobile-rail\s*\{[\s\S]*?position: sticky;[\s\S]*?top: 10px;[\s\S]*?flex-wrap: wrap; overflow: visible;/);
  });

  it("keeps Quote as the one muted fifth link and preserves its anchor focus behavior", () => {
    const nav = screen.getByRole("navigation", { name: /primary entry section navigation/i });
    expect(Array.from(nav.querySelectorAll("a")).map((link) => link.textContent?.trim())).toEqual(["World", "Targets", "Command", "Diagnostics", "Quote"]);
    const quote = within(nav).getByRole("link", { name: "Quote" });
    expect(quote).toHaveAttribute("href", "#viewer-refine-quote-panel");
    expect(quote).toHaveClass("mobile-rail__link--diagnostics");
    expect(screen.getAllByRole("link", { name: "Quote" })).toHaveLength(1);
    const panel = document.createElement("section"); panel.id = "viewer-refine-quote-panel"; const scrollIntoView = vi.fn(); panel.scrollIntoView = scrollIntoView; document.body.appendChild(panel);
    fireEvent.click(quote);
    expect(scrollIntoView).toHaveBeenCalledWith({ behavior: "auto", block: "start", inline: "nearest" });
    expect(window.location.hash).toBe("#viewer-refine-quote-panel");
  });
});
