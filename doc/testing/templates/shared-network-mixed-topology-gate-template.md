# Shared Network Mixed-Topology Gate Template

审计轮次: 1

## Meta
- `window_id`:
  - `<shared-devnet-window-id>`
- `track`:
  - `shared_devnet`
- `candidate_id`:
  - `<candidate-id>`
- `owner`:
  - `qa_engineer`

## Candidate Truth
- `candidate_bundle_ref`:
  - `<output/release-candidates/current.json>`
- `candidate_gate_summary_ref`:
  - `<output/shared-network/.../gate/.../summary.md>`

## Mixed-Topology Inputs
- `baseline_evidence_ref`:
  - `<doc/testing/evidence/p2p-mixed-topology-validation-matrix-YYYY-MM-DD.md>`
- `same_window_shared_evidence_ref`:
  - `<shared window evidence | command log | summary>`
- `proxy_drill_ref`:
  - `<triad / release-chaos proxy drill evidence>`

## Validation
- `validated_by`:
  - `<qa owner / runtime owner>`
- `validated_at`:
  - `<YYYY-MM-DD HH:MM:SS TZ>`
- `validation_expectations`:
  - `<baseline candidate_id and role boundary still match current bundle truth>`
  - `<same-window mixed-topology evidence is explicitly linked when claiming pass>`
  - `<pass uplift includes an audited producer/QA decision ref, not just a lane status flip>`
  - `<proxy drill evidence is called out as approximation, not dedicated sentry/NAT lab truth>`

## Verdict
- `lane_result`:
  - `pass | partial | block`
- `reason`:
  - `<why this is pass/partial/block>`
- `pass_uplift_decision_ref`:
  - `<required when lane_result=pass; producer/QA decision note or review doc>`

## Notes
- `pass` only if same-window shared mixed-topology evidence is pinned, reviewed against the current candidate truth, and backed by an audited producer/QA pass-uplift decision ref.
- `partial` if only the P2PARCH-6 baseline or proxy drill evidence is available.
- `block` if there is no credible mixed-topology basis for the current candidate or the evidence contradicts the gate claim.
