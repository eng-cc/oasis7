import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { WorldFeedPanel } from "./world_feed_panel.jsx";

const tr = (_locale, zh, en) => en;

describe("WorldFeedPanel", () => {
  it("keeps an explicit contextual surface for every protocol state", () => {
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "gap",
          events: [],
          worldId: "world-a",
          reorgEpoch: 4,
          gapReason: "reorg_epoch_changed",
          snapshotReloadRequired: true,
          stale: true,
        })}
        locale={() => "en"}
        tr={tr}
        onReloadSnapshot={() => {}}
      />
    ));
    expect(container.querySelector("#viewer-world-feed")).toBeTruthy();
    expect(screen.getByText("World activity is stale")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reload authoritative snapshot/i })).toBeInTheDocument();
  });

  it("renders events as ambient context and links only explicit receipt refs", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: 7, kind: "resource_change", summary: "Ore changed", detail: "ore +1", receipt_ref: null },
            { event_seq: 8, kind: "agent_spoke", summary: "Agent spoke", detail: "hello", receipt_ref: "receipt-8" },
          ],
          worldId: "world-a",
          reorgEpoch: 1,
          stale: false,
        })}
        locale={() => "en"}
        tr={tr}
        onReloadSnapshot={() => {}}
      />
    ));
    expect(screen.getByText("World Feed")).toBeInTheDocument();
    expect(screen.getByText("Ore changed")).toBeInTheDocument();
    expect(screen.getByText("Agent spoke")).toBeInTheDocument();
    expect(document.querySelectorAll("a[data-world-feed-receipt-ref]")).toHaveLength(1);
  });

  it("formats runtime kinds and keeps diagnostic JSON out of the player feed", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [{
            event_seq: 9,
            kind: "snapshot_created",
            summary: "World snapshot updated",
            detail: '{"kind":"SnapshotCreated","internal_state":"raw"}',
            receipt_ref: null,
          }],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(screen.getByText("World snapshot")).toBeInTheDocument();
    expect(screen.queryByText("snapshot_created")).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("internal_state");
  });

  it("presents World Feed events in ascending event sequence order", () => {
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: 7, kind: "resource_change", summary: "Older event", detail: "ore +1", receipt_ref: null },
            { event_seq: 8, kind: "agent_spoke", summary: "Newer event", detail: "hello", receipt_ref: null },
          ],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));
    expect(Array.from(container.querySelectorAll("[data-world-feed-event]"), (event) => event.getAttribute("data-world-feed-event"))).toEqual([
      "7",
      "8",
    ]);
  });

  it("sorts exact decimal u64 sequences without Number precision loss", () => {
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: "9007199254740993", kind: "newer", summary: "Newer exact event", detail: "", receipt_ref: null },
            { event_seq: "9007199254740992", kind: "older", summary: "Older exact event", detail: "", receipt_ref: null },
          ],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(Array.from(container.querySelectorAll("[data-world-feed-event]"), (event) => event.getAttribute("data-world-feed-event"))).toEqual([
      "9007199254740992",
      "9007199254740993",
    ]);
  });
});
