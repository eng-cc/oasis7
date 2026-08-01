import { createEffect, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { render as mount } from "solid-js/web";
import * as core from "./legacy_core.js";
import { FirstChatUnlockPreview } from "./first_chat_unlock_preview.jsx";
import { PixelWorldHost } from "./pixel_world_host.jsx";
import { MicroDepotFacilitiesPanel } from "./micro_depot_facilities_panel.jsx";
import { RecoveryOptionComparisonPanel } from "./recovery_option_comparison_panel.jsx"; import { FallbackTradeoffPanel } from "./fallback_tradeoff_panel.jsx";
import { MarketQuoteDecisionGameplayPanel, PowerSurvivalQuoteGameplayPanel, ProductValidationQuoteGameplayPanel, RefineQuoteGameplayPanel } from "./gameplay_quote_panels.jsx";
import { installMarketQuoteDecisionVisualFixture, installPowerSurvivalQuoteVisualFixture, installProductValidationQuoteVisualFixture, installRefineQuotePreflightVisualFixture, installWaitResolutionQuoteVisualFixture } from "./quote_visual_fixture_installers.js";
import { ReprioritizeActionForm } from "./reprioritize_action_form.jsx";
import { createViewerAgentClaimDisplayModel } from "./viewer_agent_claim_display_model.js";
import { fallbackTradeoffVisualFixture } from "./viewer_fallback_tradeoff_fixture.js";
import {
  HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
  LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE,
  isHostedPublicJoinDeploymentMode,
} from "./software_safe_constants.js";
import { recoveryOptionVisualFixture } from "./viewer_recovery_option_fixture.js";
const VIEWER_VISUAL_FIXTURE_GLOBAL = "__OASIS7_VIEWER_VISUAL_FIXTURES__";
const [viewerStateRevision, setViewerStateRevision] = createSignal(0);
function observeViewerStateRevision() {
  viewerStateRevision();
}

function uiLocale() { return core.state.uiLocale; }
function focusViewerAnchor(event) {
  const href = event.currentTarget.getAttribute("href"); const target = href?.startsWith("#") ? document.getElementById(href.slice(1)) : null;
  if (!target) return; event.preventDefault(); target.scrollIntoView({ behavior: "auto", block: "start", inline: "nearest" }); window.history.replaceState(null, "", href);
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
  return <div class={`empty ${props.class ?? ""}`} style={props.style}>{props.children}</div>;
}

function targetSyncProgressLines(progress, locale) {
  if (!progress) {
    return [];
  }
  const connected = progress.connectionStatus === "connected";
  const connectionText = connected
    ? tr(locale, "已连接", "connected")
    : progress.connectionStatus === "error"
      ? tr(locale, "连接错误", "connection error")
      : tr(locale, "正在连接", "connecting");
  const handshakeText = progress.serverReady
    ? tr(locale, "已完成", "server ready")
    : tr(locale, "等待服务器 hello", "waiting for server hello");
  const snapshotText = progress.snapshotReceived
    ? tr(
      locale,
      `已收到：行动体 ${progress.totalAgentCount}，地点 ${progress.totalLocationCount}`,
      `received: ${progress.totalAgentCount} agents, ${progress.totalLocationCount} locations`,
    )
    : progress.snapshotRequested
      ? tr(
        locale,
        progress.snapshotRetryCount > 0
          ? `已请求首个世界快照，重试 ${progress.snapshotRetryCount} 次`
          : "已请求首个世界快照",
        progress.snapshotRetryCount > 0
          ? `first world snapshot requested, ${progress.snapshotRetryCount} retries`
          : "first world snapshot requested",
      )
      : tr(locale, "等待首个世界快照", "waiting for first world snapshot");
  const sessionText = progress.authSyncInFlight
    ? tr(locale, "正在同步玩家会话", "syncing player session")
    : tr(
      locale,
      `状态 ${progress.authRuntimeStatus || progress.authRegistrationStatus || "pending"}`,
      `status ${progress.authRuntimeStatus || progress.authRegistrationStatus || "pending"}`,
    );
  const visibilityText = tr(
    locale,
    `快照行动体 ${progress.totalAgentCount}，当前可控 ${progress.visibleAgentCount}`,
    `snapshot agents ${progress.totalAgentCount}, visible ${progress.visibleAgentCount}`,
  );
  const lines = [
    tr(locale, `连接：${connectionText}`, `Connection: ${connectionText}`),
    tr(locale, `握手：${handshakeText}`, `Handshake: ${handshakeText}`),
    tr(locale, `快照：${snapshotText}`, `Snapshot: ${snapshotText}`),
    tr(locale, `玩家会话：${sessionText}`, `Player session: ${sessionText}`),
    tr(locale, `可见性：${visibilityText}`, `Visibility: ${visibilityText}`),
  ];
  if (progress.lastError) {
    lines.push(tr(locale, `错误：${progress.lastError}`, `Error: ${progress.lastError}`));
  }
  return lines;
}

function EntityListPendingState(props) {
  const locale = () => props.locale ?? uiLocale();
  const label = () => props.label ?? tr(locale(), "目标", "targets");
  const progress = () => props.progress ?? core.buildTargetSyncProgress();
  const progressLines = () => targetSyncProgressLines(progress(), locale());
  return (
    <div class="entity-list-pending" aria-live="polite" aria-busy="true">
      <div class="entity-list-pending__row">
        <span class="entity-list-pending__spinner" aria-hidden="true" />
        <span>
          {tr(
            locale(),
            `正在同步${label()}…`,
            `Syncing ${label()}…`,
          )}
        </span>
      </div>
      <Show when={progressLines().length > 0}>
        <div class="entity-list-pending__progress">
          <For each={progressLines()}>
            {(line) => <div>{line}</div>}
          </For>
        </div>
      </Show>
      <div class="entity-list-pending__skeleton" aria-hidden="true">
        <span />
        <span />
        <span />
      </div>
    </div>
  );
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
      <div class="stack flow-top">
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

function claimField(value, ...names) {
  if (!value || typeof value !== "object") return null;
  for (const name of names) {
    if (value[name] !== undefined && value[name] !== null) {
      return value[name];
    }
  }
  return null;
}

function compactValue(value) {
  if (value === null || value === undefined || value === "") return "-";
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : "-";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

function claimMoney(value) {
  const amount = claimField(value, "amount", "tokens", "balance", "value");
  const symbol = claimField(value, "symbol", "denom", "currency");
  if (amount !== null) {
    return symbol ? `${amount} ${symbol}` : compactValue(amount);
  }
  return compactValue(value);
}

function claimQuoteRows(quote) {
  if (!quote || typeof quote !== "object") return [];
  return [
    ["Slot", claimField(quote, "slot_index", "slot", "slot_id", "slotId", "claim_slot")],
    ["Reputation tier", claimField(quote, "reputation_tier", "reputationTier", "tier")],
    ["Owned / cap", [
      claimField(quote, "owned_claim_count", "owned", "owned_count", "ownedCount"),
      claimField(quote, "cap", "claim_cap", "claimCap"),
    ].filter((part) => part !== null && part !== undefined).join(" / ")],
    ["Total upfront", claimMoney(claimField(quote, "total_upfront_amount", "total_upfront", "totalUpfront", "upfront_total", "upfrontTotal"))],
    ["Activation fee", claimMoney(claimField(quote, "activation_fee_amount", "activation_fee", "activationFee"))],
    ["Bond", claimMoney(claimField(quote, "claim_bond_amount", "bond", "locked_bond", "lockedBond"))],
    ["Upkeep / epoch", claimMoney(claimField(quote, "upkeep_per_epoch", "upkeepPerEpoch", "upkeep"))],
    ["Eligible balance", claimMoney(claimField(quote, "eligible_claim_balance", "eligible_balance", "eligibleBalance"))],
    ["Liquid balance", claimMoney(claimField(quote, "transferable_liquid_balance", "liquid_balance", "liquidBalance"))],
    ["Restricted starter", claimMoney(claimField(quote, "restricted_starter_claim_balance", "restricted_starter_balance", "restrictedStarterBalance"))],
    ["Auto starter", claimMoney(claimField(quote, "auto_restricted_starter_claim_amount", "auto_starter_amount", "autoStarterAmount"))],
    ["Cooldown", claimField(quote, "release_cooldown_epochs", "cooldown_epochs", "cooldownEpochs", "cooldown")],
    ["Grace", claimField(quote, "grace_epochs", "graceEpochs", "grace")],
    ["Idle warning", claimField(quote, "idle_warning_epochs", "idleWarningEpochs")],
    ["Forced reclaim", claimField(quote, "forced_idle_reclaim_epochs", "forcedIdleReclaimEpochs")],
    ["Penalty bps", claimField(quote, "forced_reclaim_penalty_bps", "forcedReclaimPenaltyBps")],
    ["Reclaim terms", claimField(quote, "reclaim_terms", "reclaimTerms", "reclaim")],
  ].filter(([, value]) => value !== null && value !== undefined && value !== "");
}

const PRIMARY_CLAIM_QUOTE_LABELS = new Set(["Total upfront", "Eligible balance", "Owned / cap"]);

function claimQuoteMetricClass(label) {
  return [
    "metric",
    PRIMARY_CLAIM_QUOTE_LABELS.has(label) ? "metric--claim-primary" : null,
    label === "Total upfront" ? "metric--claim-total" : null,
  ].filter(Boolean).join(" ");
}

function claimTarget(claim) {
  return claimField(claim, "target_agent_id", "targetAgentId", "agent_id", "agentId", "target") || "agent";
}

function claimStatusText(claim) {
  const status = claimField(claim, "status", "claim_status", "claimStatus") || "active";
  const paidThrough = claimField(claim, "upkeep_paid_through_epoch", "upkeepPaidThroughEpoch");
  const grace = claimField(claim, "grace_remaining_epochs", "graceRemainingEpochs", "grace_remaining", "graceRemaining");
  const releaseReadyIn = claimField(claim, "release_ready_in_epochs", "releaseReadyInEpochs");
  const releaseReadyAt = claimField(claim, "release_ready_at_epoch", "releaseReadyAtEpoch");
  const idleWarningIn = claimField(claim, "idle_warning_in_epochs", "idleWarningInEpochs");
  const reclaim = claimField(claim, "forced_reclaim_in_epochs", "forcedReclaimInEpochs", "forced_reclaim_epoch", "forcedReclaimEpoch", "forced_reclaim_at", "forcedReclaimAt");
  return [
    `status=${status}`,
    paidThrough !== null ? `upkeep paid through epoch ${paidThrough}` : null,
    releaseReadyIn !== null ? `release ready in ${releaseReadyIn}` : null,
    releaseReadyAt !== null ? `release ready at epoch ${releaseReadyAt}` : null,
    grace !== null ? `grace remaining ${grace}` : null,
    idleWarningIn !== null ? `idle warning in ${idleWarningIn}` : null,
    reclaim !== null ? `forced reclaim in ${reclaim}` : null,
  ].filter(Boolean).join(" · ");
}

function claimOwnedDetail(claim) {
  const restrictedBond = claimField(claim, "claim_bond_locked_restricted_amount", "lockedBondRestricted");
  const liquidBond = claimField(claim, "claim_bond_locked_liquid_amount", "lockedBondLiquid");
  const restrictedSpent = claimField(claim, "upfront_restricted_spent_amount", "upfrontRestrictedSpent");
  const liquidSpent = claimField(claim, "upfront_liquid_spent_amount", "upfrontLiquidSpent");
  return [
    claimField(claim, "idle_warning", "idleWarning"),
    claimField(claim, "locked_bond_split", "lockedBondSplit"),
    restrictedBond !== null || liquidBond !== null
      ? `bond restricted=${compactValue(restrictedBond)} liquid=${compactValue(liquidBond)}`
      : null,
    restrictedSpent !== null || liquidSpent !== null
      ? `upfront restricted=${compactValue(restrictedSpent)} liquid=${compactValue(liquidSpent)}`
      : null,
  ].filter(Boolean).join(" · ");
}

function releaseClaimActionState(actions) {
  let published = false;
  let disabledReason = null;
  const available = (actions || []).some((action) => {
    const raw = `${action.actionId || ""} ${action.label || ""} ${action.protocolAction || ""}`.toLowerCase();
    const isRelease = raw.includes("release_agent_claim") || raw.includes("release claim") || raw.includes("release_agent");
    if (!isRelease) {
      return false;
    }
    published = true;
    disabledReason = action.disabledReason || disabledReason;
    return !action.disabledReason;
  });
  return { available, published, disabledReason };
}

function expansionBranchCards(gameplay, locale) {
  const goal = String(gameplay?.goalKind || "").toLowerCase();
  if (goal !== "choosefirstexpansiontradeoff" && goal !== "choosemidlooppath") {
    return [];
  }
  const actions = gameplay?.availableActions || [];
  const recommendations = Array.isArray(gameplay?.branchRecommendations)
    ? gameplay.branchRecommendations
    : [];
  if (recommendations.length === 0) {
    return gameplay?.branchHint ? [{ legacy: true }] : [];
  }
  return recommendations.map((recommendation) => {
    const action = actions.find((candidate) => candidate.actionId === recommendation.actionId) || null;
    const complete = [
      recommendation.routeLabel,
      recommendation.immediateGain,
      recommendation.futureBeatChanged,
      recommendation.riskOrLockin,
      recommendation.nextSessionHook,
    ].every((value) => Boolean(String(value || "").trim()));
    return {
      ...recommendation,
      action,
      complete,
    };
  });
}

function ClaimAgentChoiceCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const claim = () => props.claim || {};
  const quote = () => claimField(claim(), "next_claim_quote", "nextClaimQuote", "quote") || {};
  const blockedReason = () => claimField(quote(), "blocked_reason", "blockedReason");
  const ownedClaims = () => {
    const owned = claimField(claim(), "owned_claims", "ownedClaims");
    return Array.isArray(owned) ? owned : [];
  };
  const releaseActionState = () => releaseClaimActionState(props.availableActions || []);
  return (
    <PanelSection
      title={tr(locale(), "Claim-Agent Choice", "Claim-Agent Choice")}
      eyebrow={tr(locale(), "占用 / 维护 / 释放", "Claim / Maintain / Release")}
      meta={tr(locale(), "只展示现有 claim 快照与已发布动作；这里不会新增转账或 claim 规则。", "Shows only the current claim snapshot and published actions; this adds no transfer UI or claim rules.")}
    >
      <div class="badge-row">
        <Badge class={blockedReason() ? "badge badge--warn" : "badge badge--good"}>
          {blockedReason()
            ? tr(locale(), "暂缓 claim", "Wait before claiming")
            : tr(locale(), "claim 条件可读", "Claim readable")}
        </Badge>
        <Badge>{`owned=${ownedClaims().length}`}</Badge>
      </div>
      <div class="feedback-summary">
        {blockedReason()
          ? tr(locale(), "下一次 claim 需要先等待、补资金或提升资格；原始原因已收在诊断明细。", "The next claim needs waiting, funding, or eligibility first; the raw reason is kept in diagnostic detail.")
          : tr(locale(), "当前 quote 没有发布阻塞原因；玩家可以把它当成“可比较但仍需按正式动作执行”的 claim 机会。", "The current quote publishes no blocker reason; treat it as a comparable claim opportunity that still needs a canonical action to execute.")}
      </div>
      <div class="summary-grid">
        <For each={claimQuoteRows(quote())}>
          {([label, value]) => (
            <MetricCard class={claimQuoteMetricClass(label)} label={label} value={compactValue(value)} />
          )}
        </For>
      </div>
      <Show when={blockedReason()}>
        <DiagnosticDetails
          locale={locale()}
          label={tr(locale(), "Claim 阻塞诊断", "Claim blocker diagnostics")}
          value={() => ({ blocked_reason: blockedReason(), quote: quote() })}
        />
      </Show>
      <Show when={ownedClaims().length > 0}>
        <div>
          <div class="panel__title panel__title--spaced">{tr(locale(), "已占用 Agent", "Owned Claims")}</div>
          <div class="event-list">
            <For each={ownedClaims()}>
              {(owned) => (
                <EventCard
                  title={claimTarget(owned)}
                  badge={claimField(owned, "release_ready", "releaseReady") || claimField(owned, "release_ready_in_epochs", "releaseReadyInEpochs") === 0 || claimField(owned, "status") === "release_ready" ? "release ready" : claimField(owned, "release_cooldown", "releaseCooldown") ? "cooldown" : "maintain"}
                  badgeClass={claimField(owned, "release_ready", "releaseReady") || claimField(owned, "release_ready_in_epochs", "releaseReadyInEpochs") === 0 || claimField(owned, "status") === "release_ready" ? "badge badge--accent" : "badge"}
                  meta={claimStatusText(owned)}
                >
                  <div class="feedback-summary">
                    {releaseActionState().available
                      ? tr(locale(), "Release 已作为正式可用动作发布；可以从可用动作列表执行。", "Release is published as a canonical available action; execute it from the available actions list.")
                      : releaseActionState().published
                        ? tr(locale(), "Release 动作已经发布但当前不可执行；先处理可用动作列表里的阻塞原因。", "Release is published but currently disabled; resolve the blocker shown in the available actions list first.")
                      : tr(locale(), "维护方式是保持控制权与 upkeep 健康；release 只作为状态指导，直到正式动作发布。", "Maintain by keeping control and upkeep healthy; release stays guidance-only until a canonical action is published.")}
                  </div>
                  <Show when={claimOwnedDetail(owned)}>
                    <div class="feedback-detail">{claimOwnedDetail(owned)}</div>
                  </Show>
                </EventCard>
              )}
            </For>
          </div>
        </div>
      </Show>
    </PanelSection>
  );
}

function ExpansionTradeoffCards(props) {
  const locale = () => props.locale ?? uiLocale();
  const cards = () => expansionBranchCards(props.gameplay, locale());
  const legacyOnly = () => cards().length === 1 && cards()[0].legacy;
  return (
    <PanelSection
      title={tr(locale(), "扩张取舍", "Expansion Tradeoffs")}
      eyebrow={legacyOnly()
        ? tr(locale(), "旧版 / 不完整", "Legacy / Incomplete")
        : tr(locale(), "运行时推荐", "Runtime Recommendations")}
      meta={props.gameplay?.branchHint || tr(locale(), "当前分支提示尚未发布。", "No branch premise is published yet.")}
    >
      <Show
        when={!legacyOnly()}
        fallback={(
          <div class="feedback-summary">
            {tr(locale(), "结构化分支推荐不可用；此处仅保留旧版提示，不会从动作文本合成取舍字段。", "Structured branch recommendations are unavailable; only the legacy hint is shown, and no tradeoff fields are synthesized from action text.")}
          </div>
        )}
      >
        <div class="action-grid">
          <For each={cards()}>
            {(card) => (
            <EventCard
              class="event-card event-card--action"
              title={card.routeLabel || tr(locale(), "未命名路线", "Unnamed route")}
              badge={card.action
                ? card.action.disabledReason
                  ? tr(locale(), "暂不可用", "unavailable")
                  : tr(locale(), "可执行", "actionable")
                : tr(locale(), "动作未发布", "action unpublished")}
              badgeClass={card.action && !card.action.disabledReason ? "badge badge--good" : "badge badge--warn"}
              meta={props.gameplay?.goalTitle || tr(locale(), "当前扩张目标", "Current expansion goal")}
            >
              <Show when={!card.complete}>
                <div class="badge-row"><Badge class="badge badge--warn">{tr(locale(), "推荐信息不完整", "Incomplete recommendation")}</Badge></div>
              </Show>
              <div class="feedback-detail">
                <div class="metric__label">{tr(locale(), "即时收益", "Immediate gain")}</div>
                {card.immediateGain || tr(locale(), "即时收益未发布", "Immediate gain unavailable")}
              </div>
              <div class="feedback-detail">
                <div class="metric__label">{tr(locale(), "后续变化", "Future beat")}</div>
                {card.futureBeatChanged || tr(locale(), "后续变化未发布", "Future beat unavailable")}
              </div>
              <div class="feedback-detail">
                <div class="metric__label">{tr(locale(), "风险或锁定", "Risk or lock-in")}</div>
                {card.riskOrLockin || tr(locale(), "风险或锁定未发布", "Risk or lock-in unavailable")}
              </div>
              <div class="feedback-detail">
                <div class="metric__label">{tr(locale(), "下次续玩钩子", "Next-session hook")}</div>
                {card.nextSessionHook || tr(locale(), "下次续玩钩子未发布", "Next-session hook unavailable")}
              </div>
              <div class="feedback-summary">
                {card.action
                  ? card.action.disabledReason
                    ? `${card.action.label || card.action.actionId}: ${card.action.disabledReason}`
                    : card.action.label || card.action.actionId
                  : card.actionId || tr(locale(), "关联动作未发布", "Linked action unpublished")}
              </div>
            </EventCard>
            )}
          </For>
        </div>
      </Show>
    </PanelSection>
  );
}

function InlineHelpTip(props) {
  const locale = () => props.locale ?? uiLocale();
  const [isOpen, setIsOpen] = createSignal(false);
  let rootRef;

  onMount(() => {
    const handlePointerDown = (event) => {
      if (!rootRef?.contains(event.target)) {
        setIsOpen(false);
      }
    };
    const handleKeyDown = (event) => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };
    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    });
  });

  return (
    <div ref={rootRef} class="inline-help-tip" data-open={isOpen() ? "true" : "false"}>
      <button
        type="button"
        class="inline-help-tip__button"
        aria-label={props.label ?? tr(locale(), "打开比例说明", "Open scale guidance")}
        aria-describedby={props.id}
        aria-expanded={isOpen() ? "true" : "false"}
        aria-controls={props.id}
        onClick={() => setIsOpen((value) => !value)}
      >
        ?
      </button>
      <div id={props.id} class="inline-help-tip__panel" aria-hidden={isOpen() ? "false" : "true"}>
        <div class="inline-help-tip__title">{props.title ?? tr(locale(), "比例说明", "Scale Guidance")}</div>
        <div class="inline-help-tip__body">
          <For each={props.lines ?? []}>
            {(line) => <div class="feedback-detail">{line}</div>}
          </For>
        </div>
      </div>
    </div>
  );
}

