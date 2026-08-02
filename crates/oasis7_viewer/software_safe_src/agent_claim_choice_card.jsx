import { For, Show } from "solid-js";

export function AgentClaimChoiceCard(props) {
  const publishedCandidates = () => props.publishedCandidates?.() || [];
  const choiceQuote = () => props.choiceQuote?.() || null;
  const quote = () => props.quote?.() || null;
  const locale = () => props.locale?.() || "en";
  const tr = (zh, en) => props.tr?.(locale(), zh, en) || (locale() === "zh" ? zh : en);
  const fallbackReason = () => choiceQuote()?.fallback_reason || choiceQuote()?.fallbackReason;
  const choiceClass = () => choiceQuote()?.claim_choice_class
    || choiceQuote()?.claimChoiceClass
    || choiceQuote()?.recommended_claim_action
    || choiceQuote()?.recommendedClaimAction;
  const numberField = (...values) => {
    const value = values.find((candidate) => candidate !== null && candidate !== undefined && candidate !== "");
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : null;
  };
  const upfrontAmount = () => numberField(quote()?.total_upfront_amount, quote()?.totalUpfrontAmount);
  const eligibleBalance = () => numberField(quote()?.eligible_claim_balance, quote()?.eligibleClaimBalance);
  const upkeepRunway = () => numberField(
    quote()?.upkeep_runway_epochs,
    quote()?.upkeepRunwayEpochs,
    quote()?.upkeep_epochs_after_claim,
    quote()?.upkeepEpochsAfterClaim,
  );
  const isRationaleMissingDefer = () => fallbackReason() === "candidate_rationale_missing"
    && choiceClass() === "wait_or_fund_first"
    && eligibleBalance() === upfrontAmount()
    && upkeepRunway() === 0;
  const fallbackLabel = () => fallbackReason() === "candidate_rationale_missing"
    ? tr("候选路线理由尚未发布", "Candidate route rationale is not published")
    : null;
  const choiceClassLabel = () => choiceClass() === "wait_or_fund_first"
    ? tr("等待或先补足条件", "Wait or resolve prerequisites first")
    : null;

  return (
    <>
      <Show when={publishedCandidates().length > 0}>
        <div class="event-list">
          <For each={publishedCandidates()}>
            {(candidate) => (
              <div class="event-card">
                <div class="event-card__title">
                  <span>{tr("首个候选 Agent", "Slot-1 Candidate")}</span>
                  <span class="badge badge--accent">{candidate.id}</span>
                </div>
                <Show when={candidate.name}>
                  <div class="event-card__meta">{candidate.name}</div>
                </Show>
                <Show when={candidate.location_x_cm !== undefined || candidate.location_y_cm !== undefined || candidate.location_z_cm !== undefined}>
                  <div class="feedback-detail">
                    {`${candidate.location_x_cm ?? "-"}, ${candidate.location_y_cm ?? "-"}, ${candidate.location_z_cm ?? "-"} cm`}
                  </div>
                </Show>
                <Show when={candidate.body_kind || candidate.bodyKind}>
                  <div class="feedback-detail">{candidate.body_kind || candidate.bodyKind}</div>
                </Show>
                <Show when={candidate.frame_kind || candidate.frameKind}>
                  <div class="feedback-detail">{candidate.frame_kind || candidate.frameKind}</div>
                </Show>
                <Show when={Array.isArray(candidate.installed_module_ids || candidate.installedModuleIds) && (candidate.installed_module_ids || candidate.installedModuleIds).length > 0}>
                  <div class="feedback-detail">{(candidate.installed_module_ids || candidate.installedModuleIds).join(" · ")}</div>
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={isRationaleMissingDefer()}>
        <div class="event-card">
          <div class="event-card__title">
            <span>{tr("暂不确认", "Wait before confirming")}</span>
            <span class="badge badge--warn">{tr("暂缓", "Defer")}</span>
          </div>
          <div class="feedback-detail">
            {tr(
              `当前可支付 ${upfrontAmount()} upfront，但确认后只能维持 ${upkeepRunway()} 个完整 upkeep epoch。尚未发布 canonical 路线理由，因此不推荐任何候选。请在理由发布且有额外可用于 upkeep 的 eligible balance 后再评估；仅补足资金不等于被推荐。`,
              `The ${upfrontAmount()} upfront cost is payable now, but confirmation leaves ${upkeepRunway()} full upkeep epochs. No canonical route rationale is published, so no candidate is recommended. Reassess after a rationale is published and you have additional eligible upkeep balance; funding alone does not make a candidate recommended.`,
            )}
          </div>
        </div>
      </Show>
      <Show when={(fallbackLabel() || choiceClassLabel()) && !isRationaleMissingDefer()}>
        <div class="badge-row">
          <Show when={fallbackLabel()}>
            <span class="badge badge--warn">{fallbackLabel()}</span>
          </Show>
          <Show when={choiceClassLabel()}>
            <span class="badge">{choiceClassLabel()}</span>
          </Show>
        </div>
      </Show>
    </>
  );
}
