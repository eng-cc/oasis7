import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { ProductValidationQuoteCard, ProductValidationQuotePanel } from "./product_validation_quote_card.jsx";

const quote = {
  product_id: "logistics_drone",
  product_role: "explore",
  tradable: true,
  stage_before: "bootstrap",
  stage_after: "bootstrap",
  unlock_or_value_class: "scale_out",
  recommended_action: "advance_industry_stage",
  submission_allowed: true,
  missing_prerequisite: "industry_stage=scale_out",
  reachable_advance_or_recovery: "complete_reachable_industry_progress",
};

const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("ProductValidationQuoteCard", () => {
  it("renders localized hierarchy while retaining raw authoritative DTO fields in the DOM", () => {
    render(() => <ProductValidationQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("product-validation-quote");
    expect(card).toHaveAttribute("data-product-id", "logistics_drone");
    expect(card).toHaveAttribute("data-product-role", "explore");
    expect(card).toHaveAttribute("data-stage-before", "bootstrap");
    expect(card).toHaveAttribute("data-submission-allowed", "true");
    expect(within(card).getByText("Product Validation Quote")).toBeInTheDocument();
    expect(within(card).getByText(/Explore/)).toBeInTheDocument();
    expect(within(card).getByText(/Bootstrap → Bootstrap/)).toBeInTheDocument();
    expect(within(card).getByText("Scale out")).toBeInTheDocument();
    expect(within(card).getByText(/Known preflight \/ No known blocker/)).toBeInTheDocument();
    expect(within(card).getByText(/does not evaluate or predict an arbitrary module outcome/i)).toBeInTheDocument();
    expect(within(card).queryByText(/Allowed by runtime|Blocked by runtime/i)).not.toBeInTheDocument();
    expect(within(card).getByTestId("product-validation-quote-advisory")).toHaveTextContent(/advisory and the preflight found no known blocker/i);
    expect(within(card).getByTestId("product-validation-quote-recommended-action")).toHaveTextContent(/Advance industry stage/);
    expect(within(card).getByText(/Missing prerequisite/)).toHaveAttribute("data-raw-missing-prerequisite", "industry_stage=scale_out");
    expect(within(card).queryByRole("button", { name: /submit|validate|confirm/i })).not.toBeInTheDocument();
  });

  it("keeps the compatibility field while presenting a known preflight state", () => {
    render(() => <ProductValidationQuoteCard quote={quote} locale="zh" tr={tr} />);
    const card = screen.getByTestId("product-validation-quote");
    expect(card).toHaveAttribute("data-submission-allowed", "true");
    expect(within(card).getByText("已知预估 / 未发现阻塞")).toBeInTheDocument();
    expect(within(card).getByTestId("product-validation-quote-advisory")).toHaveTextContent(/这是建议，预估未发现阻塞/);
  });

  it("labels a false compatibility field as a known preflight blocker, not module prediction", () => {
    render(() => <ProductValidationQuoteCard quote={{ ...quote, submission_allowed: false }} locale="en" tr={tr} />);
    const card = screen.getByTestId("product-validation-quote");
    expect(card).toHaveAttribute("data-submission-allowed", "false");
    expect(within(card).getByText("Known preflight / Known blocker")).toBeInTheDocument();
    expect(within(card).getByTestId("product-validation-quote-advisory")).toHaveTextContent(/preflight found a known blocker/i);
    expect(within(card).queryByText(/module outcome/i)).toHaveTextContent(/does not evaluate or predict/i);
  });

  it("requests a signed read-only quote before confirmation and exposes receive state", async () => {
    const requestProductValidationQuote = vi.fn(async () => ({ ok: true }));
    render(() => <ProductValidationQuotePanel quote={null} requestProductValidationQuote={requestProductValidationQuote} locale="en" tr={tr} />);
    const panel = screen.getByTestId("product-validation-quote-panel");
    fireEvent.input(within(panel).getByRole("textbox", { name: "Product ID" }), { target: { value: "sensor_pack" } });
    fireEvent.input(within(panel).getByRole("spinbutton", { name: "Amount" }), { target: { value: "2" } });
    fireEvent.submit(screen.getByTestId("product-validation-quote-request-form"));
    await vi.waitFor(() => expect(requestProductValidationQuote).toHaveBeenCalledWith("sensor_pack", "2"));
    expect(within(panel).queryByTestId("product-validation-quote")).not.toBeInTheDocument();

    render(() => <ProductValidationQuotePanel quote={quote} requestState={{ status: "received" }} requestProductValidationQuote={requestProductValidationQuote} locale="en" tr={tr} />);
    expect(screen.getAllByRole("status").at(-1)).toHaveTextContent(/review the guidance before confirmation/i);
    expect(screen.getAllByTestId("product-validation-quote").at(-1)).toBeInTheDocument();
  });
});
