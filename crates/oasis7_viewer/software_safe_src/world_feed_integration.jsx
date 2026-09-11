import { WorldFeedPanel } from "./world_feed_panel.jsx";
import { focusViewerPanel } from "./viewer_navigation.jsx";

export function WorldFeedSurface({ core, locale, tr, onReloadSnapshot, onRetryFeed }) {
  const retryFeed = () => {
    if (typeof onRetryFeed === "function") {
      return onRetryFeed();
    }
    return core?.requestWorldFeed?.({ cursor: null });
  };

  return (
    <WorldFeedPanel
      feed={() => core.state.worldFeed}
      locale={locale}
      tr={tr}
      onReloadSnapshot={onReloadSnapshot}
      onRetryFeed={retryFeed}
      resolveModuleVisualEntity={(event) => core?.entityCollections?.().moduleVisualEntities.find((entry) => entry.id === event?.module_visual_entity_id) || null}
      onFocusModule={(event) => {
        const applied = core?.focusModuleFromEvent?.(event);
        if (applied?.kind === "module_visual") {
          focusViewerPanel("viewer-details-panel");
        }
        return applied;
      }}
    />
  );
}
