export function resourceSummary(resources) {
  if (!resources || typeof resources !== "object") {
    return "-";
  }
  return Object.entries(resources)
    .map(([key, value]) => {
      if (value && typeof value === "object") {
        return `${key}:${JSON.stringify(value)}`;
      }
      return `${key}:${value}`;
    })
    .join(" · ") || "-";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export function buildViewerEntityLists({ entityCollections, selectedSearch, isAgentVisibleToCurrentSession }) {
  const { agents, locations } = entityCollections();
  const keyword = String(selectedSearch || "").trim().toLowerCase();
  const filter = (entry, label) => !keyword || String(label).toLowerCase().includes(keyword);
  return {
    agents: agents
      .filter((agent) => isAgentVisibleToCurrentSession(agent.id))
      .filter((agent) => filter(agent, `${agent.id} ${agent.location_id}`))
      .sort((a, b) => String(a.id).localeCompare(String(b.id))),
    locations: locations
      .filter((location) => filter(location, `${location.id} ${location.name}`))
      .sort((a, b) => String(a.id).localeCompare(String(b.id))),
  };
}

export function renderViewerEntityList({ state, lists }) {
  const renderItem = (kind, entry, title, meta) => {
    const selected = state.selectedKind === kind && state.selectedId === entry.id;
    return `
      <button class="list-item" data-select-kind="${kind}" data-select-id="${escapeHtml(entry.id)}" data-selected="${selected}">
        <div class="list-item__title">${escapeHtml(title)}</div>
        <div class="list-item__meta">${escapeHtml(meta)}</div>
      </button>
    `;
  };

  return `
    <div class="stack">
      <div class="field">
        <label for="entity-search">Filter targets</label>
        <input id="entity-search" type="search" placeholder="Search agents or locations" value="${escapeHtml(state.selectedSearch)}" />
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Agents</div>
        <div class="list">
          ${lists.agents.length
            ? lists.agents
                .map((agent) => renderItem("agent", agent, agent.id, `location=${agent.location_id} · resources=${resourceSummary(agent.resources)}`))
                .join("")
            : '<div class="empty">No agents in current snapshot.</div>'}
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Locations</div>
        <div class="list">
          ${lists.locations.length
            ? lists.locations
                .map((location) => renderItem("location", location, location.name || location.id, `id=${location.id} · resources=${resourceSummary(location.resources)}`))
                .join("")
            : '<div class="empty">No locations in current snapshot.</div>'}
        </div>
      </div>
    </div>
  `;
}
