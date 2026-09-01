import { describeAgentActivity } from "./agent_activity_surface.jsx";
import { describeAgentIntent } from "./agent_intent_surface.jsx";

const FRESHNESS_LABELS = Object.freeze({
  current: ["当前", "Current"],
  "last-known": ["最近已知", "Last known"],
  stale: ["陈旧", "Stale"],
  reconnecting: ["重新连接中", "Reconnecting"],
  unknown: ["未知", "Unknown"],
  conflict: ["需要确认", "Conflict"],
  replay: ["回放", "Replay"],
  gap: ["存在缺口", "Gap"],
  reorg: ["重组中", "Reorg"],
  unavailable: ["不可用", "Unavailable"],
});

const STATE_LABELS = Object.freeze({
  idle: ["空闲", "Idle"],
  executing: ["执行中", "Executing"],
  blocked: ["受阻", "Blocked"],
  waiting: ["等待中", "Waiting"],
  unavailable: ["不可用", "Unavailable"],
  unknown: ["未知", "Unknown"],
});

const KNOWN_STATES = new Set(Object.keys(STATE_LABELS));
const KNOWN_FRESHNESS = new Set(Object.keys(FRESHNESS_LABELS));

function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function textValue(value) {
  return value === null || value === undefined ? "" : String(value).trim();
}

function normalized(value) {
  return textValue(value).toLowerCase();
}

function localeIsZh(locale) {
  return normalized(locale).startsWith("zh");
}

function localized(locale, values) {
  return values[localeIsZh(locale) ? 0 : 1];
}

function firstValue(...values) {
  return values.find((value) => textValue(value)) || null;
}

function displayIdentity(entity, fallbackId) {
  const id = textValue(entity?.id) || textValue(fallbackId) || null;
  const name = firstValue(entity?.name, entity?.label, id);
  const label = firstValue(entity?.label, entity?.name, id);
  return { name, label, id };
}

function section(value) {
  const display = textValue(value);
  return { value: display || null, state: display ? "published" : "unavailable" };
}

function stateModel(value, locale) {
  const kind = normalized(value);
  const safeKind = KNOWN_STATES.has(kind) ? kind : "unavailable";
  return { kind: safeKind, label: localized(locale, STATE_LABELS[safeKind]) };
}

function freshnessModel(value, connectionStatus, locale) {
  let kind = normalized(value);
  if (!kind) {
    const connection = normalized(connectionStatus);
    if (connection === "connecting" || connection === "reconnecting") {
      kind = "reconnecting";
    } else if (connection && connection !== "connected") {
      kind = "unavailable";
    } else {
      kind = "unknown";
    }
  }
  if (!KNOWN_FRESHNESS.has(kind)) kind = "unknown";
  return { kind, label: localized(locale, FRESHNESS_LABELS[kind]) };
}

function activityModel(activity, locale) {
  const described = describeAgentActivity(activity, locale);
  return {
    kind: described.kind,
    label: described.label,
    operation: described.operation || "",
    targetLabel: described.targetLabel || "",
    reason: described.reason || "",
    surface: {
      status: described.kind,
      operation: described.operation || null,
      target: described.targetLabel || null,
      reason: described.reason || null,
    },
  };
}

function matchingIntent(gameplay, selectedId, explicitIntent) {
  const candidate = explicitIntent ?? gameplay?.primaryIntent ?? gameplay?.primary_intent;
  if (!isRecord(candidate)) return null;
  if (normalized(candidate.agent_id) !== normalized(selectedId)) return null;
  const targetAgentId = candidate.target_agent_id ?? candidate.targetAgentId;
  if (textValue(targetAgentId) && normalized(targetAgentId) !== normalized(selectedId)) return null;
  return candidate;
}

function intentModel(candidate, selectedId, locale, connectionStatus) {
  if (!candidate) {
    return {
      kind: "unavailable",
      label: localeIsZh(locale) ? "意图不可用" : "Intent unavailable",
      agentId: null,
    };
  }
  const source = { ...candidate };
  const described = describeAgentIntent(source, locale, connectionStatus);
  return {
    kind: "matched",
    agentId: selectedId,
    status: textValue(source.status).toLowerCase() || null,
    state: described.kind,
    label: described.label,
    receiptState: described.receiptState || "not_applicable",
    source,
  };
}

function feedbackModel(feedback, locale) {
  if (!isRecord(feedback)) return { kind: "none", state: "none" };
  const stage = firstValue(feedback.stage, feedback.status);
  const value = firstValue(
    feedback.effect,
    feedback.reason,
    feedback.response?.message,
  );
  return {
    kind: "status",
    state: stage || "updated",
    label: localeIsZh(locale) ? "反馈状态" : "Feedback status",
    value,
  };
}

