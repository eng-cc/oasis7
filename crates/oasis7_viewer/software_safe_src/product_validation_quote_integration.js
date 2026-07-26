import { createProductValidationQuoteRequestModule } from "./product_validation_quote_request.js";
import { createProductValidationQuoteStateModule } from "./product_validation_quote_state.js";

export function createProductValidationQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  const stateModule = createProductValidationQuoteStateModule(dependencies);
  const requestModule = createProductValidationQuoteRequestModule(dependencies);
  return { ...stateModule, ...requestModule };
}
