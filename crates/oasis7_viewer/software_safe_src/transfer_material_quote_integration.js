import { createTransferMaterialQuoteRequestModule } from "./transfer_material_quote_request.js";
import { createTransferMaterialQuoteStateModule } from "./transfer_material_quote_state.js";

export function createTransferMaterialQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createTransferMaterialQuoteStateModule(dependencies), ...createTransferMaterialQuoteRequestModule(dependencies) };
}
