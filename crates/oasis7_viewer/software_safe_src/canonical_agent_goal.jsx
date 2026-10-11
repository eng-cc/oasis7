import { Show } from "solid-js";
import { authenticatedCanonicalGoal } from "./viewer_canonical_goal_module.js";

export function CanonicalAgentGoal(props) {
  const goal = () => authenticatedCanonicalGoal(props.state, props.agentId);
  const pending = () => props.state.lastChatFeedback?.agentId === props.agentId
    && props.state.lastChatFeedback?.stage === "pending" && props.state.lastChatFeedback?.canonicalRequest;
  const zh = () => String(props.locale || "en").startsWith("zh");
  return <>
    <Show when={goal()}>{(current) => <p data-canonical-agent-goal>{zh() ? "当前目标" : "Current goal"}: {current().message}</p>}</Show>
    <Show when={pending()}><p data-canonical-agent-goal-pending role="status">{zh() ? "目标已提交，等待世界确认。" : "Goal submitted; waiting for world confirmation."}</p></Show>
  </>;
}
