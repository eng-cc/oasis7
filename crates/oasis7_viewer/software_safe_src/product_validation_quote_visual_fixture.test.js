import { describe, expect, it, vi } from "vitest";
import { installProductValidationQuoteVisualFixture } from "./product_validation_quote_visual_fixture.js";

describe("installProductValidationQuoteVisualFixture", () => {
  it("hydrates the World Summary fixture before injecting the product quote", () => {
    const fixtures = {};
    const core = {
      injectSnapshot: vi.fn(),
      applySelection: vi.fn(),
      injectProductValidationQuoteForTest: vi.fn(),
    };
    const setFixturePlayerAuth = vi.fn();
    const viewerFixtureBaseSnapshot = vi.fn(() => ({ player_gameplay: {} }));
    installProductValidationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });

    fixtures.product_validation_quote();

    expect(core.injectSnapshot).toHaveBeenCalledWith({ player_gameplay: {} }, { returnState: false });
    expect(core.applySelection).toHaveBeenCalledWith({ kind: "agent", id: "agent-0" });
    expect(setFixturePlayerAuth).toHaveBeenCalledOnce();
    expect(core.injectProductValidationQuoteForTest).toHaveBeenCalledWith(expect.objectContaining({
      product_id: "logistics_drone",
      submission_allowed: true,
    }));
  });
});
