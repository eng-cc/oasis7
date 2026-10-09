import { For, Show } from "solid-js";

const record = (value) => value && typeof value === "object" && !Array.isArray(value);
const text = (value) => typeof value === "string" && value.trim() ? value.trim() : null;
const strings = (value) => Array.isArray(value) ? value.map(text).filter(Boolean) : [];
const number = (value) => typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : null;
const pretty = (value) => text(value)?.replace(/[_-]/g, " ") || null;
function actualEventSummary(event) {
  if (!record(event) || !text(event.type)) return null;
  const detail = ["factory_id", "recipe_id", "accepted_batches", "kind", "amount", "ready_at"]
    .map((key) => text(event[key]) || number(event[key]) !== null
      ? `${pretty(key)}: ${typeof event[key] === "string" ? pretty(event[key]) : event[key]}` : null)
    .filter(Boolean);
  return [pretty(event.type), ...detail].join(" · ");
}

export function buildControlProofAgencyDisplayModel(proof = {}) {
  const agency = record(proof.agency) ? proof.agency : {};
  const candidate = record(agency.causal_receipt) ? agency.causal_receipt : null;
  const receipt = candidate && text(proof.intentId) && candidate.intent_id === proof.intentId
    && text(proof.agentId) && text(candidate.receipt_id) && text(candidate.commit_id)
    && number(candidate.action_id) !== null ? candidate : null;
  const events = receipt && Array.isArray(receipt.domain_event_refs)
    ? receipt.domain_event_refs.filter((ref) => number(ref) !== null) : [];
  const applied = receipt?.disposition === "applied" && events.length > 0;
  const notApplied = receipt?.disposition === "not_applied";
  const prediction = record(receipt?.expected_consequence) ? receipt.expected_consequence : {};
  const unverified = prediction.provenance === "agent_explanation_unverified";
  const memory = record(agency.referenced_memory_context) ? agency.referenced_memory_context : null;
  const corrections = Array.isArray(agency.memory_corrections) ? agency.memory_corrections
    .filter((row) => record(row) && row.agent_id === proof.agentId
      && ["accepted", "applied", "ignored", "stale"].includes(row.status)).map((row) => ({
      id: text(row.correction_id), status: row.status, reason: pretty(row.reason),
      revision: number(row.memory_revision), target: text(row.target_memory_id),
      earliestDecision: text(row.earliest_decision_request_id),
      earliestDigest: text(row.earliest_request_digest),
      committedDecision: text(row.committed_decision_request_id),
      committedDigest: text(row.committed_request_digest),
      receipt: text(row.runtime_receipt_id), action: text(row.action_id),
    })) : [];
  return {
    status: applied ? "applied" : notApplied ? "not_applied" : "unavailable",
    unavailableReason: candidate && !receipt ? "receipt identity does not match the accepted intent"
      : pretty(agency.unavailable_reason) || "waiting for a committed runtime receipt",
    receiptId: text(receipt?.receipt_id), commitId: text(receipt?.commit_id),
    actionId: number(receipt?.action_id), actionKind: pretty(receipt?.action_kind), events,
    intentId: text(proof.intentId), effectIntentId: text(receipt?.effect_intent_id),
    reason: text(receipt?.primary_reason), nextStep: pretty(proof.primaryNextStep) || pretty(receipt?.next_step) || text(proof.nextMove),
    prediction: unverified ? text(prediction.prediction) : null,
    stakes: receipt?.stakes?.provenance === "agent_explanation_unverified" ? text(receipt.stakes.summary) : null,
    alternatives: receipt?.alternative?.provenance === "agent_explanation_unverified" ? strings(receipt.alternative.alternatives) : [],
    evidence: strings(receipt?.evidence_refs), correctionRefs: strings(receipt?.correction_refs),
    interruptionRefs: strings(receipt?.interruption_refs),
    ownerControlRefs: strings(receipt?.owner_control_refs),
    actual: (applied || notApplied) && Array.isArray(receipt?.actual_consequence)
      ? receipt.actual_consequence.map(actualEventSummary).filter(Boolean) : [],
    authorizations: (Array.isArray(agency.delegation_authorizations) ? agency.delegation_authorizations : [])
      .filter((row) => record(row?.grant) && row.grant.agent_id === proof.agentId && row.grant.resource_kind === "electricity"),
    receiptAuthorization: record(receipt?.authorization) && receipt.authorization.grant?.agent_id === proof.agentId
      && receipt.authorization.grant?.resource_kind === "electricity" ? receipt.authorization : null,
    dissent: text(receipt?.dissent), overrideActor: text(receipt?.override_actor), hardBoundary: text(receipt?.hard_boundary),
    memory: memory ? {
      revision: number(memory.revision), scope: pretty(memory.scope), source: pretty(memory.source),
      decision: text(memory.decision_request_id), stale: memory.stale === true,
      used: memory.used_for_decision === true, hint: pretty(memory.correction_hint),
      entries: Array.isArray(memory.entries) ? memory.entries.filter((entry) => text(entry?.id) && text(entry?.summary)).map((entry) => ({ id: entry.id, summary: entry.summary })) : [],
      sources: Array.isArray(memory.sources) ? memory.sources.filter(record).map((source) => ({
        id: text(source.memory_id), receipt: text(source.origin_receipt_id), corrections: strings(source.correction_refs),
      })) : [],
    } : null,
    corrections,
  };
}

