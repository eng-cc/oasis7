import { Show } from "solid-js";

const INTENT_STATUS_LABELS = {
  proposed: ["已提出", "Proposed"],
  submitted: ["已提交", "Submitted"],
  accepted: ["已接受", "Accepted"],
  accepted_new: ["已接受", "Accepted"],
  reprioritized: ["已接受", "Accepted"],
  blocked: ["受阻", "Blocked"],
  completed: ["已完成", "Completed"],
  rejected: ["已拒绝", "Rejected"],
  expired: ["已过期", "Expired"],
  cancelled: ["已取消", "Cancelled"],
  superseded: ["已替换", "Replaced"],
};

function textValue(value) {
  return value === null || value === undefined ? "" : String(value).trim();
}

function intentCopy(locale, key) {
  const zh = String(locale || "").toLowerCase().startsWith("zh");
  const copy = {
    heading: ["当前意图", "Current Intent"],
    unavailable: ["意图不可用", "Intent unavailable"],
    hiddenControlLost: ["意图已隐藏 — 控制权丢失", "Intent hidden — control lost"],
    stale: ["陈旧意图", "Stale intent"],
    current: ["当前", "Current"],
    reconnecting: ["重新连接中", "Reconnecting"],
    needsConfirmation: ["需要确认", "Needs confirmation"],
    reason: ["原因", "Reason"],
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

function describeAgentIntent(intent, locale = "en") {
  if (!intent || typeof intent !== "object") {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable") };
  }

  const controlState = textValue(intent.control_state).toLowerCase();
  if (controlState === "control_lost") {
    return { kind: "control_lost", label: intentCopy(locale, "hiddenControlLost") };
  }
  if (controlState === "unavailable") {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable") };
  }

  const status = textValue(intent.status).toLowerCase();
  const label = statusLabel(locale, status);
  if (!label) {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable") };
  }

  const freshness = textValue(intent.freshness).toLowerCase();
  const message = textValue(intent.message || intent.summary);
  if (freshness === "stale") {
    // A retained intent can still carry a terminal-looking status, but stale
    // authority must not be presented as a fresh completion or acceptance.
    return { kind: "stale", label: intentCopy(locale, "stale"), statusLabel: "", message };
  }
  if (freshness === "reconnecting") {
    return { kind: "reconnecting", label: intentCopy(locale, "reconnecting"), statusLabel: label, message };
  }
  if (freshness === "conflict") {
    return { kind: "conflict", label: intentCopy(locale, "needsConfirmation"), statusLabel: label, message };
  }
  if (freshness !== "current") {
    return { kind: "unavailable", label: intentCopy(locale, "unavailable") };
  }

  return {
    kind: "current",
    label: intentCopy(locale, "current"),
    statusLabel: label,
    message,
  };
}

export function AgentIntentSurface(props) {
  const locale = () => props.locale || "en";
  const model = () => describeAgentIntent(props.intent, locale());

  return (
    <section class="agent-intent" data-agent-intent-state={model().kind} aria-live="polite">
      <div class="agent-intent__heading metric__label">{intentCopy(locale(), "heading")}</div>
      <Show when={model().kind === "control_lost" || model().kind === "unavailable"}>
        <div class="agent-intent__state">{model().label}</div>
      </Show>
      <Show when={model().kind !== "control_lost" && model().kind !== "unavailable"}>
        <div class="agent-intent__status-row">
          <span class="agent-intent__state">{model().label}</span>
          <Show when={model().statusLabel}>
            <span class="badge badge--accent">{model().statusLabel}</span>
          </Show>
        </div>
        <Show when={model().message}>
          <div class="agent-intent__summary">{model().message}</div>
        </Show>
      </Show>
    </section>
  );
}
