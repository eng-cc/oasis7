import { Show } from "solid-js";
import {
  activityCopy,
  describeAgentActivity,
} from "./agent_activity_display_model.js";

export { describeAgentActivity } from "./agent_activity_display_model.js";

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
