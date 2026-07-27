import { createPowerSurvivalQuoteRequestModule } from "./power_survival_quote_request.js";
import { createPowerSurvivalQuoteStateModule } from "./power_survival_quote_state.js";

export function createPowerSurvivalQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createPowerSurvivalQuoteStateModule(dependencies), ...createPowerSurvivalQuoteRequestModule(dependencies) };
}
