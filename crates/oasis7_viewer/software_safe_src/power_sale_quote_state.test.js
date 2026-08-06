import { describe, expect, it, vi } from "vitest";
import { createPowerSaleQuoteStateModule } from "./power_sale_quote_state.js";

function createModule(testApi = false) {
  const state = { powerSaleQuote: null, powerSaleQuoteRequest: { status: "idle", error: null } };
  return { state, module: createPowerSaleQuoteStateModule({ clone: structuredClone, isTestApiEnabled: () => testApi, render: vi.fn(), state }) };
}

describe("power sale quote test injection", () => {
  it("is test-api gated while allowing a deterministic fixture to hydrate visible quote state", () => {
    const blocked = createModule(false);
    expect(() => blocked.module.injectPowerSaleQuoteForTest({ sale_amount: 10 })).toThrow(/test_api=1/);
    const allowed = createModule(true);
    expect(allowed.module.injectPowerSaleQuoteForTest({ seller_agent_id: "agent-0", sale_amount: 10, production_interrupt_risk: true })).toEqual(expect.objectContaining({ sale_amount: 10 }));
    expect(allowed.state.powerSaleQuoteRequest).toEqual({ status: "received", error: null });
  });
});
