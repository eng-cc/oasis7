import { Show } from "solid-js";

export const PLAYER_SAFE_COPY_SCHEMA_VERSION = 1;

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

export const AGENT_INTENT_SUMMARIES = Object.freeze({
  proposed: "Agent guidance is proposed and not yet accepted.",
  submitted: "Agent guidance was submitted and awaits runtime acceptance.",
  accepted: "Agent guidance accepted; the Agent will evaluate its next world action.",
  blocked: "Agent guidance is blocked pending a runtime recheck.",
  completed: "Agent guidance completed with a confirmed world receipt.",
  rejected: "Agent guidance was rejected by runtime authority.",
  expired: "Agent guidance expired before execution.",
  cancelled: "Agent guidance was cancelled before completion.",
  superseded: "Agent guidance was replaced by newer guidance.",
});

const REASON_ALLOWLIST = Object.freeze({
  missing_material: "World prerequisites changed before execution.",
  material_shortage: "World prerequisites changed before execution.",
  permission_changed: "The requested operation is no longer authorized.",
  ownership_changed: "The controlling session changed before completion.",
  world_precondition_changed: "The world position changed before execution.",
  precondition_changed: "The world position changed before execution.",
  agent_unavailable: "The Agent is not available for this intent.",
  duplicate_request: "The duplicate request was already recorded.",
  superseded_by_replacement: "A newer intent has taken over.",
});

const NEXT_STEP_ALLOWLIST = Object.freeze({
  unavailable: "Stop and refresh the world snapshot before retrying.",
  missing_receipt: "Wait for a committed world receipt, then refresh.",
  stale: "Refresh the world state before acting.",
  conflict: "Review the latest world state and reselect an intent.",
  reconnecting: "Wait for the runtime connection to recover.",
  control_lost: "Reselect the Agent after control is restored.",
  read_only: "Reselect the Agent in a controllable session.",
  unauthorized: "Request access before viewing this intent.",
  blocked: "Recheck runtime state before resuming.",
});

const ALLOWED_CONTROL_STATES = new Set(["controllable", "read_only", "control_lost", "unauthorized", "unavailable"]);
const ALLOWED_FRESHNESS = new Set(["current", "stale", "reconnecting", "conflict"]);
const TERMINAL_INTENT_STATUSES = new Set(["completed", "rejected", "expired", "cancelled", "superseded"]);
const COPY_KEY_FIELDS = ["copy_schema_version", "summary_schema_version", "player_copy_schema_version"];
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
  if (!normalized) return "";
  const chars = Array.from(normalized);
  return chars.length <= MAX_PLAYER_SAFE_COPY_CHARS
    ? normalized
    : `${chars.slice(0, MAX_PLAYER_SAFE_COPY_CHARS - 1).join("")}…`;
}

function counterIdentity(value) {
  if (typeof value === "number") return Number.isSafeInteger(value) && value >= 0 ? String(value) : null;
  const raw = textValue(value);
  if (!/^\d+$/.test(raw)) return null;
  try { return BigInt(raw).toString(); } catch (_error) { return null; }
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
  if (!receiptRef || typeof receiptRef !== "object") return false;
  const receiptIdentity = textValue(receiptRef.receipt_id);
  const receiptEventId = receiptIdentity.startsWith("world-event:")
    ? counterIdentity(receiptIdentity.slice("world-event:".length))
    : null;
  if (textValue(receiptRef.intent_id) !== textValue(intent?.intent_id)
    || textValue(receiptRef.world_id) !== textValue(intent?.world_id)
    || receiptEventId === null
    || receiptEventId === "0") return false;
  return counterIdentity(receiptRef.reorg_epoch) === counterIdentity(intent?.reorg_epoch)
    && counterIdentity(receiptRef.logical_time) === counterIdentity(intent?.logical_time)
    && counterIdentity(receiptRef.event_seq) === counterIdentity(intent?.event_seq);
}

function copy(locale, key) {
  const zh = String(locale || "").toLowerCase().startsWith("zh");
  const values = {
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
  }[key];
  return values ? values[zh ? 0 : 1] : key;
}

function statusLabel(locale, status) {
  const values = INTENT_STATUS_LABELS[status];
  return values ? values[String(locale || "").toLowerCase().startsWith("zh") ? 0 : 1] : "";
}

