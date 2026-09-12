import { For, Show } from "solid-js";
import { compareUnsignedDecimal } from "./world_feed_state.js";
import { pixelWorldMajorEventPresentation } from "./pixel_world_presentation.js";
import { pixelWorldReadableModuleLabel } from "./pixel_world_identity.js";

function readFeed(props) {
  return typeof props.feed === "function" ? props.feed() : props.feed || {};
}

function readableModuleLabel(module, locale) {
  return pixelWorldReadableModuleLabel(
    module,
    module?.id,
    String(locale).trim().toLowerCase().startsWith("zh"),
  );
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

function statusBadgeLabel(locale, tr, status) {
  const copy = {
    loading: ["同步中", "SYNCING"],
    ready: ["实时", "LIVE"],
    empty: ["暂无动态", "NO EVENTS"],
    replay: ["回放", "REPLAY"],
    gap: ["断档", "GAP"],
    unavailable: ["不可用", "UNAVAILABLE"],
  }[status] || ["不可用", "UNAVAILABLE"];
  return tr(locale, copy[0], copy[1]);
}

function statusBadgeClass(status) {
  const stateClass = `world-feed__status-badge world-feed__status-badge--${status || "unknown"}`;
  if (status === "ready") return `badge badge--accent ${stateClass}`;
  if (status === "replay") return `badge badge--accent ${stateClass}`;
  if (status === "gap" || status === "unavailable") return `badge badge--warn ${stateClass}`;
  return `badge ${stateClass}`;
}

function reasonCopy(locale, tr, feed) {
  if (feed.status === "gap") {
    if (feed.gapReason === "event_identity_conflict") {
      return tr(
        locale,
        "运行时发现同一世界事件身份对应互相冲突的载荷。已清除动态，必须重新加载权威快照。",
        "The runtime found conflicting payloads for the same world-event identity. The feed was cleared; reload the authoritative snapshot.",
      );
    }
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

function eventKindLabel(event, locale, tr) {
  const normalized = String(event?.kind || "world_update")
    .trim()
    .replace(/[^a-zA-Z0-9]+/g, " ")
    .replace(/\s+/g, " ");
  const known = {
    snapshot_created: ["世界快照", "World snapshot"],
    resource_change: ["资源变化", "Resource change"],
    agent_spoke: ["Agent 动态", "Agent activity"],
    crisis_spawned: ["危机发生", "Crisis started"],
    crisis_resolved: ["危机解决", "Crisis resolved"],
    crisis_timed_out: ["危机超时", "Crisis timed out"],
    major_world_event: ["重大世界事件", "Major world event"],
  }[String(event?.kind || "").toLowerCase()];
  if (known) return tr(locale, known[0], known[1]);
  if (!normalized) return tr(locale, "世界更新", "World update");
  return normalized.replace(/\b\w/g, (character) => character.toUpperCase());
}

function majorEventStatusCopy(event, locale, tr) {
  const presentation = pixelWorldMajorEventPresentation(event?.major_event, locale);
  return typeof tr === "function" ? presentation.label : presentation.label;
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
  const summaryStatusLabel = () => statusBadgeLabel(locale(), tr, status());
  const latestEvent = () => presentationEvents().at(-1) || null;
  const shouldReload = () => status() !== "unavailable"
    && Boolean(feed().snapshotReloadRequired || status() === "gap");

  return (
    <details
      id="viewer-world-feed"
      class="panel panel--world-feed"
      data-viewer-overlay="feed"
      data-viewer-surface="world-feed"
      data-world-feed-status={status()}
      aria-live="polite"
    >
      <summary
        class="panel__header panel__header--stack world-feed__summary"
        data-world-feed-latest={latestEvent()?.event_seq == null ? undefined : String(latestEvent().event_seq)}
      >
        <div class="panel__eyebrow">{tr(locale(), "环境上下文", "Ambient Context")}</div>
        <div class="world-feed__summary-line">
          <div class="panel__title">{tr(locale(), "World Feed", "World Feed")}</div>
          <span class={statusBadgeClass(status())} data-world-feed-summary-status={status()}>{summaryStatusLabel()}</span>
        </div>
        <div class="panel__meta-copy">
          {tr(locale(), "只读的运行时环境投影；不会替代 Action Receipt，也不会证明玩家动作成功。", "Read-only runtime context; it never replaces Action Receipt or proves a player action succeeded.")}
        </div>
        <Show
          when={latestEvent()}
          fallback={(
            <div class="world-feed__latest world-feed__latest--empty" data-world-feed-latest-empty="true">
              {status() === "loading"
                ? tr(locale(), "等待最新环境动态。", "Waiting for the latest ambient activity.")
                : status() === "unavailable"
                  ? tr(locale(), "最新环境动态不可用。", "The latest ambient activity is unavailable.")
                  : tr(locale(), "暂无最新环境动态。", "No latest ambient activity is available.")}
            </div>
          )}
        >
          <div class="world-feed__latest" data-world-feed-latest="true">
            <span class="world-feed__latest-copy">{`${tr(locale(), "最新", "Latest")}: ${latestEvent().summary} · ${eventKindLabel(latestEvent(), locale(), tr)}`}</span>
          </div>
        </Show>
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
        <Show when={status() === "unavailable"}>
          <div class="toolbar world-feed__recovery">
            <button
              type="button"
              data-world-feed-action="retry-world-feed"
              onClick={() => props.onRetryFeed?.()}
            >
              {tr(locale(), "重试 World Feed", "Retry World Feed")}
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
                  ? tr(
                    locale(),
                    "权威运行时尚未发布世界更新中的事件。此动态仅作上下文；请继续当前玩家目标，下一次权威世界更新后动态会刷新。",
                    "No authoritative world update has published events yet. This feed is context only—continue your Player goal; the feed will update after the next authoritative world update.",
                  )
                  : status() === "unavailable"
                    ? tr(locale(), "重试 World Feed 以检查最新环境动态。", "Retry World Feed to check for the latest ambient activity.")
                    : tr(locale(), "没有可显示的环境事件。", "No ambient events are available to display.")}
            </div>
          }
        >
          <div class="event-list world-feed__events" data-world-feed-events="true">
            <For each={presentationEvents()}>
              {(event) => (
                <article
                  class={`event-card world-feed__event${event.major_event ? ` world-feed__event--major ${pixelWorldMajorEventPresentation(event.major_event, locale()).shape.split(" ").map((token) => `world-feed__event--${token}`).join(" ")}` : ""}`}
                  data-world-feed-event={event.event_seq}
                  data-world-feed-major-event={event.major_event ? event.event_seq : undefined}
                  data-major-event-category={event.major_event?.category}
                  data-major-event-lifecycle={event.major_event?.lifecycle}
                  data-major-event-severity={event.major_event?.severity}
                >
                  <div class="event-card__header">
                    <div class="event-card__title">{event.summary}</div>
                    <span class="badge">{`#${event.event_seq}`}</span>
                  </div>
                  <div class="event-card__meta">{eventKindLabel(event, locale(), tr)}</div>
                  <Show when={event.major_event}>
                    <div
                      class="feedback-detail world-feed__major-event-status"
                      data-major-event-status={pixelWorldMajorEventPresentation(event.major_event, locale()).shape}
                      role={event.major_event.freshness === "current" && status() === "ready" ? "status" : undefined}
                      aria-live={event.major_event.freshness === "current" && status() === "ready" ? "polite" : undefined}
                    >
                      {majorEventStatusCopy(event, locale(), tr)}
                    </div>
                  </Show>
                  <Show when={event.receipt_ref != null}>
                    <a
                      data-world-feed-receipt-ref={event.receipt_ref}
                      class="world-feed__receipt-link"
                      href={`#viewer-action-receipt`}
                    >
                      {tr(locale(), `查看明确回执 ${event.receipt_ref}`, `View explicit receipt ${event.receipt_ref}`)}
                    </a>
                  </Show>
                  <Show when={props.resolveModuleVisualEntity?.(event)}>
                    {(module) => (
                      <button
                        type="button"
                        class="world-feed__module-locate"
                        data-world-feed-module-locate={module().id}
                        aria-label={`${tr(locale(), "定位模块", "Locate module")} ${readableModuleLabel(module(), locale())}`}
                        onClick={() => props.onFocusModule?.(event)}
                      >
                        {tr(locale(), "定位模块", "Locate module")}: {readableModuleLabel(module(), locale())}
                      </button>
                    )}
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