function receiptModel(receipt) {
  if (!isRecord(receipt)) return { present: false, state: "none" };
  const targetAgentId = firstValue(receipt.target_agent_id, receipt.targetAgentId);
  return {
    present: receipt.present === true,
    state: firstValue(receipt.state) || (receipt.present === true ? "present" : "none"),
    confidence: firstValue(receipt.confidence),
    targetAgentId,
  };
}

function emptyEntityModel(selected, snapshot, locale) {
  const kind = normalized(selected?.kind) || "unknown";
  const id = textValue(selected?.id) || null;
  const collection = isRecord(snapshot?.model?.[`${kind}s`]) ? snapshot.model[`${kind}s`] : {};
  const entity = id ? collection[id] : null;
  return {
    kind,
    id,
    identity: displayIdentity(entity, id),
    state: stateModel("unavailable", locale),
    freshness: freshnessModel("unavailable", "unavailable", locale),
    activity: null,
    objective: null,
    nextMove: null,
    blocker: null,
    playerLeverage: null,
    intent: null,
    feedback: { kind: "none", state: "none" },
    receipt: { present: false, state: "none" },
    unavailableReason: localeIsZh(locale)
      ? "该实体的专属上下文尚未发布；请等待对应投影同步。"
      : "Context unavailable: this entity has no published context projection yet; wait for its projection to sync.",
  };
}

export function buildAgentContextDisplayModel(input = {}) {
  const locale = input.locale || "en";
  const selected = input.selected || {};
  const kind = normalized(selected.kind);
  const id = textValue(selected.id) || null;
  const snapshot = input.snapshot || {};
  if (kind !== "agent") return emptyEntityModel(selected, snapshot, locale);

  const agent = input.agent || snapshot?.model?.agents?.[id] || null;
  const visible = input.agentVisible !== false;
  if (!id || !agent || !visible) {
    return {
      kind: "agent",
      id,
      identity: displayIdentity(agent, id),
      state: stateModel("unavailable", locale),
      freshness: freshnessModel("unavailable", "unavailable", locale),
      activity: activityModel(null, locale),
      objective: section(null),
      nextMove: section(null),
      blocker: section(null),
      playerLeverage: section(null),
      intent: intentModel(null, id, locale, input.connectionStatus),
      feedback: feedbackModel(null, locale),
      receipt: receiptModel(null),
      unavailableReason: localeIsZh(locale)
        ? "当前 Agent 对本地会话不可用。"
        : "This Agent is unavailable to the current session.",
    };
  }

  const suppliedGameplay = isRecord(input.gameplay) ? input.gameplay : {};
  const gameplayAgentId = textValue(firstValue(
    suppliedGameplay.agent_id,
    suppliedGameplay.agentId,
    suppliedGameplay.target_agent_id,
    suppliedGameplay.targetAgentId,
  ));
  // The gameplay summary is normally player-global.  It may contribute
  // Agent Context fields only when its authority explicitly binds it to the
  // selected Agent; selection alone must never manufacture that binding.
  const gameplay = gameplayAgentId === id ? suppliedGameplay : {};
  // Intent carries its own Agent identity and is validated independently of
  // whether the surrounding gameplay summary is Agent-bound.
  const candidateIntent = matchingIntent(suppliedGameplay, id, input.intent);
  const connectionStatus = input.connectionStatus || "unknown";
  const activity = activityModel(agent.activity, locale);
  const state = stateModel(firstValue(agent.state, agent.status), locale);
  const freshness = freshnessModel(
    firstValue(input.freshness, agent.freshness, candidateIntent?.freshness),
    connectionStatus,
    locale,
  );
  const locationId = firstValue(agent.location_id, agent.locationId);
  const location = locationId ? snapshot?.model?.locations?.[locationId] : null;
  const progression = gameplay.progressionProof || gameplay.progression_proof || {};

  return {
    kind: "agent",
    id,
    identity: displayIdentity(agent, id),
    location: location ? displayIdentity(location, locationId) : null,
    state,
    freshness,
    activity,
    objective: section(firstValue(gameplay.objective, gameplay.goalTitle, gameplay.goal_title)),
    nextMove: section(firstValue(
      gameplay.nextStepHint,
      gameplay.next_step_hint,
      gameplay.narrativeNextStep,
      gameplay.narrative_next_step,
    )),
    blocker: section(firstValue(
      gameplay.blockerDetail,
      gameplay.blocker_detail,
      gameplay.narrativeBlockerDetail,
      gameplay.narrative_blocker_detail,
    )),
    playerLeverage: section(firstValue(
      progression.leverageVerdict,
      progression.leverage_verdict,
      gameplay.playerLeverage,
      gameplay.player_leverage,
    )),
    intent: intentModel(candidateIntent, id, locale, connectionStatus),
    feedback: feedbackModel(input.feedback, locale),
    receipt: receiptModel(input.receipt),
    connectionStatus,
    control: firstValue(input.controlState, candidateIntent?.control_state, candidateIntent?.controlState) || "unknown",
  };
}
