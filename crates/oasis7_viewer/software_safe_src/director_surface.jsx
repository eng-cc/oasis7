import { createSignal, For, onCleanup, onMount, Show } from "solid-js";

function text(locale, zh, en) {
  return String(locale || "en").toLowerCase().startsWith("zh") ? zh : en;
}

export function directorRecoveryText(locale, state) {
  const reason = state?.reason;
  if (state?.status === "pending") {
    return text(locale, "正在向服务器核验 Director 权限…", "Validating the Director capability with the server…");
  }
  if (reason === "not_authorized") {
    return text(locale, "当前账号没有 Director 权限。请通过受支持的操作员入口恢复。", "This account is not authorized for Director. Recover through the supported operator entry point.");
  }
  if (reason === "reconnect_required") {
    return text(locale, "连接或会话需要恢复；已回到 Player。世界与当前选择保持不变。", "The connection or session needs recovery; Player mode is restored. The world and current selection are unchanged.");
  }
  if (reason === "revoked") {
    return text(locale, "Director 权限已失效；已清除本地 Director 视图。请恢复受支持的操作员会话。", "The Director capability is no longer valid; the local Director view was cleared. Recover a supported operator session.");
  }
  if (reason === "expired") {
    return text(locale, "Director 权限已过期；已回到 Player。请重新请求服务器核验。", "The Director capability expired; Player mode is restored. Request server validation again.");
  }
  if (reason === "player_exit") {
    return text(locale, "已退出 Director；世界状态与当前选择保持不变。", "Director exited; world state and current selection are unchanged.");
  }
  if (state?.status === "denied") {
    return text(locale, "服务器没有授予 Director 权限。当前仍保持 Player。", "The server did not grant Director. Player mode remains active.");
  }
  if (state?.status === "unavailable") {
    return text(locale, "Director 权限服务暂不可用。当前仍保持 Player，请稍后重试。", "The Director capability service is unavailable. Player mode remains active; try again later.");
  }
  return text(locale, "Director 仅在服务器明确核验成功后临时开放。", "Director opens only after explicit server validation.");
}

function readSnapshot(core) {
  const snapshot = core?.state?.snapshot;
  const model = snapshot?.model || {};
  const selectedKind = String(core?.state?.selectedKind || "").trim();
  const selectedId = String(core?.state?.selectedId || "").trim();
  return {
    worldId: String(core?.state?.worldId || "").trim() || "-",
    logicalTime: Number(snapshot?.time ?? core?.state?.logicalTime ?? 0),
    eventSeq: Number(core?.state?.eventSeq || 0),
    agents: Object.keys(model.agents || {}).length,
    locations: Object.keys(model.locations || {}).length,
    events: Array.isArray(core?.state?.recentEvents) ? core.state.recentEvents.length : 0,
    selected: selectedKind && selectedId ? `${selectedKind}:${selectedId}` : "-",
  };
}

