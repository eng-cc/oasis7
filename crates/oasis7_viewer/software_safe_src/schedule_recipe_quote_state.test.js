import { describe, expect, it, vi } from "vitest";
import { createScheduleRecipeQuoteStateModule } from "./schedule_recipe_quote_state.js";

function createModule(status = "idle") {
  const state = { scheduleRecipeQuote: null, scheduleRecipeQuoteRequest: { status, error: null } };
  const module = createScheduleRecipeQuoteStateModule({ clone: structuredClone, getSearchParams: () => new URLSearchParams(), isTestApiEnabled: () => false, render: vi.fn(), state });
  return { module, state };
}

describe("schedule recipe quote state", () => {
  it("accepts a reply only for the pending request and marks it received", () => {
    const { module, state } = createModule("pending");
    expect(module.handleScheduleRecipeQuote({ factory_id: "factory-0", recipe_id: "assemble_hardware" })).toBe(true);
    expect(state.scheduleRecipeQuoteRequest).toEqual({ status: "received", error: null });
    expect(module.handleScheduleRecipeQuote({ factory_id: "late" })).toBe(false);
  });

  it("keeps an error visible only for this quote action and invalidates snapshot-bound values", () => {
    const { module, state } = createModule("pending");
    expect(module.handleScheduleRecipeQuoteError({ action_id: "quote_schedule_recipe", message: "rejected" })).toBe(true);
    expect(state.scheduleRecipeQuoteRequest).toEqual({ status: "error", error: "rejected" });
    expect(module.handleScheduleRecipeQuoteError({ action_id: "quote_refine_compound" })).toBe(false);
    state.scheduleRecipeQuote = { factory_id: "factory-old" };
    module.invalidateScheduleRecipeQuote();
    expect(state.scheduleRecipeQuote).toBeNull();
    expect(state.scheduleRecipeQuoteRequest).toEqual({ status: "idle", error: null });
  });
});
