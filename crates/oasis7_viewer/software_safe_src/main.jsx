import { For, Show, createSignal } from "solid-js";
import { render as mount } from "solid-js/web";

import * as core from "./legacy_core.js";

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
  return (
    <details class="diagnostic">
      <summary>{props.label ?? "Raw diagnostics"}</summary>
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

function renderResourceSummary(resources) {
  return core.resourceSummary(resources);
}

function TargetsPanel() {
  const lists = () => core.modelLists();

  return (
    <div class="stack">
      <div class="field">
        <label for="entity-search">Filter targets</label>
        <input
          id="entity-search"
          type="search"
          placeholder="Search agents or locations"
          value={core.getSelectedSearch()}
          onInput={(event) => core.setSelectedSearch(event.currentTarget.value)}
        />
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Agents</div>
        <div class="list">
          <Show
            when={lists().agents.length > 0}
            fallback={<EmptyState>No agents in current snapshot.</EmptyState>}
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
                    {`location=${agent.location_id} · resources=${renderResourceSummary(agent.resources)}`}
                  </div>
                </button>
              )}
            </For>
          </Show>
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Locations</div>
        <div class="list">
          <Show
            when={lists().locations.length > 0}
            fallback={<EmptyState>No locations in current snapshot.</EmptyState>}
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
                    {`id=${location.id} · resources=${renderResourceSummary(location.resources)}`}
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
  const state = core.state;
  const controlFeedback = () => core.snapshotControlFeedback(state.lastControlFeedback);
  const promptFeedback = () => core.snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => core.snapshotSemanticFeedback(state.lastChatFeedback);
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback());
  const authSurface = () => core.buildAuthSurfaceModel();
  const hostedActionMatrixView = () => core.buildHostedActionMatrixView();
  const hostedRecoveryHint = () => core.buildHostedRecoveryHint();
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
        <Badge class={core.connectionBadgeClass()}>{state.connectionStatus}</Badge>
        <Badge>{`debugViewer=${state.debugViewerMode}:${state.debugViewerStatus}`}</Badge>
        <Badge>{`rendererClass=${state.rendererClass}`}</Badge>
        <Badge>{`controlProfile=${state.controlProfile}`}</Badge>
      </div>
      <div class="summary-grid">
        <MetricCard label="Logical Time" value={state.logicalTime} />
        <MetricCard label="Event Seq" value={state.eventSeq} />
        <MetricCard label="World" value={state.worldId || "-"} />
        <MetricCard label="Viewer Server" value={state.server || "-"} />
      </div>
      <div class="badge-row">
        <Badge>{`ws=${state.wsUrl || "-"}`}</Badge>
        <Badge>{`reason=${state.softwareSafeReason || "-"}`}</Badge>
        <Badge>{`renderer=${state.renderer || "n/a"}`}</Badge>
      </div>
      <PanelSection title="Execution Lanes">
        <div class="badge-row">
          <Badge class="badge badge--accent">debug_viewer</Badge>
          <Badge>{`status=${state.debugViewerStatus}`}</Badge>
          <Badge>{`renderMode=${state.renderMode}`}</Badge>
          <Badge>{`fallback=${state.softwareSafeReason || "-"}`}</Badge>
        </div>
        <EmptyState style="margin-top:-2px;">
          debug_viewer is a read-only subscription lane for runtime snapshots/events; closing the viewer
          does not stop the agent lane.
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
                <Badge>{`fallback=${debug().fallback_reason || "-"}`}</Badge>
              </div>
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
              <div class="panel__title">Hosted Recovery</div>
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
        <PanelSection title="Hosted Action Matrix">
          <EmptyState>
            This is the hosted public-join truth surface exported by the launcher. QA should read these
            action ids directly instead of inferring from button state alone.
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
      <PlaybackControls controlFeedback={controlFeedback()} />
      <div class="summary-grid">
        <MetricCard label="Prompt Feedback" value={promptFeedback()?.stage || "idle"}>
          <Show when={promptFeedbackDisplay()}>
            <Badge class={promptFeedbackDisplay().badgeClass}>
              {promptFeedbackDisplay().label}
            </Badge>
          </Show>
        </MetricCard>
        <MetricCard label="Chat Feedback" value={chatFeedback()?.stage || "idle"}>
          <Show when={chatFeedbackDisplay()}>
            <Badge class={chatFeedbackDisplay().badgeClass}>
              {chatFeedbackDisplay().label}
            </Badge>
          </Show>
        </MetricCard>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Recent Events</div>
        <div class="event-list">
          <Show when={state.recentEvents.length > 0} fallback={<EmptyState>Waiting for live events…</EmptyState>}>
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

function PlaybackControls(props) {
  const [stepCount, setStepCount] = createSignal(3);

  return (
    <PanelSection title="Playback Controls">
      <div class="toolbar">
        <button data-action="play" onClick={() => core.sendControl("play", null)}>Play</button>
        <button data-action="pause" onClick={() => core.sendControl("pause", null)}>Pause</button>
        <button data-action="step" onClick={() => core.sendControl("step", null)}>Step x1</button>
      </div>
      <div class="control-grid">
        <div class="field">
          <label for="step-count">Step count</label>
          <input
            id="step-count"
            type="number"
            min="1"
            step="1"
            value={stepCount()}
            onInput={(event) => setStepCount(Math.max(1, Math.floor(Number(event.currentTarget.value || 1))))}
          />
        </div>
        <div class="field" style="align-self:end;">
          <button
            data-action="step-count"
            onClick={() => core.sendControl("step", { count: Math.max(1, Math.floor(stepCount() || 1)) })}
          >
            Step custom count
          </button>
        </div>
      </div>
      <Show
        when={props.controlFeedback}
        fallback={<EmptyState>No control feedback yet.</EmptyState>}
      >
        {(feedback) => (
          <>
            <div class="badge-row">
              <Badge>{`action=${feedback().action}`}</Badge>
              <Badge>{`stage=${feedback().stage}`}</Badge>
              <Badge>{`Δtick=${feedback().deltaLogicalTime}`}</Badge>
              <Badge>{`Δevent=${feedback().deltaEventSeq}`}</Badge>
            </div>
            <div class="feedback-summary">
              {feedback().effect || feedback().reason || "Control feedback updated."}
            </div>
            <DiagnosticDetails value={feedback()} />
          </>
        )}
      </Show>
    </PanelSection>
  );
}

