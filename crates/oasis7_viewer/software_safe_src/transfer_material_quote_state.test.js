import { describe, expect, it, vi } from "vitest";
import { createTransferMaterialQuoteStateModule } from "./transfer_material_quote_state.js";

function setup(status = "pending") {
  const state = { transferMaterialQuote: null, transferMaterialQuoteRequest: { status, error: null } };
  return { state, module: createTransferMaterialQuoteStateModule({ clone: structuredClone, getSearchParams: () => new URLSearchParams(), isTestApiEnabled: () => false, render: vi.fn(), state }) };
}

describe("transfer material quote state", () => {
  it("accepts only pending responses and invalidates snapshot-bound quotes", () => {
    const { state, module } = setup();
    expect(module.handleTransferMaterialQuote({ expected_received_amount: 18 })).toBe(true);
    expect(state.transferMaterialQuoteRequest.status).toBe("received");
    module.invalidateTransferMaterialQuote();
    expect(state.transferMaterialQuote).toBeNull();
    expect(state.transferMaterialQuoteRequest).toEqual({ status: "idle", error: null });
  });
  it("maps only its own protocol errors", () => {
    const { state, module } = setup();
    expect(module.handleTransferMaterialQuoteError({ action_id: "quote_power_sale", code: "x" })).toBe(false);
    expect(module.handleTransferMaterialQuoteError({ action_id: "quote_transfer_material", code: "x" })).toBe(true);
    expect(state.transferMaterialQuoteRequest.status).toBe("error");
  });
});
