import { Show } from "solid-js";

export const INTENT_STATUS_LABELS = {
  proposed: ["已提出", "Proposed"],
  submitted: ["已提交", "Submitted"],
  accepted: ["已接受", "Accepted"],
  blocked: ["受阻", "Blocked"],
  completed: ["已完成", "Completed"],
  rejected: ["已拒绝", "Rejected"],
  expired: ["已过期", "Expired"],
  cancelled: ["已取消", "Cancelled"],
  superseded: ["已替换", "Replaced"],
};

const INTENT_REASON_LABELS = {
  missing_material: ["材料不足", "Missing material"],
  material_shortage: ["材料短缺", "Material shortage"],
  permission_changed: ["权限已变化", "Permission changed"],
  ownership_changed: ["所有权已变化", "Ownership changed"],
  world_precondition_changed: ["世界前置条件已变化", "World precondition changed"],
  precondition_changed: ["前置条件已变化", "Precondition changed"],
  agent_unavailable: ["行动体不可用", "Agent unavailable"],
  duplicate_request: ["重复请求已合并", "Duplicate request coalesced"],
  superseded_by_replacement: ["已由替代意图接管", "Replaced by a newer intent"],
};

const ALLOWED_CONTROL_STATES = new Set(["controllable", "read_only", "control_lost", "unauthorized", "unavailable"]);
const ALLOWED_FRESHNESS = new Set(["current", "stale", "reconnecting", "conflict"]);
const TERMINAL_INTENT_STATUSES = new Set(["completed", "rejected", "expired", "cancelled", "superseded"]);
const MAX_PLAYER_SAFE_COPY_CHARS = 160;
const SENSITIVE_INTERNAL_COPY = /system[_ ]?prompt|provider(?:[_ ]?rationale)?|chain[_ ]?of[_ ]?thought|memory\s*:|trace\s*:|debug\s*:|auth(?:[_ ]?(?:token|secret|proof)|\s*:)|cost[_ ]?cents/i;

function textValue(value) {
  return typeof value === "string" ? value.trim() : "";
}

export function boundedSafeText(value) {
  const normalized = textValue(value)
    .split(/\r?\n/)
    .filter((line) => !SENSITIVE_INTERNAL_COPY.test(line))
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
  if (!normalized) {
    return "";
  }
  const chars = Array.from(normalized);
  return chars.length <= MAX_PLAYER_SAFE_COPY_CHARS
    ? normalized
    : `${chars.slice(0, MAX_PLAYER_SAFE_COPY_CHARS - 1).join("")}…`;
}

function counterIdentity(value) {
  if (typeof value === "number") {
    return Number.isSafeInteger(value) && value >= 0 ? String(value) : null;
  }
  const raw = textValue(value);
  if (!/^\d+$/.test(raw)) {
    return null;
  }
  try {
    return BigInt(raw).toString();
  } catch (_error) {
    return null;
  }
}

function hasAuthoritativePosition(intent) {
  return textValue(intent.agent_id).length > 0
    && textValue(intent.world_id).length > 0
    && counterIdentity(intent.reorg_epoch) !== null
    && counterIdentity(intent.logical_time) !== null
    && counterIdentity(intent.event_seq) !== null
    && counterIdentity(intent.updated_at) !== null;
}

export function hasReceiptReference(receiptRef, intent) {
  if (!receiptRef || typeof receiptRef !== "object") {
    return false;
  }
  const receiptIdentity = textValue(receiptRef.receipt_id);
  const receiptEventId = receiptIdentity.startsWith("world-event:")
    ? counterIdentity(receiptIdentity.slice("world-event:".length))
    : null;
  if (
    textValue(receiptRef.intent_id) !== textValue(intent?.intent_id)
    || textValue(receiptRef.world_id) !== textValue(intent?.world_id)
    || receiptEventId === null
    || receiptEventId === "0"
  ) {
    return false;
  }
  return counterIdentity(receiptRef.reorg_epoch) === counterIdentity(intent?.reorg_epoch)
    && counterIdentity(receiptRef.logical_time) === counterIdentity(intent?.logical_time)
    && counterIdentity(receiptRef.event_seq) === counterIdentity(intent?.event_seq);
}

