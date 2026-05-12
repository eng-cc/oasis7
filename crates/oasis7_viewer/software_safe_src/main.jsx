import { createSignal, For, Show } from "solid-js";
import { render as mount } from "solid-js/web";

import * as core from "./legacy_core.js";
import { PixelWorldHost } from "./pixel_world_host.jsx";

function uiLocale() {
  return core.state.uiLocale;
}

function tr(locale, zh, en) {
  return core.isLocaleZh(locale) ? zh : en;
}

function localeCode(locale) {
  return core.isLocaleZh(locale) ? "zh" : "en";
}

function buildViewerEntryUrls(locale) {
  const softwareSafeUrl = new URL(window.location.href);
  softwareSafeUrl.searchParams.set("locale", localeCode(locale));
  softwareSafeUrl.searchParams.delete("language");

  return {
    softwareSafeUrl: softwareSafeUrl.toString(),
  };
}

function Badge(props) {
  return <span class={props.class ?? "badge"}>{props.children}</span>;
}

function EmptyState(props) {
  return <div class="empty" style={props.style}>{props.children}</div>;
}

function JsonBlock(props) {
  return <pre class="json">{JSON.stringify(props.value, null, 2)}</pre>;
}

function DiagnosticDetails(props) {
  const locale = () => props.locale ?? uiLocale();
  const [isOpen, setIsOpen] = createSignal(false);
  const resolvedValue = () => (typeof props.value === "function" ? props.value() : props.value);
  return (
    <details class="diagnostic" onToggle={(event) => setIsOpen(event.currentTarget.open)}>
      <summary>{props.label ?? tr(locale(), "原始诊断", "Raw diagnostics")}</summary>
      <div class="stack" style="margin-top:10px;">
        <Show when={props.note}>
          <div class="feedback-detail">{props.note}</div>
        </Show>
        <Show when={isOpen()}>
          <JsonBlock value={resolvedValue()} />
        </Show>
      </div>
    </details>
  );
}

function FeedbackCard(props) {
  return (
    <div class="feedback-card">
      <div class="badge-row">
        <Badge class={props.display.badgeClass}>{props.display.label}</Badge>
        <Show when={props.display.code}>
          <Badge>{`code=${props.display.code}`}</Badge>
        </Show>
      </div>
      <div class="feedback-summary">{props.display.summary}</div>
      <Show when={props.display.detail}>
        <div class="feedback-detail">{props.display.detail}</div>
      </Show>
      <Show when={props.feedback}>
        <DiagnosticDetails value={props.feedback} />
      </Show>
    </div>
  );
}

function MetricCard(props) {
  return (
    <div class="metric">
      <div class="metric__label">{props.label}</div>
      <div class="metric__value">{props.value}</div>
      <Show when={props.children}>
        <div class="badge-row" style="margin-top:8px;">
          {props.children}
        </div>
      </Show>
    </div>
  );
}

function EventCard(props) {
  return (
    <div class={props.class ?? "event-card"}>
      <div class="event-card__title">
        <span>{props.title}</span>
        <Show when={props.badge}>
          <span class={props.badgeClass ?? "badge"}>{props.badge}</span>
        </Show>
      </div>
      <Show when={props.meta}>
        <div class="event-card__meta">{props.meta}</div>
      </Show>
      {props.children}
    </div>
  );
}

function PanelSection(props) {
  return (
    <div class={`panel panel--nested ${props.class ?? ""}`}>
      <div class="panel__header">
        <div class="stack" style="gap:4px;">
          <Show when={props.eyebrow}>
            <div class="panel__eyebrow">{props.eyebrow}</div>
          </Show>
          <div class="panel__title">{props.title}</div>
          <Show when={props.meta}>
            <div class="panel__meta-copy">{props.meta}</div>
          </Show>
        </div>
      </div>
      <div class="panel__body stack">{props.children}</div>
    </div>
  );
}

function CalloutCard(props) {
  return (
    <div class={`callout ${props.variant === "warn" ? "callout--warn" : ""}`}>
      <div class="callout__header">
        <div class="callout__title">{props.title}</div>
        <Show when={props.badge}>
          <Badge class={props.badgeClass ?? "badge badge--warn"}>{props.badge}</Badge>
        </Show>
      </div>
      <div class="callout__body">{props.children}</div>
    </div>
  );
}

function EmptyEntityRecoveryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const gameplay = () => (typeof props.gameplay === "function" ? props.gameplay() : props.gameplay);

  return (
    <CalloutCard
      title={props.title ?? tr(locale(), "当前快照没有可继续游玩的实体", "Current Snapshot Has No Playable Entities")}
      badge={gameplay()?.blockerKind || "blocked"}
      badgeClass="badge badge--warn"
      variant="warn"
    >
      <div class="feedback-summary">
        {gameplay()?.blockerDetail
          || tr(
            locale(),
            "runtime 已发布玩法摘要，但当前快照还没有可选 Agent 或地点。",
            "Runtime published gameplay summary, but the current snapshot still has no selectable agents or locations.",
          )}
      </div>
      <Show when={gameplay()?.nextStepHint}>
        <div class="feedback-detail">{gameplay().nextStepHint}</div>
      </Show>
      <Show when={gameplay()?.entityCounts}>
        <div class="badge-row">
          <Badge>{`agents=${gameplay().entityCounts.agents}`}</Badge>
          <Badge>{`locations=${gameplay().entityCounts.locations}`}</Badge>
        </div>
      </Show>
      <div class="feedback-detail">
        {tr(
          locale(),
          "如果中间栏仍保留“刷新快照”动作，先从那里重拉一次；如果数量仍然是 0，就需要修复或重启 runtime world bootstrap。",
          "If the middle column still exposes a refresh action, pull a fresh snapshot there first. If the counts stay at 0, repair or restart the runtime world bootstrap.",
        )}
      </div>
    </CalloutCard>
  );
}

function ViewerEntryMenu() {
  const locale = () => uiLocale();
  const viewerEntryUrls = () => buildViewerEntryUrls(locale());

  return (
    <details class="entry-menu">
      <summary class="entry-menu__toggle">{tr(locale(), "入口", "Entry")}</summary>
      <div class="entry-menu__panel stack">
        <div>
          <div class="panel__title" style="margin-bottom:10px;">
            {tr(locale(), "语言与 Viewer 入口", "Language and Viewer Entry")}
          </div>
          <div class="feedback-detail">
            {tr(
              locale(),
              "主玩法继续留在当前页面；这里只保留语言切换。",
              "Primary gameplay stays on this page. This menu only keeps locale switching.",
            )}
          </div>
        </div>
        <div class="toolbar">
          <button
            data-locale="zh"
            disabled={locale() === "zh"}
            onClick={() => core.setViewerLocale("zh")}
          >
            中文
          </button>
          <button
            data-locale="en"
            disabled={locale() === "en"}
            onClick={() => core.setViewerLocale("en")}
          >
            English
          </button>
        </div>
        <div class="badge-row">
          <Badge>{`locale=${localeCode(locale())}`}</Badge>
        </div>
        <div class="feedback-detail">{viewerEntryUrls().softwareSafeUrl}</div>
      </div>
    </details>
  );
}

function gameplayStatusBadgeClass(status) {
  return status === "blocked"
    ? "badge badge--warn"
    : status === "branch_ready"
      ? "badge badge--good"
      : "badge badge--accent";
}

function renderResourceSummary(resources) {
  return core.resourceSummary(resources);
}

