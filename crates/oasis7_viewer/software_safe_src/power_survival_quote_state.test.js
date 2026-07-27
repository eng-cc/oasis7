import { describe, expect, it, vi } from "vitest";
import { createPowerSurvivalQuoteStateModule } from "./power_survival_quote_state.js";

function createModule(requestStatus = "idle") {
  const state = {
    powerSurvivalQuote: null,
    powerSurvivalQuoteRequest: { status: requestStatus, error: null },
  };
  const module = createPowerSurvivalQuoteStateModule({
    clone: structuredClone,
    getSearchParams: () => new URLSearchParams(),
    isTestApiEnabled: () => false,
    render: vi.fn(),
    state,
  });
  return { module, state };
}

describe("power survival quote response freshness", () => {
  it("accepts only the response for the single pending request", () => {
    const { module, state } = createModule("pending");
    expect(module.handlePowerSurvivalQuote({ seller_agent_id: "agent-1" })).toBe(true);
    expect(state.powerSurvivalQuote).toEqual({ seller_agent_id: "agent-1" });
    expect(module.handlePowerSurvivalQuote({ seller_agent_id: "agent-stale" })).toBe(false);
    expect(state.powerSurvivalQuote).toEqual({ seller_agent_id: "agent-1" });
  });

  it("invalidates both a visible quote and an in-flight response after snapshot state changes", () => {
    const { module, state } = createModule("pending");
    state.powerSurvivalQuote = { seller_agent_id: "agent-old" };
    module.invalidatePowerSurvivalQuote();
    expect(state.powerSurvivalQuote).toBeNull();
    expect(state.powerSurvivalQuoteRequest.status).toBe("idle");
    expect(module.handlePowerSurvivalQuote({ seller_agent_id: "agent-late" })).toBe(false);
    expect(module.handlePowerSurvivalQuoteError({ action_id: "quote_power_survival", message: "late rejection" })).toBe(false);
    expect(state.powerSurvivalQuote).toBeNull();
    expect(state.powerSurvivalQuoteRequest.status).toBe("idle");
  });
});