function intentCopy(locale, key) {
  const zh = String(locale || "").toLowerCase().startsWith("zh");
  const copy = {
    heading: ["当前意图", "Current Intent"],
    unavailable: ["意图不可用", "Intent unavailable"],
    hiddenControlLost: ["意图已隐藏 — 控制权丢失", "Intent hidden — control lost"],
    hiddenReadOnly: ["意图已隐藏 — 只读观察", "Intent hidden — read-only"],
    hiddenUnauthorized: ["意图已隐藏 — 未获授权", "Intent hidden — unauthorized"],
    stale: ["陈旧意图", "Stale intent"],
    current: ["当前", "Current"],
    reconnecting: ["重新连接中", "Reconnecting"],
    offline: ["意图不可用 — 世界连接已断开", "Intent unavailable — world connection lost"],
    needsConfirmation: ["需要确认", "Needs confirmation"],
    reason: ["原因", "Reason"],
    reasonUnavailable: ["原因暂不可用", "Reason unavailable"],
    nextStep: ["下一步", "Next step"],
    receipt: ["世界回执已确认", "World receipt confirmed"],
    receiptMissing: ["等待世界回执", "World receipt missing"],
    replayed: ["重复请求已合并；没有创建新的意图。", "Duplicate request coalesced; no new intent was created."],
    replaced: ["这条意图已由较新的意图接管。", "This intent was replaced by a newer intent."],
    recheckBeforeResume: ["重新检查运行时状态后再恢复。", "Recheck runtime state before resuming."],
  }[key];
  return copy ? copy[zh ? 0 : 1] : key;
}

function statusLabel(locale, status) {
  const label = INTENT_STATUS_LABELS[status];
  if (!label) {
    return "";
  }
  return String(locale || "").toLowerCase().startsWith("zh") ? label[0] : label[1];
}

function describeIntentReason(intent, locale, status) {
  if (!TERMINAL_INTENT_STATUSES.has(status) && status !== "blocked") {
    return { reasonLabel: "", reasonSummary: "" };
  }
  const reasonCode = textValue(intent.reason_code).toLowerCase();
  if (!reasonCode) {
    return { reasonLabel: "", reasonSummary: "" };
  }
  const label = INTENT_REASON_LABELS[reasonCode];
  if (!label) {
    return { reasonLabel: intentCopy(locale, "reasonUnavailable"), reasonSummary: "" };
  }
  const zh = String(locale || "").toLowerCase().startsWith("zh");
  return {
    reasonLabel: label[zh ? 0 : 1],
    reasonSummary: boundedSafeText(intent.reason_summary),
  };
}

function connectionPresentationState(connectionStatus) {
  const status = textValue(connectionStatus).toLowerCase();
  if (!status || status === "connected") {
    return "connected";
  }
  if (status === "connecting" || status === "reconnecting") {
    return "reconnecting";
  }
  return "offline";
}

function receiptPresentation(intent, locale) {
  if (intent.status !== "completed") {
    return { state: "not_applicable", label: "" };
  }
  return hasReceiptReference(intent.receipt_ref, intent)
    ? { state: "confirmed", label: intentCopy(locale, "receipt") }
    : { state: "missing", label: intentCopy(locale, "receiptMissing") };
}

