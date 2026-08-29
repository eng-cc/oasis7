export function pixelWorldReadableAgentLabel(agent, fallbackId = "") {
  const id = String(agent?.id || fallbackId || "").trim();
  const explicitLabel = String(agent?.name || agent?.label || "").trim();
  if (explicitLabel && explicitLabel !== id) return explicitLabel;
  const numericAgentId = id.match(/^agent[-_](\d+)$/i);
  return numericAgentId ? `Agent ${numericAgentId[1]}` : explicitLabel || id;
}

export function pixelWorldReadableLocationLabel(location, fallbackId = "", isLocaleZh = false) {
  const id = String(location?.id || fallbackId || "").trim();
  const explicitLabel = String(location?.name || location?.label || "").trim();
  if (explicitLabel && explicitLabel !== id) return explicitLabel;
  const numericLocationId = id.match(/^location[-_](\d+)$/i);
  if (numericLocationId) return isLocaleZh ? `地点 ${numericLocationId[1]}` : `Location ${numericLocationId[1]}`;
  return explicitLabel || id;
}

export function pixelWorldReadableEntityText(text, visualState, isLocaleZh = false) {
  return String(text || "").replace(/\bagent[-_]\d+\b/gi, (id) => {
    const agent = visualState?.agents?.find((candidate) => candidate.id === id);
    const label = pixelWorldReadableAgentLabel(agent, id);
    const numericAgent = label.match(/^Agent (\d+)$/i);
    return numericAgent && isLocaleZh ? `行动体 ${numericAgent[1]}` : label;
  });
}

export function pixelWorldSelectedEntityLabel(visualState, selection, isLocaleZh = false) {
  if (!selection) return "";
  if (selection.kind === "agent") {
    const agent = visualState.agents.find((candidate) => candidate.id === selection.id);
    const label = pixelWorldReadableAgentLabel(agent, selection.id);
    const numericAgent = label.match(/^Agent (\d+)$/i);
    return numericAgent && isLocaleZh ? `行动体 ${numericAgent[1]}` : label;
  }
  const location = visualState.locations.find((candidate) => candidate.id === selection.id);
  return pixelWorldReadableLocationLabel(location, selection.id, isLocaleZh);
}
