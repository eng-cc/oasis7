export function createFragmentRefillPreviewStateModule({ clone, state }) {
  function handleFragmentRefillPreview(quote) {
    if (!quote || typeof quote !== "object" || state.fragmentRefillPreviewRequest?.status !== "pending") return false;
    state.fragmentRefillPreview = clone(quote);
    state.fragmentRefillPreviewRequest = { status: "received", error: null };
    return true;
  }

  function handleFragmentRefillPreviewError(error) {
    if (String(error?.action_id || "").trim() !== "preview_fragment_replenishment") return false;
    if (state.fragmentRefillPreviewRequest?.status === "pending") {
      state.fragmentRefillPreviewRequest = { status: "error", error: String(error?.message || error?.code || "fragment refill preview request failed") };
    }
    return true;
  }

  function invalidateFragmentRefillPreview() {
    state.fragmentRefillPreview = null;
    state.fragmentRefillPreviewRequest = { status: "idle", error: null };
  }

  return { handleFragmentRefillPreview, handleFragmentRefillPreviewError, invalidateFragmentRefillPreview };
}