function Metric(props) {
  return <div class="metric" style={{ "min-width": "0" }}><div class="metric__label" style={{ "overflow-wrap": "anywhere" }}>{props.label}</div><div class="metric__value" style={{ "white-space": "normal", "overflow-wrap": "anywhere" }}>{props.value}</div></div>;
}

export function ControlProofPanel(props) {
  const proof = () => props.proof || {};
  const model = () => buildControlProofAgencyDisplayModel(proof());
  const tr = (zh, en) => props.tr(props.locale, zh, en);
  const unavailable = () => tr("未提供", "Unavailable");
  const value = (item) => typeof item === "number" ? number(item) ?? unavailable()
    : item === null || item === undefined || item === "" ? unavailable() : item;
  const joined = (items) => items?.length ? items.join(" · ") : unavailable();
  const budgetUnit = (kind) => kind === "electricity" ? tr("电力额度", "electricity units") : unavailable();
  const status = () => model().status === "applied" ? tr("世界效果已提交", "World effect committed")
    : model().status === "not_applied" ? tr("世界效果未生效", "World effect not applied")
      : tr("等待权威回执", "Awaiting authoritative receipt");
  const correctionStatus = (state) => ({
    accepted: tr("已接受，等待后续决定", "Accepted; awaiting next decision"),
    applied: tr("纠正上下文已用于提交决定", "Corrected context used in committed decision"),
    ignored: tr("未用于提交结果", "Not used in committed result"),
    stale: tr("版本已过期", "Revision stale"),
  })[state];
  return <div class="event-card" data-testid="control-proof-panel" data-proof-status={model().status}>
    <div class="event-card__title"><span>{tr("控制证明", "Control Proof")}</span><span class="badge">{status()}</span></div>
    <div class="event-card__meta">{tr("玩家意图、Agent 预测与运行时世界结果分别展示。", "Player intent, Agent predictions and runtime world results are shown separately.")}</div>
    <Show when={props.fixture}><div class="feedback-detail">{tr("展示测试样本：不代表真实运行时执行。", "Display fixture: no runtime execution evidence.")}</div></Show>
    <div class="feedback-summary">{model().status === "unavailable" ? tr("详细因果证明尚不可用；请读取运行时下一步。", "Detailed causal proof is unavailable; read the runtime next step.") : status()}</div>
    <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }}>
      <Metric label={tr("玩家意图", "Player Intent")} value={value(proof().intent)} />
      <Metric label={tr("意图身份", "Intent identity")} value={value(model().intentId)} />
      <Metric label={tr("世界结果", "Actual world result")} value={model().status === "unavailable" ? unavailable() : joined(model().actual)} />
      <Metric label={tr("记录的决定 / 约束原因", "Recorded decision / constraint reason")} value={value(model().reason)} />
      <Metric label={tr("恢复动作", "Recovery Move")} value={value(proof().recovery)} />
      <Metric label={tr("下一步", "Next Move")} value={value(model().nextStep)} />
    </div>
    <Show when={model().status === "unavailable"}><div class="feedback-detail">{model().unavailableReason}</div></Show>
    <Show when={model().receiptId}><div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }} data-testid="control-proof-receipt">
      <Metric label={tr("提交回执", "Committed receipt")} value={value(model().receiptId)} />
      <Metric label={tr("提交身份", "Commit identity")} value={value(model().commitId)} />
      <Metric label={tr("实际动作", "Actual action")} value={`${value(model().actionKind)} · ${value(model().actionId)}`} />
      <Metric label={tr("领域事件引用", "Domain event references")} value={joined(model().events)} />
      <Metric label={tr("效果意图引用", "Effect intent reference")} value={value(model().effectIntentId)} />
    </div></Show>
    <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }} data-testid="control-proof-prediction">
      <Metric label={tr("Agent 预测（未经验证）", "Agent prediction (unverified)")} value={value(model().prediction)} />
      <Metric label={tr("Agent 利害说明（未经验证）", "Agent stakes (unverified)")} value={value(model().stakes)} />
      <Metric label={tr("Agent 替代方案（未经验证）", "Agent alternatives (unverified)")} value={joined(model().alternatives)} />
      <Metric label={tr("证据引用", "Evidence references")} value={joined(model().evidence)} />
      <Metric label={tr("纠正引用", "Correction references")} value={joined(model().correctionRefs)} />
      <Metric label={tr("中断引用", "Interruption references")} value={joined(model().interruptionRefs)} />
      <Metric label={tr("所有者控制引用", "Owner control references")} value={joined(model().ownerControlRefs)} />
    </div>
    <For each={model().authorizations}>{(authorization) => <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }} data-testid="control-proof-authorization">
      <Metric label={tr("授权来源 / 签发者", "Grant source / issuer")} value={`${value(authorization.grant.source_id)} · ${value(authorization.grant.issuer_id)}`} />
      <Metric label={tr("授权身份 / 版本", "Grant identity / revision")} value={`${value(authorization.grant.grant_id)} · ${value(authorization.grant.revision)}`} />
      <Metric label={tr("对象 / 动作范围", "Object / action scope")} value={`${value(authorization.grant.object_id)} · ${joined(strings(authorization.grant.action_kinds).map(pretty))}`} />
      <Metric label={tr("授权周期 / 有效区间", "Grant period / valid interval")} value={`${value(authorization.grant.period_id)} · ${value(authorization.grant.valid_from_tick)}–${value(authorization.grant.valid_until_tick)}`} />
      <Metric label={tr("电力累计支出 / 剩余 / 上限", "Electricity spent / remaining / limit")} value={`${value(authorization.spent_units)} / ${value(authorization.remaining_units)} / ${value(authorization.grant.limit_units)} ${budgetUnit(authorization.grant.resource_kind)}`} />
      <Metric label={tr("授权状态", "Grant status")} value={authorization.grant.revoked ? tr("已撤销", "Revoked") : tr("以运行时授权状态为准", "Read runtime grant state")} />
    </div>}</For>
    <Show when={!model().authorizations.length}><Metric label={tr("当前授权", "Current authorization")} value={unavailable()} /></Show>
    <Show when={model().receiptAuthorization}><Metric label={tr("本回执授权电力成本", "This receipt's authorized electricity cost")} value={`${value(model().receiptAuthorization.cost_units)} ${budgetUnit(model().receiptAuthorization.grant?.resource_kind)}`} /></Show>
    <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }}>
      <Metric label={tr("Agent 异议", "Agent dissent")} value={value(model().dissent)} />
      <Metric label={tr("Override 来源", "Override actor")} value={value(model().overrideActor)} />
      <Metric label={tr("不可越过的边界", "Hard boundary")} value={value(model().hardBoundary)} />
    </div>
    <Show when={model().memory} fallback={<Metric label={tr("已引用记忆", "Referenced memory")} value={unavailable()} />}>
      <div data-testid="control-proof-memory">
        <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }}>
          <Metric label={tr("记忆范围 / 版本", "Memory scope / revision")} value={`${value(model().memory?.scope)} · ${value(model().memory?.revision)}`} />
          <Metric label={tr("记忆来源", "Memory source")} value={value(model().memory?.source)} />
          <Metric label={tr("用于决定", "Decision usage")} value={model().memory?.used ? tr("已用于提交决定", "Used in committed decision") : tr("仅进入准备中的请求", "Included in prepared request only")} />
          <Metric label={tr("记忆新鲜度", "Memory freshness")} value={model().memory?.stale ? tr("已过期；刷新后纠正", "Stale; refresh before correction") : tr("以当前版本纠正", "Correct against current revision")} />
          <Metric label={tr("纠正提示", "Correction hint")} value={value(model().memory?.hint)} />
        </div>
        <For each={model().memory?.entries}>{(entry) => <Metric label={entry.id} value={entry.summary} />}</For>
        <For each={model().memory?.sources}>{(source) => <Metric label={tr("记忆原始回执", "Memory origin receipt")} value={`${value(source.id)} · ${value(source.receipt)} · ${joined(source.corrections)}`} />}</For>
      </div>
    </Show>
    <For each={model().corrections}>{(correction) => <div class="summary-grid" style={{ "grid-template-columns": "repeat(auto-fit, minmax(min(100%, 180px), 1fr))" }} data-testid="control-proof-correction" data-correction-status={correction.status}>
      <Metric label={tr("记忆纠正", "Memory correction")} value={`${value(correction.id)} · ${correctionStatus(correction.status)}`} />
      <Metric label={tr("纠正原因 / 版本", "Correction reason / revision")} value={`${value(correction.reason)} · ${value(correction.revision)}`} />
      <Metric label={tr("最早准备请求", "Earliest prepared request")} value={value(correction.earliestDecision)} />
      <Metric label={tr("最早准备摘要", "Earliest prepared request digest")} value={value(correction.earliestDigest)} />
      <Metric label={tr("实际提交决定请求", "Committed decision request")} value={value(correction.committedDecision)} />
      <Metric label={tr("实际提交请求摘要", "Committed request digest")} value={value(correction.committedDigest)} />
      <Metric label={tr("纠正关联回执 / 动作", "Correction receipt / action")} value={`${value(correction.receipt)} · ${value(correction.action)}`} />
    </div>}</For>
  </div>;
}

