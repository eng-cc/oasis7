import { within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { renderViewerApp as renderViewerAppFixture } from "./test_support/viewer_app_fixture.jsx";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: () => <div data-testid="pixel-world-host" />,
}));

const HEAVY_UI_TEST_TIMEOUT_MS = 60000;
let dispose = null;

function sampleSnapshot(overrides = {}) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    ...overrides,
    config: {
      ...base.config,
      ...(overrides.config || {}),
    },
    model: {
      ...base.model,
      agent_player_bindings: {
        "agent-0": "local-test-player-bound",
      },
      agent_player_public_key_bindings: {
        "agent-0": "abcdef0123456789abcdef0123456789",
      },
      ...(overrides.model || {}),
    },
    player_gameplay: {
      ...base.player_gameplay,
      ...(overrides.player_gameplay || {}),
    },
  };
}

async function renderViewerApp({ snapshot = sampleSnapshot() } = {}) {
  const app = await renderViewerAppFixture(snapshot);
  dispose = app.dispose;
  return app;
}

afterEach(() => {
  dispose?.();
  dispose = null;
});

describe("first-delivery preview viewer contract", () => {
  it("normalizes and renders first-delivery preview fields on expansion tradeoff cards", async () => {
    const { container } = await renderViewerApp({ snapshot: sampleSnapshot({ player_gameplay: {
      ...sampleSnapshot().player_gameplay,
      goal_kind: "ChooseMidLoopPath",
      goal_title: "Choose your mid-loop path",
      branch_hint: "Compare the first delivery before committing.",
      branch_recommendations: [{
        action_id: "schedule_recipe_smelter_alloy_plate",
        route_label: "Deepen the smelter line",
        immediate_gain: "Convert the stable line into alloy output",
        future_beats: ["Serve a regional alloy need", "Open a resilient specialist route"],
        risk_or_lockin: "Consumes the current smelter slot",
        next_session_hook: "Return to choose the next advanced recipe",
        first_delivery_preview: {
          local_need: "Regional fabricators need dependable alloy plates",
          expected_output: "Two alloy plates from the first smelter batch",
          required_inputs: ["iron_ingot × 2", "copper_wire × 2"],
          value_timing: "After one smelter run completes",
          leverage_class_unlocked: "regional_material_supplier",
          return_visit_hook: "Return to fulfill the next regional alloy order",
        },
      }],
      available_actions: [{
        action_id: "schedule_recipe_smelter_alloy_plate",
        label: "Schedule alloy plate",
        protocol_action: "gameplay_action.submit",
        disabled_reason: null,
      }],
    } }) });
    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(within(stagePanel).getByText("First delivery preview")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Local need")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Regional fabricators need dependable alloy plates")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Expected output")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Two alloy plates from the first smelter batch")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Required inputs")).toBeInTheDocument();
  expect(within(stagePanel).getByText(/Iron ingot × 2/)).toBeInTheDocument();
  expect(within(stagePanel).getByText(/Copper wire × 2/)).toBeInTheDocument();
    expect(within(stagePanel).getByText("Value timing")).toBeInTheDocument();
    expect(within(stagePanel).getByText("After one smelter run completes")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Leverage unlocked")).toBeInTheDocument();
  expect(within(stagePanel).getByText("Regional material supplier")).toBeInTheDocument();
  expect(stagePanel).not.toHaveTextContent("regional_material_supplier");
    expect(within(stagePanel).getByText("Return visit hook")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Return to fulfill the next regional alloy order")).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
