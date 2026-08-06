import { createPowerSaleQuoteRequestModule } from "./power_sale_quote_request.js";
import { createPowerSaleQuoteStateModule } from "./power_sale_quote_state.js";

export function createPowerSaleQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createPowerSaleQuoteStateModule(dependencies), ...createPowerSaleQuoteRequestModule(dependencies) };
}
