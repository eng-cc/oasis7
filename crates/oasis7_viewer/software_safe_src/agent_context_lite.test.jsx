import { cleanup, render, screen, within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { AgentContextLite } from "./agent_context_lite.jsx";
import { buildAgentContextDisplayModel } from "./viewer_agent_context_display_model.js";

const AGENT_ID = "agent-0";
const LOCATION_ID = "loc-0";

const agent = {
  id: AGENT_ID,
  name: "Atlas",
  label: "Northline Scout",
  location_id: LOCATION_ID,
  state: "executing",
  freshness: "current",
  activity: {
    status: "executing",
    operation: "route",
    target: "Factory Anchor",
    updated_at: 12,
  },
};

const location = {
  id: LOCATION_ID,
  name: "Factory Anchor",
  label: "Factory Anchor",
};

const intent = {
  schema_version: 2,
  intent_id: "intent:agent-0:accepted",
  agent_id: AGENT_ID,
  target_agent_id: AGENT_ID,
  world_id: "world-test",
  reorg_epoch: 0,
  logical_time: 12,
  event_seq: 4,
  updated_at: 12,
  status: "accepted",
  source_class: "runtime_projection",
  freshness: "current",
  control_state: "controllable",
  copy_schema_version: 1,
  summary: "Agent guidance accepted; the Agent will evaluate its next world action.",
};

const gameplay = {
  agent_id: AGENT_ID,
  objective: "Stabilize the first production line before expanding.",
  nextStepHint: "Replenish upstream materials, then advance again to confirm the line resumes.",
  blockerDetail: "iron input exhausted at factory-0",
  progressionProof: {
    leverageVerdict: "watch: recovery can restore the first capability",
    leverageClass: "repair_elasticity",
  },
  primaryIntent: intent,
};

function contextModel(selected = { kind: "agent", id: AGENT_ID }, overrides = {}) {
  return buildAgentContextDisplayModel({
    snapshot: {
      model: {
        agents: { [AGENT_ID]: agent },
        locations: { [LOCATION_ID]: location },
      },
    },
    selected,
    gameplay,
    connectionStatus: "connected",
    freshness: "current",
    feedback: null,
    receipt: null,
    locale: "en",
    ...overrides,
  });
}

afterEach(() => cleanup());

describe("AgentContextLite player-facing contract", () => {
  it("renders selected-Agent identity, activity, freshness, objective, blocker, leverage, and matching Intent", () => {
    render(() => <AgentContextLite model={contextModel()} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface).toHaveAttribute("data-agent-context-kind", "agent");
    expect(within(surface).getByText("Atlas")).toBeInTheDocument();
    expect(within(surface).getByText("Northline Scout")).toBeInTheDocument();
    expect(within(surface).getByText("Executing Route")).toBeInTheDocument();
    expect(within(surface).getByText("Current")).toBeInTheDocument();
    expect(within(surface).getByText(gameplay.objective)).toBeInTheDocument();
    expect(within(surface).getByText(gameplay.nextStepHint)).toBeInTheDocument();
    expect(within(surface).getByText(gameplay.blockerDetail)).toBeInTheDocument();
    expect(within(surface).getByText(gameplay.progressionProof.leverageVerdict)).toBeInTheDocument();
    expect(within(surface).getByText("Accepted")).toBeInTheDocument();
  });

  it("renders explicit unavailable and waiting copy for missing Agent-bound gameplay fields", () => {
    const model = contextModel({ kind: "agent", id: AGENT_ID }, {
      gameplay: { ...gameplay, agent_id: null, primaryIntent: intent },
    });

    render(() => <AgentContextLite model={model} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface.querySelectorAll('[data-agent-context-field-state="unavailable"]')).toHaveLength(4);
    expect(within(surface).getByText("Objective")).toBeInTheDocument();
    expect(within(surface).getByText("Next Move")).toBeInTheDocument();
    expect(within(surface).getByText("Blocker")).toBeInTheDocument();
    expect(within(surface).getByText("Player Leverage")).toBeInTheDocument();
    expect(surface).toHaveTextContent(/Unavailable.*waiting for authoritative Agent projection/i);
  });

  it("keeps mismatched or null Intent visibly unavailable without leaking a private Agent plan", () => {
    const mismatched = {
      ...intent,
      intent_id: "intent:other-agent:accepted",
      agent_id: "agent-1",
      target_agent_id: "agent-1",
    };
    const mismatchedModel = contextModel({ kind: "agent", id: AGENT_ID }, {
      gameplay: { ...gameplay, primaryIntent: mismatched },
    });
    const nullModel = contextModel({ kind: "agent", id: AGENT_ID }, {
      gameplay: { ...gameplay, primaryIntent: null, primary_intent: null },
    });

    const first = render(() => <AgentContextLite model={mismatchedModel} locale="en" />);
    const mismatchedSurface = screen.getByRole("region", { name: /Agent Context/i });
    expect(mismatchedSurface).toHaveAttribute("data-agent-context-intent", "unavailable");
    expect(mismatchedSurface).toHaveTextContent(/Intent unavailable/i);
    expect(mismatchedSurface).not.toHaveTextContent("Accepted");
    first.unmount();

    render(() => <AgentContextLite model={nullModel} locale="en" />);
    const nullSurface = screen.getByRole("region", { name: /Agent Context/i });
    expect(nullSurface).toHaveAttribute("data-agent-context-intent", "unavailable");
    expect(nullSurface).toHaveTextContent(/Intent unavailable/i);
    expect(nullSurface).not.toHaveTextContent("primary_intent");
  });

  it("does not render an unbound global gameplay summary for the selected Agent in a multi-Agent world", () => {
    const globalGameplay = {
      ...gameplay,
      agent_id: null,
      objective: "Global player objective must stay outside Agent Context.",
      nextStepHint: "Global player next step must stay outside Agent Context.",
      blockerDetail: "Global player blocker must stay outside Agent Context.",
      progressionProof: {
        ...gameplay.progressionProof,
        leverageVerdict: "Global player leverage must stay outside Agent Context.",
      },
      primaryIntent: intent,
    };
    const model = contextModel({ kind: "agent", id: AGENT_ID }, {
      snapshot: {
        model: {
          agents: {
            [AGENT_ID]: agent,
            "agent-1": { ...agent, id: "agent-1", name: "Other Agent", label: "Other Scout" },
          },
          locations: { [LOCATION_ID]: location },
        },
      },
      gameplay: globalGameplay,
    });

    render(() => <AgentContextLite model={model} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface).not.toHaveTextContent("Global player objective must stay outside Agent Context.");
    expect(surface).not.toHaveTextContent("Global player next step must stay outside Agent Context.");
    expect(surface).not.toHaveTextContent("Global player blocker must stay outside Agent Context.");
    expect(surface).not.toHaveTextContent("Global player leverage must stay outside Agent Context.");
    expect(surface).toHaveTextContent("Accepted");
  });

  it("renders honest unavailable state for a selected non-Agent and never reuses previous Agent context", () => {
    render(() => <AgentContextLite model={contextModel({ kind: "location", id: LOCATION_ID })} locale="en" />);

    const surface = screen.getByRole("region", { name: /Entity Context/i });
    expect(surface).toHaveAttribute("data-agent-context-kind", "location");
    expect(within(surface).getByText("Factory Anchor")).toBeInTheDocument();
    expect(surface).toHaveTextContent(/unavailable|待同步/i);
    expect(surface).not.toHaveTextContent("Atlas");
    expect(surface).not.toHaveTextContent("Executing Route");
    expect(surface).not.toHaveTextContent(/repair_elasticity|Accepted|Stabilize the first production line/i);
  });

  it("does not promote non-causal feedback to a second formal Receipt", () => {
    const model = contextModel({ kind: "agent", id: AGENT_ID }, {
      feedback: {
        kind: "gameplay_action",
        agentId: AGENT_ID,
        stage: "ack",
        effect: "The request was queued for runtime evaluation.",
      },
      receipt: null,
    });

    render(() => <AgentContextLite model={model} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface).toHaveAttribute("data-agent-context-receipt", "none");
    expect(surface.querySelectorAll('[data-viewer-overlay="receipt"]')).toHaveLength(0);
    expect(surface.querySelectorAll("#viewer-action-receipt")).toHaveLength(0);
    expect(surface).not.toHaveTextContent(/Action Receipt|World change confirmed/i);
  });

  it("clears Agent A feedback from the rendered Context after selection switches to Agent B", () => {
    const feedback = {
      kind: "chat",
      agentId: AGENT_ID,
      stage: "ack",
      effect: "Agent A received the instruction.",
    };
    const firstModel = contextModel({ kind: "agent", id: AGENT_ID }, { feedback });
    const secondModel = contextModel({ kind: "agent", id: "agent-1" }, {
      snapshot: {
        model: {
          agents: {
            [AGENT_ID]: agent,
            "agent-1": { ...agent, id: "agent-1", name: "Boreal", label: "Southline Scout" },
          },
          locations: { [LOCATION_ID]: location },
        },
      },
      feedback,
    });

    const first = render(() => <AgentContextLite model={firstModel} locale="en" />);
    const firstSurface = screen.getByRole("region", { name: /Agent Context/i });
    expect(firstSurface).toHaveTextContent(feedback.effect);
    first.unmount();

    render(() => <AgentContextLite model={secondModel} locale="en" />);
    const secondSurface = screen.getByRole("region", { name: /Agent Context/i });
    expect(secondSurface).not.toHaveTextContent(feedback.effect);
    expect(secondSurface.querySelectorAll('[data-agent-context-feedback="status"]')).toHaveLength(0);
  });

  it("can reference one supplied Receipt without owning or duplicating the formal Receipt surface", () => {
    const model = contextModel({ kind: "agent", id: AGENT_ID }, {
      receipt: {
        present: true,
        state: "confirmed",
        confidence: "world_delta",
        title: "Action Receipt",
        summary: "The world change was confirmed.",
        target_agent_id: AGENT_ID,
      },
    });
    const host = document.createElement("div");
    host.innerHTML = '<div id="viewer-action-receipt" data-viewer-overlay="receipt">Existing receipt</div>';
    document.body.appendChild(host);

    render(() => <AgentContextLite model={model} locale="en" />);

    expect(document.querySelectorAll('[data-viewer-overlay="receipt"]')).toHaveLength(1);
    expect(document.querySelectorAll("#viewer-action-receipt")).toHaveLength(1);
  });

  it("does not expose a Receipt targeting another Agent in the selected context", () => {
    const model = contextModel({ kind: "agent", id: AGENT_ID }, {
      receipt: {
        present: true,
        state: "confirmed",
        confidence: "world_delta",
        target_agent_id: "agent-1",
      },
    });

    render(() => <AgentContextLite model={model} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface).toHaveAttribute("data-agent-context-receipt", "none");
    expect(surface).not.toHaveAttribute("data-agent-context-receipt", "present");
    expect(surface.querySelectorAll('[data-viewer-overlay="receipt"]')).toHaveLength(0);
  });

  it("keeps completed Intent lifecycle separate from the single formal Receipt presentation", () => {
    const completedIntent = {
      ...intent,
      status: "completed",
      summary: "Agent guidance completed with a confirmed world receipt.",
      receipt_ref: {
        receipt_id: "world-event:4",
        intent_id: intent.intent_id,
        world_id: intent.world_id,
        reorg_epoch: intent.reorg_epoch,
        logical_time: intent.logical_time,
        event_seq: intent.event_seq,
      },
    };
    const model = contextModel(undefined, {
      gameplay: { ...gameplay, primaryIntent: completedIntent },
    });

    render(() => <AgentContextLite model={model} locale="en" />);

    const surface = screen.getByRole("region", { name: /Agent Context/i });
    expect(surface).toHaveTextContent("Completed");
    expect(surface).not.toHaveTextContent("World receipt confirmed");
    expect(surface.querySelectorAll('[data-viewer-overlay="receipt"]')).toHaveLength(0);
  });
});
