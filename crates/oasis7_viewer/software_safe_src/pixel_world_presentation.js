function isZhLocale(locale) {
  return String(locale || "").trim().toLowerCase().startsWith("zh");
}

function localized(locale, zh, en) {
  return isZhLocale(locale) ? zh : en;
}

function normalizedToken(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[-\s]+/g, "_");
}

const BLOCKER_PRESENTATIONS = {
  runtime_snapshot_empty_entities: {
    label: ["尚未发布世界实体", "No published entities yet"],
    reason: [
      "权威快照尚未发布可用的 Agent 或地点。",
      "The authoritative snapshot has no published agents or locations yet.",
    ],
    nextAction: [
      "重新加载权威快照，或等待第一个世界实体发布。",
      "Reload the authoritative snapshot or wait for the first published entity.",
    ],
  },
  material_shortage: {
    label: ["缺料", "Missing Material"],
    reason: ["当前行动缺少已发布的物料投入。", "The current action is missing a published material input."],
    nextAction: ["先恢复物料流，再重新检查下一步。", "Restore material flow, then recheck the next step."],
  },
  power_shortage: {
    label: ["缺电", "Missing Power"],
    reason: ["当前行动缺少已发布的供电能力。", "The current action is missing published power capacity."],
    nextAction: ["先恢复供电，再重新检查下一步。", "Restore power, then recheck the next step."],
  },
  governance_gate: {
    label: ["治理限制", "Governance Restriction"],
    reason: ["当前行动缺少已发布的许可或治理前提。", "The current action is missing a published permission or governance prerequisite."],
    nextAction: ["先满足许可或治理前提，再重新检查下一步。", "Satisfy the permission or governance prerequisite, then recheck the next step."],
  },
  no_progress: {
    label: ["没有前进", "No Forward Progress"],
    reason: ["最近一次已确认执行没有产生新的世界进展。", "The latest confirmed execution produced no new world progress."],
    nextAction: ["查看明确回执与下一步提示，再决定是否重试。", "Read the explicit receipt and next step before retrying."],
  },
  runtime_sync_unavailable: {
    label: ["运行时同步不可用", "Runtime Sync Unavailable"],
    reason: ["权威运行时同步当前不可验证。", "Authoritative runtime sync is not verifiable right now."],
    nextAction: ["重试权威快照同步。", "Retry the authoritative snapshot sync."],
  },
  execution_world_not_ready: {
    label: ["执行世界未就绪", "Execution World Not Ready"],
    reason: ["执行所需的权威世界状态尚未就绪。", "The authoritative world state required for execution is not ready."],
    nextAction: ["等待或重新加载权威世界状态。", "Wait for or reload the authoritative world state."],
  },
  product_validation: {
    label: ["产品验证失败", "Product Validation Failed"],
    reason: ["当前请求未通过已发布的产品验证。", "The current request did not pass published product validation."],
    nextAction: ["修正请求前提，再重新检查可用动作。", "Correct the request prerequisites, then recheck available actions."],
  },
  product_validation_rejected: {
    label: ["产品验证失败", "Product Validation Failed"],
    reason: ["当前请求在执行前被已发布的产品规则拒绝。", "The current request was rejected by published product rules before execution."],
    nextAction: ["修正请求前提，再重新检查可用动作。", "Correct the request prerequisites, then recheck available actions."],
  },
};

export function pixelWorldBlockerPresentation(code, locale) {
  const rawCode = String(code || "").trim();
  const token = normalizedToken(rawCode);
  const copy = BLOCKER_PRESENTATIONS[token];
  if (!copy) {
    return {
      code: rawCode || null,
      label: localized(locale, "当前阻塞", "Current blocker"),
      reason: localized(locale, "当前阻塞原因尚未发布可读说明。", "A readable reason for the current blocker has not been published."),
      nextAction: localized(locale, "等待下一次权威状态更新。", "Wait for the next authoritative state update."),
    };
  }
  return {
    code: rawCode || token,
    label: localized(locale, copy.label[0], copy.label[1]),
    reason: localized(locale, copy.reason[0], copy.reason[1]),
    nextAction: localized(locale, copy.nextAction[0], copy.nextAction[1]),
  };
}

const CONNECTION_PRESENTATIONS = {
  connected: ["世界连接：在线", "World connection: ONLINE", "online"],
  connecting: ["世界连接：连接中", "World connection: CONNECTING", "connecting"],
  reconnecting: ["世界连接：重新连接中", "World connection: RECONNECTING", "reconnecting"],
  closed: ["世界连接：已关闭", "World connection: CLOSED", "closed"],
  error: ["世界连接：离线", "World connection: OFFLINE", "offline"],
};

