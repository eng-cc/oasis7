import { createFragmentRefillPreviewRequestModule } from "./fragment_refill_preview_request.js";
import { createFragmentRefillPreviewStateModule } from "./fragment_refill_preview_state.js";

export function createFragmentRefillPreviewIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createFragmentRefillPreviewStateModule(dependencies), ...createFragmentRefillPreviewRequestModule(dependencies) };
}
