import { WorldFeedPanel } from "./world_feed_panel.jsx";

export function WorldFeedSurface({ core, locale, tr, onReloadSnapshot }) {
  return (
    <WorldFeedPanel
      feed={() => core.state.worldFeed}
      locale={locale}
      tr={tr}
      onReloadSnapshot={onReloadSnapshot}
    />
  );
}
