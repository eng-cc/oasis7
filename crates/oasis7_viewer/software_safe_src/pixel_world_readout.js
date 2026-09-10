function isZhLocale(locale) {
  return String(locale || "").trim().toLowerCase().startsWith("zh");
}

function tr(locale, zh, en) {
  return isZhLocale(locale) ? zh : en;
}

function warn(label, state) {
  return {
    label,
    className: `badge badge--warn pixel-world-readout__status pixel-world-readout__status--${state}`,
  };
}

export function resolvePixelWorldReadoutStatus(locale, connectionStatus, worldFeed = {}) {
  const status = String(connectionStatus || "").trim().toLowerCase();
  const feedStatus = String(worldFeed.status || "").trim().toLowerCase();
  if (feedStatus === "unavailable") return warn(tr(locale, "不可用", "UNAVAILABLE"), "unavailable");
  if (feedStatus === "gap") return warn(tr(locale, "断档", "GAP"), "gap");
  if (feedStatus === "replay") return { label: tr(locale, "回放", "REPLAY"), className: "badge badge--accent pixel-world-readout__status pixel-world-readout__status--replay" };
  if (feedStatus === "empty") return { label: tr(locale, "暂无动态", "NO EVENTS"), className: "badge pixel-world-readout__status pixel-world-readout__status--empty" };
  if (worldFeed.stale) return warn(tr(locale, "陈旧", "STALE"), "stale");
  if (status === "connecting" || status === "reconnecting") return warn(tr(locale, "正在重连", "RECONNECTING"), "reconnecting");
  if (status !== "connected") return warn(tr(locale, "离线", "OFFLINE"), "offline");
  return feedStatus === "ready"
    ? { label: tr(locale, "实时", "LIVE"), className: "badge badge--good pixel-world-readout__status pixel-world-readout__status--ready" }
    : warn(tr(locale, "同步中", "SYNCING"), "syncing");
}
