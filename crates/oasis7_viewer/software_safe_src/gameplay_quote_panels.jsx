import { ProductValidationQuotePanel } from "./product_validation_quote_card.jsx";
import { RefineQuotePreflightPanel } from "./refine_quote_preflight_card.jsx";

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
