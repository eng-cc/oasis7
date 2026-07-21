import { describe, expect, it } from "vitest";
import { buildViewerEntityLists, renderViewerEntityList } from "./viewer_entity_list_renderer.js";

describe("viewer entity list renderer", () => {
  it("renders only visible agents, filters targets, and escapes display values", () => {
    const state = {
      selectedKind: "agent",
      selectedId: "agent-1",
      selectedSearch: "alpha",
    };
    const lists = buildViewerEntityLists({
      selectedSearch: state.selectedSearch,
      entityCollections: () => ({
        agents: [
          { id: "agent-1", location_id: "alpha-base", resources: { ore: 3 } },
          { id: "agent-2", location_id: "hidden-base", resources: { ore: 9 } },
        ],
        locations: [
          { id: "alpha<dock", name: "Alpha <Dock>", resources: { water: 2 } },
        ],
      }),
      isAgentVisibleToCurrentSession: (agentId) => agentId === "agent-1",
    });
    const html = renderViewerEntityList({ state, lists });

    expect(html).toContain('data-select-id="agent-1"');
    expect(html).toContain('data-selected="true"');
    expect(html).not.toContain("agent-2");
    expect(html).toContain("Alpha &lt;Dock&gt;");
    expect(html).toContain('data-select-id="alpha&lt;dock"');
  });
});
