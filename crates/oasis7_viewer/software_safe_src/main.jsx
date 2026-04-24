import { For, Show } from "solid-js";
import { render as mount } from "solid-js/web";

import * as core from "./legacy_core.js";

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
  const standardUrl = new URL(window.location.href);
  standardUrl.pathname = standardUrl.pathname.replace(/software_safe\.html$/, "");
  if (!standardUrl.pathname) {
    standardUrl.pathname = "/";
  }
  standardUrl.searchParams.set("render_mode", "standard");
  standardUrl.searchParams.set("locale", localeCode(locale));
  standardUrl.searchParams.delete("language");
  standardUrl.searchParams.delete("software_safe_reason");

  const softwareSafeUrl = new URL(window.location.href);
  softwareSafeUrl.searchParams.set("locale", localeCode(locale));
  softwareSafeUrl.searchParams.delete("language");

  return {
    softwareSafeUrl: softwareSafeUrl.toString(),
    standardUrl: standardUrl.toString(),
  };
}

function openViewerUrl(url) {
  window.open(url, "_blank", "noopener");
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
  return (
    <details class="diagnostic">
      <summary>{props.label ?? tr(locale(), "原始诊断", "Raw diagnostics")}</summary>
      <div class="stack" style="margin-top:10px;">
        <Show when={props.note}>
          <div class="feedback-detail">{props.note}</div>
        </Show>
        <JsonBlock value={props.value} />
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
    <div class="event-card">
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
    <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
      <div class="panel__header">
        <div class="panel__title">{props.title}</div>
      </div>
      <div class="panel__body stack">{props.children}</div>
    </div>
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
              "主玩法继续留在当前页面；这里只保留语言切换和标准 Viewer 跳转。",
              "Primary gameplay stays on this page. This menu only keeps locale switching and the standard Viewer jump.",
            )}
          </div>
        </div>
        <div class="toolbar">
          <button
            data-locale="zh"
            disabled={locale() === "zh"}
            onClick={() => core.setSoftwareSafeLocale("zh")}
          >
            中文
          </button>
          <button
            data-locale="en"
            disabled={locale() === "en"}
            onClick={() => core.setSoftwareSafeLocale("en")}
          >
            English
          </button>
        </div>
        <div class="toolbar">
          <button
            data-entry="standard-viewer-current-locale"
            onClick={() => openViewerUrl(viewerEntryUrls().standardUrl)}
          >
            {tr(locale(), "打开标准 Viewer", "Open standard Viewer")}
          </button>
        </div>
        <div class="badge-row">
          <Badge>{`locale=${localeCode(locale())}`}</Badge>
        </div>
        <div class="feedback-detail">{viewerEntryUrls().standardUrl}</div>
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