function FeedbackCard(props) {
  const feedbackStage = () => normalizedFeedbackStage(props.feedbackStage);
  return (
    <div
      class="feedback-card"
      data-feedback-stage={feedbackStage()}
      role={props.liveRegion ? "status" : undefined}
      aria-live={props.liveRegion ? "polite" : undefined}
    >
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

const {
  agentBindingForId,
  agentClaimUsesCurrentBoundAgent,
  buildAgentClaimAction,
  buildAgentClaimTargets,
  describeAgentSessionStatus,
  hasAgentClaimSessionBoundary,
  hasExecutableAgentClaim,
  normalizedId,
} = createViewerAgentClaimDisplayModel({ state: core.state, tr });

function normalizedFeedbackStage(stage) {
  const value = String(stage || "").trim().toLowerCase();
  if (["ack", "sent", "queued", "completed", "blocked", "rejected", "error"].includes(value)) {
    return value;
  }
  return undefined;
}

function MetricCard(props) {
  return (
    <div class={props.class ?? "metric"}>
      <div class="metric__label">{props.label}</div>
      <div class="metric__value">{props.value}</div>
      <Show when={props.detail}>
        <div class="feedback-detail flow-top--tight">{props.detail}</div>
      </Show>
      <Show when={props.children}>
        <div class="badge-row badge-row--tight">
          {props.children}
        </div>
      </Show>
    </div>
  );
}

function EventCard(props) {
  return (
    <div class={props.class ?? "event-card"} data-action-state={props.actionState}>
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
        <div class="stack stack--compact">
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
    <div
      class={`callout ${props.variant === "warn" ? "callout--warn" : ""} ${props.class ?? ""}`}
      data-callout-kind={props.kind ?? ""}
    >
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

function HostedLoginForm(props) {
  const locale = () => props.locale ?? uiLocale();
  const clearHostedLoginError = () => {
    if (core.state.hostedLogin.error != null || core.state.hostedLogin.retryAfterSeconds != null) {
      core.state.hostedLogin.error = null;
      core.state.hostedLogin.retryAfterSeconds = null;
      core.requestRender();
    }
  };
  return (
    <div class="stack">
      <div class="control-grid">
        <div class="field">
          <label for={props.handleId ?? "hosted-login-handle"}>
            {tr(locale(), "邮箱", "Email")}
          </label>
          <input
            id={props.handleId ?? "hosted-login-handle"}
            type="email"
            autocomplete="email"
            value={core.state.hostedLogin.handle}
            onInput={(event) => {
              core.state.hostedLogin.handle = String(event.currentTarget.value || "");
              clearHostedLoginError();
            }}
          />
        </div>
      </div>
      <div class="toolbar">
        <button
          data-auth-action="start-login"
          disabled={core.state.hostedLogin.startInFlight}
          onClick={() => {
            void core.startHostedAccountLogin();
          }}
        >
          {tr(locale(), "请求登录验证码", "Request Login Code")}
        </button>
      </div>
      <Show when={core.state.hostedLogin.challengeId}>
        <div class="badge-row">
          <Badge>{`challenge=${core.state.hostedLogin.challengeId}`}</Badge>
          <Badge>{`target=${core.state.hostedLogin.maskedLoginHint || "-"}`}</Badge>
          <Badge>{`delivery=${core.state.hostedLogin.deliveryMode || "-"}`}</Badge>
          <Badge>{core.state.hostedLogin.accountExists ? "account=existing" : "account=new"}</Badge>
        </div>
        <div class="field">
          <label for={props.codeId ?? "hosted-login-code"}>
            {tr(locale(), "验证码", "Verification Code")}
          </label>
          <input
            id={props.codeId ?? "hosted-login-code"}
            type="text"
            autocomplete="off"
            value={core.state.hostedLogin.code}
            onInput={(event) => {
              core.state.hostedLogin.code = String(event.currentTarget.value || "");
              clearHostedLoginError();
            }}
          />
        </div>
        <div class="toolbar">
          <button
            data-auth-action="complete-login"
            disabled={core.state.hostedLogin.completeInFlight || core.state.auth.issueInFlight}
            onClick={() => {
              void core.completeHostedAccountLogin();
            }}
          >
            {tr(locale(), "登录并领取玩家会话", "Sign In and Acquire Player Session")}
          </button>
        </div>
      </Show>
      <Show when={core.state.hostedLogin.error}>
        <div class="stack">
          <EmptyState>{core.state.hostedLogin.error}</EmptyState>
          <Show when={core.state.hostedLogin.retryAfterSeconds != null}>
            <div class="badge-row">
              <Badge>{`retry_after=${core.state.hostedLogin.retryAfterSeconds}s`}</Badge>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}

function shouldShowHostedLoginGate() {
  return !core.state.auth.available
    && isHostedPublicJoinDeploymentMode(core.state.hostedAccess?.deployment_mode);
}

function focusableElements(root) {
  return [...root.querySelectorAll(
    [
      "a[href]",
      "button:not([disabled])",
      "input:not([disabled])",
      "select:not([disabled])",
      "textarea:not([disabled])",
      "[tabindex]:not([tabindex='-1'])",
    ].join(","),
  )].filter((element) => !element.hasAttribute("aria-hidden"));
}

function HostedLoginGate() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  let dialogRef;
  let previousFocus = null;

  onMount(() => {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    queueMicrotask(() => {
      const firstFocusable = dialogRef ? focusableElements(dialogRef)[0] : null;
      (firstFocusable || dialogRef)?.focus();
    });
  });

  onCleanup(() => {
    if (previousFocus && document.contains(previousFocus)) {
      previousFocus.focus();
    }
  });

  const trapDialogFocus = (event) => {
    if (event.key !== "Tab" || !dialogRef) {
      return;
    }
    const focusables = focusableElements(dialogRef);
    if (focusables.length === 0) {
      event.preventDefault();
      dialogRef.focus();
      return;
    }
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <Show when={shouldShowHostedLoginGate()}>
      <div
        class="auth-gate"
        data-viewer-fixture-state="hosted_login_gate"
        role="dialog"
        aria-modal="true"
        aria-labelledby="hosted-login-gate-title"
        tabIndex="-1"
        ref={dialogRef}
        onKeyDown={trapDialogFocus}
      >
        <div class="auth-gate__dialog">
          <div class="auth-gate__header">
            <div>
              <div class="panel__eyebrow">{tr(locale(), "标准用户流程", "Standard User Flow")}</div>
              <h1 id="hosted-login-gate-title" class="auth-gate__title">
                {tr(locale(), "登录邮箱后进入游戏", "Sign In With Email")}
              </h1>
            </div>
            <Badge class="badge badge--warn">auth=missing</Badge>
          </div>
          <div class="feedback-summary">
            {tr(
              locale(),
              "当前是托管公开加入模式。先领取玩家会话，再进入聊天、玩法动作和后续授权。",
              "This is hosted public join. Acquire a player session first, then continue to chat, gameplay actions, and later authorization.",
            )}
          </div>
          <HostedLoginForm
            locale={locale()}
            handleId="gate-hosted-login-handle"
            codeId="gate-hosted-login-code"
          />
          <Show when={core.state.auth.rebindNotice || core.state.auth.error}>
            <EmptyState>{core.state.auth.rebindNotice || core.state.auth.error}</EmptyState>
          </Show>
        </div>
      </div>
    </Show>
  );
}

function EmptyEntityRecoveryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const gameplay = () => (typeof props.gameplay === "function" ? props.gameplay() : props.gameplay);
  const firstAgentClaimAction = () =>
    (gameplay()?.availableActions || []).find((action) => action.actionId === "claim_first_agent");
  const firstAgentClaimDisabledReason = () =>
    gameplayActionDisabledReason(firstAgentClaimAction(), gameplay(), locale());

  return (
    <CalloutCard
      class="empty-entity-recovery"
      kind="empty_world_recovery"
      title={props.title ?? tr(locale(), "认领第一个 Agent", "Claim Your First Agent")}
      badge={gameplay()?.blockerKind || "blocked"}
      badgeClass={firstAgentClaimAction() && !firstAgentClaimDisabledReason() ? "badge badge--good" : "badge badge--warn"}
      variant={firstAgentClaimAction() && !firstAgentClaimDisabledReason() ? null : "warn"}
    >
      <div class="feedback-summary">
        {firstAgentClaimDisabledReason()
          ? firstAgentClaimDisabledReason()
          : firstAgentClaimAction()
          ? tr(
              locale(),
              "这是新用户入口：当前还没有可玩实体，先用正式玩法动作认领你的第一个 Agent。",
              "This is the new-user entry: there are no playable entities yet, so claim your first Agent through the canonical gameplay action.",
            )
          : gameplay()?.blockerDetail
            || tr(
              locale(),
              "运行时已发布玩法摘要，但当前快照还没有可选行动体或地点。",
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
      <Show when={firstAgentClaimAction()}>
        {(action) => (
          <div class="toolbar">
            <button
              class={gameplayActionButtonClass(action())}
              aria-busy={gameplayActionButtonBusyAttrs(action())}
              disabled={gameplayActionButtonDisabled(action(), gameplay(), locale())}
              onClick={() => renderGameplayAction(action())}
            >
              {gameplayActionDisplayLabel(action(), locale())}
            </button>
          </div>
        )}
      </Show>
      <div class="feedback-detail">
        {firstAgentClaimAction()
          ? tr(
              locale(),
              "认领提交后等待链上提交与快照同步；同步完成后第一个 Agent 会出现在世界里。",
              "After submitting the claim, wait for chain submission and snapshot sync; the first Agent appears once the committed world updates.",
            )
          : tr(
              locale(),
              "如果中间栏仍保留“刷新快照”动作，先从那里重拉一次；如果数量仍然是 0，就需要修复或重启运行时世界引导流程。",
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
          <div class="panel__title panel__title--spaced">
            {tr(locale(), "语言与观察器入口", "Language and Viewer Entry")}
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

function gameplayStageToneClass(status) {
  return status === "blocked"
    ? "hero-focus-card__value hero-focus-card__value--warn"
    : status === "branch_ready"
      ? "hero-focus-card__value hero-focus-card__value--good"
      : "hero-focus-card__value hero-focus-card__value--accent";
}

function gameplayStageLabel(status, locale) {
  if (status === "blocked") {
    return tr(locale, "当前受阻", "Blocked Now");
  }
  if (status === "branch_ready") {
    return tr(locale, "可以推进", "Ready to Act");
  }
  if (status === "active") {
    return tr(locale, "正在推进", "In Motion");
  }
  if (status === "completed") {
    return tr(locale, "阶段完成", "Stage Complete");
  }
  return status || tr(locale, "等待同步", "Waiting for Sync");
}

function goalExecutionBadgeClass(state) {
  return state === "blocked" || state === "rejected"
    ? "badge badge--warn"
    : state === "completed"
      ? "badge badge--good"
      : "badge badge--accent";
}

const PENDING_GAMEPLAY_FEEDBACK_STAGES = new Set(["accepted", "submitted", "queued", "ack", "registering", "signing", "sent"]);
const GAMEPLAY_ACTION_BUSY_STAGES = new Set(["queued", "registering", "signing", "sent"]);
const GAMEPLAY_ACTION_PENDING_MIN_MS = 900;
let gameplayActionPendingClearTimer = null;

function gameplayActionKey(action) {
  if (!action) {
    return "";
  }
  const actionId = normalizedId(action.actionId || action.action_id || action.protocolAction || action.protocol_action || action.executeKind);
  const targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id || action.actorAgentId || action.actor_agent_id);
  return `${actionId}::${targetAgentId}`;
}

function gameplayActionBlockedReasonId(action) {
  const key = gameplayActionKey(action).replace(/[^a-zA-Z0-9_-]+/g, "-");
  return `gameplay-action-${key || "unknown"}-blocked-reason`;
}

function gameplayActionFeedbackMatches(action, feedback = core.snapshotSemanticFeedback(core.state.lastGameplayActionFeedback)) {
  if (!action || !feedback || feedback.kind !== "gameplay_action") {
    return false;
  }
  const actionId = normalizedId(action.actionId || action.action_id || action.protocolAction || action.protocol_action || action.executeKind);
  const feedbackAction = normalizedId(feedback.action);
  if (!actionId || !feedbackAction || !feedbackAction.includes(actionId)) {
    return false;
  }
  const targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id || action.actorAgentId || action.actor_agent_id);
  const feedbackAgentId = normalizedId(feedback.agentId || feedback.targetAgentId);
  return !targetAgentId || !feedbackAgentId || targetAgentId === feedbackAgentId;
}

function clearGameplayActionPending(action = null) {
  if (gameplayActionPendingClearTimer != null) {
    window.clearTimeout(gameplayActionPendingClearTimer);
    gameplayActionPendingClearTimer = null;
  }
  if (action && core.state.gameplayActionPending.actionKey !== gameplayActionKey(action)) {
    return;
  }
  core.state.gameplayActionPending.actionKey = null;
  core.state.gameplayActionPending.label = null;
  core.state.gameplayActionPending.startedAtUnixMs = null;
  core.requestRender();
}

function markGameplayActionPending(action, label) {
  const key = gameplayActionKey(action);
  if (!key) {
    return;
  }
  if (gameplayActionPendingClearTimer != null) {
    window.clearTimeout(gameplayActionPendingClearTimer);
  }
  core.state.gameplayActionPending.actionKey = key;
  core.state.gameplayActionPending.label = label || normalizedId(action.label || action.actionId || action.executeKind);
  core.state.gameplayActionPending.startedAtUnixMs = Date.now();
  gameplayActionPendingClearTimer = window.setTimeout(() => {
    gameplayActionPendingClearTimer = null;
    if (!gameplayActionFeedbackMatches(action) || !GAMEPLAY_ACTION_BUSY_STAGES.has(normalizedId(core.state.lastGameplayActionFeedback?.stage).toLowerCase())) {
      clearGameplayActionPending(action);
    }
  }, GAMEPLAY_ACTION_PENDING_MIN_MS);
  core.requestRender();
}

function gameplayActionFeedbackInFlight(action) {
  const feedback = core.snapshotSemanticFeedback(core.state.lastGameplayActionFeedback);
  return gameplayActionFeedbackMatches(action, feedback)
    && GAMEPLAY_ACTION_BUSY_STAGES.has(normalizedId(feedback.stage).toLowerCase());
}

function gameplayActionPendingFor(action) {
  const key = gameplayActionKey(action);
  return Boolean(key && core.state.gameplayActionPending.actionKey === key) || gameplayActionFeedbackInFlight(action);
}

function isPendingFirstAgentClaimSync(action, gameplay) {
  if (action?.actionId !== "claim_first_agent") {
    return false;
  }
  if (gameplay?.blockerKind !== "runtime_snapshot_empty_entities") {
    return false;
  }
  const feedback = gameplay?.recentFeedback || core.snapshotSemanticFeedback(core.state.lastGameplayActionFeedback);
  const feedbackAction = String(feedback?.action || "").trim();
  const feedbackStage = String(feedback?.stage || "").trim().toLowerCase();
  return feedbackAction.includes("claim_first_agent") && PENDING_GAMEPLAY_FEEDBACK_STAGES.has(feedbackStage);
}

function gameplayActionControlBoundaryReason(action, locale) {
  const actionId = normalizedId(action?.actionId || action?.action_id);
  const protocolAction = normalizedId(action?.protocolAction || action?.protocol_action);
  const targetAgentId = normalizedId(action?.targetAgentId || action?.target_agent_id);
  if (!targetAgentId || actionId === "claim_first_agent") {
    return null;
  }
  if (protocolAction === "request_snapshot" || protocolAction === "live_control.step" || protocolAction === "live_control.play") {
    return null;
  }
  const boundAgentId = normalizedId(core.state.auth.boundAgentId);
  if (!boundAgentId) {
    return tr(
      locale,
      "当前账号尚未绑定 Agent；这个动作来自共享世界快照，不能作为当前账号动作执行。",
      "The current account has no bound Agent; this action came from the shared world snapshot and cannot be executed as the current account.",
    );
  }
  if (targetAgentId !== boundAgentId) {
    return tr(
      locale,
      `这个动作目标是 ${targetAgentId}，但当前账号绑定的是 ${boundAgentId}。`,
      `This action targets ${targetAgentId}, but the current account is bound to ${boundAgentId}.`,
    );
  }
  return null;
}

function gameplayActionDisabledReason(action, gameplay, locale) {
  if (action?.disabledReason) {
    return action.disabledReason;
  }
  if (isPendingFirstAgentClaimSync(action, gameplay)) {
    return tr(
      locale,
      "认领已提交，正在等待链上 committed 快照同步。",
      "Claim submitted; waiting for the committed chain snapshot to sync.",
    );
  }
  const controlBoundaryReason = gameplayActionControlBoundaryReason(action, locale);
  if (controlBoundaryReason) {
    return controlBoundaryReason;
  }
  return null;
}

function gameplayActionButtonLabel(action, locale) {
  if (action.actionId === "claim_first_agent") {
    return tr(locale, "认领第一个 Agent", "Claim First Agent");
  }
  if (action.actionId === "claim_starter_oc") {
    return tr(locale, "领取初始 OC", "Claim Starter OC");
  }
  if (action.executeKind === "claim_agent") {
    return tr(locale, "认领 Agent", "Claim Agent");
  }
  if (action.executeKind === "request_snapshot") {
    return tr(locale, "刷新快照", "Refresh Snapshot");
  }
  if (action.executeKind === "step") {
    return tr(locale, "推进一步", "Advance One Step");
  }
  if (action.executeKind === "play") {
    return tr(locale, "恢复实时推进", "Resume Live Play");
  }
  if (action.executeKind === "agent_chat") {
    return tr(locale, "切到聊天面板", "Use Chat Panel");
  }
  return tr(locale, "提交玩法动作", "Submit Gameplay Action");
}

function gameplayActionBusyLabel(action, locale) {
  if (action?.executeKind === "request_snapshot") {
    return tr(locale, "刷新中...", "Refreshing...");
  }
  if (action?.executeKind === "step") {
    return tr(locale, "推进中...", "Advancing...");
  }
  if (action?.executeKind === "play") {
    return tr(locale, "恢复中...", "Resuming...");
  }
  if (action?.actionId === "claim_starter_oc") {
    return tr(locale, "确认中...", "Confirming...");
  }
  if (action?.actionId === "claim_first_agent" || action?.executeKind === "claim_agent") {
    return tr(locale, "提交中...", "Submitting...");
  }
  return tr(locale, "处理中...", "Working...");
}

function gameplayActionDisplayLabel(action, locale, fallback = null) {
  if (gameplayActionPendingFor(action)) {
    return gameplayActionBusyLabel(action, locale);
  }
  return fallback ?? gameplayActionButtonLabel(action, locale);
}

function gameplayActionButtonClass(action) {
  return gameplayActionPendingFor(action) ? "is-loading" : "";
}

function gameplayActionButtonBusyAttrs(action) {
  return gameplayActionPendingFor(action) ? "true" : "false";
}

function gameplayActionButtonDisabled(action, gameplay, locale) {
  return Boolean(gameplayActionDisabledReason(action, gameplay, locale) || gameplayActionPendingFor(action));
}

function gameplayActionTestId(action, role = "available") {
  if (role === "recommended") {
    return "viewer-playthrough-action-recommended";
  }
  if (action?.executeKind === "request_snapshot") {
    return "viewer-available-action-request-snapshot";
  }
  if (action?.executeKind === "step") {
    return "viewer-available-action-step";
  }
  if (action?.executeKind === "play") {
    return "viewer-available-action-play";
  }
  const raw = action?.actionId || action?.protocolAction || action?.executeKind || "unknown";
  const safe = String(raw).trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "unknown";
  return `viewer-playthrough-action-${safe}`;
}

function gameplayActionDetail(action, gameplay, locale) {
  if (action?.actionId === "claim_first_agent") {
    return action?.disabledReason
      || tr(locale, "新用户空世界入口：提交后会创建并绑定第一个 starter Agent。", "New-user empty-world entry: submitting creates the first starter Agent.");
  }
  if (action?.actionId === "claim_starter_oc") {
    return action?.disabledReason
      || tr(locale, "领取一次性初始 OC，解锁第一次 LLM/Agent chat。", "Claim one-time starter OC to unlock the first LLM/Agent chat.");
  }
  return action?.playerDetail
    || action?.disabledReason
    || gameplay?.economicSurface?.repairAction
    || gameplay?.narrativeNextStep
    || tr(locale, "可以直接从正式网页入口执行。", "Playable directly from the formal Web entry.");
}

function starterOcAction(gameplay) {
  if (starterOcOnboardingCompletedForCurrentAgent()) {
    return null;
  }
  const pendingTargetAgentId = normalizedId(starterOcOnboardingState.targetAgentId);
  if (starterOcOnboardingState.pending && pendingTargetAgentId) {
    return null;
  }
  const existing = (gameplay?.availableActions || []).find((action) => action.actionId === "claim_starter_oc" && !action.disabledReason);
  if (existing) {
    return existing;
  }
  return null;
}

const starterOcOnboardingState = {
  pending: false,
  targetAgentId: null,
  completedTargetAgentId: null,
};
const [starterOcOnboardingRevision, setStarterOcOnboardingRevision] = createSignal(0);
let starterOcBackgroundConfirmTimer = null;

function touchStarterOcOnboardingState() {
  setStarterOcOnboardingRevision((value) => value + 1);
}

function markStarterOcClaimPending(action) {
  if (action?.actionId !== "claim_starter_oc") {
    return;
  }
  starterOcOnboardingState.pending = true;
  starterOcOnboardingState.targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id);
  starterOcOnboardingState.completedTargetAgentId = null;
  touchStarterOcOnboardingState();
}

function scheduleStarterOcBackgroundConfirmation() {
  if (starterOcBackgroundConfirmTimer != null) {
    window.clearTimeout(starterOcBackgroundConfirmTimer);
  }
  starterOcBackgroundConfirmTimer = window.setTimeout(() => {
    const refreshAction = (core.buildGameplaySummary(uiLocale()).availableActions || [])
      .find((action) => action.executeKind === "request_snapshot");
    if (refreshAction) {
      core.sendGameplayAction(refreshAction);
    }
    starterOcBackgroundConfirmTimer = null;
  }, 450);
}

function clearStarterOcClaimPending() {
  starterOcOnboardingState.pending = false;
  starterOcOnboardingState.targetAgentId = null;
  touchStarterOcOnboardingState();
}

function completeStarterOcOnboarding() {
  starterOcOnboardingState.completedTargetAgentId = normalizedId(
    starterOcOnboardingState.targetAgentId || core.state.auth.boundAgentId,
  );
  clearStarterOcClaimPending();
  touchStarterOcOnboardingState();
}

export function __markStarterOcOnboardingCompleteForTest(agentId = core.state.auth.boundAgentId) {
  starterOcOnboardingState.pending = false;
  starterOcOnboardingState.targetAgentId = null;
  starterOcOnboardingState.completedTargetAgentId = normalizedId(agentId);
  touchStarterOcOnboardingState();
}

function starterOcClaimPendingForCurrentAgent() {
  starterOcOnboardingRevision();
  if (!starterOcOnboardingState.pending) {
    return false;
  }
  const targetAgentId = normalizedId(starterOcOnboardingState.targetAgentId);
  return Boolean(targetAgentId && targetAgentId === normalizedId(core.state.auth.boundAgentId));
}

function starterOcOnboardingCompletedForCurrentAgent() {
  starterOcOnboardingRevision();
  const completedTargetAgentId = normalizedId(starterOcOnboardingState.completedTargetAgentId);
  return Boolean(
    completedTargetAgentId
      && completedTargetAgentId === normalizedId(core.state.auth.boundAgentId),
  );
}

function starterOcCreditVisibleForCurrentAgent() {
  const agentId = normalizedId(core.state.auth.boundAgentId);
  if (!agentId) {
    return false;
  }
  const snapshot = core.state.snapshot || {};
  const model = snapshot.model || {};
  const runtimeState = model.state || snapshot.state || model;
  const starterOcClaim = runtimeState.starter_oc_claims?.[agentId]
    || runtimeState.starterOcClaims?.[agentId]
    || model.starter_oc_claims?.[agentId]
    || model.starterOcClaims?.[agentId]
    || snapshot.starter_oc_claims?.[agentId]
    || snapshot.starterOcClaims?.[agentId]
    || null;
  if (starterOcClaim) {
    return true;
  }
  const balance = runtimeState.main_token_balances?.[agentId]
    || runtimeState.mainTokenBalances?.[agentId]
    || model.main_token_balances?.[agentId]
    || model.mainTokenBalances?.[agentId]
    || snapshot.main_token_balances?.[agentId]
    || snapshot.mainTokenBalances?.[agentId]
    || null;
  const liquidBalance = Number(claimField(balance, "liquid_balance", "liquidBalance", "liquid", "balance") || 0);
  return Number.isFinite(liquidBalance) && liquidBalance > 0;
}

function rawStarterOcActionAvailable() {
  return (core.state.snapshot?.player_gameplay?.available_actions || [])
    .some((action) => action?.action_id === "claim_starter_oc");
}

function starterOcSubmittedFeedback() {
  if (starterOcOnboardingCompletedForCurrentAgent() || starterOcCreditVisibleForCurrentAgent()) {
    return null;
  }
  const feedback = core.state.lastGameplayActionFeedback;
  if (
    feedback?.kind === "gameplay_action"
    && String(feedback?.action || "").includes("claim_starter_oc")
    && (feedback?.accepted || feedback?.stage === "ack" || feedback?.stage === "sent")
  ) {
    return feedback;
  }
  const runtimeFeedback = core.state.snapshot?.player_gameplay?.recent_feedback;
  if (
    String(runtimeFeedback?.action || "").includes("claim_starter_oc")
    && ["accepted", "queued", "sent"].includes(String(runtimeFeedback?.stage || ""))
  ) {
    return runtimeFeedback;
  }
  return null;
}

function starterOcFeedbackNeedsLocalAdvance(feedback = starterOcSubmittedFeedback()) {
  if (!feedback) {
    return false;
  }
  const stage = String(feedback.stage || "").toLowerCase();
  if (stage === "submitted") {
    return false;
  }
  const effect = String(feedback.effect || "").toLowerCase();
  return ["accepted", "ack", "queued", "sent"].includes(stage)
    || effect.includes("queued gameplay action")
    || effect.includes("advance");
}

function shouldShowStarterOcRequiredGate(gameplay = core.buildGameplaySummary(uiLocale())) {
  return Boolean(starterOcAction(gameplay) || starterOcSubmittedFeedback() || starterOcClaimPendingForCurrentAgent());
}

function visibleGameplayActionsForPanels(gameplay) {
  const actions = Array.isArray(gameplay?.availableActions) ? gameplay.availableActions : [];
  if (!shouldShowStarterOcRequiredGate(gameplay)) {
    return actions;
  }
  return actions.filter((action) => action.actionId !== "claim_starter_oc");
}

function gameplayProgressionAction(gameplay) {
  return (gameplay?.availableActions || []).find((action) => action.executeKind === "step")
    || (gameplay?.availableActions || []).find((action) => action.executeKind === "request_snapshot")
    || null;
}

function firstAgentChatAction(gameplay) {
  return (gameplay?.availableActions || []).find((action) => action.executeKind === "agent_chat" && !gameplayActionDisabledReason(action, gameplay, uiLocale()))
    || null;
}

function StarterOcGuide(props) {
  const locale = () => props.locale;
  return (
    <div class="stack stack--compact">
      <div class="feedback-summary">
        {tr(
          locale(),
          "等待同步时不用空等：先了解下一步。第一笔 OC 是新手启动资金，入账后会解锁第一次 Agent 聊天和早期玩法操作。",
          "Do not idle through sync: learn the next step now. The first OC is starter budget; once credited, it unlocks the first Agent chat and early gameplay actions.",
        )}
      </div>
      <div class="summary-grid">
        <div class="metric">
          <div class="metric__label">{tr(locale(), "第一笔 OC", "First OC")}</div>
          <div class="metric__value">{tr(locale(), "新手启动资金", "Starter budget")}</div>
        </div>
        <div class="metric">
          <div class="metric__label">{tr(locale(), "用途", "Use")}</div>
          <div class="metric__value">{tr(locale(), "解锁 Agent 聊天", "Unlock Agent chat")}</div>
        </div>
        <div class="metric">
          <div class="metric__label">{tr(locale(), "玩法目标", "Play Goal")}</div>
          <div class="metric__value">{tr(locale(), "指挥 Agent 恢复产线", "Guide the Agent")}</div>
        </div>
      </div>
    </div>
  );
}

function StarterOcOnboardingPanel(props) {
  const locale = () => props.locale;
  const gameplay = () => props.gameplay;
  const action = () => starterOcAction(gameplay());
  const waitingForFirstAgent = () => Boolean(props.waitingForFirstAgent);
  const hideActionButton = () => Boolean(props.hideActionButton);
  return (
    <div class="stack stack--compact">
      <StarterOcGuide locale={locale()} />
      <Show when={!hideActionButton()}>
        <Show
          when={action()}
          fallback={(
            <div class="feedback-detail">
              {waitingForFirstAgent()
                ? tr(
                  locale(),
                  "当前还在等第一个 Agent 写入 committed 快照；OC 按钮会在 Agent 同步后自动出现。",
                  "The first Agent is still waiting for the committed snapshot; the OC button appears automatically after the Agent syncs.",
                )
                : tr(
                  locale(),
                  "如果聊天提示 OC 不足，回到这里领取初始 OC。",
                  "If chat says OC is missing, return here to claim starter OC.",
                )}
            </div>
          )}
        >
          {(starterAction) => (
            <div class="toolbar">
              <button
                class={gameplayActionButtonClass(starterAction())}
                aria-busy={gameplayActionButtonBusyAttrs(starterAction())}
                disabled={gameplayActionButtonDisabled(starterAction(), gameplay(), locale())}
                onClick={() => renderGameplayAction(starterAction())}
              >
                {gameplayActionDisplayLabel(starterAction(), locale())}
              </button>
            </div>
          )}
        </Show>
      </Show>
      <Show when={hideActionButton() && !action()}>
        <div class="feedback-detail">
          {waitingForFirstAgent()
            ? tr(
              locale(),
              "当前还在等第一个 Agent 写入 committed 快照；OC 按钮会在 Agent 同步后自动出现。",
              "The first Agent is still waiting for the committed snapshot; the OC button appears automatically after the Agent syncs.",
            )
            : tr(
              locale(),
              "如果聊天提示 OC 不足，回到这里领取初始 OC。",
              "If chat says OC is missing, return here to claim starter OC.",
            )}
        </div>
      </Show>
    </div>
  );
}

function StarterOcRequiredGate() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const [autoConfirmAttempts, setAutoConfirmAttempts] = createSignal(0);
  const [manualConfirmAttempts, setManualConfirmAttempts] = createSignal(0);
  const [lastConfirmMode, setLastConfirmMode] = createSignal("auto");
  const gameplay = () => core.buildGameplaySummary(locale());
  const action = () => starterOcAction(gameplay());
  const submittedFeedback = () => starterOcSubmittedFeedback();
  const pendingCredit = () => starterOcClaimPendingForCurrentAgent() || Boolean(submittedFeedback());
  const creditConfirmed = () => pendingCredit()
    && (starterOcCreditVisibleForCurrentAgent() || Boolean(firstAgentChatAction(gameplay())))
    && !rawStarterOcActionAvailable();
  const progressionAction = () => gameplayProgressionAction(gameplay());
  const snapshotRefreshAction = () => (gameplay()?.availableActions || []).find((action) => action.executeKind === "request_snapshot") || null;
  const gateOpen = () => shouldShowStarterOcRequiredGate(gameplay());
  const firstChatUnlockPreview = () => core.state.snapshot?.player_gameplay?.agent_claim?.first_chat_unlock_preview || null;
  const chatAction = () => firstAgentChatAction(gameplay());
  const confirmationAction = () => {
    const refreshAction = snapshotRefreshAction();
    const advanceAction = progressionAction();
    return starterOcFeedbackNeedsLocalAdvance(submittedFeedback())
      ? advanceAction || refreshAction
      : refreshAction || advanceAction;
  };
  const visibleConfirmAttempt = () => lastConfirmMode() === "manual"
    ? Math.max(manualConfirmAttempts(), 1)
    : Math.min(autoConfirmAttempts() + 1, 3);
  const confirmStatusLabel = () => {
    if (creditConfirmed()) {
      return tr(locale(), "已入账", "Credited");
    }
    if (lastConfirmMode() === "manual") {
      return gameplayActionPendingFor(snapshotRefreshAction())
        ? tr(locale(), "手动确认中", "Manual confirmation")
        : tr(locale(), "等待手动确认回执", "Waiting for manual confirmation");
    }
    return autoConfirmAttempts() >= 3
      ? tr(locale(), "等待手动确认", "Waiting for manual confirmation")
      : tr(locale(), "自动确认中", "Auto-confirming");
  };
  const confirmProgressLabel = () => {
    if (creditConfirmed()) {
      return tr(locale(), "完成", "Done");
    }
    if (lastConfirmMode() === "manual") {
      return tr(
        locale(),
        `手动第 ${visibleConfirmAttempt()} 次确认`,
        `Manual check ${visibleConfirmAttempt()}`,
      );
    }
    return tr(locale(), `第 ${visibleConfirmAttempt()} 次确认`, `Check ${visibleConfirmAttempt()} of 3`);
  };
  const confirmSummaryCopy = () => {
    if (creditConfirmed()) {
      return tr(
        locale(),
        "第一笔 OC 已经写入本地快照。现在可以开始第一次 Agent 聊天，后续早期玩法动作也会解锁。",
        "The first OC is now visible in the local snapshot. You can start the first Agent chat and continue early gameplay actions.",
      );
    }
    if (lastConfirmMode() === "manual") {
      return gameplayActionPendingFor(snapshotRefreshAction())
        ? tr(
          locale(),
          "已发起手动确认。本地世界正在刷新快照，确认这笔初始 OC 是否已经写入。",
          "Manual confirmation started. The local world is refreshing the snapshot to verify whether the starter OC is visible.",
        )
        : tr(
          locale(),
          "自动确认还没有看到入账结果；可以再次手动刷新确认，或等待下一次快照同步。",
          "Auto-confirmation has not seen the credit yet; retry manual refresh or wait for the next snapshot sync.",
        );
    }
    return autoConfirmAttempts() >= 3
      ? tr(
        locale(),
        "自动确认已经跑完 3 次，仍未看到入账结果。可以手动再确认一次，或等待运行时快照继续同步。",
        "Auto-confirmation has completed 3 checks without seeing the credit. Retry manual confirmation or wait for the runtime snapshot to keep syncing.",
      )
      : tr(
        locale(),
        "领取请求已经提交。系统正在自动推进并刷新本地世界，确认这笔初始 OC 写入可见快照。",
        "The claim was submitted. The system is automatically advancing and refreshing the local world to confirm the starter OC in the visible snapshot.",
      );
  };
  const primaryAction = () => {
    if (creditConfirmed()) {
      return chatAction();
    }
    return pendingCredit() ? confirmationAction() : action();
  };
  let primaryButtonRef;
  let scheduledAutoConfirmAttempt = -1;
  let autoConfirmTimer = null;
  let autoCompleteTimer = null;

  createEffect(() => {
    if (gateOpen()) {
      window.setTimeout(() => primaryButtonRef?.focus(), 0);
    }
  });

  createEffect(() => {
    if (creditConfirmed()) {
      if (autoCompleteTimer == null) {
        autoCompleteTimer = window.setTimeout(() => {
          completeStarterOcOnboarding();
          setAutoConfirmAttempts(0);
          setManualConfirmAttempts(0);
          setLastConfirmMode("auto");
          core.requestRender();
          autoCompleteTimer = null;
        }, 1200);
      }
      return;
    }
    if (autoCompleteTimer != null) {
      window.clearTimeout(autoCompleteTimer);
      autoCompleteTimer = null;
    }
    if (!pendingCredit() || creditConfirmed()) {
      scheduledAutoConfirmAttempt = -1;
      if (!pendingCredit()) {
        setAutoConfirmAttempts(0);
        setManualConfirmAttempts(0);
        setLastConfirmMode("auto");
      }
      return;
    }
    const nextAction = confirmationAction();
    const attempt = autoConfirmAttempts();
    if (!nextAction || attempt >= 3 || scheduledAutoConfirmAttempt === attempt) {
      return;
    }
    if (gameplayActionButtonDisabled(nextAction, gameplay(), locale())) {
      return;
    }
    scheduledAutoConfirmAttempt = attempt;
    autoConfirmTimer = window.setTimeout(() => {
      setLastConfirmMode("auto");
      renderGameplayAction(nextAction);
      setAutoConfirmAttempts((value) => value + 1);
    }, attempt === 0 ? 450 : 1600);
  });

  onCleanup(() => {
    if (autoConfirmTimer != null) {
      window.clearTimeout(autoConfirmTimer);
    }
    if (autoCompleteTimer != null) {
      window.clearTimeout(autoCompleteTimer);
    }
  });

  return (
    <Show when={gateOpen()}>
      {() => (
        <div
          class="auth-gate"
          role="dialog"
          aria-modal="true"
          aria-labelledby="starter-oc-gate-title"
          data-viewer-fixture-state="starter_oc_required_gate"
        >
          <div class="auth-gate__dialog">
            <div class="auth-gate__header">
              <div>
                <div class="panel__eyebrow">{tr(locale(), "新手必经步骤", "Required Onboarding Step")}</div>
                <h1 id="starter-oc-gate-title" class="auth-gate__title">
                  {creditConfirmed()
                    ? tr(locale(), "OC 已入账", "OC Credited")
                    : pendingCredit()
                    ? tr(locale(), "正在确认 OC 入账", "Confirming OC Credit")
                    : tr(locale(), "领取第一笔 OC", "Claim Your First OC")}
                </h1>
              </div>
              <Badge class={creditConfirmed() ? "badge badge--good" : pendingCredit() ? "badge badge--accent" : "badge badge--good"}>
                {creditConfirmed() ? "credited" : pendingCredit() ? "syncing" : "ready"}
              </Badge>
            </div>
            <Show
              when={!pendingCredit()}
              fallback={(
                <div class="stack stack--compact">
                  <div class="feedback-summary">
                    {confirmSummaryCopy()}
                  </div>
                  <div class="summary-grid">
                    <div class="metric">
                      <div class="metric__label">{tr(locale(), "状态", "Status")}</div>
                      <div class="metric__value">
                        {creditConfirmed()
                          ? tr(locale(), "已入账", "Credited")
                          : confirmStatusLabel()}
                      </div>
                    </div>
                    <div class="metric">
                      <div class="metric__label">{tr(locale(), "进度", "Progress")}</div>
                      <div class="metric__value">
                        {creditConfirmed()
                          ? tr(locale(), "完成", "Done")
                          : confirmProgressLabel()}
                      </div>
                    </div>
                    <div class="metric">
                      <div class="metric__label">{tr(locale(), "你可以做什么", "What To Do")}</div>
                      <div class="metric__value">
                        {creditConfirmed()
                          ? tr(locale(), "开始聊天", "Start chat")
                          : tr(locale(), "先看玩法说明", "Read the guide")}
                      </div>
                    </div>
                  </div>
                  <StarterOcGuide locale={locale()} />
                  <Show when={submittedFeedback()}>
                    {(feedback) => (
                      <FeedbackCard
                        feedback={feedback()}
                        feedbackStage={feedback().stage}
                        display={core.describeSemanticFeedback(feedback(), locale())}
                        liveRegion
                      />
                    )}
                  </Show>
                </div>
              )}
            >
              <Show
                when={firstChatUnlockPreview()}
                fallback={<StarterOcOnboardingPanel gameplay={gameplay()} locale={locale()} hideActionButton={true} />}
              >
                {(preview) => <FirstChatUnlockPreview preview={preview()} locale={locale()} tr={tr} />}
              </Show>
            </Show>
            <Show when={!firstChatUnlockPreview()}><div class="feedback-detail">
              {creditConfirmed()
                ? tr(locale(), "OC 会作为第一次 LLM/Agent chat 的启动预算；用它向 Agent 发第一条指令，推动产线恢复。", "OC is the starter budget for the first LLM/Agent chat. Use it to send the first command and move production forward.")
                : pendingCredit()
                ? tr(locale(), "不用空等：系统会自动推进确认。若本地世界暂时没有回执，下面的按钮可以手动补一次确认。", "No need to idle: confirmation runs automatically. If the local world has not responded yet, the button below can retry one confirmation.")
                : tr(locale(), "这是进入 Agent 聊天和早期玩法动作前必须完成的一步。领取后会进入入账确认。", "This step is required before Agent chat and early gameplay actions. Claiming it moves you to credit confirmation.")}
            </div></Show>
            <div class="toolbar">
              <Show when={primaryAction()}>
                {(nextAction) => (
                  <button
                    ref={primaryButtonRef}
                    data-testid="viewer-playthrough-action-claim-starter-oc"
                    class={gameplayActionButtonClass(nextAction())}
                    aria-busy={gameplayActionButtonBusyAttrs(nextAction())}
                    disabled={gameplayActionButtonDisabled(nextAction(), gameplay(), locale())}
                    onClick={() => {
                      if (creditConfirmed()) {
                        completeStarterOcOnboarding();
                        setAutoConfirmAttempts(0);
                        setManualConfirmAttempts(0);
                        setLastConfirmMode("auto");
                      }
                      if (pendingCredit()) {
                        setLastConfirmMode("manual");
                        setManualConfirmAttempts((value) => value + 1);
                      }
                      renderGameplayAction(nextAction());
                    }}
                  >
                    {gameplayActionDisplayLabel(
                      nextAction(),
                      locale(),
                      creditConfirmed()
                        ? tr(locale(), "开始第一次 Agent 聊天", "Start First Agent Chat")
                        : pendingCredit()
                        ? tr(locale(), "手动再确认一次", "Retry Confirmation")
                        : gameplayActionButtonLabel(nextAction(), locale()),
                    )}
                  </button>
                )}
              </Show>
              <Show when={creditConfirmed() && !primaryAction()}>
                <button
                  ref={primaryButtonRef}
                  onClick={() => {
                    completeStarterOcOnboarding();
                    setAutoConfirmAttempts(0);
                    setManualConfirmAttempts(0);
                    setLastConfirmMode("auto");
                    core.requestRender();
                  }}
                >
                  {tr(locale(), "继续", "Continue")}
                </button>
              </Show>
            </div>
          </div>
        </div>
      )}
    </Show>
  );
}

function renderGameplayAction(action) {
  if (action.executeKind === "agent_chat") {
    core.applySelection({ kind: "agent", id: action.targetAgentId });
    return;
  }
  markGameplayActionPending(action, gameplayActionButtonLabel(action, uiLocale()));
  if (action.actionId === "claim_starter_oc") {
    markStarterOcClaimPending(action);
  }
  const result = core.sendGameplayAction(action);
  if (result && result.ok === false) {
    clearGameplayActionPending(action);
  } else if (
    result
    && result.ok === true
    && !result.feedback
    && action.executeKind !== "request_snapshot"
    && !(starterOcClaimPendingForCurrentAgent() && ["step", "play"].includes(action.executeKind))
  ) {
    clearGameplayActionPending(action);
  }
  if (action.actionId === "claim_starter_oc" && result && result.ok === false) {
    clearStarterOcClaimPending();
  } else if (action.actionId === "claim_starter_oc") {
    scheduleStarterOcBackgroundConfirmation();
    core.requestRender();
  }
  return result;
}

function AgentClaimSessionBoundaryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const agentClaim = () => props.gameplay?.agentClaim || null;
  return (
    <CalloutCard
      title={tr(locale(), "当前账号尚未绑定 Agent", "Current Account Has No Bound Agent")}
      badge="observe"
      badgeClass="badge badge--accent"
    >
      <div class="feedback-summary">
        {tr(
          locale(),
          "这个世界已经有 Agent，但当前账号还没有可作为 claimer 的绑定 Agent。",
          "This world already has Agents, but the current account has no bound Agent that can act as the claimer.",
        )}
      </div>
      <div class="feedback-detail">
        {tr(
          locale(),
          "可以先观察世界对象；认领入口必须来自当前会话绑定和 canonical slot-1 quote，不能从世界里的第一个 Agent 推断。",
          "You can observe world objects first; claim entry must come from the current session binding and canonical slot-1 quote, not from the first Agent in the world.",
        )}
      </div>
      <div class="badge-row">
        <Badge>{`boundAgent=${core.state.auth.boundAgentId || "-"}`}</Badge>
        <Badge>{`claimer=${agentClaim()?.claimer_agent_id || "-"}`}</Badge>
        <Badge>{`owned=${agentClaim()?.owned_claim_count ?? 0}/${agentClaim()?.claim_cap ?? "-"}`}</Badge>
      </div>
    </CalloutCard>
  );
}

function AgentClaimPanel(props) {
  const locale = () => props.locale ?? uiLocale();
  const [selectedTargetId, setSelectedTargetId] = createSignal("");
  const agentClaim = () => props.gameplay?.agentClaim || null;
  const quote = () => agentClaim()?.next_claim_quote || null;
  const targets = () => buildAgentClaimTargets(core.state.snapshot, agentClaim());
  const selectedTarget = () => {
    const current = selectedTargetId();
    if (current && targets().some((target) => target.id === current)) {
      return current;
    }
    return targets()[0]?.id || "";
  };
  const claimAction = () => buildAgentClaimAction(agentClaim(), selectedTarget());
  const disabledReason = () => {
    if (!agentClaim()) {
      return tr(locale(), "当前快照没有发布 Agent claim 数据。", "The current snapshot has no Agent claim data.");
    }
    if (!selectedTarget()) {
      return tr(locale(), "当前没有可认领的 Agent。", "There is no claimable agent right now.");
    }
    return claimAction()?.disabledReason || null;
  };

  return (
    <CalloutCard
      title={tr(locale(), "认领 Agent", "Agent Claim")}
      badge={quote() ? `slot=${quote().slot_index}` : "claim"}
      badgeClass={disabledReason() ? "badge badge--warn" : "badge badge--good"}
    >
      <div class="feedback-summary">
        {agentClaim()?.objective
          || tr(locale(), "选择一个未被占用的 Agent，并用当前玩家会话提交认领。", "Pick an unclaimed agent and submit the claim with the current player session.")}
      </div>
      <div class="feedback-detail">
        {agentClaim()?.progress_detail
          || tr(locale(), "首次 slot-1 认领可以使用专用 starter claim 额度补足前置费用。", "The first slot-1 claim can use the dedicated starter claim allowance for upfront costs.")}
      </div>
      <div class="badge-row">
        <Badge>{`claimer=${agentClaim()?.claimer_agent_id || "-"}`}</Badge>
        <Badge>{`owned=${agentClaim()?.owned_claim_count ?? 0}/${agentClaim()?.claim_cap ?? "-"}`}</Badge>
        <Badge>{`eligible=${quote()?.eligible_claim_balance ?? agentClaim()?.slot_1_eligible_claim_balance ?? "-"}`}</Badge>
        <Badge>{`upfront=${quote()?.total_upfront_amount ?? "-"}`}</Badge>
      </div>
      <div class="control-grid">
        <div class="field">
          <label for="agent-claim-target">
            {tr(locale(), "目标 Agent", "Target Agent")}
          </label>
          <select
            id="agent-claim-target"
            value={selectedTarget()}
            onInput={(event) => setSelectedTargetId(event.currentTarget.value)}
          >
            <For each={targets()}>
              {(target) => (
                <option value={target.id}>
                  {`${target.name}${target.isClaimer ? ` (${tr(locale(), "当前绑定", "current binding")})` : ""}`}
                </option>
              )}
            </For>
          </select>
        </div>
      </div>
      <div class="toolbar">
        <button
          class={gameplayActionButtonClass(claimAction())}
          aria-busy={gameplayActionButtonBusyAttrs(claimAction())}
          disabled={Boolean(disabledReason()) || gameplayActionPendingFor(claimAction())}
          onClick={() => {
            const action = claimAction();
            if (action) {
              renderGameplayAction(action);
            }
          }}
        >
          {gameplayActionDisplayLabel(claimAction(), locale(), tr(locale(), "认领 Agent", "Claim Agent"))}
        </button>
      </div>
    </CalloutCard>
  );
}

function gameplayProgressLabel(progressPercent, locale) {
  return progressPercent == null
    ? tr(locale, "进度待发布", "Progress Pending")
    : tr(locale, `进度 ${progressPercent}%`, `Progress ${progressPercent}%`);
}

function chatEntryTitle(entry, locale) {
  const target = entry.targetAgentId || entry.agentId || "agent";
  if (entry.source === "error") {
    return `${target} ${tr(locale, "回复失败", "reply failed")}`;
  }
  if (entry.source === "player") {
    return `${tr(locale, "玩家", "Player")} -> ${target}`;
  }
  return `${entry.agentId || target} ${tr(locale, "回应", "Reply")}`;
}

function chatEntryCardClass(entry) {
  if (entry.source === "error") return "event-card event-card--chat-error";
  if (entry.source === "player") return "event-card event-card--chat-player";
  return "event-card event-card--chat-agent";
}

function chatEntryMeta(entry, locale) {
  if (entry.source === "error") {
    const code = entry.code ? ` · code=${entry.code}` : "";
    return `${entry.speaker || "runtime"}${code} · tick=${Number(entry.tick || 0)}`;
  }
  const speaker = entry.source === "player"
    ? entry.playerId || entry.speaker || tr(locale, "玩家", "Player")
    : entry.speaker || entry.agentId || "agent";
  const location = entry.locationId || tr(locale, "未知位置", "unknown location");
  return `${speaker} · ${location}`;
}

function chatEntryMessage(entry, locale) {
  const message = String(entry.message || "").trim();
  if (entry.source === "error") {
    const prefix = tr(locale, "Agent 回复没有完成", "Agent reply did not complete");
    return message ? `${prefix}: ${message}` : prefix;
  }
  return message || tr(locale, "这条消息没有可读正文。", "This message has no readable text.");
}

function connectionStatusLabel(status, locale) {
  if (status === "connected") {
    return tr(locale, "世界在线", "World Live");
  }
  if (status === "connecting") {
    return tr(locale, "正在连入世界", "Connecting to World");
  }
  if (status === "closed") {
    return tr(locale, "连接已关闭", "Connection Closed");
  }
  return tr(locale, `连接异常：${status || "unknown"}`, `Connection Issue: ${status || "unknown"}`);
}

function renderResourceSummary(resources) {
  return core.resourceSummary(resources);
}

function WorldStageHero() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const authSurface = () => core.buildAuthSurfaceModel();
  const presentationScale = () => core.buildWorldScaleSurface(locale()).presentationScale;
  const selectedLabel = () =>
    core.state.selectedKind && core.state.selectedId
      ? `${core.state.selectedKind}:${core.state.selectedId}`
      : null;
  const identityKindLabel = () => {
    const source = String(authSurface().source || core.state.auth.source || "").trim();
    if (!core.state.auth.available) {
      return tr(locale(), "访客 / 未登录", "Guest / Not Signed In");
    }
    if (source === "hosted_browser_storage" || source === "hosted_player_session_issue") {
      return tr(locale(), "邮箱登录身份", "Hosted Account Identity");
    }
    if (source === "local_test_api_ephemeral" || source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
      return tr(locale(), "本地测试身份", "Local Test Identity");
    }
    return authSurface().currentTier || tr(locale(), "玩家身份", "Player Identity");
  };
  const publicKeyShort = () =>
    core.state.auth.publicKey
      ? `${String(core.state.auth.publicKey).slice(0, 12)}...`
      : "-";
  const identityDetail = () => {
    if (!core.state.auth.available) {
      return tr(
        locale(),
        "当前还没有玩家 session；本地测试动作会按需生成临时玩家 key，托管公开模式才需要邮箱登录。",
        "No player session is active yet. Local test actions generate an ephemeral player key on demand; only hosted public join requires email sign-in.",
      );
    }
    return [
      `player=${core.state.auth.playerId || "-"}`,
      `pubkey=${publicKeyShort()}`,
      `session=${core.state.auth.registrationStatus || core.state.auth.runtimeStatus || "-"}`,
      `agent=${core.state.auth.boundAgentId || "-"}`,
    ].join(" · ");
  };
  const identityMeta = () => {
    const source = authSurface().source || core.state.auth.source || "-";
    if (!core.state.auth.available) {
      return `source=${source}`;
    }
    const loginNote = String(source) === "hosted_browser_storage"
      || String(source) === "hosted_player_session_issue"
      ? tr(locale(), "已通过托管账号会话", "hosted account session")
      : tr(locale(), "不是邮箱登录账号", "not an email login account");
    return `source=${source} · ${loginNote}`;
  };
  const selectionHint = () =>
    core.state.selectedKind && core.state.selectedId
      ? tr(locale(), "右侧指挥面板会围绕这个对象展开。", "The command surface on the right now follows this target.")
      : tr(locale(), "先从左侧锁定一个行动体或地点，再进入右侧指挥面板。", "Lock onto an agent or location from the left before entering the command surface.");
  const stageLabel = () => gameplayStageLabel(gameplaySummary()?.stageStatus, locale());
  const nextStepCopy = () =>
    gameplaySummary()?.narrativeNextStep
    || tr(locale(), "先读世界状态，再决定是否推进、恢复或对目标发消息。", "Read the world first, then decide whether to advance, resume, or message the target.");
  const acceptedIntentTitle = () =>
    gameplaySummary()?.acceptedIntentSummary
    || tr(locale(), "先提交一条明确意图", "Commit one clear intent first");
  const acceptedIntentDetail = () =>
    gameplaySummary()?.acceptedIntentTarget
      ? tr(
        locale(),
        `当前意图正围绕 ${gameplaySummary().acceptedIntentTarget} 展开。`,
        `The current intent is centered on ${gameplaySummary().acceptedIntentTarget}.`,
      )
      : selectionHint();
  const refreshSnapshotAction = () =>
    (gameplaySummary()?.availableActions || []).find((action) => action.executeKind === "request_snapshot")
    || {
      actionId: "request_snapshot",
      action_id: "request_snapshot",
      label: "Request snapshot",
      protocolAction: "request_snapshot",
      protocol_action: "request_snapshot",
      executeKind: "request_snapshot",
      targetAgentId: null,
      disabledReason: null,
    };
  const primaryActionContext = () =>
    gameplaySummary()?.recommendedAction?.label
    || gameplaySummary()?.narrativeNextStep
    || gameplaySummary()?.nextStepHint
    || gameplaySummary()?.objective
    || "";
  const primaryRefreshLabel = () => {
    const context = primaryActionContext();
    return context
      ? tr(locale(), `刷新快照，确认：${context}`, `Refresh Snapshot to verify: ${context}`)
      : tr(locale(), "刷新快照，确认当前玩法状态", "Refresh Snapshot to verify the current gameplay state");
  };
  const primaryStepLabel = () => {
    const context = primaryActionContext();
    return context
      ? tr(locale(), `推进一步，尝试：${context}`, `Advance One Step toward: ${context}`)
      : tr(locale(), "推进一步，尝试当前下一步", "Advance One Step toward the current next move");
  };

  return (
    <div class="stage-hero stage-hero--compact" data-stage-state={gameplaySummary()?.blockerKind || "ready"}>
      <div class="stage-hero__topline">
        <div class="stack stack--hero">
          <div class="stage-hero__eyebrow-row">
            <div class="stage-hero__eyebrow">{tr(locale(), "工业世界指挥桌", "Industrial World Command Desk")}</div>
            <InlineHelpTip
              locale={locale()}
              id="viewer-stage-scale-tip"
              label={tr(locale(), "打开表现层比例说明", "Open presentation scale guidance")}
              title={tr(locale(), "表现层说明", "Presentation Notes")}
              lines={[
                presentationScale().markerTruthNote,
                presentationScale().zoomTruthNote,
                presentationScale().softwareSafeNote,
              ]}
            />
          </div>
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
      <Show when={selectedLabel()}>
        {(selected) => (
          <div class="badge-row stage-hero__selection">
            <Badge class="badge badge--accent">{tr(locale(), "当前选择", "Current Selection")}</Badge>
            <Badge>{selected()}</Badge>
          </div>
        )}
      </Show>
      <div class="hero-focus-grid hero-focus-grid--compact">
        <div class="hero-focus-card">
          <div class="hero-focus-card__label">{tr(locale(), "局势", "Situation")}</div>
          <div class={gameplayStageToneClass(gameplaySummary()?.stageStatus)}>
            {stageLabel()}
          </div>
          <div class="hero-focus-card__detail">{gameplayProgressLabel(gameplaySummary()?.progressPercent, locale())}</div>
        </div>
        <div class="hero-focus-card">
          <div class="hero-focus-card__label">{tr(locale(), "已接受意图", "Accepted Intent")}</div>
          <div class="hero-focus-card__value hero-focus-card__value--body">{acceptedIntentTitle()}</div>
          <div class="hero-focus-card__detail">{acceptedIntentDetail()}</div>
        </div>
        <div
          class="hero-focus-card hero-focus-card--next-step"
          data-testid="viewer-next-step-card"
        >
          <div class="hero-focus-card__label">{tr(locale(), "下一步", "Next Step")}</div>
          <div class="hero-focus-card__value hero-focus-card__value--body">{nextStepCopy()}</div>
        </div>
        <div class="hero-focus-card" data-testid="viewer-identity-card">
          <div class="hero-focus-card__label">{tr(locale(), "当前身份", "Current Identity")}</div>
          <div class="hero-focus-card__value hero-focus-card__value--body">{identityKindLabel()}</div>
          <div class="hero-focus-card__detail">{identityDetail()}</div>
          <div class="hero-focus-card__detail">{identityMeta()}</div>
        </div>
      </div>
      <div class="toolbar" aria-label={tr(locale(), "主要玩法动作", "Primary gameplay actions")}>
        <button
          type="button"
          data-testid="viewer-playthrough-action-request-snapshot"
          aria-label={primaryRefreshLabel()}
          class={gameplayActionButtonClass(refreshSnapshotAction())}
          aria-busy={gameplayActionButtonBusyAttrs(refreshSnapshotAction())}
          disabled={gameplayActionPendingFor(refreshSnapshotAction())}
          onClick={() => renderGameplayAction(refreshSnapshotAction())}
        >
          {gameplayActionDisplayLabel(refreshSnapshotAction(), locale(), tr(locale(), "刷新快照", "Refresh Snapshot"))}
        </button>
        <button
          type="button"
          data-testid="viewer-playthrough-action-step"
          aria-label={primaryStepLabel()}
          onClick={() => core.sendControl("step", { count: 1 })}
        >
          {tr(locale(), "推进一步", "Advance One Step")}
        </button>
      </div>
      <div class="feedback-detail" data-testid="viewer-primary-action-preview">
        {primaryActionContext()
          ? tr(locale(), `推荐上下文：${primaryActionContext()}`, `Recommended context: ${primaryActionContext()}`)
          : tr(locale(), "先读目标和下一步，再选择刷新或推进。", "Read the goal and next step before choosing refresh or advance.")}
      </div>
      <Show when={gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities"}>
        <EmptyEntityRecoveryCard
          locale={locale()}
          gameplay={gameplaySummary}
          title={tr(locale(), "恢复世界快照", "Recover World Snapshot")}
        />
      </Show>
      <div class="stage-hero__mobile-shortcuts" aria-label={tr(locale(), "移动端快速入口", "Mobile quick actions")}>
        <a class="mobile-rail__link" href="#viewer-targets-panel">{tr(locale(), "选择目标", "Select Target")}</a>
        <a class="mobile-rail__link" href="#viewer-details-panel">{tr(locale(), "进入指挥", "Command")}</a>
      </div>
      <Show when={core.state.connectionStatus !== "connected"}>
        <CalloutCard
          title={tr(locale(), "世界连接需要注意", "World Connection Needs Attention")}
          badge={connectionStatusLabel(core.state.connectionStatus, locale())}
          badgeClass={core.connectionBadgeClass()}
          variant="warn"
        >
          <div class="feedback-summary">
            {tr(
              locale(),
              "首屏优先展示世界与目标；只有连接异常时，才把连接状态抬到这里提示你。",
              "This entry keeps the world and target first, and only elevates connection status when it needs attention.",
            )}
          </div>
        </CalloutCard>
      </Show>
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
      <a class="mobile-rail__link mobile-rail__link--diagnostics" href="#viewer-diagnostics-panel">
        {tr(locale(), "诊断", "Diagnostics")}
      </a>
      <a class="mobile-rail__link mobile-rail__link--diagnostics" href="#viewer-refine-quote-panel" onClick={focusViewerAnchor}>{tr(locale(), "报价", "Quote")}</a>
    </nav>
  );
}

function TargetsPanel() {
  observeViewerStateRevision();
  const lists = () => core.modelLists();
  const locale = () => uiLocale();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const firstAgentClaimAction = () =>
    (gameplaySummary()?.availableActions || []).find((action) => action.actionId === "claim_first_agent");
  const firstAgentClaimWaiting = () => Boolean(gameplayActionDisabledReason(firstAgentClaimAction(), gameplaySummary(), locale()));
  const hasSnapshot = () => Boolean(core.state.snapshot);
  const selectedLabel = () => {
    observeViewerStateRevision();
    if (!core.state.selectedKind || !core.state.selectedId) {
      return null;
    }
    if (core.state.selectedKind === "agent" && !core.isAgentVisibleToCurrentSession(core.state.selectedId)) {
      return null;
    }
    return `${core.state.selectedKind}:${core.state.selectedId}`;
  };
  const isSelectedTarget = (kind, id) => {
    observeViewerStateRevision();
    return core.state.selectedKind === kind && core.state.selectedId === id;
  };

  return (
    <div class="stack">
      <Show when={selectedLabel()}>
        {(selected) => (
          <div class="badge-row">
            <Badge class="badge badge--accent">{tr(locale(), "已锁定目标", "Locked Target")}</Badge>
            <Badge>{selected()}</Badge>
          </div>
        )}
      </Show>
      <EmptyState>
        {tr(
          locale(),
          "先从这里锁定一个行动体或地点。中间查看局势，右侧只处理你当前选中的目标。",
          "Lock onto an agent or location here first. Read the world in the middle, then use the right column only for the selected target.",
        )}
      </EmptyState>
      <Show when={firstAgentClaimAction()}>
        {(action) => (
          <CalloutCard
            title={tr(locale(), "认领第一个 Agent", "Claim Your First Agent")}
            badge={firstAgentClaimWaiting() ? "waiting" : "ready"}
            badgeClass={firstAgentClaimWaiting() ? "badge badge--accent" : "badge badge--good"}
            variant={firstAgentClaimWaiting() ? "warn" : null}
          >
            <div class="feedback-summary">
              {gameplayActionDisabledReason(action(), gameplaySummary(), locale())
                || tr(
                  locale(),
                  "当前是新用户空世界：先认领第一个 Agent，它会在链上提交并同步后出现在行动体列表。",
                  "This is a new-user empty world: claim the first Agent first, then it will appear in the agent list after chain submission and sync.",
                )}
            </div>
            <Show when={firstAgentClaimWaiting()}>
              <StarterOcOnboardingPanel
                gameplay={gameplaySummary()}
                locale={locale()}
                waitingForFirstAgent={true}
              />
            </Show>
            <div class="toolbar">
              <button
                class={gameplayActionButtonClass(action())}
                aria-busy={gameplayActionButtonBusyAttrs(action())}
                disabled={gameplayActionButtonDisabled(action(), gameplaySummary(), locale())}
                onClick={() => renderGameplayAction(action())}
              >
                {gameplayActionDisplayLabel(action(), locale())}
              </button>
            </div>
          </CalloutCard>
        )}
      </Show>
      <div class="field">
        <label for="entity-search">{tr(locale(), "筛选目标", "Filter targets")}</label>
        <input
          id="entity-search"
          type="search"
          placeholder={tr(locale(), "搜索行动体或地点", "Search agents or locations")}
          value={core.getSelectedSearch()}
          onInput={(event) => core.setSelectedSearch(event.currentTarget.value)}
        />
      </div>
      <div>
        <div class="panel__title panel__title--spaced">{tr(locale(), "行动体", "Agents")}</div>
        <div class="list">
          <Show
            when={lists().agents.length > 0}
            fallback={
              hasSnapshot()
                ? <EmptyState>{tr(locale(), "当前快照里没有行动体。", "No agents in current snapshot.")}</EmptyState>
                : <EntityListPendingState locale={locale()} label={tr(locale(), "行动体", "agents")} />
            }
          >
            <For each={lists().agents}>
              {(agent, index) => {
                const status = () => describeAgentSessionStatus(agent.id, locale());
                return (
                  <button
                    class="list-item"
                    data-testid={index() === 0 ? "viewer-playthrough-select-agent" : `viewer-select-agent-${agent.id}`}
                    data-select-kind="agent"
                    data-select-id={agent.id}
                    data-agent-session-status={status().kind}
                    data-selected={isSelectedTarget("agent", agent.id)}
                    onClick={() => core.applySelection({ kind: "agent", id: agent.id })}
                  >
                    <div class="list-item__header">
                      <div class="list-item__title">{agent.id}</div>
                      <Show when={isSelectedTarget("agent", agent.id)}>
                        <span class="list-item__selected-label">{tr(locale(), "已选中", "Selected")}</span>
                      </Show>
                    </div>
                    <div class="badge-row">
                      <Badge class={status().badgeClass}>{status().badge}</Badge>
                      <Show when={status().binding.playerId}>
                        <Badge>{`boundPlayer=${status().binding.playerId}`}</Badge>
                      </Show>
                    </div>
                    <div class="list-item__meta">
                      {`${tr(locale(), "地点", "location")}=${agent.location_id} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(agent.resources)}`}
                    </div>
                    <div class="list-item__meta">{status().detail}</div>
                  </button>
                );
              }}
            </For>
          </Show>
        </div>
      </div>
      <div>
        <div class="panel__title panel__title--spaced">{tr(locale(), "地点", "Locations")}</div>
        <div class="list">
          <Show
            when={lists().locations.length > 0}
            fallback={
              hasSnapshot()
                ? <EmptyState>{tr(locale(), "当前快照里没有地点。", "No locations in current snapshot.")}</EmptyState>
                : <EntityListPendingState locale={locale()} label={tr(locale(), "地点", "locations")} />
            }
          >
            <For each={lists().locations}>
              {(location) => (
                <button
                  class="list-item"
                  data-testid={`viewer-select-location-${location.id}`}
                  data-select-kind="location"
                  data-select-id={location.id}
                  data-selected={isSelectedTarget("location", location.id)}
                  onClick={() => core.applySelection({ kind: "location", id: location.id })}
                >
                  <div class="list-item__header">
                    <div class="list-item__title">{location.name || location.id}</div>
                    <Show when={isSelectedTarget("location", location.id)}>
                      <span class="list-item__selected-label">{tr(locale(), "已选中", "Selected")}</span>
                    </Show>
                  </div>
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
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const state = core.state;
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(gameplaySummary());
  const gameplayActionFeedback = () => core.snapshotSemanticFeedback(state.lastGameplayActionFeedback);
  const promptFeedback = () => core.snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => core.snapshotSemanticFeedback(state.lastChatFeedback);
  const gameplayActionFeedbackDisplay = () => core.describeSemanticFeedback(gameplayActionFeedback(), locale());
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const authSurface = () => core.buildAuthSurfaceModel();
  const hostedActionMatrixView = () => core.buildHostedActionMatrixView();
  const hostedRecoveryHint = () => core.buildHostedRecoveryHint(locale());
  const tierBadgeClass = (status) =>
    status === "active"
      || status === "active_legacy_preview"
      || status === "active_hosted_issue"
      || status === "active_hosted_session"
      || status === "preview_backend_reauth_available"
      ? "badge badge--good"
      : status === "issued_pending_register" || status === "upgrade_after_player_session" || status === "preview_only"
        ? "badge badge--accent"
      : status === "superseded"
        ? "badge"
        : "badge badge--warn";
  const showRebindNotice = () =>
    Boolean(state.auth.pendingRequestedAgentId)
      && (state.auth.pendingForceRebind
        || state.auth.runtimeStatus === "rebind_retrying"
        || state.auth.runtimeStatus === "rebind_registering");
  const showPlayerSessionSurface = () =>
    !!hostedRecoveryHint()
      || (
        !state.auth.available
        && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)
      )
      || showRebindNotice();
  const diagnosticsSummaryBadges = () => [
    `auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}`,
    `events=${state.recentEvents.length}`,
  ];

  return (
    <>
      <Show when={!starterOcGateOpen() && starterOcAction(gameplaySummary())}>
        <CalloutCard
          title={tr(locale(), "领取第一笔 OC", "Claim Your First OC")}
          badge="ready"
          badgeClass="badge badge--good"
        >
          <StarterOcOnboardingPanel gameplay={gameplaySummary()} locale={locale()} />
        </CalloutCard>
      </Show>
      <details class="gameplay-details-surface" id="viewer-gameplay-details" open>
      <summary class="gameplay-details-surface__summary">
        <div class="diagnostic-surface__title">
          <span>{tr(locale(), "玩法明细", "Gameplay Details")}</span>
          <span class="diagnostic-surface__meta">
            {tr(locale(), "世界棋盘上方已保留目标、下一步和回执；这里展开看完整状态机与经济明细。", "The world board already carries objective, next move, and receipt; expand here for the full state machine and economy details.")}
          </span>
        </div>
        <Badge>{diagnosticsSummaryBadges().join(" · ")}</Badge>
      </summary>
      <div class="stack flow-top">
        <PanelSection
          title={tr(locale(), "正式玩法摘要", "Formal Gameplay Summary")}
          eyebrow={tr(locale(), "玩家主路径", "Player Path")}
          meta={tr(locale(), "先看目标、阻塞和下一步，再决定是否进入右侧指挥区。", "Read the goal, blocker, and next step first, then decide whether to enter the command surface.")}
        >
        <Show
          when={gameplaySummary()}
          fallback={<EmptyState>{tr(locale(), "等待首条规范玩法快照…", "Waiting for the first canonical gameplay snapshot…")}</EmptyState>}
        >
          {(gameplay) => (
            <>
              <div class="badge-row">
                <Badge class={gameplayStatusBadgeClass(gameplay().stageStatus)}>
                  {gameplayStageLabel(gameplay().stageStatus, locale())}
                </Badge>
                <Badge class="badge badge--accent">
                  {gameplayProgressLabel(gameplay().progressPercent, locale())}
                </Badge>
              </div>
              <EventCard
                title={tr(locale(), "控制证明", "Control Proof")}
                badge={gameplay().controlProof?.state || gameplay().executionState || "-"}
                badgeClass={goalExecutionBadgeClass(gameplay().controlProof?.state || gameplay().executionState)}
                meta={tr(locale(), "把玩家意图、世界后果、恢复动作和下一步串成一条首局可读链。", "Connect player intent, world consequence, recovery, and next move into one first-session-readable chain.")}
              >
                <div class="feedback-summary">
                  {gameplay().controlProof?.summary
                    || tr(locale(), "等待控制证明链路发布。", "Waiting for the control proof chain.")}
                </div>
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "玩家意图", "Player Intent")}
                    value={gameplay().controlProof?.intent || tr(locale(), "待提交", "not submitted")}
                  />
                  <MetricCard
                    label={tr(locale(), "世界后果", "World Consequence")}
                    value={gameplay().controlProof?.consequence || tr(locale(), "待回执", "waiting for receipt")}
                  />
                  <MetricCard
                    label={tr(locale(), "恢复动作", "Recovery Move")}
                    value={gameplay().controlProof?.recovery || tr(locale(), "待发布", "not published")}
                  />
                  <MetricCard
                    label={tr(locale(), "下一步", "Next Move")}
                    value={gameplay().controlProof?.nextMove || tr(locale(), "等待运行时指引", "waiting for runtime guidance")}
                  />
                </div>
              </EventCard>
              <PanelSection
                title={tr(locale(), "吸引力证明", "Attraction Proof")}
                eyebrow={tr(locale(), "TASK-GAME-076: 0-30 分钟", "TASK-GAME-076: 0-30 Minutes")}
                meta={tr(locale(), "只从 canonical player_gameplay 派生首局吸引力证据；缺失项会显示为等待或未验证。", "Derives first-session attraction evidence only from canonical player_gameplay; missing signals stay waiting or unverified.")}
              >
                <div class="badge-row">
                  <Badge>{gameplay().attractionProof?.verdict || "unverified"}</Badge>
                </div>
                <div class="feedback-summary">
                  {gameplay().attractionProof?.summary
                    || tr(locale(), "等待吸引力证据发布。", "Waiting for attraction proof.")}
                </div>
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "我造成了什么", "What I caused")}
                    value={gameplay().attractionProof?.whatICaused || tr(locale(), "等待玩家导致的世界变化", "waiting for player-caused world change")}
                  />
                  <MetricCard
                    label={tr(locale(), "新选择", "New option")}
                    value={gameplay().attractionProof?.newOption || tr(locale(), "等待新选择", "waiting for new option")}
                  />
                  <MetricCard
                    label={tr(locale(), "为什么继续", "Why continue")}
                    value={gameplay().attractionProof?.whyContinue || tr(locale(), "等待下一分支", "waiting for next branch")}
                  />
                  <MetricCard
                    label={tr(locale(), "等待代价", "Waiting cost")}
                    value={gameplay().attractionProof?.waitingCost || tr(locale(), "等待 / 未验证", "waiting/unverified")}
                  />
                  <MetricCard
                    label={tr(locale(), "恢复", "Recovery")}
                    value={gameplay().attractionProof?.recovery || tr(locale(), "等待恢复路径", "waiting for recovery path")}
                  />
                </div>
              </PanelSection>
              <PanelSection
                title={tr(locale(), "玩家能动性动词", "Agency Moves")}
                eyebrow={tr(locale(), "P1: 打断 / 重排 / 纠偏", "P1: Interrupt / Reprioritize / Correct")}
                meta={tr(locale(), "只展示已由玩法快照发布或可从现有状态推导的能动性入口，不在 viewer 里伪造新动作。", "Shows only agency entries published by the gameplay snapshot or derived from current state; the viewer does not invent new actions.")}
              >
                <div class="feedback-summary">
                  {gameplay().agencyMoves?.summary
                    || tr(locale(), "等待玩家能动性动词发布。", "Waiting for player agency moves.")}
                </div>
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "打断", "Interrupt")}
                    value={gameplay().agencyMoves?.interrupt || tr(locale(), "未验证", "unverified")}
                  />
                  <MetricCard
                    label={tr(locale(), "重排", "Reprioritize")}
                    value={gameplay().agencyMoves?.reprioritize || tr(locale(), "未验证", "unverified")}
                  />
                  <MetricCard
                    label={tr(locale(), "纠偏", "Correction")}
                    value={gameplay().agencyMoves?.correction || tr(locale(), "等待替代意图", "waiting for replacement intent")}
                  />
                  <MetricCard
                    label={tr(locale(), "交接结果", "Handoff")}
                    value={gameplay().agencyMoves?.handoff || tr(locale(), "等待新旧意图交接", "waiting for handoff")}
                  />
                </div>
              </PanelSection>
              <PanelSection
                title={tr(locale(), "首胜与反刷", "First Win & Anti-Grind")}
                eyebrow={tr(locale(), "P1: 小玩家第一场工业胜利", "P1: Small-Player First Industrial Win")}
                meta={tr(locale(), "把玩家动作、世界变化和 leverage 类型放在一起，避免首胜只变成产量数字。", "Pairs player action, world change, and leverage class so the first win is not reduced to output volume.")}
              >
                <div class="feedback-summary">
                  {gameplay().progressionProof?.summary
                    || tr(locale(), "等待首胜与反刷证据发布。", "Waiting for first-win and anti-grind evidence.")}
                </div>
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "首胜目标", "First Win")}
                    value={gameplay().progressionProof?.firstWinGoal || tr(locale(), "待发布", "not published")}
                  />
                  <MetricCard
                    label={tr(locale(), "玩家动作", "Player Action")}
                    value={gameplay().progressionProof?.playerAction || tr(locale(), "待提交", "not submitted")}
                  />
                  <MetricCard
                    label={tr(locale(), "世界变化", "World Change")}
                    value={gameplay().progressionProof?.worldChange || tr(locale(), "待回执", "waiting for receipt")}
                  />
                  <MetricCard
                    label={tr(locale(), "反刷 leverage", "Anti-Grind Leverage")}
                    value={gameplay().progressionProof?.antiGrind || tr(locale(), "待验证", "unverified")}
                    detail={gameplay().progressionProof?.leverageVerdict}
                  />
                </div>
              </PanelSection>
              <PanelSection
                title={tr(locale(), "成熟世界承接", "Mature-World Continuation")}
                eyebrow={tr(locale(), "P2: 修复 / 重建 / 转向", "P2: Repair / Rebuild / Pivot")}
                meta={tr(locale(), "呈现世界变复杂之后，小玩家是否仍有独立承接路径和可复盘短故事。", "Shows whether small players retain independent continuation paths and replayable story evidence after the world becomes complex.")}
              >
                <div class="feedback-summary">
                  {gameplay().matureWorldContinuation?.summary
                    || tr(locale(), "等待成熟世界承接证据发布。", "Waiting for mature-world continuation evidence.")}
                </div>
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "依赖状态", "Dependency")}
                    value={gameplay().matureWorldContinuation?.dependencyStatus || tr(locale(), "未验证", "unverified")}
                  />
                  <MetricCard
                    label={tr(locale(), "恢复路径", "Recovery Path")}
                    value={gameplay().matureWorldContinuation?.recoveryPath || tr(locale(), "等待运行时指引", "waiting for runtime guidance")}
                  />
                  <MetricCard
                    label={tr(locale(), "分享回放", "Share Replay")}
                    value={gameplay().shareReplay?.snippet || tr(locale(), "等待可复盘片段", "waiting for replayable snippet")}
                    detail={gameplay().shareReplay?.summary}
                  />
                </div>
                <RecoveryOptionComparisonPanel
                  continuation={gameplay().matureWorldContinuation}
                  locale={locale()}
                  tr={tr}
                />
              </PanelSection>
              <EventCard
                title={tr(locale(), "已接受意图", "Accepted Intent")}
                badge={gameplay().acceptedIntentScope || gameplay().executionStateLabel || "-"}
                badgeClass={goalExecutionBadgeClass(gameplay().executionState)}
                meta={
                  gameplay().acceptedIntentTarget
                    ? tr(locale(), `当前作用对象 ${gameplay().acceptedIntentTarget}`, `Current target ${gameplay().acceptedIntentTarget}`)
                    : tr(locale(), "当前主意图", "Current primary intent")
                }
              >
                <div class="feedback-summary">{gameplay().acceptedIntentSummary}</div>
                <div class="feedback-detail">{gameplay().acceptedIntentDetail}</div>
                <Show when={gameplay().resumeAnchor}>
                  <div class="badge-row">
                    <Badge>{tr(locale(), "续玩锚点", "Resume Anchor")}</Badge>
                  </div>
                  <div class="feedback-detail">{gameplay().resumeAnchor}</div>
                </Show>
              </EventCard>
              <EventCard
                title={tr(locale(), "目标执行状态", "Goal Execution")}
                badge={gameplay().executionStateLabel || gameplay().executionState || "-"}
                badgeClass={goalExecutionBadgeClass(gameplay().executionState)}
                meta={tr(locale(), "统一状态机：Accepted -> Executing -> Blocked / Completed / Rejected", "Unified state machine: Accepted -> Executing -> Blocked / Completed / Rejected")}
              >
                <div class="badge-row">
                  <For each={gameplay().executionStateMachine || []}>
                    {(item) => (
                      <Badge class={gameplay().executionState === item.id ? goalExecutionBadgeClass(item.id) : "badge"}>
                        {item.label}
                      </Badge>
                    )}
                  </For>
                </div>
                <div class="feedback-summary">
                  {gameplay().executionSummary
                    || tr(locale(), "等待目标执行状态更新。", "Waiting for goal execution state updates.")}
                </div>
                <Show when={gameplay().executionCauseLabel}>
                  <div class="badge-row">
                    <Badge>{gameplay().executionCauseLabel}</Badge>
                  </div>
                </Show>
                <Show when={gameplay().executionCauseDetail}>
                  <div class="feedback-detail">{gameplay().executionCauseDetail}</div>
                </Show>
              </EventCard>
              <EventCard
                title={gameplay().goalTitle || tr(locale(), "当前目标", "Current Goal")}
                badge={gameplay().progressPercent == null ? "n/a" : `${gameplay().progressPercent}%`}
                badgeClass="badge badge--accent"
                meta={gameplay().objective || tr(locale(), "当前还没有目标说明。", "No objective text yet.")}
              >
                <Show when={gameplay().progressDetail}>
                  <div class="feedback-detail">{gameplay().progressDetail}</div>
                </Show>
                <Show when={gameplay().blockerKind || gameplay().narrativeBlockerDetail}>
                  <div class="badge-row badge-row--spaced">
                    <Badge class="badge badge--warn">
                      {gameplay().blockerLabel || gameplay().blockerKind || tr(locale(), "当前阻塞", "Current Blocker")}
                    </Badge>
                  </div>
                  <div class="feedback-detail">
                    {gameplay().narrativeBlockerDetail || tr(locale(), "当前玩法被阻塞，需要显式恢复。", "Gameplay is blocked and needs explicit recovery.")}
                  </div>
                </Show>
                <Show when={gameplay().blockerSupplementalDetail}>
                  <div class="feedback-detail">{gameplay().blockerSupplementalDetail}</div>
                </Show>
                <div class="badge-row badge-row--spaced">
                  <Badge class="badge badge--accent">{tr(locale(), "下一步", "Next Step")}</Badge>
                </div>
                <div class="feedback-summary">
                  {gameplay().narrativeNextStep || tr(locale(), "等待下一次运行时指引更新。", "Wait for the next runtime guidance update.")}
                </div>
                <Show when={gameplay().branchHint}>
                  <div class="feedback-detail">{gameplay().branchHint}</div>
                </Show>
                <Show when={gameplay().entityCounts}>
                  <div class="badge-row">
                    <Badge>{`agents=${gameplay().entityCounts.agents}`}</Badge>
                    <Badge>{`locations=${gameplay().entityCounts.locations}`}</Badge>
                  </div>
                </Show>
              </EventCard>
              <FallbackTradeoffPanel options={gameplay().fallbackTradeoffPreview} noSafeFallbackHandoff={gameplay().noSafeFallbackHandoff} locale={locale()} tr={tr} />
              <Show when={gameplay().validationUnlockPreview}>
                {(preview) => (
                  <EventCard
                    title={tr(locale(), "产品验证预览", "Product Validation Preview")}
                    badge={preview().stageStatusLabel || "unknown"}
                    badgeClass={preview().stageStatus === "available" ? "badge badge--good" : "badge badge--warn"}
                    meta={preview().localizedValueSummary || tr(locale(), "验证结果未声明新的能力；请根据现有角色和阶段决定下一步。", "The validation result declares no new capability; use the existing role and stage to choose the next move.")}
                  >
                    <div class="feedback-summary" data-testid="validation-unlock-preview">
                      {`${preview().productId || tr(locale(), "未知产品", "Unknown product")} · ${preview().roleLabel || tr(locale(), "未知", "unknown")} · ${preview().tradable ? tr(locale(), "可交易", "tradable") : tr(locale(), "不可交易", "not tradable")}`}
                    </div>
                    <div class="feedback-detail">
                      {`${tr(locale(), "阶段", "Stage")}: ${preview().currentStageLabel || tr(locale(), "未知", "unknown")} / ${preview().requiredStageLabel || tr(locale(), "未知", "unknown")}`}
                    </div>
                    <Show when={preview().localizedNextStepHint}>
                      <div class="feedback-detail">{preview().localizedNextStepHint}</div>
                    </Show>
                  </EventCard>
                )}
              </Show>
              <PanelSection
                title={tr(locale(), "能力经济可读性", "Capability Economics")}
                eyebrow={tr(locale(), "下一步会带来什么", "What The Next Move Changes")}
                meta={tr(locale(), "把当前玩法拆成投入、产出、新用途、修复动作和下一步效果，帮助玩家判断现在该补资源、推进一步，还是换目标。", "Break the current loop into input, output, new use, repair move, and next effect so the player can choose whether to refill resources, advance one step, or switch targets.")}
              >
                <div class="summary-grid">
                  <MetricCard
                    label={tr(locale(), "投入", "Input")}
                    value={gameplay().economicSurface?.input || tr(locale(), "待发布", "not published")}
                  />
                  <MetricCard
                    label={tr(locale(), "产出", "Output")}
                    value={gameplay().economicSurface?.output || tr(locale(), "待发布", "not published")}
                  />
                  <MetricCard
                    label={tr(locale(), "新用途", "New Use")}
                    value={gameplay().economicSurface?.unlockedValue || tr(locale(), "待发布", "not published")}
                  />
                  <MetricCard
                    label={tr(locale(), "修复动作", "Repair Move")}
                    value={gameplay().economicSurface?.repairAction || tr(locale(), "待发布", "not published")}
                    detail={gameplay().economicSurface?.blockerLabel
                      ? tr(locale(), `当前阻塞归类: ${gameplay().economicSurface.blockerLabel}`, `Current blocker class: ${gameplay().economicSurface.blockerLabel}`)
                      : null}
                  />
                  <MetricCard
                    label={tr(locale(), "下一步价值", "Next Value")}
                    value={gameplay().economicSurface?.nextValue || tr(locale(), "待发布", "not published")}
                  />
                </div>
              </PanelSection>
              <MicroDepotFacilitiesPanel
                facilities={gameplay().microDepotFacilities}
                locale={locale}
                tr={tr}
              />
              <RefineQuoteGameplayPanel core={core} locale={locale()} tr={tr} />
              <PowerSurvivalQuoteGameplayPanel core={core} locale={locale()} tr={tr} />
              <MarketQuoteDecisionGameplayPanel core={core} locale={locale()} tr={tr} />
              <Show when={gameplay().agentClaim}>
                <ClaimAgentChoiceCard
                  locale={locale()}
                  claim={gameplay().agentClaim}
                  availableActions={gameplay().availableActions}
                />
              </Show>
              <Show when={expansionBranchCards(gameplay(), locale()).length > 0}>
                <ExpansionTradeoffCards gameplay={gameplay()} locale={locale()} />
              </Show>
              <Show when={gameplay().recentFeedback}>
                {(feedback) => (
                  <EventCard
                    title={tr(locale(), "最近玩法反馈", "Recent Gameplay Feedback")}
                    badge={feedback().stage || "-"}
                    badgeClass={
                      feedback().stage === "blocked" ? "badge badge--warn" : "badge badge--good"
                    }
                    meta={
                      feedback().action
                        ? tr(locale(), `来自动作 ${feedback().action}`, `From action ${feedback().action}`)
                        : tr(locale(), "最近一条玩法回执", "Most recent gameplay feedback")
                    }
                  >
                    <div class="feedback-summary">
                      {feedback().effect
                        || feedback().reason
                        || tr(locale(), "最新回执已更新，但还没有新的世界级后果。", "The latest feedback is in, but there is no new world-level consequence yet.")}
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
              <Show when={!starterOcGateOpen() && gameplayActionFeedback()}>
                {(feedback) => (
                  <FeedbackCard
                    feedback={feedback()}
                    feedbackStage={feedback().stage}
                    display={gameplayActionFeedbackDisplay()}
                    liveRegion
                  />
                )}
              </Show>
              <ProductValidationQuoteGameplayPanel core={core} locale={locale()} tr={tr} />
              <Show when={gameplay().recommendedAction}>
                {(action) => (
                  <CalloutCard
                    title={tr(locale(), "推荐动作", "Recommended Action")}
                    badge={action().executeKind || "ready"}
                    badgeClass="badge badge--good"
                  >
                    <div class="feedback-summary">
                      {action().label || action().actionId || tr(locale(), "当前存在一条更合适的推进动作。", "One action is currently the best next move.")}
                    </div>
                    <div class="feedback-detail">
                      {gameplayActionDisabledReason(action(), gameplay(), locale())
                        || gameplayActionDetail(action(), gameplay(), locale())}
                    </div>
                    <div class="toolbar">
                      <button
                        data-testid={gameplayActionTestId(action(), "recommended")}
                        class={gameplayActionButtonClass(action())}
                        aria-busy={gameplayActionButtonBusyAttrs(action())}
                        disabled={gameplayActionButtonDisabled(action(), gameplay(), locale())}
                        onClick={() => renderGameplayAction(action())}
                      >
                        {gameplayActionDisplayLabel(action(), locale())}
                      </button>
                    </div>
                  </CalloutCard>
                )}
              </Show>
              <Show when={!shouldShowStarterOcRequiredGate(gameplay()) && (gameplay().recommendedAction?.actionId === "claim_starter_oc" || starterOcAction(gameplay()))}>
                <CalloutCard
                  title={tr(locale(), "领取第一笔 OC", "Claim Your First OC")}
                  badge={starterOcAction(gameplay()) ? "ready" : "next"}
                  badgeClass={starterOcAction(gameplay()) ? "badge badge--good" : "badge badge--accent"}
                >
                  <StarterOcOnboardingPanel gameplay={gameplay()} locale={locale()} />
                </CalloutCard>
              </Show>
              <Show when={hasExecutableAgentClaim(core.state.snapshot, gameplay().agentClaim)}>
                <AgentClaimPanel gameplay={gameplay()} locale={locale()} />
              </Show>
              <Show when={hasAgentClaimSessionBoundary(gameplay().agentClaim)}>
                <AgentClaimSessionBoundaryCard gameplay={gameplay()} locale={locale()} />
              </Show>
              <div>
                <div class="panel__title panel__title--spaced">{tr(locale(), "可用玩法动作", "Available Gameplay Actions")}</div>
                <div class="action-grid">
                  <Show
                    when={visibleGameplayActionsForPanels(gameplay()).length > 0}
                    fallback={<EmptyState>{tr(locale(), "当前还没有发布规范玩法动作。", "No canonical gameplay actions published yet.")}</EmptyState>}
                  >
                    <For each={visibleGameplayActionsForPanels(gameplay())}>
                      {(action) => {
                        const disabledReason = () => gameplayActionDisabledReason(action, gameplay(), locale());
                        const actionState = () => (disabledReason() ? "blocked" : "ready");
                        const blockedReasonId = gameplayActionBlockedReasonId(action);
                        return (
                        <EventCard
                          class="event-card event-card--action"
                          actionState={actionState()}
                          title={action.label || action.actionId || "unknown_action"}
                          badge={gameplay().recommendedAction?.actionId === action.actionId
                            ? tr(locale(), "recommended", "recommended")
                            : disabledReason() ? tr(locale(), "受阻", "Blocked") : "ready"}
                          badgeClass={gameplay().recommendedAction?.actionId === action.actionId
                            ? "badge badge--accent"
                            : disabledReason() ? "badge badge--warn" : "badge badge--good"}
                          meta={
                            action.targetAgentId
                              ? tr(locale(), `作用对象 ${action.targetAgentId}`, `Acts on ${action.targetAgentId}`)
                              : tr(locale(), "世界级动作", "World-level action")
                          }
                        >
                          <Show
                            when={disabledReason()}
                            fallback={<div class="feedback-detail">{gameplayActionDetail(action, gameplay(), locale())}</div>}
                          >
                            <div class="feedback-detail" id={blockedReasonId}>{disabledReason()}</div>
                            <Show when={gameplay().nextStepHint}>
                              <div class="feedback-detail">{gameplay().nextStepHint}</div>
                            </Show>
                            <div class="feedback-summary">
                              <a href="#viewer-details-panel">
                                {tr(locale(), "重试前先查看下一步或玩法详情。", "Review Next Move or Gameplay Details before retrying.")}
                              </a>
                            </div>
                          </Show>
                          <Show
                            when={action.executeKind === "request_snapshot" || action.executeKind === "step" || action.executeKind === "play" || action.executeKind === "gameplay_action" || action.executeKind === "claim_first_agent" || action.executeKind === "claim_starter_oc"}
                          >
                            <div class="toolbar">
                              <button
                                data-testid={gameplayActionTestId(action)}
                                aria-label={action.label || action.actionId || undefined}
                                class={gameplayActionButtonClass(action)}
                                aria-busy={gameplayActionButtonBusyAttrs(action)}
                                disabled={gameplayActionButtonDisabled(action, gameplay(), locale())}
                                aria-describedby={disabledReason() ? blockedReasonId : undefined}
                                onClick={() => renderGameplayAction(action)}
                              >
                                {gameplayActionDisplayLabel(action, locale())}
                              </button>
                            </div>
                          </Show>
                          <Show when={action.executeKind === "reprioritize" && !disabledReason()}>
                            <ReprioritizeActionForm action={action} locale={locale()} tr={tr} observeState={observeViewerStateRevision} />
                          </Show>
                          <Show when={action.executeKind === "agent_chat"}>
                            <div class="toolbar">
                              <button
                                data-testid={gameplayActionTestId(action)}
                                aria-label={action.label || action.actionId || undefined}
                                class={gameplayActionButtonClass(action)}
                                aria-busy={gameplayActionButtonBusyAttrs(action)}
                                disabled={gameplayActionButtonDisabled(action, gameplay(), locale())}
                                aria-describedby={disabledReason() ? blockedReasonId : undefined}
                                onClick={() => renderGameplayAction(action)}
                              >
                                {gameplayActionDisplayLabel(action, locale())}
                              </button>
                            </div>
                          </Show>
                        </EventCard>
                        );
                      }}
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
      <Show when={showPlayerSessionSurface()}>
        <PanelSection
          title={tr(locale(), "进入会话", "Player Access")}
          eyebrow={tr(locale(), "只在需要时出现", "Only When Needed")}
          meta={tr(locale(), "只有当玩家会话缺失、重绑中或需要恢复时，这里才会打断主玩法路径。", "This only interrupts the main path when the player session is missing, rebinding, or needs recovery.")}
        >
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
            {(hint) => <EmptyState>{hint().detail}</EmptyState>}
          </Show>
          <Show
            when={
              !state.auth.available
              && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)
            }
          >
            <HostedLoginForm locale={locale()} />
          </Show>
          <Show when={state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE}>
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
      <details id="viewer-diagnostics-panel" class="panel diagnostic-surface" data-viewer-surface="diagnostics">
        <summary class="panel__header diagnostic-surface__summary">
          <div class="diagnostic-surface__title">
            <div class="panel__title">{tr(locale(), "运行诊断", "Runtime Diagnostics")}</div>
            <div class="diagnostic-surface__meta">
              {tr(
                locale(),
                "执行通道、认证/会话、托管矩阵与最近事件都收在这里，避免它们继续抢占主玩法首屏。",
                "Execution lanes, auth/session truth, hosted matrix, and recent events live here so they no longer dominate the primary gameplay viewport.",
              )}
            </div>
          </div>
          <div class="badge-row">
            <For each={diagnosticsSummaryBadges()}>
              {(label) => <Badge class="badge badge--diagnostic">{label}</Badge>}
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
          <PanelSection title={tr(locale(), "执行通道", "Execution Lanes")}>
            <Show
              when={core.selectedAgentExecutionDebugContext()}
              fallback={
                <EmptyState>
                  {tr(
                    locale(),
                    "先选中一个行动体，才能查看当前执行通道元数据。",
                    "Select an agent to inspect the current execution-lane metadata.",
                  )}
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
                  <EmptyState class="flow-lift--tight">
                    {tr(
                      locale(),
                      "上面的通道徽标表示 phase-1 期望执行契约；下面的提供方检查徽标表示 runtime_live 基于 /v1/provider/info 和 /v1/provider/health 的真实探测结果。",
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
          <div class="toolbar">
            <Show when={state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE}>
              <button
                data-auth-action="logout"
                onClick={() => {
                  void core.logoutHostedPlayerSession();
                }}
              >
                {tr(locale(), "释放玩家会话", "Release Player Session")}
              </button>
            </Show>
          </div>
          <Show when={hostedRecoveryHint()}>
            {(hint) => <EmptyState>{hint().detail}</EmptyState>}
          </Show>
          <Show
            when={
              !state.auth.available
              && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)
            }
          >
            <HostedLoginForm
              locale={locale()}
              channelId="diag-hosted-login-channel"
              handleId="diag-hosted-login-handle"
              codeId="diag-hosted-login-code"
            />
          </Show>
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
              {tr(
                locale(),
                "玩家会话正在切换到请求的行动体；注册成功后，当前动作会继续执行。",
                "Player session is switching to the requested agent and the current action will continue after registration succeeds.",
              )}
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
            <PanelSection title={tr(locale(), "会话阶梯", "Session Ladder")}>
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
                  "这里是启动器导出的托管公开加入真值面。质检应直接读取这些动作编号，而不是只靠按钮状态推断。",
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
            <MetricCard label={tr(locale(), "提示词反馈", "Prompt Feedback")} value={promptFeedback()?.stage || "idle"}>
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
            <div class="panel__title panel__title--spaced">{tr(locale(), "最近事件", "Recent Events")}</div>
            <div class="event-list">
              <Show when={state.recentEvents.length > 0} fallback={<EmptyState>{tr(locale(), "等待实时事件…", "Waiting for live events…")}</EmptyState>}>
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
      </details>
    </>
  );
}

function InteractionPanel() {
  const revision = () => observeViewerStateRevision();
  const locale = () => uiLocale();
  const selectedAgentId = () => {
    revision();
    return core.selectedAgentId();
  };
  const agentId = () => {
    const id = normalizedId(selectedAgentId());
    if (!id || !core.isAgentVisibleToCurrentSession(id)) {
      return null;
    }
    return id;
  };
  const gameplaySummary = () => {
    revision();
    return core.buildGameplaySummary(locale());
  };
  const authSurface = () => {
    revision();
    return core.buildAuthSurfaceModel();
  };
  const promptCapability = () => authSurface().capabilities.prompt_control;
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const mainTokenTransferCapability = () => authSurface().capabilities.main_token_transfer;
  const mainTokenTransferPolicy = () => core.hostedActionPolicy("main_token_transfer");
  const binding = () => {
    revision();
    return core.selectedAgentBindingInfo();
  };
  const selectedAgentStatus = () => describeAgentSessionStatus(agentId(), locale());
  const canControlSelectedAgent = () => selectedAgentStatus().isCurrentSessionAgent;
  const selectedAgentControlReason = () => selectedAgentStatus().detail;
  const promptFeedback = () => {
    revision();
    return core.snapshotSemanticFeedback(core.state.lastPromptFeedback);
  };
  const chatFeedback = () => {
    revision();
    return core.snapshotSemanticFeedback(core.state.lastChatFeedback);
  };
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const promptVersionState = () => core.describePromptVersionState(promptFeedback(), locale());
  const chatHistory = () =>
    {
      revision();
      return core.state.chatHistory
        .filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId())
        .slice(0, 12);
    };
  const interactionEnabled = () => promptCapability().enabled;
  const promptControlsEnabled = () => interactionEnabled() && canControlSelectedAgent();
  const chatControlsEnabled = () => {
    revision();
    return chatCapability().enabled && canControlSelectedAgent() && !core.isAgentChatInFlight();
  };
  const commandStarterOcAction = () => starterOcAction(gameplaySummary());
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(gameplaySummary());
  const promptOverridesVisible = () => {
    revision();
    return !!core.state.promptOverridesVisible;
  };
  const assetLaneStatusText = () =>
    mainTokenTransferCapability().enabled
      ? tr(locale(), "仅预览", "preview_only")
      : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () =>
    mainTokenTransferCapability().enabled
      ? tr(
          locale(),
          "契约表明这个通道具备 strong_auth 级 main_token_transfer 能力，但观察器这里仍然不会直接暴露转账表单。",
          "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here.",
        )
      : mainTokenTransferCapability().reason;
  const promptSettingsSummary = () =>
    promptOverridesVisible()
      ? tr(
          locale(),
          "高级提示词设置已展开；你可以继续做预览、应用、回滚，页面也会显示最近一次反馈。",
          "Advanced prompt settings are expanded; preview/apply/rollback and the latest prompt feedback are visible.",
        )
      : tr(
          locale(),
          "提示词覆盖默认收起，避免把操作员级编辑控件直接堆在主入口。显式展开后仍可做预览、应用、回滚，`__AW_TEST__.sendPromptControl(...)` 也保持可用。",
          "Prompt Overrides stay hidden by default so operator-level editing controls do not dominate the primary entry. Expanding them keeps preview/apply/rollback available, and `__AW_TEST__.sendPromptControl(...)` remains available.",
        );
  const promptSettingsButtonLabel = () =>
    promptOverridesVisible()
      ? tr(locale(), "收起提示词覆盖", "Hide Prompt Overrides")
      : tr(locale(), "显示提示词覆盖", "Show Prompt Overrides");
  const playerSessionReadyCopy = () =>
    tr(
      locale(),
      "当前 Agent 已绑定到你的本地玩家会话。可以直接发送第一条聊天指令；提示词和资产治理能力先收在后置区域。",
      "This Agent is bound to your local player session. Send the first chat command here; prompt and asset/governance controls stay in the deferred area.",
    );
  const commandBoundaryCopy = () =>
    canControlSelectedAgent()
      ? playerSessionReadyCopy()
      : selectedAgentControlReason();

  return (
    <Show
      when={agentId()}
      fallback={
        <Show
          when={selectedAgentId()}
          fallback={
            <Show
              when={gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities"}
              fallback={<EmptyState>{tr(locale(), "先选中一个行动体，才能解锁提示词和聊天控制。", "Select an agent to unlock prompt/chat controls.")}</EmptyState>}
            >
              <EmptyEntityRecoveryCard locale={locale()} gameplay={gameplaySummary} />
            </Show>
          }
        >
          <EmptyState>
            {tr(
              locale(),
              "当前账号还没有可操作的 Agent。请先认领你的第一个 Agent，或等待绑定同步完成。",
              "This account has no controllable Agent yet. Claim your first Agent, or wait for binding sync to complete.",
            )}
          </EmptyState>
        </Show>
      }
    >
      <div class="stack command-surface" data-command-agent={agentId()} data-command-chat-history={String(chatHistory().length)}>
      <div class="badge-row command-surface__target-row">
        <Badge class="badge badge--accent">{tr(locale(), "当前交互目标", "Current Target")}</Badge>
        <Badge>{`agent=${agentId()}`}</Badge>
        <Badge class={selectedAgentStatus().badgeClass}>{selectedAgentStatus().badge}</Badge>
        <Badge class={chatControlsEnabled() ? "badge badge--good" : "badge badge--warn"}>
          {chatControlsEnabled() ? tr(locale(), "聊天可用", "Chat Ready") : tr(locale(), "聊天受限", "Chat Limited")}
        </Badge>
      </div>
      <Show
        when={interactionEnabled() && canControlSelectedAgent()}
        fallback={
          <EmptyState class="command-surface__auth-boundary">
            {commandBoundaryCopy()}
          </EmptyState>
        }
      >
        <div class="badge-row command-surface__auth-boundary">
          <Badge class="badge badge--good">{authSurface().currentTier}</Badge>
          <Badge>{`player=${core.state.auth.playerId}`}</Badge>
          <Badge>{`source=${authSurface().source}`}</Badge>
        </div>
        <EmptyState class="command-surface__auth-boundary">{playerSessionReadyCopy()}</EmptyState>
      </Show>
      <div class="badge-row command-surface__capability-row command-surface__diagnostic-strip">
        <Badge class="badge badge--diagnostic">{`boundPlayer=${binding()?.playerId || "-"}`}</Badge>
        <Badge class="badge badge--diagnostic">{`boundKey=${binding()?.publicKey ? `${binding().publicKey.slice(0, 10)}…` : "-"}`}</Badge>
        <Badge class={promptControlsEnabled() ? "badge badge--good" : "badge badge--warn"}>
          {`prompt=${promptControlsEnabled() ? "enabled" : promptCapability().code || "agent_not_bound"}`}
        </Badge>
        <Badge class={chatControlsEnabled() ? "badge badge--good" : "badge badge--warn"}>
          {`chat=${chatControlsEnabled() ? "enabled" : chatCapability().code || "agent_not_bound"}`}
        </Badge>
        <Badge class={mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
          {`mainToken=${assetLaneStatusText()}`}
        </Badge>
      </div>
      <EmptyState class="command-surface__asset-boundary">{assetLaneDetail()}</EmptyState>
      <Show when={!starterOcGateOpen() && canControlSelectedAgent() && commandStarterOcAction()}>
        {(action) => (
          <div class="toolbar">
            <button
              class={gameplayActionButtonClass(action())}
              aria-busy={gameplayActionButtonBusyAttrs(action())}
              disabled={gameplayActionButtonDisabled(action(), gameplaySummary(), locale())}
              onClick={() => renderGameplayAction(action())}
            >
              {gameplayActionDisplayLabel(action(), locale())}
            </button>
          </div>
        )}
      </Show>
      <PanelSection
        class="command-surface__chat-panel"
        title={tr(locale(), "行动体聊天", "Agent Chat")}
        eyebrow={tr(locale(), "指挥面板", "Command Surface")}
        meta={tr(locale(), "向当前目标发消息并读回复。", "Message the current target and read replies.")}
      >
        <div class="field">
          <label for="agent-chat-message">{tr(locale(), "消息", "Message")}</label>
              <textarea
                id="agent-chat-message"
                rows="4"
                placeholder={tr(locale(), "给当前选中的行动体发一条消息", "Send a message to the selected agent")}
                disabled={!chatControlsEnabled()}
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
              disabled={!chatControlsEnabled()}
              onClick={() => core.sendAgentChat(agentId(), core.state.chatDraft.message)}
            >
            {tr(locale(), "发送聊天", "Send Chat")}
          </button>
        </div>
        <Show when={chatFeedback()} fallback={<EmptyState>{tr(locale(), "还没有聊天反馈。", "No chat feedback yet.")}</EmptyState>}>
          {(feedback) => <FeedbackCard feedback={feedback()} display={chatFeedbackDisplay()} />}
        </Show>
        <div>
          <div class="panel__title panel__title--spaced">{tr(locale(), "消息流", "Message Flow")}</div>
          <div class="event-list">
            <Show when={chatHistory().length > 0} fallback={<EmptyState>{tr(locale(), "这个行动体还没有聊天历史。", "No chat history for this agent yet.")}</EmptyState>}>
              <For each={chatHistory()}>
                {(entry) => (
                  <EventCard
                    class={chatEntryCardClass(entry)}
                    title={chatEntryTitle(entry, locale())}
                    badge={`tick=${Number(entry.tick || 0)}`}
                    meta={chatEntryMeta(entry, locale())}
                  >
                    <div class="feedback-summary">{chatEntryMessage(entry, locale())}</div>
                    <DiagnosticDetails value={entry} />
                  </EventCard>
                )}
              </For>
            </Show>
          </div>
        </div>
      </PanelSection>
      <PanelSection
        class="command-surface__advanced-panel"
        title={tr(locale(), "高级提示词设置", "Advanced Prompt Settings")}
        eyebrow={tr(locale(), "高级控制", "Advanced Controls")}
        meta={tr(locale(), "保留操作员级提示词控制，但默认收起，不与玩家主路径竞争。", "Operator-level prompt controls stay available here, but collapsed by default so they do not compete with the player path.")}
      >
        <div class="badge-row">
          <Badge>{`activePrompt=v${promptVersionState().currentVersion}`}</Badge>
          <Badge>{`nextRollback=v${promptVersionState().nextRollbackTargetVersion}`}</Badge>
          <Show when={promptVersionState().restoredFromVersion != null}>
            <Badge>{`restoredFrom=v${promptVersionState().restoredFromVersion}`}</Badge>
          </Show>
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
              disabled={!canControlSelectedAgent()}
              onClick={() => core.togglePromptOverridesVisible()}
            >
            {promptSettingsButtonLabel()}
          </button>
        </div>
      </PanelSection>
      <Show when={promptOverridesVisible()}>
        <PanelSection title={tr(locale(), "提示词覆盖", "Prompt Overrides")}>
          <div class="feedback-detail">{promptVersionState().summary}</div>
          <div class="feedback-detail">{promptVersionState().detail}</div>
          <Show
            when={
              authSurface().capabilities.prompt_control.enabled
              && isHostedPublicJoinDeploymentMode(core.state.hostedAccess?.deployment_mode)
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
            <label for="prompt-system">{tr(locale(), "系统提示词覆盖", "System Prompt Override")}</label>
            <textarea
                id="prompt-system"
                rows="4"
                disabled={!promptControlsEnabled()}
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
                disabled={!promptControlsEnabled()}
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
                disabled={!promptControlsEnabled()}
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
                disabled={!promptControlsEnabled()}
                onClick={() => core.sendPromptControl("preview", null)}
            >
              {tr(locale(), "预览提示词", "Preview Prompt")}
            </button>
              <button
                data-prompt-action="apply"
                disabled={!promptControlsEnabled()}
                onClick={() => core.sendPromptControl("apply", null)}
            >
              {tr(locale(), "应用提示词", "Apply Prompt")}
            </button>
          </div>
          <div class="toolbar">
            <div class="field field--inline-flex">
              <label for="prompt-rollback-version">{tr(locale(), "下一次回滚目标版本", "Next Rollback Target Version")}</label>
              <input
                id="prompt-rollback-version"
                  type="number"
                  min="0"
                  step="1"
                  disabled={!promptControlsEnabled()}
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
                disabled={!promptControlsEnabled()}
                onClick={() => {
                core.sendPromptControl("rollback", {
                  toVersion: Number(core.state.promptDraft.rollbackTargetVersion || 0),
                });
              }}
            >
              {tr(locale(), "回滚提示词", "Rollback Prompt")}
            </button>
          </div>
          <Show when={promptFeedback()} fallback={<EmptyState>{tr(locale(), "还没有提示词反馈。", "No prompt feedback yet.")}</EmptyState>}>
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
            <EmptyState class="empty--danger">{core.state.strongAuth.lastGrantError}</EmptyState>
          </Show>
        </PanelSection>
      </Show>
      <PanelSection
        class="command-surface__asset-panel"
        title={tr(locale(), "资产 / 治理通道", "Asset / Governance Lane")}
        eyebrow={tr(locale(), "后置能力", "Deferred Surface")}
        meta={tr(locale(), "这类能力保留在右侧底部，只作为边界说明，不再抢占聊天与主玩法路径。", "These capabilities stay at the bottom of the right column as boundary guidance instead of competing with chat and the main player path.")}
      >
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
            || tr(locale(), "当前通道没有 main_token_transfer 的托管动作策略。", "No hosted action policy is available for main_token_transfer on this lane.")}
        </EmptyState>
        <div class="toolbar">
          <button disabled>{tr(locale(), "主代币转账（这里暂未开放）", "Main Token Transfer (Not Exposed Here Yet)")}</button>
        </div>
      </PanelSection>
      </div>
    </Show>
  );
}

function DetailsPanel() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const worldScaleSurface = () => core.buildWorldScaleSurface(locale());
  const worldMetaSummary = () => {
    const physicalTruth = worldScaleSurface().physicalTruth;
    const segments = [];
    if (physicalTruth.worldBoundsLabel) {
      segments.push(
        tr(locale(), "世界边界", "World Bounds") + ` ${physicalTruth.worldBoundsLabel}`,
      );
    }
    const nearestLocation = physicalTruth.nearestLocations[0];
    if (nearestLocation?.distanceLabel) {
      segments.push(
        tr(locale(), "最近距离", "Nearest") + ` ${nearestLocation.distanceLabel}`,
      );
    }
    return segments.length > 0 ? segments.join(" · ") : tr(locale(), "当前未发布世界尺度摘要。", "No world scale summary is published yet.");
  };
  const selectedLabel = () =>
    core.state.selectedKind && core.state.selectedId && !hiddenSelectedAgent()
      ? `${core.state.selectedKind}:${core.state.selectedId}`
      : tr(locale(), "未选择", "nothing selected");
  const hiddenSelectedAgent = () =>
    core.state.selectedKind === "agent"
    && core.state.selectedId
    && !core.isAgentVisibleToCurrentSession(core.state.selectedId);
  const hasVisibleSelectedObject = () => core.state.selectedObject && !hiddenSelectedAgent();
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
        <Badge class="badge badge--accent">{tr(locale(), "当前命令目标", "Current Command Target")}</Badge>
        <Badge>{selectedLabel()}</Badge>
      </div>
      <Show
        when={!hiddenSelectedAgent()}
        fallback={
          <EmptyState>
            {tr(
              locale(),
              "当前账号还没有可控 Agent。请先完成认领或等待自己的 Agent 绑定同步。",
              "The current account has no controllable Agent yet. Claim one or wait for your own Agent binding to sync.",
            )}
          </EmptyState>
        }
      >
        <InteractionPanel />
      </Show>
      <Show
        when={hasVisibleSelectedObject()}
        fallback={
          gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities"
            ? (
              <EmptyEntityRecoveryCard
                locale={locale()}
                gameplay={gameplaySummary}
                title={tr(locale(), "对象明细暂时不可用", "Object Details Are Temporarily Unavailable")}
              />
            )
            : <EmptyState>{tr(locale(), "请先从左侧列表选一个行动体或地点。", "Select an agent or location from the left list.")}</EmptyState>
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
        <div class="panel__title panel__title--spaced">{tr(locale(), "世界规模", "World Scale")}</div>
        <div class="badge-row">
          <Badge>{`agents=${snapshotCounts().agents}`}</Badge>
          <Badge>{`locations=${snapshotCounts().locations}`}</Badge>
          <Badge>{`promptProfiles=${snapshotCounts().promptProfiles}`}</Badge>
          <Badge>{`debugContexts=${snapshotCounts().executionDebugContexts}`}</Badge>
          <Badge>{tr(locale(), "snapshot.config.space", "snapshot.config.space")}</Badge>
        </div>
        <div class="feedback-detail flow-top">{worldMetaSummary()}</div>
        <Show when={hasSnapshotDiagnostics()}>
          <DiagnosticDetails
            locale={locale()}
            label={tr(locale(), "展开原始快照诊断", "Expand Raw Snapshot Diagnostics")}
            note={tr(
              locale(),
              "只在需要排查快照结构或托管接入原始字段时展开。",
              "Expand only when you need to inspect the raw snapshot shape or hosted access fields.",
            )}
            value={snapshotSummary}
          />
        </Show>
      </div>
      <Show when={core.state.lastError}>
        <div>
          <div class="panel__title panel__title--spaced panel__title--danger">{tr(locale(), "最近错误", "Last Error")}</div>
          <pre class="json">{core.state.lastError}</pre>
        </div>
      </Show>
    </div>
  );
}

function AppShell() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const diagnosticsVisualFixture = () => viewerVisualFixtureNameFromQuery() === "gameplay_diagnostics_expanded";
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(core.buildGameplaySummary(locale()));
  return (
    <>
      <MobileJumpRail />
      <HostedLoginGate />
      <StarterOcRequiredGate />
      <section
        class="panel panel--targets"
        id="viewer-targets-panel"
        data-viewer-surface="targets"
        aria-hidden={starterOcGateOpen() ? "true" : undefined}
        inert={starterOcGateOpen() ? true : undefined}
      >
        <div class="panel__header panel__header--stack">
          <div class="panel__eyebrow">{tr(locale(), "导航", "Navigate")}</div>
          <div class="panel__title">{tr(locale(), "目标", "Targets")}</div>
          <div class="panel__meta-copy">{tr(locale(), "先锁定对象，再进入世界舞台或右侧指挥面板。", "Lock onto a target first, then move into the stage or command surface.")}</div>
        </div>
        <div class="panel__body">
          <TargetsPanel />
        </div>
      </section>
      <section
        class="panel panel--stage"
        id="viewer-stage-panel"
        data-viewer-surface="stage"
        aria-hidden={starterOcGateOpen() ? "true" : undefined}
        inert={starterOcGateOpen() ? true : undefined}
      >
        <div class="panel__body panel__body--stage">
          <div class="stack">
            <Show when={diagnosticsVisualFixture()}>
              <WorldSummaryPanel />
            </Show>
            <WorldStageHero />
            <PixelWorldHost locale={locale()} />
            <Show when={!diagnosticsVisualFixture()}>
              <WorldSummaryPanel />
            </Show>
          </div>
        </div>
      </section>
      <section
        class="panel panel--details"
        id="viewer-details-panel"
        data-viewer-surface="command"
        aria-hidden={starterOcGateOpen() ? "true" : undefined}
        inert={starterOcGateOpen() ? true : undefined}
      >
        <div class="panel__header panel__header--stack">
          <div class="panel__eyebrow">{tr(locale(), "指挥与核查", "Command and Inspect")}</div>
          <div class="panel__title">{tr(locale(), "交互与明细", "Interact and Inspect")}</div>
          <div class="panel__meta-copy">
            {tr(locale(), "只有锁定目标后才进入这里。聊天优先，提示词与对象核查继续后置。", "Enter this column only after locking a target. Chat comes first; prompt controls and raw inspection stay behind it.")}
          </div>
        </div>
        <div class="panel__body">
          <DetailsPanel />
        </div>
      </section>
    </>
  );
}

