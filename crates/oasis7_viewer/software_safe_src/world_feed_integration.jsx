import { WorldFeedPanel } from "./world_feed_panel.jsx";

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
    />
  );
}
