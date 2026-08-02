import { For, Show } from "solid-js";

function humanizeIdentifier(value) {
  const normalized = String(value || "")
    .trim()
    .replace(/[_-]+/g, " ")
    .toLowerCase();
  return normalized ? `${normalized[0].toUpperCase()}${normalized.slice(1)}` : "";
}

export function AgentClaimChoiceCard(props) {
  const publishedCandidates = () => props.publishedCandidates?.() || [];
  const choiceQuote = () => props.choiceQuote?.() || null;
  const fallbackReason = () => choiceQuote()?.fallback_reason || choiceQuote()?.fallbackReason;
  const choiceClass = () => choiceQuote()?.claim_choice_class
    || choiceQuote()?.claimChoiceClass
    || choiceQuote()?.recommended_claim_action
    || choiceQuote()?.recommendedClaimAction;

  return (
    <>
      <Show when={publishedCandidates().length > 0}>
        <div class="event-list">
          <For each={publishedCandidates()}>
            {(candidate) => (
              <div class="event-card">
                <div class="event-card__title">
                  <span>Slot-1 Candidate</span>
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
      <Show when={fallbackReason() || choiceClass()}>
        <div class="badge-row">
          <Show when={fallbackReason()}>
            <span class="badge badge--warn">{humanizeIdentifier(fallbackReason())}</span>
          </Show>
          <Show when={choiceClass()}>
            <span class="badge">{humanizeIdentifier(choiceClass())}</span>
          </Show>
        </div>
      </Show>
    </>
  );
}
