import { AGENT_INTENT_SUMMARIES } from "./agent_intent_surface.jsx";

export const AGENT_CONTEXT_FIXTURE_MODES = Object.freeze(["rich", "unavailable"]);
export const AGENT_CONTEXT_FIXTURE_STATES = Object.freeze(["current", "stale", "reconnecting"]);
export const AGENT_CONTEXT_FIXTURE_COPIES = Object.freeze(["short", "long"]);

const FIXTURE_SCHEMA = "agent-context-fixture/v1";
const MEASUREMENT_HOOK = "groups-fields";

const COPY = Object.freeze({
  en: Object.freeze({
    short: Object.freeze({
      objective: "Stabilize the first production line before expanding.",
      nextStepHint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      blockerDetail: "iron input exhausted at factory-0",
      leverageVerdict: "Watch: recovery can restore the first capability.",
    }),
    long: Object.freeze({
      objective: "Stabilize the first production line before expanding while keeping reserve for the next material interruption.",
      nextStepHint: "Replenish upstream materials at Factory Anchor, advance one beat, and confirm the line resumes before expanding again.",
      blockerDetail: "iron input remains exhausted at factory-0; production waits for a confirmed upstream refill before resuming.",
      leverageVerdict: "Restore the upstream material path first; this preserves the current line, makes the next move observable, and keeps the expansion option open without inventing a new control.",
    }),
  }),
  zh: Object.freeze({
    short: Object.freeze({
      objective: "先稳定第一条生产线，再考虑扩张。",
      nextStepHint: "补充上游材料，然后推进一个节拍确认生产线恢复。",
      blockerDetail: "factory-0 的铁输入已耗尽",
      leverageVerdict: "先恢复上游材料路径，当前能力即可继续运转。",
    }),
    long: Object.freeze({
      objective: "先稳定第一条生产线，再扩张，同时保留储备应对下一次材料中断。",
      nextStepHint: "在 Factory Anchor 补充材料，推进一个节拍确认生产线恢复，再决定是否扩张。",
      blockerDetail: "最近检查后，factory-0 的铁输入仍然耗尽；确认上游补充前，生产无法恢复。",
      leverageVerdict: "先恢复上游材料路径；这样可以保住当前生产线，让下一步可观察，并在不凭空增加控制项的情况下保留扩张选项。",
    }),
  }),
});

function oneOf(value, choices, fallback) {
  return choices.includes(value) ? value : fallback;
}

function queryValue(params, name, fallback) {
  return String(params.get(name) || fallback).trim().toLowerCase();
}

function localeKey(locale) {
  return String(locale || "").toLowerCase().startsWith("zh") ? "zh" : "en";
}

function fixtureIntent(state) {
  return {
    schema_version: 2,
    intent_id: "agent-context-fixture:intent",
    status: "accepted",
    summary: AGENT_INTENT_SUMMARIES.accepted,
    source_class: "runtime_projection",
    freshness: state.state,
    control_state: "controllable",
    agent_id: "agent-0",
    target_agent_id: "agent-0",
    world_id: "agent-context-fixture-world",
    reorg_epoch: 0,
    logical_time: "9007199254740993",
    updated_at: "9007199254740993",
    event_seq: "9007199254740994",
    reason_code: null,
    reason_summary: null,
    next_step: null,
  };
}

export function buildAgentContextFixtureState(search = window.location.search || "") {
  const params = new URLSearchParams(search);
  return {
    mode: oneOf(queryValue(params, "agent_context_mode", "unavailable"), AGENT_CONTEXT_FIXTURE_MODES, "unavailable"),
    state: oneOf(queryValue(params, "agent_context_state", "current"), AGENT_CONTEXT_FIXTURE_STATES, "current"),
    copy: oneOf(queryValue(params, "agent_context_copy", "short"), AGENT_CONTEXT_FIXTURE_COPIES, "short"),
  };
}