export function DirectorSurface(props) {
  const locale = () => props.locale?.() || props.locale || "en";
  const [revision, setRevision] = createSignal(0);
  onMount(() => {
    const unsubscribe = props.controller?.subscribe?.(() => setRevision((value) => value + 1));
    if (typeof unsubscribe === "function") onCleanup(unsubscribe);
  });
  const state = () => {
    revision();
    return props.controller?.getState?.() || { mode: "player", status: "idle", capability: null };
  };
  const snapshot = () => readSnapshot(props.core);
  const exit = () => {
    props.controller?.exit?.();
    window.location.hash = "#viewer-stage-panel";
    document.getElementById("viewer-stage-panel")?.focus();
  };
  return (
    <Show when={state().mode === "director"}>
      <section
        id="viewer-director-panel"
        class="panel director-surface"
        data-viewer-surface="director"
        data-director-mode="active"
        tabIndex="-1"
        aria-labelledby="viewer-director-title"
      >
        <div class="panel__header panel__header--stack">
          <div class="panel__eyebrow">{text(locale(), "服务器核验视图", "Server-validated visibility")}</div>
          <div class="panel__title" id="viewer-director-title">{text(locale(), "Director", "Director")}</div>
          <div class="panel__meta-copy">
            {text(locale(), "仅提高世界可见密度；不增加命令、进度推进或本地持久化。", "Visibility density only; no commands, progress changes, or local persistence.")}
          </div>
          <button id="viewer-director-exit" type="button" class="panel__route-close" onClick={exit}>
            {text(locale(), "退出 Director", "Exit Director")}
          </button>
        </div>
        <div class="panel__body stack">
          <div class="badge-row" aria-label={text(locale(), "Director 权限状态", "Director capability status")}>
            <span class="badge badge--good">server_validated</span>
            <span class="badge">{state().capability?.issuer || "-"}</span>
            <span class="badge">{state().capability?.expiresAtUnixMs ? `expires=${state().capability.expiresAtUnixMs}` : "expires=-"}</span>
          </div>
          <div class="director-density-grid">
            <div class="hero-focus-card"><div class="hero-focus-card__label">{text(locale(), "世界", "World")}</div><div class="hero-focus-card__value hero-focus-card__value--body">{snapshot().worldId}</div></div>
            <div class="hero-focus-card"><div class="hero-focus-card__label">{text(locale(), "逻辑时间", "Logical Time")}</div><div class="hero-focus-card__value hero-focus-card__value--body">{snapshot().logicalTime}</div></div>
            <div class="hero-focus-card"><div class="hero-focus-card__label">{text(locale(), "事件序号", "Event Sequence")}</div><div class="hero-focus-card__value hero-focus-card__value--body">{snapshot().eventSeq}</div></div>
            <div class="hero-focus-card"><div class="hero-focus-card__label">{text(locale(), "当前选择", "Current Selection")}</div><div class="hero-focus-card__value hero-focus-card__value--body">{snapshot().selected}</div></div>
          </div>
          <div class="badge-row">
            <span class="badge">{`agents=${snapshot().agents}`}</span>
            <span class="badge">{`locations=${snapshot().locations}`}</span>
            <span class="badge">{`recentEvents=${snapshot().events}`}</span>
          </div>
          <div class="feedback-detail">
            {text(locale(), "此视图只读，退出或权限失效不会清空世界快照或当前选择。", "This view is read-only; exit or capability loss does not clear the world snapshot or current selection.")}
          </div>
          <div class="director-density-list" aria-label={text(locale(), "Director 可见性摘要", "Director visibility summary")}>
            <For each={[
              text(locale(), "世界快照", "World snapshot"),
              text(locale(), "空间对象密度", "Spatial entity density"),
              text(locale(), "最近事件窗口", "Recent event window"),
            ]}>
              {(label) => <div class="director-density-list__row"><span>{label}</span><span class="badge badge--diagnostic">{text(locale(), "只读", "read-only")}</span></div>}
            </For>
          </div>
        </div>
      </section>
    </Show>
  );
}

export function DirectorEntryCard(props) {
  const locale = () => props.locale?.() || props.locale || "en";
  const [revision, setRevision] = createSignal(0);
  onMount(() => {
    const unsubscribe = props.controller?.subscribe?.(() => setRevision((value) => value + 1));
    if (typeof unsubscribe === "function") onCleanup(unsubscribe);
  });
  const state = () => {
    revision();
    return props.controller?.getState?.() || { mode: "player", status: "idle" };
  };
  return (
    <div class="director-entry-card" data-director-status={state().status || "idle"}>
      <div class="panel__title">{text(locale(), "Director 可见性", "Director Visibility")}</div>
      <div class="feedback-detail">{directorRecoveryText(locale(), state())}</div>
      <div class="toolbar">
        <button
          id="viewer-director-entry"
          type="button"
          class="button button--secondary"
          disabled={state().status === "pending"}
          onClick={() => props.onRequest?.()}
        >
          {state().status === "pending" ? text(locale(), "正在核验…", "Validating…") : text(locale(), "打开 Director", "Open Director")}
        </button>
      </div>
    </div>
  );
}