export { AppShell };

function viewerVisualFixtureNameFromQuery() {
  return viewerTestApiEnabled()
    ? String(new URLSearchParams(window.location.search || "").get("viewer_visual_fixture") || "").trim() || null
    : null;
}

function viewerTestApiEnabled() {
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}

function viewerFixtureBaseSnapshot(overrides = {}) {
  const base = {
    time: 12,
    config: {
      space: {
        width_cm: 10_000_000,
        depth_cm: 5_000_000,
        height_cm: 1_000_000,
      },
    },
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Agent 0",
          location_id: "loc-0",
          pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 },
          resources: { alloy: 3 },
        },
        "agent-1": {
          id: "agent-1",
          name: "Agent 1",
          location_id: "loc-1",
          pos: { x_cm: 6_900_000, y_cm: 1_150_000, z_cm: 0 },
          resources: {},
        },
      },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Factory Anchor",
          pos: { x_cm: 7_150_000, y_cm: 2_200_000, z_cm: 0 },
          profile: { radius_cm: 55_000, radiation_emission_per_tick: 0, material: "silicate" },
          fragment_profile: {
            blocks: {
              blocks: [
                {
                  origin_cm: { x_cm: -36_000, y_cm: 0, z_cm: -22_000 },
                  size_cm: { x_cm: 28_000, y_cm: 7_500, z_cm: 20_000 },
                  density_kg_per_m3: 3200,
                  compounds: { ppm: { silicate_matrix: 800_000, water_ice: 200_000 } },
                },
                {
                  origin_cm: { x_cm: 4_000, y_cm: 1_000, z_cm: -12_000 },
                  size_cm: { x_cm: 42_000, y_cm: 8_000, z_cm: 18_000 },
                  density_kg_per_m3: 7800,
                  compounds: { ppm: { iron_nickel_alloy: 900_000, sulfide_ore: 100_000 } },
                },
                {
                  origin_cm: { x_cm: -18_000, y_cm: 500, z_cm: 18_000 },
                  size_cm: { x_cm: 34_000, y_cm: 6_000, z_cm: 24_000 },
                  density_kg_per_m3: 5200,
                  compounds: { ppm: { sulfide_ore: 620_000, hydrated_mineral: 380_000 } },
                },
                {
                  origin_cm: { x_cm: 30_000, y_cm: 0, z_cm: 24_000 },
                  size_cm: { x_cm: 22_000, y_cm: 4_500, z_cm: 16_000 },
                  density_kg_per_m3: 2600,
                  compounds: { ppm: { silicate_matrix: 700_000, rare_earth_oxide: 300_000 } },
                },
              ],
            },
          },
          resources: { iron: 0 },
        },
        "loc-1": {
          id: "loc-1",
          name: "Assembly Nexus",
          pos: { x_cm: 4_550_000, y_cm: 1_200_000, z_cm: 0 },
          profile: { radius_cm: 38_000, radiation_emission_per_tick: 0, material: "alloy" },
          resources: {},
        },
      },
      agent_prompt_profiles: {
        "agent-0": {
          agent_id: "agent-0",
          version: 3,
          updated_by: "viewer-bound",
          system_prompt: "Keep the first production line recoverable.",
          short_term_goal: "Report the blocker and wait for material recovery.",
          long_term_goal: "Restore sustainable capability without inventing extra automation.",
        },
      },
      agent_execution_debug_contexts: {
        "agent-0": {
          provider_mode: "runtime_live",
          execution_mode: "phase_1",
          environment_class: "software_safe_viewer",
          observation_schema_version: "viewer.v1",
          action_schema_version: "agent_chat.v1",
          agent_profile: "default",
          provider_check_status: "ok",
          provider_check_source: "fixture",
          fallback_reason: null,
          provider_reported_capabilities: ["agent_chat"],
          provider_reported_supported_action_sets: ["agent_chat"],
        },
      },
      agent_player_bindings: {
        "agent-0": "viewer-bound",
        "agent-1": "viewer-other",
      },
      agent_player_public_key_bindings: {
        "agent-0": "oc:pk:viewer-session-key",
        "agent-1": "oc:pk:viewer-other-session-key",
      },
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      execution_state: "blocked",
      accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
      intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
      intent_scope: "gameplay_action",
      intent_target: "agent-0",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      causality_kind: "world_constraint",
      causality_detail: "iron input exhausted at factory-0",
      last_world_change: "Smelter build request reached factory-0; iron shortage blocks construction.",
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      recovery_path_kind: "repair_rebuild_or_pivot",
      recovery_path_detail: "Choose the local recovery path that best fits the current constraint.",
      major_power_dependency_status: "independent_path_available",
      repair_available: true,
      rebuild_available: true,
      pivot_available: true,
      recovery_options: recoveryOptionVisualFixture(), fallback_tradeoff_preview: fallbackTradeoffVisualFixture(),
      no_safe_fallback_reason: "No repair or reroute action is currently available for this blocked intent.", required_next_decision_action_id: "return_to_goal_selection", required_next_decision_class: "return_to_goal_selection",
      available_actions: [
        {
          action_id: "build_factory_smelter_mk1",
          target_agent_id: "agent-0",
          label: "Build smelter mk1",
          protocol_action: "gameplay_action.submit",
          disabled_reason: null,
        },
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          disabled_reason: null,
        },
      ],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2,
      },
      agent_claim: null,
      micro_depot_facilities: [
        {
          facility_id: "depot-regional-01",
          owner_claim_id: "claim-regional-01",
          status: "active",
          location_id: "loc-1",
          service_radius_cm: 250_000,
          inventory_revision: 7,
          available_units_by_kind: { data: 5, repair_kit: 2 },
          throughput_epoch: 11,
          throughput_remaining_units: 13,
          throughput_limit_units_per_epoch: 16,
          supported_resource_kinds: ["data", "repair_kit"],
          module_id: "regional.micro_depot",
          module_version: "0.2.0",
          wasm_hash: "sha256:micro-depot-public-evidence-1234567890",
          upkeep_paid: true,
          last_receipt_id: "receipt-micro-depot-public-01",
          last_proposal_hash: "sha256:proposal-public-01",
          available_actions: ["service_micro_depot_repair", "reclaim_micro_depot"],
        },
      ],
    },
  };
  return {
    ...base,
    ...overrides,
    config: {
      ...base.config,
      ...(overrides.config || {}),
    },
    model: {
      ...base.model,
      ...(overrides.model || {}),
    },
    player_gameplay: {
      ...base.player_gameplay,
      ...(overrides.player_gameplay || {}),
    },
  };
}