export function pixelWorldConnectionPresentation(status, locale) {
  const token = normalizedToken(status);
  const copy = CONNECTION_PRESENTATIONS[token] || CONNECTION_PRESENTATIONS.error;
  return {
    label: localized(locale, copy[0], copy[1]),
    state: copy[2],
    className: `badge pixel-world-readout__connection pixel-world-readout__connection--${copy[2]}`,
  };
}

const FEED_PRESENTATIONS = {
  loading: ["动态新鲜度：同步中", "Feed freshness: SYNCING", "syncing"],
  ready: ["动态新鲜度：实时", "Feed freshness: LIVE", "live"],
  empty: ["动态新鲜度：暂无动态", "Feed freshness: NO EVENTS", "empty"],
  replay: ["动态新鲜度：回放", "Feed freshness: REPLAY", "replay"],
  gap: ["动态新鲜度：断档", "Feed freshness: GAP", "gap"],
  unavailable: ["动态新鲜度：不可用", "Feed freshness: UNAVAILABLE", "unavailable"],
};

export function pixelWorldFeedFreshnessPresentation(status, stale = false, locale) {
  const token = normalizedToken(status);
  const copy = token === "ready" && stale
    ? ["动态新鲜度：陈旧", "Feed freshness: STALE", "stale"]
    : FEED_PRESENTATIONS[token] || FEED_PRESENTATIONS.unavailable;
  return {
    label: localized(locale, copy[0], copy[1]),
    state: copy[2],
    className: `badge pixel-world-readout__feed pixel-world-readout__feed--${copy[2]}`,
  };
}

const LIFECYCLE_LABELS = {
  active: ["进行中", "active"],
  resolved: ["已解决", "resolved"],
  timed_out: ["已超时", "timed out"],
};

export function pixelWorldMajorEventPresentation(event, locale) {
  const value = event || {};
  const lifecycleToken = normalizedToken(value.lifecycle);
  const lifecycle = LIFECYCLE_LABELS[lifecycleToken];
  const numericSeverity = Number(value.severity);
  const hasSeverity = Number.isInteger(numericSeverity) && numericSeverity >= 1 && numericSeverity <= 5;
  const severity = hasSeverity ? String(numericSeverity) : "unknown";
  const severityText = hasSeverity
    ? localized(locale, `严重度 ${severity}`, `severity ${severity}`)
    : localized(locale, "严重度未发布", "severity unavailable");
  const lifecycleText = lifecycle
    ? localized(locale, `危机${lifecycle[0]}`, `Crisis ${lifecycle[1]}`)
    : localized(locale, "危机状态已记录", "Crisis lifecycle recorded");
  const lifecycleState = lifecycleToken && lifecycle ? lifecycleToken : "recorded";
  return {
    label: `${lifecycleText} · ${severityText}`,
    severity,
    lifecycle: lifecycleState,
    shape: `severity-${severity} lifecycle-${lifecycleState}`,
  };
}

function worldBoundValue(bounds, snakeName, camelName) {
  const value = bounds?.[snakeName] ?? bounds?.[camelName];
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function countValue(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) && numeric >= 0 ? Math.floor(numeric) : 0;
}

export function pixelWorldSparseScenePresentation(data = {}, locale) {
  const routeCount = countValue(data.routeCount ?? data.routes);
  const terrainCount = countValue(data.terrainCount ?? data.terrain);
  const locationCount = countValue(data.locationCount ?? data.locations);
  const agentCount = countValue(data.agentCount ?? data.agents);
  const width = worldBoundValue(data.worldBounds, "width_cm", "widthCm");
  const depth = worldBoundValue(data.worldBounds, "depth_cm", "depthCm");
  return {
    routes: routeCount === 0
      ? localized(locale, "此快照没有已发布路线", "No published routes in this snapshot")
      : localized(locale, `已发布路线：${routeCount}`, `Published routes: ${routeCount}`),
    terrain: terrainCount === 0
      ? localized(locale, "此快照没有已发布地形", "No published terrain in this snapshot")
      : localized(locale, `已发布地形：${terrainCount}`, `Published terrain: ${terrainCount}`),
    entities: localized(locale, `已发布实体：${agentCount} 个 Agent · ${locationCount} 个地点`, `Published entities: ${agentCount} agents · ${locationCount} locations`),
    bounds: width != null && depth != null
      ? localized(locale, `已发布范围：${width} × ${depth} cm`, `Published bounds: ${width} × ${depth} cm`)
      : localized(locale, "范围未发布", "Published bounds unavailable"),
    hasSparseTopology: routeCount === 0 || terrainCount === 0,
  };
}