function TargetsPanel() {
  const lists = () => core.modelLists();
  const locale = () => uiLocale();

  return (
    <div class="stack">
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
                    {`id=${location.id} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(location.resources)}`}
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
  const promptFeedback = () => core.snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => core.snapshotSemanticFeedback(state.lastChatFeedback);
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

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge class="badge badge--accent">software_safe</Badge>
        <Badge class="badge badge--accent">{tr(locale(), "正式 Web 主入口", "Formal Web Entry")}</Badge>
        <Badge class={core.connectionBadgeClass()}>
          {tr(locale(), "连接状态", "connection")}={state.connectionStatus}
        </Badge>
        <Badge>{`debugViewer=${state.debugViewerMode}:${state.debugViewerStatus}`}</Badge>
        <Badge>{`rendererClass=${state.rendererClass}`}</Badge>
        <Badge>{`controlProfile=${state.controlProfile}`}</Badge>
      </div>
      <PanelSection title={tr(locale(), "正式玩法摘要", "Formal Gameplay Summary")}>
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
              <EventCard title={tr(locale(), "下一步", "Next Step")} badge={gameplay().stageStatus || "-"}>
                <div class="feedback-summary">
                  {gameplay().nextStepHint || tr(locale(), "等待下一次 runtime 指引更新。", "Wait for the next runtime guidance update.")}
                </div>
                <Show when={gameplay().branchHint}>
                  <div class="feedback-detail">{gameplay().branchHint}</div>
                </Show>
              </EventCard>
              <Show when={gameplay().blockerKind || gameplay().blockerDetail}>
                <EventCard
                  title={tr(locale(), "阻塞 / 交接", "Blocked / Handoff")}
                  badge={gameplay().blockerKind || "blocked"}
                  badgeClass="badge badge--warn"
                >
                  <div class="feedback-summary">
                    {gameplay().blockerDetail || tr(locale(), "当前玩法被阻塞，需要显式恢复。", "Gameplay is blocked and needs explicit recovery.")}
                  </div>
                  <div class="feedback-detail">{gameplay().assetGovernanceHandoff}</div>
                </EventCard>
              </Show>
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
              <div>
                <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "可用玩法动作", "Available Gameplay Actions")}</div>
                <div class="event-list">
                  <Show
                    when={gameplay().availableActions.length > 0}
                    fallback={<EmptyState>{tr(locale(), "当前还没有发布 canonical gameplay 动作。", "No canonical gameplay actions published yet.")}</EmptyState>}
                  >
                    <For each={gameplay().availableActions}>
                      {(action) => (
                        <EventCard
                          title={action.label || action.actionId || "unknown_action"}
                          badge={action.disabledReason ? "handoff" : "ready"}
                          badgeClass={action.disabledReason ? "badge badge--warn" : "badge badge--good"}
                          meta={`protocol=${action.protocolAction || "-"} · target=${action.targetAgentId || "-"}`}
                        >
                          <div class="feedback-detail">
                            {action.disabledReason
                              || tr(locale(), "无需打开 visual QA viewer，也可以直接从正式 Web 入口执行。", "Playable from the formal Web entry without opening the visual QA viewer.")}
                          </div>
                        </EventCard>
                      )}
                    </For>
                  </Show>
                </div>
              </div>
              <EventCard title={tr(locale(), "缺失动作交接", "Missing Action Handoff")} badge="explicit" badgeClass="badge badge--warn">
                <div class="feedback-summary">{gameplay().assetGovernanceHandoff}</div>
                <div class="feedback-detail">
                  {tr(
                    locale(),
                    "资产 / 治理相关能力请走下面的单独 lane。本页刻意不把转账表单塞进 primary Web entry。",
                    "Use the Asset / Governance Lane below for policy visibility. This page intentionally keeps transfer forms out of the primary Web entry.",
                  )}
                </div>
              </EventCard>
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
      <div class="badge-row">
        <Badge>{`ws=${state.wsUrl || "-"}`}</Badge>
        <Badge>{`entryReason=${state.softwareSafeReason || "-"}`}</Badge>
        <Badge>{`renderer=${state.renderer || "n/a"}`}</Badge>
      </div>
      <PanelSection title={tr(locale(), "执行 Lane", "Execution Lanes")}>
        <div class="badge-row">
          <Badge class="badge badge--accent">debug_viewer</Badge>
          <Badge>{`status=${state.debugViewerStatus}`}</Badge>
          <Badge>{`renderMode=${state.renderMode}`}</Badge>
          <Badge>{`entryReason=${state.softwareSafeReason || "-"}`}</Badge>
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
                  debug().provider_check_error ||
                  debug().provider_reported_capabilities?.length ||
                  debug().provider_reported_supported_action_sets?.length
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
      <Show when={state.auth.rebindNotice}>
        <EmptyState>{state.auth.rebindNotice}</EmptyState>
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
      <Show when={hostedRecoveryHint()}>
        {(hint) => (
          <div
            class="panel panel--nested"
            style="background:rgba(255,255,255,0.02); border-color:rgba(255,184,77,0.35);"
          >
            <div class="panel__header">
              <div class="panel__title">{tr(locale(), "托管恢复", "Hosted Recovery")}</div>
            </div>
            <div class="panel__body stack">
              <div class="badge-row">
                <Badge class="badge badge--warn">{hint().kind}</Badge>
                <Badge>{hint().title}</Badge>
              </div>
              <EmptyState>{hint().detail}</EmptyState>
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
            </div>
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
            Acquire Hosted Player Session
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
            Release Hosted Player Session
          </button>
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
  );
}

