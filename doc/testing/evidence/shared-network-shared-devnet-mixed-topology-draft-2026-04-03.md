# Shared Network Mixed-Topology Gate Evidence

审计轮次: 2

## Meta
- `window_id`:
  - `shared-devnet-20260324-06`
- `track`:
  - `shared_devnet`
- `candidate_id`:
  - `shared-devnet-20260324-05`
- `owner`:
  - `qa_engineer`

## Candidate Truth
- `candidate_bundle_ref`:
  - `output/release-candidates/shared-devnet-20260324-05.json`
- `candidate_gate_summary_ref`:
  - `output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md`

## Mixed-Topology Inputs
- `baseline_evidence_ref`:
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md`
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md`
- `same_window_shared_evidence_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-follow-up-window-2026-03-24.md`
  - `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md`
- `proxy_drill_ref`:
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md`
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md`

## Validation
- `validated_by`:
  - `qa_engineer + producer_system_designer`
- `validated_at`:
  - `2026-04-07 12:22:00 CST`
- `validation_expectations`:
  - `baseline candidate_id and role boundary still match current shared-devnet bundle truth`
  - `same-window shared-devnet evidence is explicitly linked even when the lane stays partial`
  - `proxy drill evidence is called out as approximation, not dedicated sentry/NAT lab truth`

## Verdict
- `lane_result`:
  - `partial`
- `reason`:
  - `P2PARCH-6` matrix baseline and same-window shared-devnet rehearsal evidence are now pinned together, but the latest 2026-04-07 full proxy execution still fails with audited consensus/recovery signatures and has not crossed the dedicated shared-window pass bar.
- `pass_uplift_decision_ref`:
  - `<not applicable while lane_result=partial; required for pass uplift>`

## Notes
- This document upgrades the lane from an implicit missing scaffold to an explicit audited `partial` evidence packet.
- It is sufficient to keep `shared_devnet` gate semantics honest, but not sufficient to promote `shared_devnet` to `pass`.
- The next uplift still requires stronger same-window mixed-topology evidence, or an approved decision that current proxy/shared-window evidence is enough for this track.
- 当前已知可直接补充到该 lane 的真实环境是 `1` 个本机节点 + `2` 个阿里云节点；这组环境适合继续补 `P2PARCH-6` 的真实三节点 mixed-topology drill，但单凭它仍不足以替代 same-window shared-network gate、producer/QA pass-uplift decision ref，或更强的 dedicated sentry/NAT lab truth。
