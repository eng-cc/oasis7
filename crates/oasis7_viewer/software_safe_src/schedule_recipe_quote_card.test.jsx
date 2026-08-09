import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { ScheduleRecipeQuoteCard, ScheduleRecipeQuotePanel } from "./schedule_recipe_quote_card.jsx";

const normalQuote = {
  owner_agent_id: "agent-0", factory_id: "factory-0", recipe_id: "assemble_hardware", batches: 2,
  base_duration_ticks: 6, electricity_cost: 12, electricity_after: 88, hardware_cost: 4,
  data_output: 8, finished_product_id: "hardware", finished_product_units: 2,
  local_shortage_delay_ticks: 0, shortage_reason: "none", recommended_pre_step: "schedule_now",
  runway_before_ticks: 40, runway_after_ticks: 40, downtime_threshold_ppm: 250000,
  continue_production_risk: "normal", maintenance_pressure_delta: "unchanged",
  recommended_maintenance_action: "none",
};

function tr(locale, zh, en) { return locale === "zh" ? zh : en; }

describe("ScheduleRecipeQuoteCard", () => {
  it("keeps the pre-submit quote read-only while separating electricity from battery runway", () => {
    render(() => <ScheduleRecipeQuoteCard quote={normalQuote} locale="en" tr={tr} />);
    const card = screen.getByTestId("schedule-recipe-quote");

    expect(card).toHaveAttribute("data-quote-kind", "preflight");
    expect(within(card).getByText("Schedule Recipe Quote")).toBeInTheDocument();
    expect(within(card).getByText(/read-only quote.*does not schedule production/i)).toBeInTheDocument();
    expect(within(card).getByText("Electricity after scheduling")).toBeInTheDocument();
    expect(within(card).getByText("Battery runway")).toBeInTheDocument();
    expect(within(card).getByText("88")).toBeInTheDocument();
    expect(within(card).getByText("40 → 40 ticks")).toBeInTheDocument();
    expect(within(card).getByText(/Maintenance: not tracked for this quote/i)).toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: /schedule|submit|commit/i })).not.toBeInTheDocument();
  });

  it.each(["critical", "shutdown"])("warns before neutral metrics for %s power risk", (continueProductionRisk) => {
    render(() => <ScheduleRecipeQuoteCard quote={{ ...normalQuote, continue_production_risk: continueProductionRisk, recommended_pre_step: "restore_power", runway_before_ticks: 3, runway_after_ticks: 3, local_shortage_delay_ticks: 5, shortage_reason: "local_hardware_shortage" }} locale="en" tr={tr} />);
    const card = screen.getByTestId("schedule-recipe-quote");
    const warning = within(card).getByTestId("schedule-recipe-quote-risk");
    const grid = card.querySelector(".summary-grid");

    expect(warning).toHaveClass("feedback-summary--warn");
    expect(warning.compareDocumentPosition(grid) & Node.DOCUMENT_POSITION_FOLLOWING).not.toBe(0);
    expect(within(warning).getByText(continueProductionRisk === "critical" ? /Critical power risk/ : /Shutdown risk/)).toBeInTheDocument();
    expect(within(card).getByTestId("schedule-recipe-quote-recommendation")).toHaveTextContent(/Restore power before scheduling production/i);
    expect(within(card).getByText(/Local shortage: 5 ticks/i)).toBeInTheDocument();
  });

  it("keeps normal risk neutral while keeping electricity and runway adjacent", () => {
    render(() => <ScheduleRecipeQuoteCard quote={normalQuote} locale="en" tr={tr} />);
    const card = screen.getByTestId("schedule-recipe-quote");
    expect(within(card).getByTestId("schedule-recipe-quote-risk")).not.toHaveClass("feedback-summary--warn");
    const labels = Array.from(card.querySelectorAll(".summary-grid .metric__label")).map((element) => element.textContent);
    expect(labels.indexOf("Battery runway")).toBe(labels.indexOf("Electricity after scheduling") + 1);
  });

  it("keeps the request reachable and presents pending or sanitized error feedback", async () => {
    const requestScheduleRecipeQuote = vi.fn(async () => ({ ok: true }));
    render(() => <ScheduleRecipeQuotePanel quote={null} requestState={{ status: "pending", error: null }} requestScheduleRecipeQuote={requestScheduleRecipeQuote} locale="en" tr={tr} />);
    const panel = screen.getByTestId("schedule-recipe-quote-panel");
    expect(within(panel).getByRole("status")).toHaveTextContent(/requesting quote/i);
    expect(within(panel).queryByTestId("schedule-recipe-quote")).not.toBeInTheDocument();

    render(() => <ScheduleRecipeQuotePanel quote={null} requestState={{ status: "error", error: "quote_schedule_recipe rejected" }} requestScheduleRecipeQuote={requestScheduleRecipeQuote} locale="en" tr={tr} />);
    expect(screen.getAllByRole("alert").at(-1)).toHaveTextContent(/Could not get the schedule quote/i);
    expect(screen.getAllByRole("alert").at(-1)).not.toHaveTextContent("quote_schedule_recipe rejected");

    render(() => <ScheduleRecipeQuotePanel quote={normalQuote} requestState={{ status: "received", error: null }} requestScheduleRecipeQuote={requestScheduleRecipeQuote} locale="en" tr={tr} />);
    fireEvent.submit(screen.getAllByTestId("schedule-recipe-quote-request-form").at(-1));
    await vi.waitFor(() => expect(requestScheduleRecipeQuote).toHaveBeenCalledWith("factory-0", "assemble_hardware", "1"));
  });
});
