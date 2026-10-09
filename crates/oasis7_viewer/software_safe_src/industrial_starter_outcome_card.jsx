import { Show } from "solid-js";

export function IndustrialStarterOutcomeCard(props) {
  const tr = (zh, en) => props.locale() === "zh" ? zh : en;
  const materials = items => items.map(item => `${item.kind} × ${item.amount}`).join(" · ");
  return <Show when={props.profile}>
    <section class="panel-section" aria-label={tr("首条工业线成果", "Starter industrial outcome")}>
      <h3>{tr("首条工业线成果", "Starter industrial outcome")}</h3>
      <p>{props.profile.profileId} · revision {props.profile.profileRevision}</p>
      <Show when={props.profile.settled} fallback={<p>{tr("首产物数量与配方用电尚未发布。", "First settled output quantity and recipe power have not been published.")}</p>}>
        {settled => <>
          <p><strong>{tr("已结算产出", "Settled output")}: {materials(settled().produce)}</strong></p>
          <p>{tr("实际投入材料", "Committed input materials")}: {materials(settled().consume)}</p>
          <p>{tr("已结算配方用电需求", "Settled recipe power requirement")}: {settled().powerRequired}</p>
          <p>{tr("接受批次", "Accepted batches")}: {settled().acceptedBatches} · job {settled().jobId} · tick {settled().settledAt}</p>
          <p>{tr("产出归属 Agent", "Output owner Agent")}: {settled().owner}</p>
          <p>{tr("首产物里程碑已记录（production_only）。稳定运行与交付仍需分别验证。", "First output milestone recorded (production_only). Stability and delivery require separate verification.")}</p>
        </>}
      </Show>
      <p>{props.profile.candidateAvailable
        ? tr("下一候选：Assembler MK1。具备候选资格，建造仍需满足当前材料、用电与授权条件。", "Next candidate: Assembler MK1. Eligibility is available; construction still requires current materials, power and authority.")
        : tr("Assembler MK1 候选资格尚未满足。", "Assembler MK1 candidate eligibility is not yet available.")}</p>
    </section>
  </Show>;
}