function unavailable(locale, nextStep = NEXT_STEP_ALLOWLIST.unavailable, extra = {}) {
  return { kind: "unavailable", label: copy(locale, "unavailable"), nextStep, receiptState: "not_applicable", ...extra };
}

function copyVersion(intent) {
  const explicit = COPY_KEY_FIELDS.map((field) => intent[field]).find((value) => value !== undefined && value !== null);
  return explicit === undefined ? PLAYER_SAFE_COPY_SCHEMA_VERSION : explicit;
}

function allowlistedIntentCopy(intent, status) {
  if (copyVersion(intent) !== PLAYER_SAFE_COPY_SCHEMA_VERSION) return { valid: false, value: "" };
  const expected = AGENT_INTENT_SUMMARIES[status];
  const key = textValue(intent.summary_key || intent.summaryKey);
  if (key && key !== status) return { valid: false, value: "" };
  const supplied = textValue(intent.summary ?? intent.message);
  if (!supplied || supplied !== expected) return { valid: false, value: "" };
  return { valid: true, value: expected };
}

function allowlistedReason(intent, status) {
  const key = textValue(intent.reason_code || intent.reason_key || intent.reasonKey).toLowerCase();
  const supplied = textValue(intent.reason_summary);
  if (!key) return supplied ? { valid: false, label: "", summary: "" } : { valid: true, label: "", summary: "" };
  if (!Object.prototype.hasOwnProperty.call(REASON_ALLOWLIST, key)) return { valid: false, label: "", summary: "" };
  const declaredKey = textValue(intent.reason_key || intent.reasonKey).toLowerCase();
  if (declaredKey && declaredKey !== key) return { valid: false, label: "", summary: "" };
  if (supplied && supplied !== REASON_ALLOWLIST[key]) return { valid: false, label: "", summary: "" };
  if (!TERMINAL_INTENT_STATUSES.has(status) && status !== "blocked") return { valid: true, label: "", summary: "" };
  return { valid: true, label: key, summary: REASON_ALLOWLIST[key] };
}

function allowlistedNextStep(intent, status, stateKind) {
  const declared = textValue(intent.next_step_key || intent.nextStepKey).toLowerCase();
  if (declared && !Object.prototype.hasOwnProperty.call(NEXT_STEP_ALLOWLIST, declared)) return { valid: false, value: "" };
  const fallbackKey = stateKind === "current" && status === "blocked" ? "blocked" : stateKind;
  const expected = NEXT_STEP_ALLOWLIST[declared || fallbackKey] || "";
  const supplied = textValue(intent.next_step || intent.next_step_hint);
  if (supplied && supplied !== expected) return { valid: false, value: "" };
  return { valid: true, value: expected };
}

