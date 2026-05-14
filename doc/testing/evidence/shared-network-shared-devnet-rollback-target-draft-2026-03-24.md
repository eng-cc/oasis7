# Shared Network Rollback Target

审计轮次: 1

## Meta
- `window_id`:
  - `shared-devnet-20260324-06`
- `track`:
  - `shared_devnet`
- `candidate_id`:
  - `shared-devnet-20260324-05`
- `owner`:
  - `liveops_community`

## Current Candidate
- `candidate_bundle_ref`:
  - `output/release-candidates/shared-devnet-20260324-05.json`
- `candidate_gate_ref`:
  - `output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md`

## Fallback Candidate
- `fallback_candidate_id`:
  - `shared-devnet-bootstrap-fallback`
- `fallback_class`:
  - `bootstrap_restore_ready`
- `fallback_candidate_bundle_ref`:
  - `<output/release-candidates/bootstrap-fallback.json>`
- `fallback_gate_ref`:
  - `<bootstrap restore checklist or audited fallback summary>`
- `fallback_track_result`:
  - `bootstrap_restore_ready`
- `fallback_owner_ref`:
  - `<operator checklist | approval record>`

## Rollback Readiness
- `restore_steps_ref`:
  - `<pending>`
- `validated_by`:
  - `<liveops owner / runtime owner>`
- `validated_at`:
  - `<YYYY-MM-DD HH:MM:SS TZ>`
- `restoration_scope`:
  - `<runtime build | world snapshot | governance manifest>`

## Verdict
- `lane_result`:
  - `partial`
- `reason`:
  - first shared-devnet pass may use a `bootstrap_restore_ready` fallback, but this draft still lacks audited `restore_steps_ref`, `fallback_owner_ref`, and concrete restoration scope evidence

## Notes
- `pass` if:
  - `fallback_class=formal_pass_candidate` and fallback candidate is a formal previous shared-devnet `pass` candidate; or
  - `fallback_class=bootstrap_restore_ready`, current track is still pursuing the first shared-devnet `pass`, and `restore_steps_ref` + `fallback_owner_ref` + `restoration_scope` are all pinned and audited.
- `partial` if there is only a local/provisional fallback, or a bootstrap fallback is named but the audited restore contract is incomplete.
- `block` if fallback truth is missing, inconsistent, or not restorable.