function emptyWorldRecoverySnapshot() {
  return viewerFixtureBaseSnapshot({
    model: {
      agents: {},
      locations: {},
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
      agent_player_bindings: {},
      agent_player_public_key_bindings: {},
    },
    player_gameplay: {
      stage_id: "world_bootstrap",
      stage_status: "blocked",
      execution_state: "blocked",
      goal_kind: "RecoverCapability",
      goal_title: "Recover world snapshot",
      objective: "Recover the world before issuing commands.",
      progress_detail: "No agents or locations are available in the current snapshot.",
      progress_percent: 0,
      blocker_kind: "runtime_snapshot_empty_entities",
      blocker_detail: "The viewer is missing a valid world snapshot.",
      causality_kind: "world_constraint",
      causality_detail: "empty snapshot contains zero agents and zero locations",
      next_step_hint: "Request a fresh snapshot; if entity counts stay at zero, repair or restart the runtime world bootstrap.",
      available_actions: [
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          disabled_reason: null,
        },
      ],
      recent_feedback: null,
      agent_claim: null,
    },
  });
}

function setFixturePlayerAuth() {
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "viewer-bound",
    publicKey: "oc:pk:viewer-session-key",
    privateKey: "ed25519-fixture-private-key",
    releaseToken: "fixture-release-token",
    source: "hosted_browser_storage",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
  };
}