export function describeAgentIntent(intent, locale = "en", connectionStatus = "connected") {
  if (!intent || typeof intent !== "object") return unavailable(locale);
  if (intent.schema_version !== 2 || textValue(intent.source_class) !== "runtime_projection") return unavailable(locale);

  const connection = textValue(connectionStatus).toLowerCase();
  if (connection === "connecting" || connection === "reconnecting") return { kind: "reconnecting", label: copy(locale, "reconnecting"), nextStep: NEXT_STEP_ALLOWLIST.reconnecting, receiptState: "not_applicable" };
  if (connection && connection !== "connected") return unavailable(locale, NEXT_STEP_ALLOWLIST.unavailable, { label: copy(locale, "offline") });

  const controlState = textValue(intent.control_state).toLowerCase();
  if (!ALLOWED_CONTROL_STATES.has(controlState)) return unavailable(locale);
  if (controlState === "control_lost") return { kind: controlState, label: copy(locale, "hiddenControlLost"), nextStep: NEXT_STEP_ALLOWLIST.control_lost, receiptState: "hidden" };
  if (controlState === "read_only") return { kind: controlState, label: copy(locale, "hiddenReadOnly"), nextStep: NEXT_STEP_ALLOWLIST.read_only, receiptState: "hidden" };
  if (controlState === "unauthorized") return { kind: controlState, label: copy(locale, "hiddenUnauthorized"), nextStep: NEXT_STEP_ALLOWLIST.unauthorized, receiptState: "hidden" };
  if (controlState === "unavailable" || !textValue(intent.intent_id) || !hasAuthoritativePosition(intent)) return unavailable(locale);

  const status = textValue(intent.status).toLowerCase();
  if (!statusLabel(locale, status)) return unavailable(locale);
  const freshness = textValue(intent.freshness).toLowerCase();
  if (!ALLOWED_FRESHNESS.has(freshness)) return unavailable(locale);

  const receiptState = status === "completed"
    ? (hasReceiptReference(intent.receipt_ref, intent) ? "confirmed" : "missing")
    : "not_applicable";
  const receiptLabel = receiptState === "confirmed" ? copy(locale, "receipt") : receiptState === "missing" ? copy(locale, "receiptMissing") : "";
  if (receiptState === "missing") return unavailable(locale, NEXT_STEP_ALLOWLIST.missing_receipt, { receiptState, receiptLabel });

  const safeCopy = allowlistedIntentCopy(intent, status);
  if (!safeCopy.valid) return unavailable(locale, NEXT_STEP_ALLOWLIST.unavailable, { receiptState });
  const reason = allowlistedReason(intent, status);
  if (!reason.valid) return unavailable(locale, NEXT_STEP_ALLOWLIST.unavailable, { receiptState });

  const stateKind = freshness === "stale" ? "stale" : freshness === "reconnecting" ? "reconnecting" : freshness === "conflict" ? "conflict" : "current";
  const nextStep = allowlistedNextStep(intent, status, stateKind);
  if (!nextStep.valid) return unavailable(locale, NEXT_STEP_ALLOWLIST.unavailable, { receiptState });
  const lifecycleNote = intent.duplicate === true || intent.replayed === true || intent.replay === true
    ? copy(locale, "replayed")
    : textValue(intent.replaced_by) ? copy(locale, "replaced") : "";
  const base = {
    kind: stateKind,
    label: stateKind === "stale" ? copy(locale, "stale") : stateKind === "conflict" ? copy(locale, "needsConfirmation") : stateKind === "reconnecting" ? copy(locale, "reconnecting") : copy(locale, "current"),
    statusLabel: stateKind === "stale" ? "" : statusLabel(locale, status),
    message: safeCopy.value,
    receiptState,
    receiptLabel,
    reasonLabel: reason.label,
    reasonSummary: reason.summary,
    lifecycleNote,
    nextStep: nextStep.value,
  };
  return base;
}

export function AgentIntentSurface(props) {
  const locale = () => props.locale || "en";
  const model = () => describeAgentIntent(props.intent, locale(), props.connectionStatus);
  const hidden = () => ["control_lost", "read_only", "unauthorized", "unavailable"].includes(model().kind);
  return (
    <section class="agent-intent" data-agent-intent-state={model().kind} data-agent-intent-receipt-state={model().receiptState || "not_applicable"} aria-live="polite">
      <div class="agent-intent__heading metric__label">{copy(locale(), "heading")}</div>
      <div class="agent-intent__state">{model().label}</div>
      <Show when={!hidden()}>
        <div class="agent-intent__status-row">
          <Show when={model().statusLabel}><span class="badge badge--accent">{model().statusLabel}</span></Show>
        </div>
        <Show when={model().message}><div class="agent-intent__summary">{model().message}</div></Show>
        <Show when={model().receiptLabel}><div class="agent-intent__detail agent-intent__receipt"><span class="metric__label">{model().receiptLabel}</span></div></Show>
        <Show when={model().reasonLabel || model().reasonSummary}><div class="agent-intent__detail"><span class="metric__label">{copy(locale(), "reason")}</span><span class="agent-intent__summary">{model().reasonSummary}</span></div></Show>
        <Show when={model().lifecycleNote}><div class="agent-intent__detail agent-intent__lifecycle">{model().lifecycleNote}</div></Show>
      </Show>
      <Show when={hidden() && model().receiptLabel}><div class="agent-intent__detail agent-intent__receipt"><span class="metric__label">{model().receiptLabel}</span></div></Show>
      <Show when={model().nextStep}><div class="agent-intent__detail agent-intent__next-step"><span class="metric__label">{copy(locale(), "nextStep")}</span><span class="agent-intent__summary">{model().nextStep}</span></div></Show>
    </section>
  );
}