function InteractionPanel() {
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
  const promptFeedbackDisplay = () => core.describeSemanticFeedback(promptFeedback());
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback());
  const promptVersionState = () => core.describePromptVersionState(promptFeedback());
  const chatHistory = () =>
    core.state.chatHistory
      .filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId())
      .slice(0, 12);
  const interactionEnabled = () => promptCapability().enabled;
  const assetLaneStatusText = () =>
    mainTokenTransferCapability().enabled
      ? "preview_only"
      : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () =>
    mainTokenTransferCapability().enabled
      ? "Contract marks main_token_transfer as strong_auth-capable on this lane, but software_safe still exposes no transfer form here."
      : mainTokenTransferCapability().reason;

  if (!agentId()) {
    return <EmptyState>Select an agent to unlock prompt/chat controls.</EmptyState>;
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
      <Show when={debugContext()?.provider_mode === "openclaw_local_http"}>
        <EmptyState>
          {`Selected agent currently runs through OpenClaw(Local HTTP) in ${
            debugContext()?.execution_mode || "headless_agent"
          }; software_safe stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.`}
        </EmptyState>
      </Show>
      <Show when={debugContext()?.provider_mode !== "openclaw_local_http"}>
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
            <label for="strong-auth-approval-code">Backend Approval Code</label>
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
          <label for="prompt-system">System Prompt Override</label>
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
          <label for="prompt-short">Short-Term Goal Override</label>
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
          <label for="prompt-long">Long-Term Goal Override</label>
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
            Preview Prompt
          </button>
          <button
            data-prompt-action="apply"
            disabled={!promptCapability().enabled}
            onClick={() => core.sendPromptControl("apply", null)}
          >
            Apply Prompt
          </button>
        </div>
        <div class="toolbar">
          <div class="field" style="margin:0; min-width:180px; flex:1;">
            <label for="prompt-rollback-version">Next Rollback Target Version</label>
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
            Rollback Prompt
          </button>
        </div>
        <Show when={promptFeedback()} fallback={<EmptyState>No prompt feedback yet.</EmptyState>}>
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
      <PanelSection title="Asset / Governance Lane">
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
            || "No hosted action policy is available for main_token_transfer on this lane."}
        </EmptyState>
        <div class="toolbar">
          <button disabled>Main Token Transfer (Not Exposed Here Yet)</button>
        </div>
      </PanelSection>
      <PanelSection title="Agent Chat">
        <div class="field">
          <label for="agent-chat-message">Message</label>
          <textarea
            id="agent-chat-message"
            rows="4"
            placeholder="Send a message to the selected agent"
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
            Send Chat
          </button>
        </div>
        <Show when={chatFeedback()} fallback={<EmptyState>No chat feedback yet.</EmptyState>}>
          {(feedback) => <FeedbackCard feedback={feedback()} display={chatFeedbackDisplay()} />}
        </Show>
        <div>
          <div class="panel__title" style="margin-bottom:10px;">Message Flow</div>
          <div class="event-list">
            <Show when={chatHistory().length > 0} fallback={<EmptyState>No chat history for this agent yet.</EmptyState>}>
              <For each={chatHistory()}>
                {(entry) => (
                  <EventCard
                    title={
                      entry.source === "player"
                        ? `player → ${entry.targetAgentId || entry.agentId || "agent"}`
                        : `${entry.agentId || "agent"} spoke`
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
    </div>
  );
}

function DetailsPanel() {
  const selectedLabel = () =>
    core.state.selectedKind && core.state.selectedId
      ? `${core.state.selectedKind}:${core.state.selectedId}`
      : "nothing selected";
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

  return (
    <div class="stack">
      <div class="badge-row">
        <Badge class="badge badge--accent">Selected</Badge>
        <Badge>{selectedLabel()}</Badge>
      </div>
      <InteractionPanel />
      <Show when={core.state.selectedObject} fallback={<EmptyState>Select an agent or location from the left list.</EmptyState>}>
        <JsonBlock value={core.clone(core.state.selectedObject)} />
      </Show>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Snapshot Summary</div>
        <JsonBlock value={snapshotSummary()} />
      </div>
      <Show when={core.state.lastError}>
        <div>
          <div class="panel__title" style="margin-bottom:10px; color: var(--bad);">Last Error</div>
          <pre class="json">{core.state.lastError}</pre>
        </div>
      </Show>
    </div>
  );
}

function AppShell() {
  return (
    <>
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">Targets</div>
        </div>
        <div class="panel__body">
          <TargetsPanel />
        </div>
      </section>
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">World Summary</div>
        </div>
        <div class="panel__body">
          <WorldSummaryPanel />
        </div>
      </section>
      <section class="panel">
        <div class="panel__header">
          <div class="panel__title">Details</div>
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
