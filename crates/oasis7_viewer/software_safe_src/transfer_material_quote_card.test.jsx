import { render, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { TransferMaterialQuoteCard, TransferMaterialQuotePanel } from "./transfer_material_quote_card.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;
const quote = { requester_agent_id: "agent-0", from_ledger: "site:source", to_ledger: "site:destination", kind: "iron_ingot", requested_amount: 20, submission_feasible: true, max_transferable_amount: 40, sent_amount: 20, distance_km: 200, loss_bps: 5, expected_loss_amount: 2, expected_received_amount: 18, source_amount_before: 40, source_amount_after: 20, destination_amount_before: 0, destination_expected_amount_after: 18, ticks_until_arrival: 2, ready_at: 3, effective_priority: "standard", priority_reason: "material_default_priority", inflight_before: 0, inflight_capacity: 2, recommendation: "submit_transfer", conditional: true };

describe("TransferMaterialQuote", () => {
  it("renders player-readable arrival, loss, capacity, priority, and recommendation without raw enums", () => {
    const view = render(() => <TransferMaterialQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = within(view.getByTestId("transfer-material-quote"));
    expect(card.getByText(/Iron ingot/)).toBeInTheDocument();
    expect(card.queryByText("iron_ingot")).not.toBeInTheDocument();
    expect(card.getByText(/Expected received/)).toBeInTheDocument();
    expect(card.getAllByText(/18/).length).toBeGreaterThan(0);
    expect(card.getByText(/Submit the transfer/)).toBeInTheDocument();
    expect(card.getByText(/Standard priority/)).toBeInTheDocument();
    expect(card.getByText(/Conditional quote/)).toBeInTheDocument();
    expect(card.queryByText("material_default_priority")).not.toBeInTheDocument();
  });

  it("renders the known material label in Chinese without exposing the raw key", () => {
    const view = render(() => <TransferMaterialQuoteCard quote={quote} locale="zh" tr={tr} />);
    const card = within(view.getByTestId("transfer-material-quote"));
    expect(card.getByText(/铁锭/)).toBeInTheDocument();
    expect(card.queryByText("iron_ingot")).not.toBeInTheDocument();
  });

  it("renders path identity, ordered routes, tariff electricity, reroute count, and restore-power guidance", () => {
    const routeQuote = {
      ...quote,
      submission_feasible: false,
      path_id: "path:source-relay-destination",
      route_ids: ["route:source-relay", "route:relay-destination"],
      tariff_electricity_total: 12,
      reroute_count: 1,
      recommendation: "restore_power_or_use_lower_tariff_route",
    };
    const view = render(() => <TransferMaterialQuoteCard quote={routeQuote} locale="en" tr={tr} />);
    const cardElement = view.getByTestId("transfer-material-quote");
    const card = within(cardElement);
    expect(cardElement).toHaveTextContent(/Path identity.*path:source-relay-destination/i);
    expect(cardElement).toHaveTextContent(/Routes.*route:source-relay.*route:relay-destination/i);
    expect(cardElement).toHaveTextContent(/Tariff electricity.*12/i);
    expect(cardElement).toHaveTextContent(/Reroute count.*1/i);
    expect(card.getByTestId("transfer-material-quote-recommendation")).toHaveTextContent(/restore power|lower-tariff route/i);
    expect(cardElement).not.toHaveTextContent("restore_power_or_use_lower_tariff_route");
  });

  it("renders unavailable-path guidance without leaking the capacity recommendation enum", () => {
    const unavailableQuote = {
      ...quote,
      submission_feasible: false,
      path_id: null,
      route_ids: ["route:blocked"],
      recommendation: "wait_for_transit_capacity",
    };
    const view = render(() => <TransferMaterialQuoteCard quote={unavailableQuote} locale="en" tr={tr} />);
    const cardElement = view.getByTestId("transfer-material-quote");
    const card = within(cardElement);
    expect(cardElement).toHaveTextContent(/Path unavailable/i);
    expect(card.getByTestId("transfer-material-quote-recommendation")).toHaveTextContent(/Wait for transit capacity/i);
    expect(cardElement).not.toHaveTextContent("wait_for_transit_capacity");
  });

  it("submits the complete logistics form through the controlled request callback", async () => {
    const requestTransferMaterialQuote = vi.fn(() => Promise.resolve({ ok: true }));
    const view = render(() => <TransferMaterialQuotePanel requesterAgentId="agent-0" quote={null} requestState={{ status: "idle" }} requestTransferMaterialQuote={requestTransferMaterialQuote} locale="zh" tr={tr} />);
    const materialInput = view.getByLabelText("物料");
    expect(materialInput).toHaveValue("铁锭");
    expect(materialInput).not.toHaveValue("iron_ingot");
    await view.getByTestId("transfer-material-quote-request-form").requestSubmit();
    expect(requestTransferMaterialQuote).toHaveBeenCalledWith("agent-0", "site:source", "site:destination", "iron_ingot", "20", "200", "");
  });
});