export function describeAgentIntent(intent, locale = "en", connectionStatus = "connected") {
  if (!intent || typeof intent !== "object") {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable"), receiptState: "not_applicable" };
  }

  if (intent.schema_version !== 2 || textValue(intent.source_class) !== "runtime_projection") {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable"), receiptState: "not_applicable" };
  }

  const connection = connectionPresentationState(connectionStatus);
  if (connection === "reconnecting") {
    return { kind: "reconnecting", label: intentCopy(locale, "reconnecting"), statusLabel: "", message: "", receiptState: "not_applicable" };
  }
  if (connection === "offline") {
    return { kind: "unavailable", label: intentCopy(locale, "offline"), receiptState: "not_applicable" };
  }

  const controlState = textValue(intent.control_state).toLowerCase();
  if (!ALLOWED_CONTROL_STATES.has(controlState)) {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable"), receiptState: "not_applicable" };
  }
  if (controlState === "control_lost") {
    return { kind: "control_lost", label: intentCopy(locale, "hiddenControlLost"), receiptState: "hidden" };
  }
  if (controlState === "read_only") {
    return { kind: "read_only", label: intentCopy(locale, "hiddenReadOnly"), receiptState: "hidden" };
  }
  if (controlState === "unauthorized") {
    return { kind: "unauthorized", label: intentCopy(locale, "hiddenUnauthorized"), receiptState: "hidden" };
  }
  if (controlState === "unavailable" || !textValue(intent.intent_id) || !hasAuthoritativePosition(intent)) {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable"), receiptState: "not_applicable" };
  }

  const status = textValue(intent.status).toLowerCase();
  const label = statusLabel(locale, status);
  const freshness = textValue(intent.freshness).toLowerCase();
  if (!label || !ALLOWED_FRESHNESS.has(freshness)) {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable"), receiptState: "not_applicable" };
  }

  const receipt = receiptPresentation(intent, locale);
  if (status === "completed" && receipt.state === "missing") {
    return {
      kind: "unavailable",
      label: intentCopy(locale, "unavailable"),
      receiptState: "missing",
      receiptLabel: receipt.label,
    };
  }

  const message = boundedSafeText(intent.summary || intent.message);
  const reason = describeIntentReason(intent, locale, status);
  const nextStep = status === "blocked"
    ? boundedSafeText(intent.next_step || intent.next_step_hint)
      || (intent.resume_required ? intentCopy(locale, "recheckBeforeResume") : "")
    : "";
  const lifecycleNote = intent.duplicate === true || intent.replayed === true || intent.replay === true
    ? intentCopy(locale, "replayed")
    : textValue(intent.replaced_by)
      ? intentCopy(locale, "replaced")
      : "";
  if (freshness === "stale") {
    return { kind: "stale", label: intentCopy(locale, "stale"), statusLabel: "", message, receiptState: receipt.state, receiptLabel: receipt.label, lifecycleNote, ...reason, nextStep };
  }
  if (freshness === "reconnecting") {
    return { kind: "reconnecting", label: intentCopy(locale, "reconnecting"), statusLabel: label, message, receiptState: receipt.state, receiptLabel: receipt.label, lifecycleNote, ...reason, nextStep };
  }
  if (freshness === "conflict") {
    return { kind: "conflict", label: intentCopy(locale, "needsConfirmation"), statusLabel: label, message, receiptState: receipt.state, receiptLabel: receipt.label, lifecycleNote, ...reason, nextStep };
  }

  return {
    kind: "current",
    label: intentCopy(locale, "current"),
    statusLabel: label,
    message,
    receiptState: receipt.state,
    receiptLabel: receipt.label,
    lifecycleNote,
    ...reason,
    nextStep,
  };
}

export function AgentIntentSurface(props) {
  const locale = () => props.locale || "en";
  const model = () => describeAgentIntent(props.intent, locale(), props.connectionStatus);
  const hidden = () => ["control_lost", "read_only", "unauthorized", "unavailable"].includes(model().kind);
  return (
    <section class="agent-intent" data-agent-intent-state={model().kind} data-agent-intent-receipt-state={model().receiptState || "not_applicable"} aria-live="polite">
      <div class="agent-intent__heading metric__label">{intentCopy(locale(), "heading")}</div>
      <Show when={hidden()}>
        <div class="agent-intent__state">{model().label}</div>
      </Show>
      <Show when={!hidden()}>
        <div class="agent-intent__status-row">
          <span class="agent-intent__state">{model().label}</span>
          <Show when={model().statusLabel}>
            <span class="badge badge--accent">{model().statusLabel}</span>
          </Show>
        </div>
        <Show when={model().message}>
          <div class="agent-intent__summary">{model().message}</div>
        </Show>
        <Show when={model().receiptLabel}>
          <div class="agent-intent__detail agent-intent__receipt">
            <span class="metric__label">{model().receiptLabel}</span>
          </div>
        </Show>
        <Show when={model().reasonLabel || model().reasonSummary}>
          <div class="agent-intent__detail">
            <span class="metric__label">{intentCopy(locale(), "reason")}</span>
            <span class="agent-intent__summary">{model().reasonLabel}{model().reasonSummary ? `: ${model().reasonSummary}` : ""}</span>
          </div>
        </Show>
        <Show when={model().lifecycleNote}>
          <div class="agent-intent__detail agent-intent__lifecycle">{model().lifecycleNote}</div>
        </Show>
        <Show when={model().nextStep}>
          <div class="agent-intent__detail">
            <span class="metric__label">{intentCopy(locale(), "nextStep")}</span>
            <span class="agent-intent__summary">{model().nextStep}</span>
          </div>
        </Show>
      </Show>
    </section>
  );
}
