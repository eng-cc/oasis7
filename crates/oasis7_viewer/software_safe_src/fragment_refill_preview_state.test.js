import { describe, expect, it } from "vitest";
import { createFragmentRefillPreviewStateModule } from "./fragment_refill_preview_state.js";

function createModule(requestKey = "2:-1:0") {
  const state = {
    fragmentRefillPreview: null,
    fragmentRefillPreviewRequest: { status: "pending", error: null, requestKey },
  };
  return {
    module: createFragmentRefillPreviewStateModule({ clone: structuredClone, state }),
    state,
  };
}

describe("fragment refill preview response correlation", () => {
  it("ignores delayed or malformed replies without clearing the newer pending request", () => {
    const { module, state } = createModule("2:-1:0");

    module.invalidateFragmentRefillPreview();
    state.fragmentRefillPreviewRequest = { status: "pending", error: null, requestKey: "7:8:9" };

    expect(module.handleFragmentRefillPreview({ chunk: { x: 2, y: -1, z: 0 } })).toBe(false);
    expect(state.fragmentRefillPreview).toBeNull();
    expect(state.fragmentRefillPreviewRequest).toEqual({ status: "pending", error: null, requestKey: "7:8:9" });

    expect(module.handleFragmentRefillPreview({ chunk: { x: "7", y: 8, z: 9 } })).toBe(false);
    expect(state.fragmentRefillPreviewRequest).toEqual({ status: "pending", error: null, requestKey: "7:8:9" });

    const current = { chunk: { x: 7, y: 8, z: 9 }, replenishment_due: true };
    expect(module.handleFragmentRefillPreview(current)).toBe(true);
    expect(state.fragmentRefillPreview).toEqual(current);
    expect(state.fragmentRefillPreviewRequest).toEqual({ status: "received", error: null });
  });
});