function WorldStageHero() {
  const locale = () => uiLocale();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const selectedLabel = () =>
    core.state.selectedKind && core.state.selectedId
      ? `${core.state.selectedKind}:${core.state.selectedId}`
      : tr(locale(), "未选中目标", "no_target_selected");

  return (
    <div class="stage-hero">
      <div class="stage-hero__topline">
        <div class="stack" style="gap:10px;">
          <div class="stage-hero__eyebrow">{tr(locale(), "工业世界指挥桌", "Industrial World Command Desk")}</div>
          <div class="stage-hero__title">
            {gameplaySummary()?.goalTitle || tr(locale(), "进入世界，先看局势，再做动作", "Read the world first, then act.")}
          </div>
          <div class="stage-hero__lede">
            {gameplaySummary()?.nextStepHint
              || gameplaySummary()?.objective
              || tr(
                locale(),
                "这张入口页优先保留世界、目标和关键动作；高级诊断与治理能力按需展开。",
                "This entry keeps the world, objective, and primary actions in front. Advanced diagnostics and governance stay on demand.",
              )}
          </div>
        </div>
        <ViewerEntryMenu />
      </div>
      <div class="badge-row">
        <Badge class="badge badge--accent">viewer</Badge>
        <Badge class="badge badge--accent">{tr(locale(), "正式可玩 Web 入口", "formal playable web entry")}</Badge>
        <Badge class={core.connectionBadgeClass()}>
          {tr(locale(), "连接", "connection")}={core.state.connectionStatus}
        </Badge>
        <Badge>{`world=${core.state.worldId || "-"}`}</Badge>
        <Badge>{`${tr(locale(), "目标", "target")}=${selectedLabel()}`}</Badge>
        <Show when={gameplaySummary()?.stageStatus}>
          <Badge class={gameplayStatusBadgeClass(gameplaySummary().stageStatus)}>
            {`stage=${gameplaySummary().stageStatus}`}
          </Badge>
        </Show>
      </div>
    </div>
  );
}

function MobileJumpRail() {
  const locale = () => uiLocale();
  return (
    <nav class="mobile-rail" aria-label={tr(locale(), "主入口分区导航", "Primary entry section navigation")}>
      <a class="mobile-rail__link" href="#viewer-stage-panel">{tr(locale(), "世界", "World")}</a>
      <a class="mobile-rail__link" href="#viewer-targets-panel">{tr(locale(), "目标", "Targets")}</a>
      <a class="mobile-rail__link" href="#viewer-details-panel">{tr(locale(), "指挥", "Command")}</a>
    </nav>
  );
}

function TargetsPanel() {
  const lists = () => core.modelLists();
  const locale = () => uiLocale();

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge>{`agents=${lists().agents.length}`}</Badge>
        <Badge>{`locations=${lists().locations.length}`}</Badge>
      </div>
      <EmptyState>
        {tr(
          locale(),
          "先从这里锁定一个 Agent 或地点，再去世界舞台和右侧命令面板继续操作。",
          "Pick an agent or location here first, then continue in the world stage and command panel.",
        )}
      </EmptyState>
      <div class="field">
        <label for="entity-search">{tr(locale(), "筛选目标", "Filter targets")}</label>
        <input
          id="entity-search"
          type="search"
          placeholder={tr(locale(), "搜索 Agent 或地点", "Search agents or locations")}
          value={core.getSelectedSearch()}
          onInput={(event) => core.setSelectedSearch(event.currentTarget.value)}
        />
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "Agents", "Agents")}</div>
        <div class="list">
          <Show
            when={lists().agents.length > 0}
            fallback={<EmptyState>{tr(locale(), "当前快照里没有 Agent。", "No agents in current snapshot.")}</EmptyState>}
          >
            <For each={lists().agents}>
              {(agent) => (
                <button
                  class="list-item"
                  data-select-kind="agent"
                  data-select-id={agent.id}
                  data-selected={core.state.selectedKind === "agent" && core.state.selectedId === agent.id}
                  onClick={() => core.applySelection({ kind: "agent", id: agent.id })}
                >
                  <div class="list-item__title">{agent.id}</div>
                  <div class="list-item__meta">
                    {`${tr(locale(), "地点", "location")}=${agent.location_id} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(agent.resources)}`}
                  </div>
                </button>
              )}
            </For>
          </Show>
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "地点", "Locations")}</div>
        <div class="list">
          <Show
            when={lists().locations.length > 0}
            fallback={<EmptyState>{tr(locale(), "当前快照里没有地点。", "No locations in current snapshot.")}</EmptyState>}
          >
            <For each={lists().locations}>
              {(location) => (
                <button
                  class="list-item"
                  data-select-kind="location"
                  data-select-id={location.id}
                  data-selected={
                    core.state.selectedKind === "location" && core.state.selectedId === location.id
                  }
                  onClick={() => core.applySelection({ kind: "location", id: location.id })}
                >
                  <div class="list-item__title">{location.name || location.id}</div>
                  <div class="list-item__meta">
                    {`id=${location.id} · ${tr(locale(), "半径", "radius")}=${
                      core.formatPhysicalDistanceCm(location.profile?.radius_cm, locale()) || "-"
                    } · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(location.resources)}`}
                  </div>
                </button>
              )}
            </For>
          </Show>
        </div>
      </div>
    </div>
  );
}

