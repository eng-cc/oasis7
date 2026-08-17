import { For, Show } from "solid-js";
import { compareUnsignedDecimal } from "./world_feed_state.js";

function readFeed(props) {
  return typeof props.feed === "function" ? props.feed() : props.feed || {};
}

function statusCopy(locale, tr, status) {
  const copy = {
    loading: ["正在加载世界动态…", "Loading world activity…"],
    ready: ["环境上下文已更新", "Ambient context updated"],
    empty: ["暂无世界动态", "No world activity yet"],
    replay: ["回放上下文", "Replay context"],
    gap: ["世界动态已过期", "World activity is stale"],
    unavailable: ["世界动态不可用", "World activity unavailable"],
  }[status] || ["世界动态不可用", "World activity unavailable"];
  return tr(locale, copy[0], copy[1]);
}

function statusBadgeClass(status) {
  if (status === "ready") return "badge badge--accent";
  if (status === "replay") return "badge badge--accent";
  if (status === "gap" || status === "unavailable") return "badge badge--warn";
  return "badge";
}

function reasonCopy(locale, tr, feed) {
  if (feed.status === "gap") {
    const reason = String(feed.gapReason || "cursor_invalid").replace(/_/g, " ");
    return tr(
      locale,
      `游标或历史分叉不连续（${reason}）。已停止追加，必须重新加载权威快照。`,
      `The cursor or history is discontinuous (${reason}). Appending stopped; reload the authoritative snapshot.`,
    );
  }
  if (feed.status === "unavailable") {
    const reason = String(feed.unavailableReason || "source_unavailable").replace(/_/g, " ");
    return tr(
      locale,
      `运行时没有提供可验证的 World Feed（${reason}）。`,
      `The runtime did not provide a verifiable World Feed (${reason}).`,
    );
  }
  if (feed.status === "replay") {
    return tr(locale, "这是游标回放上下文，不代表玩家动作成功。", "This is cursor replay context; it does not prove a player action succeeded.");
  }
  if (feed.status === "ready") {
    return tr(locale, "环境动态仅作上下文呈现；因果仍以 Action Receipt 为准。", "Ambient activity is context only; Action Receipt remains the causal source.");
  }
  return null;
}

function eventDetail(event, locale, tr) {
  return tr(
    locale,
    `${event.kind} · ${event.detail || "无额外详情"}`,
    `${event.kind} · ${event.detail || "No additional detail"}`,
  );
}

function WorldFeedPanel(props) {
  const locale = () => (typeof props.locale === "function" ? props.locale() : props.locale || "en");
  const tr = (localeValue, zh, en) => (typeof props.tr === "function" ? props.tr(localeValue, zh, en) : en);
  const feed = () => readFeed(props);
  // Render the timeline in event-sequence order even when a replay or fixture
  // provides events out of order. The copy keeps the runtime array immutable.
  const presentationEvents = () => [...(feed().events || [])].sort(
    (left, right) => compareUnsignedDecimal(left?.event_seq, right?.event_seq),
  );
  const status = () => String(feed().status || "unavailable");
  const statusLabel = () => statusCopy(locale(), tr, status());
  const shouldReload = () => Boolean(feed().snapshotReloadRequired || feed().stale || status() === "gap");

  return (
    <details
      id="viewer-world-feed"
      class="panel panel--world-feed"
      data-viewer-overlay="feed"
      data-viewer-surface="world-feed"
      data-world-feed-status={status()}
      aria-live="polite"
    >
      <summary class="panel__header panel__header--stack world-feed__summary">
        <div class="panel__eyebrow">{tr(locale(), "环境上下文", "Ambient Context")}</div>
        <div class="panel__title">{tr(locale(), "World Feed", "World Feed")}</div>
        <div class="panel__meta-copy">
          {tr(locale(), "只读的运行时环境投影；不会替代 Action Receipt，也不会证明玩家动作成功。", "Read-only runtime context; it never replaces Action Receipt or proves a player action succeeded.")}
        </div>
      </summary>
      <div class="panel__body world-feed__body">
        <div class="world-feed__status-row">
          <span class={statusBadgeClass(status())}>{statusLabel()}</span>
          <Show when={feed().worldId}>
            <span class="badge">{`world=${feed().worldId}`}</span>
          </Show>
          <Show when={feed().reorgEpoch != null}>
            <span class="badge">{`epoch=${feed().reorgEpoch}`}</span>
          </Show>
        </div>
        <Show when={reasonCopy(locale(), tr, feed())}>
          <div class="feedback-detail world-feed__notice">{reasonCopy(locale(), tr, feed())}</div>
        </Show>
        <Show when={shouldReload()}>
          <div class="toolbar world-feed__recovery">
            <button
              type="button"
              data-world-feed-action="reload-authoritative-snapshot"
              onClick={() => props.onReloadSnapshot?.()}
            >
              {tr(locale(), "重新加载权威快照", "Reload authoritative snapshot")}
            </button>
          </div>
        </Show>
        <Show
          when={presentationEvents().length > 0}
          fallback={
            <div class="world-feed__empty" data-world-feed-empty="true">
              {status() === "loading"
                ? tr(locale(), "正在等待 world_feed/v1 响应…", "Waiting for a world_feed/v1 response…")
                : status() === "empty"
                  ? tr(locale(), "权威运行时暂未发布事件。", "The authoritative runtime has not published events yet.")
                  : tr(locale(), "没有可显示的环境事件。", "No ambient events are available to display.")}
            </div>
          }
        >
          <div class="event-list world-feed__events" data-world-feed-events="true">
            <For each={presentationEvents()}>
              {(event) => (
                <article class="event-card world-feed__event" data-world-feed-event={event.event_seq}>
                  <div class="event-card__header">
                    <div class="event-card__title">{event.summary}</div>
                    <span class="badge">{`#${event.event_seq}`}</span>
                  </div>
                  <div class="event-card__meta">{eventDetail(event, locale(), tr)}</div>
                  <Show when={event.receipt_ref != null}>
                    <a
                      data-world-feed-receipt-ref={event.receipt_ref}
                      class="world-feed__receipt-link"
                      href={`#viewer-action-receipt`}
                    >
                      {tr(locale(), `查看明确回执 ${event.receipt_ref}`, `View explicit receipt ${event.receipt_ref}`)}
                    </a>
                  </Show>
                </article>
              )}
            </For>
          </div>
        </Show>
      </div>
    </details>
  );
}

export { WorldFeedPanel };