function setFixtureChatHistory() {
  core.state.chatDraft.message = "Report nearby resources.";
  core.state.chatDraft.dirty = true;
  core.state.chatHistory = [
    {
      id: "fixture-chat-5",
      source: "agent",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "agent-0",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Awaiting material recovery before the smelter can proceed.",
      tick: 12,
      intentSeq: 5,
    },
    {
      id: "fixture-chat-4",
      source: "player",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "viewer-bound",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Hold position and confirm the blocker.",
      tick: 11,
      intentSeq: 4,
    },
    {
      id: "fixture-chat-3",
      source: "agent",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      speaker: "agent-0",
      playerId: "viewer-bound",
      locationId: "loc-0",
      message: "Factory Anchor reports iron input exhausted.",
      tick: 10,
      intentSeq: 3,
    },
  ];
  core.state.lastChatFeedback = {
    channel: "agent_chat",
    action: "agent_chat",
    stage: "acknowledged",
    ok: true,
    accepted: true,
    target: "agent-0",
    summary: "Agent chat acknowledged by the viewer fixture.",
    detail: "Recent message flow remains visible while prompt controls stay collapsed.",
    code: null,
  };
}

function setFixtureDiagnostics() {
  core.state.recentEvents = [
    { id: 24, time: 12, kind: { type: "state_sync", status: "ok" } },
    { id: 23, time: 12, kind: { type: "intent_tick", status: "blocked" } },
    { id: 22, time: 11, kind: { type: "econ_update", status: "material_shortage" } },
  ];
  core.state.eventCount = core.state.recentEvents.length;
  core.state.metrics = {
    total_ticks: 12,
    decision_trace_count: 1,
  };
}

