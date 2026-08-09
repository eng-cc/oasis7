import { ProductValidationQuotePanel } from "./product_validation_quote_card.jsx";
import { PowerSurvivalQuotePanel } from "./power_survival_quote_card.jsx";
import { PowerSaleQuotePanel } from "./power_sale_quote_card.jsx";
import { FragmentRefillPreviewPanel } from "./fragment_refill_preview_card.jsx";
import { RefineQuotePreflightPanel } from "./refine_quote_preflight_card.jsx";
import { MarketQuoteDecisionPanel } from "./market_quote_decision_card.jsx";
import { WarDeclarationQuotePanel } from "./war_declaration_quote_card.jsx";
import { GovernanceVoteQuotePanel } from "./governance_vote_quote_card.jsx";
import { ScheduleRecipeQuotePanel } from "./schedule_recipe_quote_card.jsx";

export function ScheduleRecipeQuoteGameplayPanel(props) {
  return <ScheduleRecipeQuotePanel quote={props.core.state.scheduleRecipeQuote} requestState={props.core.state.scheduleRecipeQuoteRequest} requestScheduleRecipeQuote={props.core.requestScheduleRecipeQuote} locale={props.locale} tr={props.tr} />;
}

export function RefineQuoteGameplayPanel(props) {
  return <RefineQuotePreflightPanel
    quote={props.core.state.refineQuotePreflight}
    requestState={props.core.state.refineQuoteRequest}
    requestRefineQuote={props.core.requestRefineQuote}
    locale={props.locale}
    tr={props.tr}
  />;
}

export function ProductValidationQuoteGameplayPanel(props) {
  return <ProductValidationQuotePanel
    quote={props.core.state.productValidationQuote}
    requestState={props.core.state.productValidationQuoteRequest}
    requestProductValidationQuote={props.core.requestProductValidationQuote}
    locale={props.locale}
    tr={props.tr}
  />;
}

export function PowerSurvivalQuoteGameplayPanel(props) {
  return <PowerSurvivalQuotePanel
    quote={props.core.state.powerSurvivalQuote}
    requestState={props.core.state.powerSurvivalQuoteRequest}
    requestPowerSurvivalQuote={props.core.requestPowerSurvivalQuote}
    locale={props.locale}
    tr={props.tr}
  />;
}

export function PowerSaleQuoteGameplayPanel(props) {
  return <PowerSaleQuotePanel
    quote={props.core.state.powerSaleQuote}
    requestState={props.core.state.powerSaleQuoteRequest}
    requestPowerSaleQuote={props.core.requestPowerSaleQuote}
    locale={props.locale}
    tr={props.tr}
  />;
}

export function FragmentRefillPreviewGameplayPanel(props) {
  return <FragmentRefillPreviewPanel quote={props.core.state.fragmentRefillPreview} requestState={props.core.state.fragmentRefillPreviewRequest} requestFragmentRefillPreview={props.core.requestFragmentRefillPreview} locale={props.locale} tr={props.tr} />;
}

export function MarketQuoteDecisionGameplayPanel(props) { return <MarketQuoteDecisionPanel quote={props.core.state.marketQuoteDecision} requestState={props.core.state.marketQuoteDecisionRequest} requestMarketQuoteDecision={props.core.requestMarketQuoteDecision} locale={props.locale} tr={props.tr} />; }
export function WarDeclarationQuoteGameplayPanel(props) { return <WarDeclarationQuotePanel quote={props.core.state.warDeclarationQuote} requestState={props.core.state.warDeclarationQuoteRequest} requestWarDeclarationQuote={props.core.requestWarDeclarationQuote} logicalTime={props.core.state.logicalTime} locale={props.locale} tr={props.tr} />; }
export function GovernanceVoteQuoteGameplayPanel(props) { return <GovernanceVoteQuotePanel quote={props.core.state.governanceVoteQuote} requestState={props.core.state.governanceVoteQuoteRequest} requestGovernanceVoteQuote={props.core.requestGovernanceVoteQuote} locale={props.locale} tr={props.tr} />; }
