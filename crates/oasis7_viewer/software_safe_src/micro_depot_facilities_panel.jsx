import { For, Show } from "solid-js";

function isRecord(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

function displayableStrings(value) {
  return Array.isArray(value)
    ? value
      .filter((entry) => typeof entry === "string")
      .map((entry) => entry.trim())
      .filter(Boolean)
    : [];
}

function resourceSummary(resources) {
  const entries = isRecord(resources) ? Object.entries(resources) : [];
  return entries.length
    ? entries.map(([kind, units]) => `${kind}=${units}`).join(" · ")
    : "-";
}

function shortHash(value) {
  const normalized = String(value || "").trim();
  if (normalized.length <= 18) return normalized || "-";
  return `${normalized.slice(0, 10)}…${normalized.slice(-6)}`;
}

function facilityStatusLabel(facility, locale, tr) {
  if (!facility.status) return tr(locale, "等待状态", "Waiting for status");
  return facility.status;
}

export function MicroDepotFacilitiesPanel(props) {
  const facilities = () => (Array.isArray(props.facilities) ? props.facilities : []).filter(isRecord);
  const locale = () => props.locale();
  const tr = props.tr;
  return (
    <Show when={facilities().length > 0}>
      <section class="panel panel--nested" data-testid="micro-depot-facilities-panel">
        <div class="panel__header">
          <div class="stack stack--compact">
            <div class="panel__eyebrow">{tr(locale(), "区域设施", "Regional Facility")}</div>
            <div class="panel__title">{tr(locale(), "Micro Depot", "Micro Depot")}</div>
            <div class="panel__meta-copy">
              {tr(locale(), "仅显示当前规范玩法快照已发布的状态、模块和回执证据；动作需由运行时另行发布。", "Shows only state, module, and receipt evidence published by the canonical gameplay snapshot; actions remain runtime-published.")}
            </div>
          </div>
        </div>
        <div class="panel__body stack">
          <For each={facilities()}>
            {(facility) => (
              <div class="event-card" data-testid={`micro-depot-facility-${facility.facilityId || "unknown"}`}>
                <div class="event-card__title">
                  <span>{facility.facilityId || tr(locale(), "未命名 depot", "Unnamed depot")}</span>
                  <span class="badge badge--accent">{facilityStatusLabel(facility, locale(), tr)}</span>
                </div>
                <div class="event-card__meta">
                  {`claim=${facility.ownerClaimId || "-"} · location=${facility.locationId || "-"} · ${tr(locale(), "半径", "radius")}=${facility.serviceRadiusCm ?? "-"}cm`}
                </div>
                <div class="summary-grid">
                  <div class="metric">
                    <div class="metric__label">{tr(locale(), "库存", "Inventory")}</div>
                    <div class="metric__value">{resourceSummary(facility.availableUnitsByKind)}</div>
                    <div class="feedback-detail">{`${tr(locale(), "修订", "revision")}=${facility.inventoryRevision ?? "-"}`}</div>
                  </div>
                  <div class="metric">
                    <div class="metric__label">{tr(locale(), "吞吐", "Throughput")}</div>
                    <div class="metric__value">{`${facility.throughputRemainingUnits ?? "-"}/${facility.throughputLimitUnitsPerEpoch ?? "-"}`}</div>
                    <div class="feedback-detail">{`${tr(locale(), "epoch", "epoch")}=${facility.throughputEpoch ?? "-"} · ${tr(locale(), "upkeep", "upkeep")}=${facility.upkeepPaid == null ? "-" : facility.upkeepPaid ? tr(locale(), "已付", "paid") : tr(locale(), "未付", "unpaid")}`}</div>
                  </div>
                  <div class="metric">
                    <div class="metric__label">{tr(locale(), "模块证据", "Module Evidence")}</div>
                    <div class="metric__value">{facility.moduleId || "-"}</div>
                    <div class="feedback-detail">{`${facility.moduleVersion || "-"} · wasm=${shortHash(facility.wasmHash)}`}</div>
                  </div>
                  <div class="metric">
                    <div class="metric__label">{tr(locale(), "回执 / 提案", "Receipt / Proposal")}</div>
                    <div class="metric__value">{shortHash(facility.lastReceiptId)}</div>
                    <div class="feedback-detail">{`proposal=${shortHash(facility.lastProposalHash)}`}</div>
                  </div>
                </div>
                <Show when={displayableStrings(facility.supportedResourceKinds).length > 0}>
                  <div class="feedback-detail">{`${tr(locale(), "支持资源", "Supported resources")}: ${displayableStrings(facility.supportedResourceKinds).join(", ")}`}</div>
                </Show>
                <Show when={displayableStrings(facility.availableActions).length > 0} fallback={<div class="feedback-detail">{tr(locale(), "当前快照没有发布可用 depot 动作。", "The current snapshot publishes no available depot actions.")}</div>}>
                  <div class="badge-row badge-row--spaced" aria-label={tr(locale(), "可用 depot 动作", "Available depot actions")}>
                    <For each={displayableStrings(facility.availableActions)}>{(action) => <span class="badge">{action}</span>}</For>
                  </div>
                </Show>
              </div>
            )}
          </For>
        </div>
      </section>
    </Show>
  );
}