function setFixtureHostedGate() {
  core.state.hostedAccess = {
    deployment_mode: HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
    action_matrix: [
      {
        action_id: "prompt_control_apply",
        required_auth: "strong_auth",
        availability: "public_player_plane_with_backend_reauth_preview",
        reason: "prompt_control_apply is available after browser player-session registration plus backend re-authorization",
      },
      {
        action_id: "main_token_transfer",
        required_auth: "strong_auth",
        availability: "blocked_until_strong_auth",
        reason: "main_token_transfer remains blocked; this viewer exposes no transfer form.",
      },
    ],
  };
  core.state.auth = {
    ...core.state.auth,
    available: false,
    playerId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    source: "guest_only",
    registrationStatus: "guest",
    runtimeStatus: "guest",
    error: "session validation requires hosted login",
  };
  core.state.hostedLogin.handle = "player@example.com";
  core.state.hostedLogin.challengeId = "fixture-challenge";
  core.state.hostedLogin.maskedLoginHint = "p***@example.com";
  core.state.hostedLogin.deliveryMode = "email";
  core.state.hostedLogin.accountExists = true;
  core.state.hostedLogin.error = "Enter the latest verification code to continue.";
  core.state.hostedLogin.retryAfterSeconds = 18;
}

function openFixtureDetails(name) {
  queueMicrotask(() => {
    if (name === "gameplay_diagnostics_expanded") {
      document.getElementById("viewer-gameplay-details")?.setAttribute("open", "");
      document.getElementById("viewer-diagnostics-panel")?.setAttribute("open", "");
    }
  });
}

