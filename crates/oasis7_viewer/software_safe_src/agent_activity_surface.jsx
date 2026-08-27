import { Show } from "solid-js";

const KNOWN_ACTIVITY_STATUSES = new Set(["idle", "executing", "blocked", "waiting", "unavailable"]);

function textValue(value) {
  if (value === null || value === undefined) {
    return "";
  }
  return String(value).trim();
}

function titleCaseIdentifier(value) {
  const text = textValue(value)
    .replace(/[_:-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!text || /^\d+$/.test(text) || /^(?:0x|sha256|uuid)\b/i.test(text)) {
    return "";
  }
  return text.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function humanizedActivityValue(value, fallback = "") {
  return titleCaseIdentifier(value) || fallback;
}

function activityCopy(locale, key) {
  const zh = String(locale || "").toLowerCase().startsWith("zh");
  const copy = {
    currentActivity: ["当前活动", "Current Activity"],
    unavailable: ["活动不可用", "Activity unavailable"],
    idle: ["空闲", "Idle"],
    executing: ["执行中", "Executing"],
    blocked: ["受阻", "Blocked"],
    waiting: ["等待中", "Waiting"],
    target: ["目标", "Target"],
    reason: ["原因", "Reason"],
    operation: ["操作", "Operation"],
    targetUnavailable: ["目标信息暂不可用", "Target unavailable"],
  }[key];
  return copy ? copy[zh ? 0 : 1] : key;
}

export function describeAgentActivity(activity, locale = "en") {
  if (!activity || typeof activity !== "object") {
    return {
      kind: "unavailable",
      label: activityCopy(locale, "unavailable"),
      operation: "",
      targetLabel: "",
      reason: "",
    };
  }

  const status = textValue(activity.status).toLowerCase();
  if (!KNOWN_ACTIVITY_STATUSES.has(status)) {
    return {
      kind: "unavailable",
      label: activityCopy(locale, "unavailable"),
      operation: "",
      targetLabel: "",
      reason: "",
    };
  }

  if (status === "idle" || status === "unavailable") {
    return {
      kind: status,
      label: activityCopy(locale, status),
      operation: "",
      targetLabel: "",
      reason: "",
    };
  }

  const operation = humanizedActivityValue(activity.operation);
  const targetLabel = humanizedActivityValue(activity.target, activity.target_label || activity.targetLabel || "");
  const reason = humanizedActivityValue(activity.reason, "");
  const statusLabel = activityCopy(locale, status);
  return {
    kind: status,
    label: (status === "executing" || status === "waiting") && operation
      ? `${statusLabel} ${operation}`
      : statusLabel,
    operation,
    targetLabel,
    reason,
  };
}

export function AgentActivitySurface(props) {
  const locale = () => props.locale || "en";
  const model = () => describeAgentActivity(props.activity, locale());
  return (
    <div class="agent-activity" data-agent-activity-state={model().kind}>
      <div class="agent-activity__heading metric__label">{activityCopy(locale(), "currentActivity")}</div>
      <div class="agent-activity__state">{model().label}</div>
      <Show when={model().kind === "blocked" && model().operation}>
        <div class="agent-activity__field">
          <span class="metric__label">{activityCopy(locale(), "operation")}</span>
          <span>{model().operation}</span>
        </div>
      </Show>
      <Show when={model().kind !== "unavailable" && model().kind !== "idle" && model().kind !== "unavailable" && model().targetLabel}>
        <div class="agent-activity__field">
          <span class="metric__label">{activityCopy(locale(), "target")}</span>
          <span>{model().targetLabel}</span>
        </div>
      </Show>
      <Show when={model().kind !== "unavailable" && model().kind !== "idle" && !model().targetLabel && model().operation}>
        <div class="agent-activity__field">
          <span class="metric__label">{activityCopy(locale(), "target")}</span>
          <span>{activityCopy(locale(), "targetUnavailable")}</span>
        </div>
      </Show>
      <Show when={model().kind === "blocked" && model().reason}>
        <div class="agent-activity__field">
          <span class="metric__label">{activityCopy(locale(), "reason")}</span>
          <span>{model().reason}</span>
        </div>
      </Show>
    </div>
  );
}
