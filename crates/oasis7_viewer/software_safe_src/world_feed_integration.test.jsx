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
});