function installViewerVisualFixture() {
  if (!viewerTestApiEnabled()) {
    delete window[VIEWER_VISUAL_FIXTURE_GLOBAL];
    document.body.removeAttribute("data-viewer-visual-fixture");
    return null;
  }
  const fixtures = {
    shell_selected_blocker() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
    },
    agent_chat_history() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      core.setPromptOverridesVisible(false);
    },
    gameplay_diagnostics_expanded() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      setFixtureDiagnostics();
    },
    hosted_login_gate() {
      core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
      core.applySelection({ kind: "agent", id: "agent-0" });
      setFixtureHostedGate();
    },
    empty_world_recovery() {
      core.injectSnapshot(emptyWorldRecoverySnapshot(), { returnState: false });
      core.state.selectedKind = null;
      core.state.selectedId = null;
      core.state.selectedObject = null;
    },
  };
  installRefineQuotePreflightVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installProductValidationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installPowerSurvivalQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installMarketQuoteDecisionVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  installWaitResolutionQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot });
  window[VIEWER_VISUAL_FIXTURE_GLOBAL] = fixtures;

  const fixtureName = viewerVisualFixtureNameFromQuery();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  fixtures[fixtureName]();
  document.body.setAttribute("data-viewer-visual-fixture", fixtureName);
  openFixtureDetails(fixtureName);
  return fixtureName;
}

export function mountViewerApp(root = document.getElementById("app")) {
  if (!root) {
    throw new Error("viewer root #app is missing");
  }

  core.initializeSoftwareSafeCore();
  const viewerVisualFixtureName = installViewerVisualFixture();
  if (viewerVisualFixtureName) {
    root.setAttribute("data-viewer-visual-fixture", viewerVisualFixtureName);
  } else {
    root.removeAttribute("data-viewer-visual-fixture");
  }
  let dispose = mount(() => <AppShell />, root);
  core.setRenderHook(() => setViewerStateRevision((revision) => revision + 1));

  return () => {
    core.setRenderHook(null);
    dispose();
    root.textContent = "";
  };
}

function shouldBypassAutoMountForTestApi() {
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}

const autoMountRoot = document.getElementById("app");
if (autoMountRoot) {
  mountViewerApp(autoMountRoot);
} else if (!shouldBypassAutoMountForTestApi()) {
  throw new Error("viewer root #app is missing");
}