export function buildAgentContextRichFixtureSnapshot(viewerFixtureBaseSnapshot, state, locale = "en") {
  const base = viewerFixtureBaseSnapshot();
  const selectedState = AGENT_CONTEXT_FIXTURE_STATES.includes(state?.state) ? state.state : "current";
  const selectedCopy = AGENT_CONTEXT_FIXTURE_COPIES.includes(state?.copy) ? state.copy : "short";
  const selectedMode = AGENT_CONTEXT_FIXTURE_MODES.includes(state?.mode) ? state.mode : "unavailable";
  const fixtureState = { mode: selectedMode, state: selectedState, copy: selectedCopy };
  const copy = COPY[localeKey(locale)][selectedCopy];
  const intent = fixtureIntent(fixtureState);
  const gameplay = {
    agent_id: "agent-0",
    objective: copy.objective,
    nextStepHint: copy.nextStepHint,
    blockerDetail: copy.blockerDetail,
    progressionProof: {
      leverageVerdict: copy.leverageVerdict,
      leverageClass: "repair_elasticity",
    },
    primary_intent: intent,
  };
  const playerGameplay = { ...base.player_gameplay };
  if (selectedMode === "unavailable") {
    // Keep this fixture projection-free: it must not accidentally exercise
    // the player-global gameplay fallback in Agent Context.
    delete playerGameplay.objective;
  }
  return {
    ...base,
    model: {
      ...base.model,
      agents: {
        ...base.model.agents,
        "agent-0": {
          ...base.model.agents["agent-0"],
          state: "executing",
          freshness: selectedState,
          activity: { status: "executing" },
        },
      },
    },
    player_gameplay: {
      ...playerGameplay,
      primary_intent: selectedMode === "rich" ? intent : null,
    },
    viewer_test_agent_context: {
      schema_version: 1,
      schema: FIXTURE_SCHEMA,
      mode: selectedMode,
      state: selectedState,
      copy: selectedCopy,
      measurement: MEASUREMENT_HOOK,
      gameplay: selectedMode === "rich" ? gameplay : null,
    },
  };
}

export function readAgentContextFixtureMetadata(snapshot, fixtureName, testApiEnabled) {
  if (!testApiEnabled || fixtureName !== "agent_context") return null;
  const fixture = snapshot?.viewer_test_agent_context;
  if (!fixture || !AGENT_CONTEXT_FIXTURE_MODES.includes(fixture.mode)) return null;
  return {
    mode: fixture.mode,
    state: fixture.state,
    copy: fixture.copy,
    measurement: fixture.measurement,
    schema: fixture.schema,
  };
}

export function readAgentContextFixtureGameplay(snapshot, selectedAgentId, metadata) {
  if (metadata?.mode !== "rich") return null;
  const gameplay = snapshot?.viewer_test_agent_context?.gameplay;
  return gameplay?.agent_id === selectedAgentId ? gameplay : null;
}

export function installAgentContextVisualFixture(
  fixtures,
  { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot },
) {
  fixtures.agent_context = () => {
    const state = buildAgentContextFixtureState();
    const params = new URLSearchParams(window.location.search || "");
    const locale = params.get("locale") || "en";
    const snapshot = buildAgentContextRichFixtureSnapshot(viewerFixtureBaseSnapshot, state, locale);
    core.injectSnapshot(snapshot, { returnState: false });
    core.state.connectionStatus = "connected";
    core.state.lastError = null;
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    document.body.setAttribute("data-agent-context-fixture", state.mode);
    document.body.setAttribute("data-agent-context-fixture-state", state.state);
    document.body.setAttribute("data-agent-context-fixture-copy", state.copy);
    document.body.setAttribute("data-agent-context-measurement", MEASUREMENT_HOOK);
    document.body.setAttribute("data-agent-context-fixture-schema", FIXTURE_SCHEMA);
    core.requestRender();
  };
}