function WorldSummaryPanel() {
  const locale = () => uiLocale();
  const state = core.state;
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const gameplayActionFeedback = () => core.snapshotSemanticFeedback(state.lastGameplayActionFeedback);
  const promptFeedback = () => core.snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => core.snapshotSemanticFeedback(state.lastChatFeedback);
  const gameplayActionFeedbackDisplay = () => core.describeSemanticFeedback(gameplayActionFeedback(), locale());
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const authSurface = () => core.buildAuthSurfaceModel();
  const hostedActionMatrixView = () => core.buildHostedActionMatrixView();
  const hostedRecoveryHint = () => core.buildHostedRecoveryHint(locale());
  const selectedDebug = () => core.selectedAgentExecutionDebugContext();
  const tierBadgeClass = (status) =>
    status === "active" || status === "active_legacy_preview"
      ? "badge badge--good"
      : status === "superseded"
        ? "badge"
        : "badge badge--warn";
  const showRebindNotice = () =>
    Boolean(state.auth.pendingRequestedAgentId)
      && (state.auth.pendingForceRebind
        || state.auth.runtimeStatus === "rebind_retrying"
        || state.auth.runtimeStatus === "rebind_registering");
  const showPlayerSessionSurface = () =>
    String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"
      || state.auth.available
      || !!hostedRecoveryHint();
  const diagnosticsSummaryBadges = () => [
    `debugViewer=${state.debugViewerMode}:${state.debugViewerStatus}`,
    `auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}`,
    `events=${state.recentEvents.length}`,
  ];

  return (
    <div class="stack">
      <PanelSection
        title={tr(locale(), "正式玩法摘要", "Formal Gameplay Summary")}
        eyebrow={tr(locale(), "玩家主路径", "Player Path")}
        meta={tr(locale(), "先看目标、阻塞和下一步，再决定是否进入右侧命令区。", "Read the goal, blocker, and next step first, then decide whether to enter the command surface.")}
      >
        <Show
          when={gameplaySummary()}
          fallback={<EmptyState>{tr(locale(), "等待首条 canonical gameplay 快照…", "Waiting for the first canonical gameplay snapshot…")}</EmptyState>}
        >
          {(gameplay) => (
            <>
              <div class="badge-row">
                <Badge class={gameplayStatusBadgeClass(gameplay().stageStatus)}>
                  {`stage=${gameplay().stageStatus || "-"}`}
                </Badge>
                <Badge>{`stageId=${gameplay().stageId || "-"}`}</Badge>
                <Badge>{`goal=${gameplay().goalId || "-"}`}</Badge>
                <Show when={gameplay().goalKind}>
                  <Badge>{`goalKind=${gameplay().goalKind}`}</Badge>
                </Show>
                <Badge>
                  {`progress=${
                    gameplay().progressPercent == null ? "-" : `${gameplay().progressPercent}%`
                  }`}
                </Badge>
              </div>
              <Show when={gameplay().blockerKind || gameplay().blockerDetail}>
                <CalloutCard
                  title={
                    gameplay().blockerKind === "runtime_snapshot_empty_entities"
                      ? tr(locale(), "当前阻塞：空快照", "Current Blocker: Empty Snapshot")
                      : tr(locale(), "当前阻塞", "Current Blocker")
                  }
                  badge={gameplay().blockerKind || "blocked"}
                  badgeClass="badge badge--warn"
                  variant="warn"
                >
                  <div class="feedback-summary">
                    {gameplay().blockerDetail || tr(locale(), "当前玩法被阻塞，需要显式恢复。", "Gameplay is blocked and needs explicit recovery.")}
                  </div>
                  <Show when={gameplay().blockerSupplementalDetail}>
                    <div class="feedback-detail">{gameplay().blockerSupplementalDetail}</div>
                  </Show>
                  <Show when={gameplay().nextStepHint}>
                    <div class="feedback-detail">{gameplay().nextStepHint}</div>
                  </Show>
                  <Show when={gameplay().entityCounts}>
                    <div class="badge-row">
                      <Badge>{`agents=${gameplay().entityCounts.agents}`}</Badge>
                      <Badge>{`locations=${gameplay().entityCounts.locations}`}</Badge>
                    </div>
                  </Show>
                </CalloutCard>
              </Show>
              <EventCard
                title={gameplay().goalTitle || tr(locale(), "当前目标", "Current Goal")}
                badge={gameplay().progressPercent == null ? "n/a" : `${gameplay().progressPercent}%`}
                badgeClass="badge badge--accent"
                meta={gameplay().objective || tr(locale(), "当前还没有目标说明。", "No objective text yet.")}
              >
                <Show when={gameplay().progressDetail}>
                  <div class="feedback-detail">{gameplay().progressDetail}</div>
                </Show>
              </EventCard>
              <Show when={gameplay().recentFeedback}>
                {(feedback) => (
                  <EventCard
                    title={tr(locale(), "最近玩法反馈", "Recent Gameplay Feedback")}
                    badge={feedback().stage || "-"}
                    badgeClass={
                      feedback().stage === "blocked" ? "badge badge--warn" : "badge badge--good"
                    }
                    meta={`action=${feedback().action || "-"} · Δtick=${feedback().deltaLogicalTime} · Δevent=${feedback().deltaEventSeq}`}
                  >
                    <div class="feedback-summary">
                      {feedback().effect || feedback().reason || "Gameplay feedback updated."}
                    </div>
                    <Show when={feedback().reason}>
                      <div class="feedback-detail">{feedback().reason}</div>
                    </Show>
                    <Show when={feedback().hint}>
                      <div class="feedback-detail">{feedback().hint}</div>
                    </Show>
                  </EventCard>
                )}
              </Show>
              <Show when={gameplay().firstAgentClaimApprovalRequest}>
                {(request) => (
                  (() => {
                    const display = core.describeFirstAgentClaimApprovalRequest(request(), locale());
                    return (
                  <EventCard
                    title={tr(locale(), "首个 Agent Claim 审批", "First Agent Claim Approval")}
                    badge={display?.label || "unknown"}
                    badgeClass={display?.badgeClass || "badge badge--warn"}
                    meta={display?.meta || `request=${request().requestId}`}
                  >
                    <div class="feedback-summary">
                      {display?.summary || request().status || "unknown"}
                    </div>
                    <For each={display?.details || []}>
                      {(detail) => <div class="feedback-detail">{detail}</div>}
                    </For>
                  </EventCard>
                    );
                  })()
                )}
              </Show>
              <EventCard title={tr(locale(), "下一步", "Next Step")} badge={gameplay().stageStatus || "-"}>
                <div class="feedback-summary">
                  {gameplay().nextStepHint || tr(locale(), "等待下一次 runtime 指引更新。", "Wait for the next runtime guidance update.")}
                </div>
                <Show when={gameplay().branchHint}>
                  <div class="feedback-detail">{gameplay().branchHint}</div>
                </Show>
              </EventCard>
              <Show when={gameplayActionFeedback()}>
                {(feedback) => <FeedbackCard feedback={feedback()} display={gameplayActionFeedbackDisplay()} />}
              </Show>
              <div>
                <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "可用玩法动作", "Available Gameplay Actions")}</div>
                <div class="action-grid">
                  <Show
                    when={gameplay().availableActions.length > 0}
                    fallback={<EmptyState>{tr(locale(), "当前还没有发布 canonical gameplay 动作。", "No canonical gameplay actions published yet.")}</EmptyState>}
                  >
                    <For each={gameplay().availableActions}>
                      {(action) => (
                        <EventCard
                          class="event-card event-card--action"
                          title={action.label || action.actionId || "unknown_action"}
                          badge={action.disabledReason ? "handoff" : "ready"}
                          badgeClass={action.disabledReason ? "badge badge--warn" : "badge badge--good"}
                          meta={`protocol=${action.protocolAction || "-"} · target=${action.targetAgentId || "-"}`}
                        >
                          <div class="feedback-detail">
                            {action.disabledReason
                              || tr(locale(), "无需打开 visual QA viewer，也可以直接从正式 Web 入口执行。", "Playable from the formal Web entry without opening the visual QA viewer.")}
                          </div>
                          <Show
                            when={action.executeKind === "request_snapshot" || action.executeKind === "step" || action.executeKind === "play" || action.executeKind === "gameplay_action"}
                          >
                            <div class="toolbar">
                              <button
                                disabled={Boolean(action.disabledReason)}
                                onClick={() => core.sendGameplayAction(action)}
                              >
                                {action.executeKind === "request_snapshot"
                                  ? tr(locale(), "刷新快照", "Refresh Snapshot")
                                  : action.executeKind === "step"
                                    ? tr(locale(), "推进一步", "Advance One Step")
                                    : action.executeKind === "play"
                                      ? tr(locale(), "恢复实时推进", "Resume Live Play")
                                      : tr(locale(), "提交玩法动作", "Submit Gameplay Action")}
                              </button>
                            </div>
                          </Show>
                          <Show when={action.executeKind === "agent_chat"}>
                            <div class="toolbar">
                              <button
                                disabled={Boolean(action.disabledReason)}
                                onClick={() => core.applySelection({ kind: "agent", id: action.targetAgentId })}
                              >
                                {tr(locale(), "切到聊天面板", "Use Chat Panel")}
                              </button>
                            </div>
                          </Show>
                        </EventCard>
                      )}
                    </For>
                  </Show>
                </div>
              </div>
              <CalloutCard
                title={tr(locale(), "未在此页暴露的动作", "Actions Not Exposed On This Page")}
                badge="handoff"
                badgeClass="badge badge--warn"
              >
                <div class="feedback-summary">{gameplay().assetGovernanceHandoff}</div>
                <div class="feedback-detail">
                  {tr(
                    locale(),
                    "资产 / 治理相关能力请走单独 lane；这张主入口页面只保留正式玩法所需的最小动作面。",
                    "Asset and governance actions stay on their dedicated lane; this primary entry only keeps the minimum surface needed for formal gameplay.",
                  )}
                </div>
              </CalloutCard>
            </>
          )}
        </Show>
      </PanelSection>
      <div class="summary-grid">
        <MetricCard label={tr(locale(), "逻辑时间", "Logical Time")} value={state.logicalTime} />
        <MetricCard label={tr(locale(), "事件序号", "Event Seq")} value={state.eventSeq} />
        <MetricCard label={tr(locale(), "世界", "World")} value={state.worldId || "-"} />
        <MetricCard label={tr(locale(), "Viewer 服务", "Viewer Server")} value={state.server || "-"} />
      </div>
      <Show when={showPlayerSessionSurface()}>
        <PanelSection title={tr(locale(), "玩家会话", "Player Session")}>
          <div class="badge-row">
            <Badge class={state.auth.available ? "badge badge--good" : "badge badge--warn"}>
              {`auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}`}
            </Badge>
            <Badge class="badge badge--accent">{`tier=${authSurface().currentTier}`}</Badge>
            <Badge>{`player=${state.auth.playerId || "-"}`}</Badge>
            <Badge>{`boundAgent=${state.auth.boundAgentId || "-"}`}</Badge>
          </div>
          <EmptyState>
            {hostedRecoveryHint()?.detail || state.auth.rebindNotice || authSurface().currentTierReason}
          </EmptyState>
          <Show when={hostedRecoveryHint()}>
            {(hint) => (
              <div class="toolbar">
                <button
                  data-auth-action="retry-issue"
                  disabled={state.auth.issueInFlight}
                  onClick={() => {
                    void core.retryHostedPlayerIdentityIssue();
                  }}
                >
                  {hint().cta}
                </button>
              </div>
            )}
          </Show>
          <Show
            when={
              !state.auth.available
              && String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"
              && !hostedRecoveryHint()
            }
          >
            <div class="toolbar">
              <button
                data-auth-action="retry-issue"
                disabled={state.auth.issueInFlight}
                onClick={() => {
                  void core.retryHostedPlayerIdentityIssue();
                }}
              >
                {tr(locale(), "领取玩家会话", "Acquire Player Session")}
              </button>
            </div>
          </Show>
          <Show when={state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap"}>
            <div class="toolbar">
              <button
                data-auth-action="logout"
                onClick={() => {
                  void core.logoutHostedPlayerSession();
                }}
              >
                {tr(locale(), "释放玩家会话", "Release Player Session")}
              </button>
            </div>
          </Show>
        </PanelSection>
      </Show>
      <details class="panel diagnostic-surface">
        <summary class="panel__header diagnostic-surface__summary">
          <div class="diagnostic-surface__title">
            <div class="panel__title">{tr(locale(), "运行诊断", "Runtime Diagnostics")}</div>
            <div class="diagnostic-surface__meta">
              {tr(
                locale(),
                "执行 lane、auth/session、托管矩阵与最近事件都收在这里，避免它们继续抢占主玩法首屏。",
                "Execution lanes, auth/session truth, hosted matrix, and recent events live here so they no longer dominate the primary gameplay viewport.",
              )}
            </div>
          </div>
          <div class="badge-row">
            <For each={diagnosticsSummaryBadges()}>
              {(label) => <Badge>{label}</Badge>}
            </For>
          </div>
        </summary>
        <div class="panel__body stack">
          <div class="badge-row">
            <Badge>{`ws=${state.wsUrl || "-"}`}</Badge>
            <Badge>{`entryReason=${state.viewerReason || "-"}`}</Badge>
            <Badge>{`renderer=${state.renderer || "n/a"}`}</Badge>
            <Badge>{`controlProfile=${state.controlProfile}`}</Badge>
          </div>
          <PanelSection title={tr(locale(), "执行 Lane", "Execution Lanes")}>
            <div class="badge-row">
              <Badge class="badge badge--accent">debug_viewer</Badge>
              <Badge>{`status=${state.debugViewerStatus}`}</Badge>
              <Badge>{`renderMode=${state.renderMode}`}</Badge>
              <Badge>{`entryReason=${state.viewerReason || "-"}`}</Badge>
            </div>
            <EmptyState style="margin-top:-2px;">
              {tr(
                locale(),
                "debug_viewer 是只读订阅 lane，只负责消费 runtime 快照和事件；关闭这个 viewer 不会停止 agent lane。",
                "debug_viewer is a read-only subscription lane for runtime snapshots/events; closing the viewer does not stop the agent lane.",
              )}
            </EmptyState>
            <Show
              when={selectedDebug()}
              fallback={
                <EmptyState>
                  Select an agent to compare the headless execution lane against this debug_viewer observer
                  lane.
                </EmptyState>
              }
            >
              {(debug) => (
                <>
                  <div class="badge-row">
                    <Badge class="badge badge--accent">selected agent lane</Badge>
                    <Badge>{`provider=${debug().provider_mode || "-"}`}</Badge>
                    <Badge>{`mode=${debug().execution_mode || "-"}`}</Badge>
                    <Badge>{`env=${debug().environment_class || "-"}`}</Badge>
                  </div>
                  <div class="badge-row">
                    <Badge>{`obs=${debug().observation_schema_version || "-"}`}</Badge>
                    <Badge>{`act=${debug().action_schema_version || "-"}`}</Badge>
                    <Badge>{`agentProfile=${debug().agent_profile || "-"}`}</Badge>
                    <Badge>{`providerFallback=${debug().fallback_reason || "-"}`}</Badge>
                  </div>
                  <EmptyState style="margin-top:-2px;">
                    {tr(
                      locale(),
                      "上面的 lane badge 表示 phase-1 期望执行 contract；下面的 provider check badge 表示 runtime_live 基于 /v1/provider/info 和 /v1/provider/health 的真实探测结果。",
                      "Lane badges show the expected phase-1 execution contract. Provider check badges below show the actual runtime_live probe against /v1/provider/info and /v1/provider/health.",
                    )}
                  </EmptyState>
                  <div class="badge-row">
                    <Badge class="badge badge--accent">provider check</Badge>
                    <Badge>{`status=${debug().provider_check_status || "-"}`}</Badge>
                    <Badge>{`source=${debug().provider_check_source || "-"}`}</Badge>
                    <Badge>{`fallback=${debug().provider_check_fallback_reason || "-"}`}</Badge>
                  </div>
                  <Show
                    when={
                      debug().provider_check_error
                      || debug().provider_reported_capabilities?.length
                      || debug().provider_reported_supported_action_sets?.length
                    }
                  >
                    <div class="badge-row">
                      <Badge>{`actualCaps=${(debug().provider_reported_capabilities || []).join(",") || "-"}`}</Badge>
                      <Badge>
                        {`actualActions=${(debug().provider_reported_supported_action_sets || []).join(",") || "-"}`}
                      </Badge>
                      <Badge>{`checkError=${debug().provider_check_error || "-"}`}</Badge>
                    </div>
                  </Show>
                  <JsonBlock value={debug()} />
                </>
              )}
            </Show>
          </PanelSection>
          <div class="badge-row">
            <Badge class={state.auth.available ? "badge badge--good" : "badge badge--warn"}>
              {`auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}`}
            </Badge>
            <Badge class="badge badge--accent">{`tier=${authSurface().currentTier}`}</Badge>
            <Badge>{`source=${authSurface().source}`}</Badge>
            <Badge>{`deploymentHint=${authSurface().deploymentHint}`}</Badge>
            <Badge>{`player=${state.auth.playerId || "-"}`}</Badge>
            <Badge>{`pubkey=${state.auth.publicKey ? `${state.auth.publicKey.slice(0, 10)}…` : "-"}`}</Badge>
            <Badge>{`epoch=${state.auth.sessionEpoch == null ? "-" : state.auth.sessionEpoch}`}</Badge>
            <Badge>{`runtime=${state.auth.runtimeStatus || "-"}`}</Badge>
            <Badge>{`boundAgent=${state.auth.boundAgentId || "-"}`}</Badge>
            <Badge>{`requestedAgent=${state.auth.pendingRequestedAgentId || "-"}`}</Badge>
            <Badge>{state.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle"}</Badge>
          </div>
          <Show when={state.auth.recoveryErrorCode || state.auth.recoveryErrorMessage}>
            <div class="badge-row">
              <Badge class="badge badge--warn">{`recoveryError=${state.auth.recoveryErrorCode || "-"}`}</Badge>
              <Badge>{state.auth.recoveryErrorMessage || "-"}</Badge>
            </div>
          </Show>
          <Show when={showRebindNotice()}>
            <div class="badge-row">
              <Badge class="badge badge--accent">rebind</Badge>
              <Badge>{`target=${state.auth.pendingRequestedAgentId || "-"}`}</Badge>
              <Badge>{state.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry"}</Badge>
            </div>
            <EmptyState>
              Player session is switching to the requested agent and the current action will continue after
              registration succeeds.
            </EmptyState>
          </Show>
          <Show when={state.hostedAdmission}>
            {(admission) => (
              <div class="badge-row">
                <Badge>{`activeSlots=${admission().active_player_sessions}/${admission().max_player_sessions}`}</Badge>
                <Badge>
                  {`effectiveSlots=${
                    admission().effective_player_sessions == null
                      ? "-"
                      : `${admission().effective_player_sessions}/${admission().max_player_sessions}`
                  }`}
                </Badge>
                <Badge>{`runtimeBound=${admission().runtime_bound_player_sessions ?? "-"}`}</Badge>
                <Badge>{`runtimeOnly=${admission().runtime_only_player_sessions ?? "-"}`}</Badge>
                <Badge>{`runtimeProbe=${admission().runtime_probe_status || "-"}`}</Badge>
                <Badge>{`issueBudget=${admission().remaining_issue_budget}`}</Badge>
                <Badge>{`leaseTTL=${admission().slot_lease_ttl_ms}`}</Badge>
                <Badge>{`issued=${admission().issued_players_total}`}</Badge>
                <Badge>{`released=${admission().released_players_total}`}</Badge>
              </div>
            )}
          </Show>
          <Show when={state.hostedAdmission?.runtime_probe_error}>
            <div class="badge-row">
              <Badge class="badge badge--warn">{`runtimeProbeError=${state.hostedAdmission.runtime_probe_error}`}</Badge>
            </div>
          </Show>
          <PanelSection title="Session Ladder">
            <EmptyState>{authSurface().currentTierReason}</EmptyState>
            <div class="event-list">
              <For each={authSurface().tiers}>
                {(tier) => (
                  <EventCard title={tier.label} badge={tier.status} badgeClass={tierBadgeClass(tier.status)} meta={tier.reason} />
                )}
              </For>
            </div>
            <div class="badge-row">
              <Badge class={authSurface().capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn"}>
                {`prompt=${
                  authSurface().capabilities.prompt_control.enabled
                    ? "enabled"
                    : authSurface().capabilities.prompt_control.code
                }`}
              </Badge>
              <Badge class={authSurface().capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn"}>
                {`chat=${
                  authSurface().capabilities.agent_chat.enabled
                    ? "enabled"
                    : authSurface().capabilities.agent_chat.code
                }`}
              </Badge>
              <Badge class="badge badge--warn">
                {`mainToken=${authSurface().capabilities.main_token_transfer.code}`}
              </Badge>
            </div>
            <EmptyState>{authSurface().reconnect}</EmptyState>
          </PanelSection>
          <Show when={hostedActionMatrixView().length > 0}>
            <PanelSection title={tr(locale(), "托管动作矩阵", "Hosted Action Matrix")}>
              <EmptyState>
                {tr(
                  locale(),
                  "这里是 launcher 导出的 hosted public-join 真值面。QA 应该直接读取这些 action id，而不是只靠按钮状态推断。",
                  "This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone.",
                )}
              </EmptyState>
              <div class="event-list">
                <For each={hostedActionMatrixView()}>
                  {(item) => (
                    <EventCard
                      title={item.actionId}
                      badge={item.enabled ? "enabled" : item.code || "blocked"}
                      badgeClass={item.enabled ? "badge badge--good" : "badge badge--warn"}
                      meta={`required_auth=${item.requiredAuth} · availability=${item.availability}`}
                    >
                      <EmptyState>{item.reason || "-"}</EmptyState>
                      <Show when={item.capabilityReason && item.capabilityReason !== item.reason}>
                        <EmptyState>{`viewer=${item.capabilityReason}`}</EmptyState>
                      </Show>
                    </EventCard>
                  )}
                </For>
              </div>
            </PanelSection>
          </Show>
          <div class="summary-grid">
            <MetricCard label={tr(locale(), "Prompt 反馈", "Prompt Feedback")} value={promptFeedback()?.stage || "idle"}>
              <Show when={promptFeedbackDisplay()}>
                <Badge class={promptFeedbackDisplay().badgeClass}>
                  {promptFeedbackDisplay().label}
                </Badge>
              </Show>
            </MetricCard>
            <MetricCard label={tr(locale(), "聊天反馈", "Chat Feedback")} value={chatFeedback()?.stage || "idle"}>
              <Show when={chatFeedbackDisplay()}>
                <Badge class={chatFeedbackDisplay().badgeClass}>
                  {chatFeedbackDisplay().label}
                </Badge>
              </Show>
            </MetricCard>
          </div>
          <div>
            <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "最近事件", "Recent Events")}</div>
            <div class="event-list">
              <Show when={state.recentEvents.length > 0} fallback={<EmptyState>{tr(locale(), "等待 live 事件…", "Waiting for live events…")}</EmptyState>}>
                <For each={state.recentEvents}>
                  {(event) => (
                    <EventCard
                      title={core.summarizeEventTitle(event)}
                      badge={`#${Number(event.id || 0)}`}
                      meta={`time=${Number(event.time || 0)}`}
                    >
                      <JsonBlock value={event.kind} />
                    </EventCard>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>
      </details>
    </div>
  );
}

function InteractionPanel() {
  const locale = () => uiLocale();
  const agentId = () => core.selectedAgentId();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const authSurface = () => core.buildAuthSurfaceModel();
  const promptCapability = () => authSurface().capabilities.prompt_control;
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const mainTokenTransferCapability = () => authSurface().capabilities.main_token_transfer;
  const mainTokenTransferPolicy = () => core.hostedActionPolicy("main_token_transfer");
  const binding = () => core.selectedAgentBindingInfo();
  const debugContext = () => core.selectedAgentExecutionDebugContext();
  const promptFeedback = () => core.snapshotSemanticFeedback(core.state.lastPromptFeedback);
  const chatFeedback = () => core.snapshotSemanticFeedback(core.state.lastChatFeedback);
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const promptVersionState = () => core.describePromptVersionState(promptFeedback(), locale());
  const chatHistory = () =>
    core.state.chatHistory
      .filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId())
      .slice(0, 12);
  const interactionEnabled = () => promptCapability().enabled;
  const promptOverridesVisible = () => !!core.state.promptOverridesVisible;
  const assetLaneStatusText = () =>
    mainTokenTransferCapability().enabled
      ? tr(locale(), "仅预览", "preview_only")
      : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () =>
    mainTokenTransferCapability().enabled
      ? tr(
          locale(),
          "contract 表明这个 lane 具备 strong_auth 级 main_token_transfer 能力，但 viewer 这里仍然不会直接暴露转账表单。",
          "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here.",
        )
      : mainTokenTransferCapability().reason;
  const promptSettingsSummary = () =>
    promptOverridesVisible()
      ? tr(
          locale(),
          "高级 Prompt 设置已展开；你可以继续做 preview/apply/rollback，页面也会显示最近一次反馈。",
          "Advanced prompt settings are expanded; preview/apply/rollback and the latest prompt feedback are visible.",
        )
      : tr(
          locale(),
          "Prompt Overrides 默认收起，避免把 operator 级编辑控件直接堆在主入口。显式展开后仍可做 preview/apply/rollback，`__AW_TEST__.sendPromptControl(...)` 也保持可用。",
          "Prompt Overrides stay hidden by default so operator-level editing controls do not dominate the primary entry. Expanding them keeps preview/apply/rollback available, and `__AW_TEST__.sendPromptControl(...)` remains available.",
        );
  const promptSettingsButtonLabel = () =>
    promptOverridesVisible()
      ? tr(locale(), "收起 Prompt Overrides", "Hide Prompt Overrides")
      : tr(locale(), "显示 Prompt Overrides", "Show Prompt Overrides");

  if (!agentId()) {
    if (gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities") {
      return <EmptyEntityRecoveryCard locale={locale()} gameplay={gameplaySummary} />;
    }
    return <EmptyState>{tr(locale(), "先选中一个 Agent，才能解锁 prompt/chat 控制。", "Select an agent to unlock prompt/chat controls.")}</EmptyState>;
  }

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge class="badge badge--accent">Agent Interaction</Badge>
        <Badge>{`agent=${agentId()}`}</Badge>
        <Badge>{`activePrompt=${`v${promptVersionState().currentVersion}`}`}</Badge>
        <Badge>{`nextRollback=${`v${promptVersionState().nextRollbackTargetVersion}`}`}</Badge>
        <Show when={promptVersionState().restoredFromVersion != null}>
          <Badge>{`restoredFrom=${`v${promptVersionState().restoredFromVersion}`}`}</Badge>
        </Show>
      </div>
      <Show when={debugContext()?.provider_mode === "provider_loopback_http"}>
        <EmptyState>
          {tr(
            locale(),
            `当前选中的 Agent 正通过 provider-backed loopback bridge 运行在 ${
              debugContext()?.execution_mode || "headless_agent"
            }；viewer 仍处于 debug_viewer 只读观察模式，所以这里会刻意禁用 prompt/chat。`,
            `Selected agent currently runs through the provider-backed loopback bridge in ${
              debugContext()?.execution_mode || "headless_agent"
            }; viewer stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.`,
          )}
        </EmptyState>
      </Show>
      <Show when={debugContext()?.provider_mode !== "provider_loopback_http"}>
        <Show
          when={interactionEnabled()}
          fallback={<EmptyState>{promptCapability().reason}</EmptyState>}
        >
          <div class="badge-row">
            <Badge class="badge badge--good">{authSurface().currentTier}</Badge>
            <Badge>{`player=${core.state.auth.playerId}`}</Badge>
            <Badge>{`source=${authSurface().source}`}</Badge>
          </div>
          <EmptyState>{promptCapability().reason}</EmptyState>
        </Show>
      </Show>
      <div class="badge-row">
        <Badge>{`boundPlayer=${binding()?.playerId || "-"}`}</Badge>
        <Badge>{`boundKey=${binding()?.publicKey ? `${binding().publicKey.slice(0, 10)}…` : "-"}`}</Badge>
        <Badge class={promptCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
          {`prompt=${promptCapability().enabled ? "enabled" : promptCapability().code}`}
        </Badge>
        <Badge class={chatCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
          {`chat=${chatCapability().enabled ? "enabled" : chatCapability().code}`}
        </Badge>
        <Badge class={mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
          {`mainToken=${assetLaneStatusText()}`}
        </Badge>
      </div>
      <EmptyState>{assetLaneDetail()}</EmptyState>
      <PanelSection title={tr(locale(), "资产 / 治理 Lane", "Asset / Governance Lane")}>
        <div class="badge-row">
          <Badge class={mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
            {`main_token_transfer=${assetLaneStatusText()}`}
          </Badge>
          <Badge>{`required_auth=${mainTokenTransferPolicy()?.required_auth || "-"}`}</Badge>
          <Badge>{`availability=${mainTokenTransferPolicy()?.availability || "-"}`}</Badge>
        </div>
        <EmptyState>{assetLaneDetail()}</EmptyState>
        <EmptyState>
          {mainTokenTransferPolicy()?.reason
            || tr(locale(), "当前 lane 没有 main_token_transfer 的 hosted action policy。", "No hosted action policy is available for main_token_transfer on this lane.")}
        </EmptyState>
        <div class="toolbar">
          <button disabled>{tr(locale(), "主代币转账（这里暂未开放）", "Main Token Transfer (Not Exposed Here Yet)")}</button>
        </div>
      </PanelSection>
      <PanelSection
        title={tr(locale(), "Agent 聊天", "Agent Chat")}
        eyebrow={tr(locale(), "命令面", "Command Surface")}
        meta={tr(locale(), "主舞台负责看局势；这里负责向当前目标发消息和读回复。", "The stage is for reading the situation. This surface is for messaging the current target and reading replies.")}
      >
        <div class="field">
          <label for="agent-chat-message">{tr(locale(), "消息", "Message")}</label>
          <textarea
            id="agent-chat-message"
            rows="4"
            placeholder={tr(locale(), "给当前选中的 Agent 发一条消息", "Send a message to the selected agent")}
            disabled={!chatCapability().enabled}
            value={core.state.chatDraft.message}
            onInput={(event) => {
              core.state.chatDraft.message = String(event.currentTarget.value || "");
              core.state.chatDraft.dirty = true;
            }}
          />
        </div>
        <div class="toolbar">
          <button
            data-chat-send="1"
            disabled={!chatCapability().enabled}
            onClick={() => core.sendAgentChat(agentId(), core.state.chatDraft.message)}
          >
            {tr(locale(), "发送聊天", "Send Chat")}
          </button>
        </div>
        <Show when={chatFeedback()} fallback={<EmptyState>{tr(locale(), "还没有聊天反馈。", "No chat feedback yet.")}</EmptyState>}>
          {(feedback) => <FeedbackCard feedback={feedback()} display={chatFeedbackDisplay()} />}
        </Show>
        <div>
          <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "消息流", "Message Flow")}</div>
          <div class="event-list">
            <Show when={chatHistory().length > 0} fallback={<EmptyState>{tr(locale(), "这个 Agent 还没有聊天历史。", "No chat history for this agent yet.")}</EmptyState>}>
              <For each={chatHistory()}>
                {(entry) => (
                  <EventCard
                    title={
                      entry.source === "player"
                        ? `${tr(locale(), "玩家", "player")} → ${entry.targetAgentId || entry.agentId || "agent"}`
                        : `${entry.agentId || "agent"} ${tr(locale(), "已发言", "spoke")}`
                    }
                    badge={`tick=${Number(entry.tick || 0)}`}
                    meta={`speaker=${entry.speaker || entry.playerId || "-"} · location=${entry.locationId || "-"}`}
                  >
                    <JsonBlock value={entry} />
                  </EventCard>
                )}
              </For>
            </Show>
          </div>
        </div>
      </PanelSection>
      <PanelSection
        title={tr(locale(), "高级 Prompt 设置", "Advanced Prompt Settings")}
        eyebrow={tr(locale(), "高级控制", "Advanced Controls")}
        meta={tr(locale(), "保留 operator 级 prompt 控制，但默认收起，不与玩家主路径竞争。", "Operator-level prompt controls stay available here, but collapsed by default so they do not compete with the player path.")}
      >
        <div class="badge-row">
          <Badge class={promptOverridesVisible() ? "badge badge--good" : "badge"}>
            {promptOverridesVisible()
              ? tr(locale(), "状态=已展开", "state=expanded")
              : tr(locale(), "状态=默认收起", "state=hidden_by_default")}
          </Badge>
          <Badge>{tr(locale(), "本地设置持久化", "locally persisted")}</Badge>
        </div>
        <EmptyState>{promptSettingsSummary()}</EmptyState>
        <div class="toolbar">
          <button
            data-prompt-visibility-toggle="1"
            onClick={() => core.togglePromptOverridesVisible()}
          >
            {promptSettingsButtonLabel()}
          </button>
        </div>
      </PanelSection>
      <Show when={promptOverridesVisible()}>
        <PanelSection title="Prompt Overrides">
          <div class="feedback-detail">{promptVersionState().summary}</div>
          <div class="feedback-detail">{promptVersionState().detail}</div>
          <Show
            when={
              authSurface().capabilities.prompt_control.enabled
              && String(core.state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"
            }
          >
            <div class="field">
              <label for="strong-auth-approval-code">{tr(locale(), "后端审批码", "Backend Approval Code")}</label>
              <input
                id="strong-auth-approval-code"
                type="password"
                autocomplete="off"
                value={core.state.strongAuth.approvalCode || ""}
                onInput={(event) => {
                  core.state.strongAuth.approvalCode = String(event.currentTarget.value || "");
                }}
              />
            </div>
          </Show>
          <div class="field">
            <label for="prompt-system">{tr(locale(), "System Prompt 覆盖", "System Prompt Override")}</label>
            <textarea
              id="prompt-system"
              rows="4"
              disabled={!promptCapability().enabled}
              value={core.state.promptDraft.systemPrompt}
              onInput={(event) => {
                core.state.promptDraft.systemPrompt = String(event.currentTarget.value || "");
                core.state.promptDraft.dirty = true;
              }}
            />
          </div>
          <div class="field">
            <label for="prompt-short">{tr(locale(), "短期目标覆盖", "Short-Term Goal Override")}</label>
            <textarea
              id="prompt-short"
              rows="3"
              disabled={!promptCapability().enabled}
              value={core.state.promptDraft.shortTermGoal}
              onInput={(event) => {
                core.state.promptDraft.shortTermGoal = String(event.currentTarget.value || "");
                core.state.promptDraft.dirty = true;
              }}
            />
          </div>
          <div class="field">
            <label for="prompt-long">{tr(locale(), "长期目标覆盖", "Long-Term Goal Override")}</label>
            <textarea
              id="prompt-long"
              rows="3"
              disabled={!promptCapability().enabled}
              value={core.state.promptDraft.longTermGoal}
              onInput={(event) => {
                core.state.promptDraft.longTermGoal = String(event.currentTarget.value || "");
                core.state.promptDraft.dirty = true;
              }}
            />
          </div>
          <div class="toolbar">
            <button
              data-prompt-action="preview"
              disabled={!promptCapability().enabled}
              onClick={() => core.sendPromptControl("preview", null)}
            >
              {tr(locale(), "预览 Prompt", "Preview Prompt")}
            </button>
            <button
              data-prompt-action="apply"
              disabled={!promptCapability().enabled}
              onClick={() => core.sendPromptControl("apply", null)}
            >
              {tr(locale(), "应用 Prompt", "Apply Prompt")}
            </button>
          </div>
          <div class="toolbar">
            <div class="field" style="margin:0; min-width:180px; flex:1;">
              <label for="prompt-rollback-version">{tr(locale(), "下一次回滚目标版本", "Next Rollback Target Version")}</label>
              <input
                id="prompt-rollback-version"
                type="number"
                min="0"
                step="1"
                disabled={!promptCapability().enabled}
                value={Number(core.state.promptDraft.rollbackTargetVersion || 0)}
                onInput={(event) => {
                  const nextValue = Number(event.currentTarget.value || 0);
                  core.state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
                  core.requestRender();
                }}
              />
            </div>
            <button
              data-prompt-action="rollback"
              disabled={!promptCapability().enabled}
              onClick={() => {
                core.sendPromptControl("rollback", {
                  toVersion: Number(core.state.promptDraft.rollbackTargetVersion || 0),
                });
              }}
            >
              {tr(locale(), "回滚 Prompt", "Rollback Prompt")}
            </button>
          </div>
          <Show when={promptFeedback()} fallback={<EmptyState>{tr(locale(), "还没有 Prompt 反馈。", "No prompt feedback yet.")}</EmptyState>}>
            {(feedback) => <FeedbackCard feedback={feedback()} display={promptFeedbackDisplay()} />}
          </Show>
          <Show when={core.state.strongAuth.lastGrantActionId}>
            <EmptyState>
              {`lastGrant=${core.state.strongAuth.lastGrantActionId} expiresAt=${
                core.state.strongAuth.lastGrantExpiresAtUnixMs || "-"
              }`}
            </EmptyState>
          </Show>
          <Show when={core.state.strongAuth.lastGrantError}>
            <EmptyState style="color:var(--bad);">{core.state.strongAuth.lastGrantError}</EmptyState>
          </Show>
        </PanelSection>
      </Show>
    </div>
  );
}

function DetailsPanel() {
  const locale = () => uiLocale();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const worldScaleSurface = () => core.buildWorldScaleSurface(locale());
  const selectedLabel = () =>
    core.state.selectedKind && core.state.selectedId
      ? `${core.state.selectedKind}:${core.state.selectedId}`
      : tr(locale(), "未选择", "nothing selected");
  const snapshotSummary = () => ({
    config: core.state.snapshot?.config || null,
    counts: {
      agents: Object.keys(core.state.snapshot?.model?.agents || {}).length,
      locations: Object.keys(core.state.snapshot?.model?.locations || {}).length,
      promptProfiles: Object.keys(core.state.snapshot?.model?.agent_prompt_profiles || {}).length,
      executionDebugContexts: Object.keys(core.state.snapshot?.model?.agent_execution_debug_contexts || {}).length,
    },
    metrics: core.state.metrics,
    hostedAccess: core.clone(core.state.hostedAccess),
  });
  const snapshotCounts = () => ({
    agents: Object.keys(core.state.snapshot?.model?.agents || {}).length,
    locations: Object.keys(core.state.snapshot?.model?.locations || {}).length,
    promptProfiles: Object.keys(core.state.snapshot?.model?.agent_prompt_profiles || {}).length,
    executionDebugContexts: Object.keys(core.state.snapshot?.model?.agent_execution_debug_contexts || {}).length,
  });
  const hasSnapshotDiagnostics = () =>
    !!core.state.snapshot || !!core.state.metrics || !!core.state.hostedAccess;

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge class="badge badge--accent">{tr(locale(), "已选中", "Selected")}</Badge>
        <Badge>{selectedLabel()}</Badge>
      </div>
      <InteractionPanel />
      <Show
        when={core.state.selectedObject}
        fallback={
          gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities"
            ? (
              <EmptyEntityRecoveryCard
                locale={locale()}
                gameplay={gameplaySummary}
                title={tr(locale(), "对象明细暂时不可用", "Object Details Are Temporarily Unavailable")}
              />
            )
            : <EmptyState>{tr(locale(), "请先从左侧列表选一个 Agent 或地点。", "Select an agent or location from the left list.")}</EmptyState>
        }
      >
        {(selected) => (
          <DiagnosticDetails
            locale={locale()}
            label={tr(locale(), "展开对象原始明细", "Expand Raw Object Details")}
            note={tr(
              locale(),
              "默认只保留交互面；只有在核查快照字段或诊断对象结构时再展开原始 JSON。",
              "The interaction surface stays in front by default. Expand raw JSON only when you need to inspect snapshot fields or diagnose object shape.",
            )}
            value={() => core.clone(selected())}
          />
        )}
      </Show>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "世界规模", "World Scale")}</div>
        <div class="badge-row">
          <Badge>{`agents=${snapshotCounts().agents}`}</Badge>
          <Badge>{`locations=${snapshotCounts().locations}`}</Badge>
          <Badge>{`promptProfiles=${snapshotCounts().promptProfiles}`}</Badge>
          <Badge>{`debugContexts=${snapshotCounts().executionDebugContexts}`}</Badge>
        </div>
        <div class="stack" style="margin-top:10px;">
          <MetricCard
            label={tr(locale(), "物理真值单位", "Canonical Physical Unit")}
            value={worldScaleSurface().physicalTruth.canonicalUnitLabel || "-"}
          >
            <Badge>{tr(locale(), "整数厘米", "integer centimeters")}</Badge>
          </MetricCard>
          <div class="feedback-detail">{worldScaleSurface().physicalTruth.canonicalUnitDetail}</div>
          <MetricCard
            label={tr(locale(), "世界边界", "World Bounds")}
            value={worldScaleSurface().physicalTruth.worldBoundsLabel || tr(locale(), "未发布", "not published")}
          >
            <Badge>{tr(locale(), "snapshot.config.space", "snapshot.config.space")}</Badge>
          </MetricCard>
          <div class="feedback-detail">{worldScaleSurface().physicalTruth.worldBoundsDetail}</div>
          <Show when={worldScaleSurface().physicalTruth.anchor}>
            {(anchor) => (
              <EventCard
                title={anchor().label}
                badge={anchor().kind}
                badgeClass="badge badge--accent"
                meta={`id=${anchor().id}${anchor().locationId ? ` · location=${anchor().locationId}` : ""}`}
              >
                <div class="feedback-summary">
                  {anchor().positionLabel || tr(locale(), "缺少可读坐标。", "Missing readable coordinates.")}
                </div>
                <Show when={anchor().radiusLabel}>
                  <div class="feedback-detail">
                    {tr(locale(), "地点半径真值", "Location radius truth")}={anchor().radiusLabel}
                  </div>
                </Show>
              </EventCard>
            )}
          </Show>
          <div>
            <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "最近距离样本", "Nearest Distance Samples")}</div>
            <div class="event-list">
              <Show
                when={worldScaleSurface().physicalTruth.nearestLocations.length > 0}
                fallback={
                  <EmptyState>
                    {tr(
                      locale(),
                      "当前没有足够的地点数据来给出距离样本。",
                      "The current snapshot does not expose enough locations to show distance samples.",
                    )}
                  </EmptyState>
                }
              >
                <For each={worldScaleSurface().physicalTruth.nearestLocations}>
                  {(location) => (
                    <EventCard
                      title={location.name}
                      badge={location.distanceLabel || "-"}
                      badgeClass="badge badge--good"
                      meta={`id=${location.id}`}
                    >
                      <div class="feedback-detail">
                        {tr(locale(), "真实距离", "Physical distance")}={location.distanceLabel || "-"}
                      </div>
                      <Show when={location.radiusLabel}>
                        <div class="feedback-detail">
                          {tr(locale(), "地点半径", "Location radius")}={location.radiusLabel}
                        </div>
                      </Show>
                    </EventCard>
                  )}
                </For>
              </Show>
            </div>
          </div>
          <EventCard
            title={tr(locale(), "表现层说明", "Presentation Notes")}
            badge={tr(locale(), "不要误读 marker", "Do not trust marker size")}
            badgeClass="badge badge--warn"
          >
            <div class="feedback-summary">{worldScaleSurface().presentationScale.markerTruthNote}</div>
            <div class="feedback-detail">{worldScaleSurface().presentationScale.zoomTruthNote}</div>
            <div class="feedback-detail">{worldScaleSurface().presentationScale.softwareSafeNote}</div>
          </EventCard>
          <EmptyState>
            {tr(
              locale(),
              "主状态已经在中间的“世界摘要”里展示；这里现在专门保留“厘米真值 vs 表现层夸张”的读图锚点，原始快照仍按需展开。",
              "The main runtime state already lives in World Summary; this section now reserves the reading anchors for centimeter truth vs presentation exaggeration, while raw snapshots stay collapsible.",
            )}
          </EmptyState>
        </div>
        <Show when={hasSnapshotDiagnostics()}>
          <DiagnosticDetails
            locale={locale()}
            label={tr(locale(), "展开原始快照诊断", "Expand Raw Snapshot Diagnostics")}
            note={tr(
              locale(),
              "只在需要排查快照结构或 hosted access 原始字段时展开。",
              "Expand only when you need to inspect the raw snapshot shape or hosted access fields.",
            )}
            value={snapshotSummary}
          />
        </Show>
      </div>
      <Show when={core.state.lastError}>
        <div>
          <div class="panel__title" style="margin-bottom:10px; color: var(--bad);">{tr(locale(), "最近错误", "Last Error")}</div>
          <pre class="json">{core.state.lastError}</pre>
        </div>
      </Show>
    </div>
  );
}

