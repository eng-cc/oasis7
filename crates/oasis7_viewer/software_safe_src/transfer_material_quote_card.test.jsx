import { render, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { TransferMaterialQuoteCard, TransferMaterialQuotePanel } from "./transfer_material_quote_card.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;
const quote = { requester_agent_id: "agent-0", from_ledger: "site:source", to_ledger: "site:destination", kind: "iron_ingot", requested_amount: 20, submission_feasible: true, max_transferable_amount: 40, sent_amount: 20, distance_km: 200, loss_bps: 5, expected_loss_amount: 2, expected_received_amount: 18, source_amount_before: 40, source_amount_after: 20, destination_amount_before: 0, destination_expected_amount_after: 18, ticks_until_arrival: 2, ready_at: 3, effective_priority: "standard", priority_reason: "material_default_priority", inflight_before: 0, inflight_capacity: 2, recommendation: "submit_transfer", conditional: true };

describe("TransferMaterialQuote", () => {
  it("renders player-readable arrival, loss, capacity, priority, and recommendation without raw enums", () => {
    const view = render(() => <TransferMaterialQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = within(view.getByTestId("transfer-material-quote"));
    expect(card.getByText(/Expected received/)).toBeInTheDocument();
    expect(card.getAllByText(/18/).length).toBeGreaterThan(0);
    expect(card.getByText(/Submit the transfer/)).toBeInTheDocument();
    expect(card.getByText(/Standard priority/)).toBeInTheDocument();
    expect(card.getByText(/Conditional quote/)).toBeInTheDocument();
    expect(card.queryByText("material_default_priority")).not.toBeInTheDocument();
  });

  it("submits the complete logistics form through the controlled request callback", async () => {
    const requestTransferMaterialQuote = vi.fn(() => Promise.resolve({ ok: true }));
    const view = render(() => <TransferMaterialQuotePanel requesterAgentId="agent-0" quote={null} requestState={{ status: "idle" }} requestTransferMaterialQuote={requestTransferMaterialQuote} locale="zh" tr={tr} />);
    await view.getByTestId("transfer-material-quote-request-form").requestSubmit();
    expect(requestTransferMaterialQuote).toHaveBeenCalledWith("agent-0", "site:source", "site:destination", "iron_ingot", "20", "200", "");
  });
});