function InteractionPanel() {
  const locale = () => uiLocale();
  const agentId = () => core.selectedAgentId();
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
          "contract 表明这个 lane 具备 strong_auth 级 main_token_transfer 能力，但 software_safe 这里仍然不会直接暴露转账表单。",
          "Contract marks main_token_transfer as strong_auth-capable on this lane, but software_safe still exposes no transfer form here.",
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
            }；software_safe 仍处于 debug_viewer 只读观察模式，所以这里会刻意禁用 prompt/chat。`,
            `Selected agent currently runs through the provider-backed loopback bridge in ${
              debugContext()?.execution_mode || "headless_agent"
            }; software_safe stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.`,
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
      <PanelSection title={tr(locale(), "Agent 聊天", "Agent Chat")}>
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
      <PanelSection title={tr(locale(), "高级 Prompt 设置", "Advanced Prompt Settings")}>
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
  const snapshotCounts = () => snapshotSummary().counts;
  const hasSnapshotDiagnostics = () =>
    !!core.state.snapshot || !!core.state.metrics || !!core.state.hostedAccess;

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge class="badge badge--accent">{tr(locale(), "已选中", "Selected")}</Badge>
        <Badge>{selectedLabel()}</Badge>
      </div>
      <InteractionPanel />
      <Show when={core.state.selectedObject} fallback={<EmptyState>{tr(locale(), "请先从左侧列表选一个 Agent 或地点。", "Select an agent or location from the left list.")}</EmptyState>}>
        <JsonBlock value={core.clone(core.state.selectedObject)} />
      </Show>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">{tr(locale(), "世界规模", "World Scale")}</div>
        <div class="badge-row">
          <Badge>{`agents=${snapshotCounts().agents}`}</Badge>
          <Badge>{`locations=${snapshotCounts().locations}`}</Badge>
          <Badge>{`promptProfiles=${snapshotCounts().promptProfiles}`}</Badge>
          <Badge>{`debugContexts=${snapshotCounts().executionDebugContexts}`}</Badge>
        </div>
        <EmptyState style="margin-top:10px;">
          {tr(
            locale(),
            "主状态已经在中间的“世界摘要”里展示；这里默认只保留规模信息，原始快照改为按需展开。",
            "The main runtime state already lives in World Summary; this panel now keeps only world scale by default and leaves raw snapshot data collapsed.",
          )}
        </EmptyState>
        <Show when={hasSnapshotDiagnostics()}>
          <DiagnosticDetails
            locale={locale()}
            label={tr(locale(), "展开原始快照诊断", "Expand Raw Snapshot Diagnostics")}
            note={tr(
              locale(),
              "只在需要排查快照结构或 hosted access 原始字段时展开。",
              "Expand only when you need to inspect the raw snapshot shape or hosted access fields.",
            )}
            value={snapshotSummary()}
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
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">{tr(locale(), "目标", "Targets")}</div>
        </div>
        <div class="panel__body">
          <TargetsPanel />
        </div>
      </section>
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">{tr(locale(), "世界摘要", "World Summary")}</div>
          <ViewerEntryMenu />
        </div>
        <div class="panel__body">
          <WorldSummaryPanel />
        </div>
      </section>
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">{tr(locale(), "明细", "Details")}</div>
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
  throw new Error("software_safe root #app is missing");
}

let dispose = mount(() => <AppShell />, app);
core.setRenderHook(() => {
  dispose();
  app.textContent = "";
  dispose = mount(() => <AppShell />, app);
});

core.initializeSoftwareSafeCore();