function AppShell() {
  const locale = () => uiLocale();
  return (
    <>
      <MobileJumpRail />
      <section class="panel panel--targets" id="viewer-targets-panel">
        <div class="panel__header panel__header--stack">
          <div class="panel__eyebrow">{tr(locale(), "导航", "Navigate")}</div>
          <div class="panel__title">{tr(locale(), "目标", "Targets")}</div>
          <div class="panel__meta-copy">
            {tr(locale(), "先锁定对象，再进入世界舞台或右侧命令面。", "Lock onto a target first, then move into the stage or command surface.")}
          </div>
        </div>
        <div class="panel__body">
          <TargetsPanel />
        </div>
      </section>
      <section class="panel panel--stage" id="viewer-stage-panel">
        <div class="panel__body panel__body--stage">
          <div class="stack">
            <WorldStageHero />
            <PixelWorldHost locale={locale()} />
            <WorldSummaryPanel />
          </div>
        </div>
      </section>
      <section class="panel panel--details" id="viewer-details-panel">
        <div class="panel__header panel__header--stack">
          <div class="panel__eyebrow">{tr(locale(), "指挥与核查", "Command and Inspect")}</div>
          <div class="panel__title">{tr(locale(), "明细", "Details")}</div>
          <div class="panel__meta-copy">
            {tr(locale(), "把聊天、Prompt 和对象核查留在这里，避免继续淹没主舞台。", "Keep chat, prompt controls, and object inspection here so they stop flooding the main stage.")}
          </div>
        </div>
        <div class="panel__body">
          <DetailsPanel />
        </div>
      </section>
    </>
  );
}

const app = document.getElementById("app");
if (!app) {
  throw new Error("viewer root #app is missing");
}

let dispose = mount(() => <AppShell />, app);
core.setRenderHook(() => {
  dispose();
  app.textContent = "";
  dispose = mount(() => <AppShell />, app);
});

core.initializeSoftwareSafeCore();
