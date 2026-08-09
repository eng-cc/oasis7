import { createScheduleRecipeQuoteRequestModule } from "./schedule_recipe_quote_request.js";
import { createScheduleRecipeQuoteStateModule } from "./schedule_recipe_quote_state.js";

export function createScheduleRecipeQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createScheduleRecipeQuoteStateModule(dependencies), ...createScheduleRecipeQuoteRequestModule(dependencies) };
}
