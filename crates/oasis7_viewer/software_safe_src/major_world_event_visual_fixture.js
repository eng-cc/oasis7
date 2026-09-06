export function installMajorWorldEventCrisisVisualFixture(fixtures, { core, viewerFixtureBaseSnapshot }) {
  fixtures.major_world_event_crisis = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    const mode = String(new URLSearchParams(window.location.search || "").get("major_event_state") || "current");
    const historical = mode === "replay";
    const suppressed = mode === "gap" || mode === "denied";
    core.state.worldFeed = {
      status: mode === "gap" ? "gap" : mode === "denied" ? "unavailable" : historical ? "replay" : "ready",
      schemaVersion: "world_feed/v1",
      worldId: "fixture-world",
      reorgEpoch: "0",
      cursor: "fixture-current",
      stale: suppressed,
      gapReason: mode === "gap" ? "reorg_epoch_changed" : null,
      unavailableReason: mode === "denied" ? "permission_denied" : null,
      snapshotReloadRequired: mode === "gap",
      requestInFlight: false,
      events: suppressed ? [] : [{
        event_seq: "7",
        kind: "crisis_spawned",
        summary: "Crisis event",
        detail: "",
        receipt_ref: null,
        major_event: {
          schema_version: "major_world_event/v1",
          identity: { world_id: "fixture-world", reorg_epoch: "0", event_seq: "7" },
          category: "crisis",
          subtype: "power_shortage",
          severity: 4,
          lifecycle: "active",
          source: { authority: "runtime_journal", event_kind: "crisis_spawned" },
          freshness: historical ? "last_known" : "current",
          visibility: "public",
          logical_time: "42",
          causal_reference: null,
          world_anchor: { scope: "world", entity_id: "crisis-fixture" },
        },
      }],
    };
    core.requestRender();
  };
}
