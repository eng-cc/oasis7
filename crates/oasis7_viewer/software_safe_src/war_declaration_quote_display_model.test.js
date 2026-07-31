import { describe, expect, it } from "vitest";
import { buildWarDeclarationQuoteDisplayModel } from "./war_declaration_quote_display_model.js";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("war declaration quote display model", () => {
  it("maps stable protocol codes without exposing raw enum values", () => {
    const view = buildWarDeclarationQuoteDisplayModel({
      settlement_path: "core_fallback", conflict_status: "pending_conflict", projected_outcome: "defender_wins",
      settlement_risk_code: "loss_resource_and_reputation", recommended_war_action: "gather_resources",
      alternative_action: "wait", mobilization_affordable: false,
    }, "zh", tr);

    expect(view).toMatchObject({ conflictStatus: "已有宣战正在等待处理", recommendedAction: "先收集动员资源", affordability: "动员资源不足" });
    expect(JSON.stringify(view)).not.toMatch(/pending_conflict|gather_resources/);
  });
});
