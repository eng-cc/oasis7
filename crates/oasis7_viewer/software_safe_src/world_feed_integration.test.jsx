import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { WorldFeedSurface } from "./world_feed_integration.jsx";

const tr = (_locale, _zh, en) => en;

describe("WorldFeedSurface", () => {
  it("passes unavailable-feed retry through its distinct callback", () => {
    const onRetryFeed = vi.fn();
    const onReloadSnapshot = vi.fn();
    const core = {
      state: {
        worldFeed: {
          status: "unavailable",
          events: [],
          worldId: "world-a",
          reorgEpoch: "0",
          unavailableReason: "permission_denied",
          snapshotReloadRequired: false,
          stale: true,
        },
      },
    };

    render(() => (
      <WorldFeedSurface
        core={core}
        locale={() => "en"}
        tr={tr}
        onRetryFeed={onRetryFeed}
        onReloadSnapshot={onReloadSnapshot}
      />
    ));

    fireEvent.click(screen.getByRole("button", { name: /retry world feed/i }));
    expect(onRetryFeed).toHaveBeenCalledTimes(1);
    expect(onReloadSnapshot).not.toHaveBeenCalled();
  });

  it("opens the visible command details route after locating a live module", async () => {
    const detailsPanel = document.createElement("section");
    detailsPanel.id = "viewer-details-panel";
    detailsPanel.tabIndex = -1;
    document.body.appendChild(detailsPanel);
    const core = {
      state: {
        worldFeed: {
          status: "ready",
          events: [{ event_seq: 101, kind: "ModuleVisualEntityUpserted", module_visual_entity_id: "module-relay" }],
        },
      },
      entityCollections: () => ({ moduleVisualEntities: [{ id: "module-relay", label: "Relay Seven" }] }),
      focusModuleFromEvent: vi.fn(() => ({ kind: "module_visual", id: "module-relay" })),
    };

    render(() => <WorldFeedSurface core={core} locale={() => "en"} tr={tr} />);
    fireEvent.click(screen.getByRole("button", { name: /locate module relay seven/i }));

    expect(core.focusModuleFromEvent).toHaveBeenCalledTimes(1);
    expect(window.location.hash).toBe("#viewer-details-panel");
    expect(document.activeElement).toBe(detailsPanel);
  });
});
