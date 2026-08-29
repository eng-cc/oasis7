function agentIdentityParts(agent, fallbackId = "") {
  const id = String(agent?.id || fallbackId || "").trim();
  const explicitLabel = String(agent?.name || agent?.label || "").trim();
  return {
    id,
    explicitLabel,
    hasExplicitLabel: Boolean(explicitLabel && explicitLabel !== id),
  };
}

function humanizeAgentId(id) {
  const slug = id.match(/^agent[-_](.+)$/i)?.[1] || "";
  if (!slug) return "";
  const words = slug
    .replace(/[-_]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => `${word.slice(0, 1).toUpperCase()}${word.slice(1)}`);
  return words.length > 0 ? `Agent ${words.join(" ")}` : "";
}

export function pixelWorldReadableAgentLabel(agent, fallbackId = "", isLocaleZh = false) {
  const { id, explicitLabel, hasExplicitLabel } = agentIdentityParts(agent, fallbackId);
  const numericAgentId = id.match(/^agent[-_](\d+)$/i);
  const label = hasExplicitLabel
    ? explicitLabel
    : numericAgentId
      ? `Agent ${numericAgentId[1]}`
      : humanizeAgentId(id) || explicitLabel || id;
  if (!isLocaleZh) return label;
  if (hasExplicitLabel) {
    const explicitNumericAgent = label.match(/^Agent (\d+)$/i);
    return explicitNumericAgent ? `行动体 ${explicitNumericAgent[1]}` : label;
  }
  const generatedAgent = label.match(/^Agent (.+)$/i);
  return generatedAgent ? `行动体 ${generatedAgent[1]}` : label;
}

export function pixelWorldReadableLocationLabel(location, fallbackId = "", isLocaleZh = false) {
  const id = String(location?.id || fallbackId || "").trim();
  const explicitLabel = String(location?.name || location?.label || "").trim();
  if (explicitLabel && explicitLabel !== id) return explicitLabel;
  const numericLocationId = id.match(/^(?:location|loc)[-_](\d+)$/i);
  if (numericLocationId) return isLocaleZh ? `地点 ${numericLocationId[1]}` : `Location ${numericLocationId[1]}`;
  return explicitLabel || id;
}

export function pixelWorldReadableEntityText(text, visualState, isLocaleZh = false) {
  return String(text || "").replace(/\bagent[-_][a-z0-9]+(?:[-_][a-z0-9]+)*\b/gi, (id) => {
    const agent = visualState?.agents?.find((candidate) => candidate.id === id);
    if (!agent) return id;
    return pixelWorldReadableAgentLabel(agent, id, isLocaleZh);
  });
}

export function pixelWorldSelectedEntityLabel(visualState, selection, isLocaleZh = false) {
  if (!selection) return "";
  if (selection.kind === "agent") {
    const agent = visualState.agents.find((candidate) => candidate.id === selection.id);
    return pixelWorldReadableAgentLabel(agent, selection.id, isLocaleZh);
  }
  const location = visualState.locations.find((candidate) => candidate.id === selection.id);
  return pixelWorldReadableLocationLabel(location, selection.id, isLocaleZh);
}
