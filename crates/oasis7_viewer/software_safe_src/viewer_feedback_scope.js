import { unwrap } from "solid-js/store";
import { pixelWorldScope } from "./pixel_world_scope.js";
const scopes = new WeakMap();
// Display provenance is separate from the original pending operation record.
export function bindFeedbackScope(feedback, state) {
  if (feedback && typeof feedback === "object") scopes.set(unwrap(feedback), pixelWorldScope(state));
  return feedback;
}
export function feedbackForScope(feedback, state) {
  if (!feedback || typeof feedback !== "object") return null;
  if (!scopes.has(unwrap(feedback))) bindFeedbackScope(feedback, state);
  return scopes.get(unwrap(feedback)) === pixelWorldScope(state) ? feedback : null;
}
