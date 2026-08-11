import { For, Show } from "solid-js";

export function FirstDeliveryPreview(props) {
  const preview = () => props.preview || {};
  const locale = () => props.locale;
  const tr = props.tr;
  return (
    <div class="feedback-detail first-delivery-preview">
      <div class="metric__label">{tr(locale(), "首单交付预览", "First delivery preview")}</div>
      <Show when={preview().localNeed}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "本地需求", "Local need")}</div>
          {preview().localNeed}
        </div>
      </Show>
      <Show when={preview().expectedOutput}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "预计产出", "Expected output")}</div>
          {preview().expectedOutput}
        </div>
      </Show>
      <Show when={preview().requiredInputs.length > 0}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "所需输入", "Required inputs")}</div>
          <For each={preview().requiredInputs}>
            {(input) => <div>{input}</div>}
          </For>
        </div>
      </Show>
      <Show when={preview().valueTiming}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "价值时机", "Value timing")}</div>
          {preview().valueTiming}
        </div>
      </Show>
      <Show when={preview().leverageClassUnlocked}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "解锁杠杆", "Leverage unlocked")}</div>
          {preview().leverageClassUnlocked}
        </div>
      </Show>
      <Show when={preview().returnVisitHook}>
        <div class="feedback-detail">
          <div class="metric__label">{tr(locale(), "回访钩子", "Return visit hook")}</div>
          {preview().returnVisitHook}
        </div>
      </Show>
    </div>
  );
}