// Deterministic display fixture only; the caller installs this under test_api.
// It does not establish Runtime or provider execution evidence.
export function installControlProofVisualFixture(fixtures, { core, viewerFixtureBaseSnapshot, setFixturePlayerAuth }) {
  fixtures.control_proof_applied = () => {
    const snapshot = viewerFixtureBaseSnapshot();
    snapshot.player_gameplay.primary_intent = {
      intent_id: "intent-control-proof-fixture", agent_id: "agent-0", status: "accepted",
      message: "Move feedstock to the next production site.",
      agency_read_model: {
        status: "available", causal_receipt_status: "committed_receipt_available", memory_context_status: "available",
        causal_receipt: {
          intent_id: "intent-control-proof-fixture", receipt_id: "receipt-control-proof-fixture",
          commit_id: "commit-control-proof-fixture", action_id: 19, action_kind: "move_agent",
          domain_event_refs: [42], disposition: "applied", primary_reason: "Follow the available supply route.",
          next_step: "observe_domain_result", actual_consequence: [{ type: "agent_moved" }],
          expected_consequence: { provenance: "agent_explanation_unverified", prediction: "Production may become available after arrival." },
          stakes: { provenance: "agent_explanation_unverified", summary: "Preserve scarce input materials." },
          alternative: { provenance: "agent_explanation_unverified", alternatives: ["Wait for local supply."] },
          evidence_refs: ["observation-supply-fixture"], correction_refs: ["correction-route-fixture"], interruption_refs: [],
        },
        delegation_authorizations: [{
          grant: { agent_id: "agent-0", grant_id: "grant-fixture", source_id: "owner-grant-fixture",
            issuer_id: "viewer-bound", object_id: "agent-0", action_kinds: ["move_agent"],
            revision: 1, period_id: "fixture-period", valid_from_tick: 0, valid_until_tick: 20,
            limit_units: 10, resource_kind: "electricity", revoked: false },
          cost_units: 2, spent_units: 2, remaining_units: 8,
        }],
        referenced_memory_context: {
          revision: 3, scope: "session_private", source: "private_memory_retrieval_context",
          used_for_decision: true, stale: false, correction_hint: "correct_by_memory_id_and_revision",
          entries: [{ id: "memory-supply-fixture", summary: "The earlier supply estimate was corrected." }],
        },
        memory_corrections: [{ agent_id: "agent-0", correction_id: "correction-route-fixture", status: "applied",
          reason: "corrected_context_committed_decision", memory_revision: 3,
          earliest_decision_request_id: "decision-after-correction-fixture",
          runtime_receipt_id: "receipt-control-proof-fixture", action_id: "19" }],
      },
    };
    core.injectSnapshot(snapshot, { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
  };
}
