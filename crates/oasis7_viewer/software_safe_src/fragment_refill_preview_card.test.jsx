import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { FragmentRefillPreviewPanel } from "./fragment_refill_preview_card.jsx";

const dueQuote = {
  chunk: { x: 2, y: -1, z: 0 },
  target_frag_id: "frag-7",
  current_frag_remaining_summary: "iron 8g",
  chunk_remaining_summary: "iron 8g, copper 3g",
  remaining_by_element_g: [{ element: "iron", remaining_g: 8 }, { element: "copper", remaining_g: 3 }],
  replenishment_enabled: true,
  replenishment_due: true,
  next_replenish_tick: 120,
  ticks_until_replenish: 0,
  wait_cost_ticks: 0,
  estimated_replenished_frag_count: 2,
  estimated_replenished_resource_hint: "mixed materials",
  next_industrial_goal_relevance: "current goal can use the replenished materials",
  wait_cost_summary: "available now",
  recommended_resource_action: "inspect replenished fragments",
};
const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("FragmentRefillPreviewPanel", () => {
  it("renders a due, signed read-only forecast with a localized estimate and no mutation control", () => {
    render(() => <FragmentRefillPreviewPanel quote={dueQuote} requestState={{ status: "received" }} requestFragmentRefillPreview={vi.fn()} locale="en" tr={tr} />);
    const panel = screen.getByTestId("fragment-refill-preview-panel");
    expect(panel).toHaveAttribute("data-quote-kind", "preflight");
    expect(within(panel).getByText("Material renewal forecast")).toBeInTheDocument();
    expect(within(panel).getByText("Due now")).toBeInTheDocument();
    expect(within(panel).getByText("Estimated replenished fragments")).toBeInTheDocument();
    expect(within(panel).getByText("2")).toBeInTheDocument();
    expect(within(panel).getByText(/does not replenish fragments, advance time, or create a receipt/i)).toBeInTheDocument();
    expect(within(panel).queryByRole("button", { name: /replenish|apply|commit/i })).not.toBeInTheDocument();
  });

  it("uses uncertainty-safe copy for scheduled and disabled forecasts while exposing pending, error, and stale feedback", async () => {
    const requestFragmentRefillPreview = vi.fn(async () => ({ ok: true }));
    const scheduled = { ...dueQuote, replenishment_due: false, ticks_until_replenish: 8, wait_cost_ticks: 8, estimated_replenished_frag_count: 2, recommended_resource_action: "wait_current_chunk" };
    let view = render(() => <FragmentRefillPreviewPanel quote={scheduled} requestState={{ status: "received" }} requestFragmentRefillPreview={requestFragmentRefillPreview} locale="en" tr={tr} />);
    expect(screen.getByText("Scheduled")).toBeInTheDocument();
    expect(screen.getByText(/read-only runtime estimate/i)).toBeInTheDocument();
    expect(screen.getByText(/Wait in this chunk and reassess/i)).toBeInTheDocument();
    fireEvent.input(screen.getByRole("spinbutton", { name: "Chunk X" }), { target: { value: "3" } });
    expect(screen.getByTestId("fragment-refill-preview-stale")).toHaveTextContent(/forecast is stale/i);
    expect(screen.queryByTestId("fragment-refill-preview-card")).not.toBeInTheDocument();
    fireEvent.submit(screen.getByTestId("fragment-refill-preview-request-form"));
    await vi.waitFor(() => expect(requestFragmentRefillPreview).toHaveBeenCalledWith("3", "-1", "0"));

    view.unmount();
    view = render(() => <FragmentRefillPreviewPanel quote={dueQuote} requestState={{ status: "pending" }} requestFragmentRefillPreview={requestFragmentRefillPreview} locale="en" tr={tr} />);
    expect(screen.getByRole("status")).toHaveTextContent(/Refreshing the forecast/i);
    expect(screen.getByRole("button", { name: /refreshing forecast/i })).toBeDisabled();

    view.unmount();
    view = render(() => <FragmentRefillPreviewPanel quote={dueQuote} requestState={{ status: "error" }} requestFragmentRefillPreview={requestFragmentRefillPreview} locale="en" tr={tr} />);
    expect(screen.getByRole("alert")).toHaveTextContent(/Could not get the material renewal forecast/i);
    expect(screen.queryByTestId("fragment-refill-preview-card")).not.toBeInTheDocument();

    view.unmount();
    render(() => <FragmentRefillPreviewPanel quote={{ ...dueQuote, replenishment_enabled: false, replenishment_due: false, next_replenish_tick: null, ticks_until_replenish: null, estimated_replenished_frag_count: 0, recommended_resource_action: "wait_current_chunk" }} requestState={{ status: "received" }} requestFragmentRefillPreview={requestFragmentRefillPreview} locale="en" tr={tr} />);
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getByText(/is disabled for this forecast; no renewal is promised/i)).toBeInTheDocument();
    expect(screen.getByText("No wait option")).toBeInTheDocument();
    expect(screen.queryByText(/Wait in this chunk and reassess/i)).not.toBeInTheDocument();
  });

  it("distinguishes a known zero target from an unavailable estimate", () => {
    const props = { requestState: { status: "received" }, requestFragmentRefillPreview: vi.fn(), locale: "en", tr };
    const view = render(() => <FragmentRefillPreviewPanel {...props} quote={{ ...dueQuote, replenishment_due: false, estimated_replenished_frag_count: 0, recommended_resource_action: "wait_current_chunk" }} />);
    expect(screen.getByText(/current forecast expects no new fragments/i)).toBeInTheDocument();
    expect(screen.getByText(/Check current resources, then move/i)).toBeInTheDocument();
    expect(screen.queryByText(/Wait in this chunk and reassess/i)).not.toBeInTheDocument();
    view.unmount();
    render(() => <FragmentRefillPreviewPanel {...props} quote={{ ...dueQuote, replenishment_due: false, estimated_replenished_frag_count: null, recommended_resource_action: "wait_current_chunk" }} />);
    expect(screen.getByText(/fragment-count estimate is unavailable/i)).toBeInTheDocument();
    expect(screen.getByText("Estimated replenished fragments").closest(".metric")).toHaveTextContent("Unavailable");
    expect(screen.getByText("No wait option")).toBeInTheDocument();
    expect(screen.getByText(/Recommended action: Refresh the world state before reassessing/i)).toBeInTheDocument();
    expect(screen.queryByText(/Wait in this chunk and reassess/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/current forecast expects no new fragments/i)).not.toBeInTheDocument();
  });

  it("requires a future opportunity before presenting a waiting recommendation", () => {
    render(() => <FragmentRefillPreviewPanel quote={{ ...dueQuote, replenishment_due: false, next_replenish_tick: null, ticks_until_replenish: 8, wait_cost_ticks: 8, recommended_resource_action: "wait_current_chunk" }} requestState={{ status: "received" }} requestFragmentRefillPreview={vi.fn()} locale="en" tr={tr} />);
    expect(screen.getByText("No wait option")).toBeInTheDocument();
    expect(screen.getByText(/Recommended action: Refresh the world state before reassessing/i)).toBeInTheDocument();
    expect(screen.queryByText(/Wait in this chunk and reassess/i)).not.toBeInTheDocument();
  });
});
