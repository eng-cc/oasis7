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

## Pending Checklist
- [ ] 固定 `fallback_candidate_bundle_ref`
- [ ] 固定 `fallback_gate_ref`
- [ ] 固定 `fallback_owner_ref`
- [ ] 固定至少一条 `restore_steps_ref`
- [ ] 固定 `restoration_scope`
- [ ] 回填 `validated_by`
- [ ] 回填 `validated_at`

## Pass Closure Rule
- 只有以下五类字段同时齐全时，`rollback_target_ready` 才能升到 `pass`：
  - fallback bundle：`fallback_candidate_bundle_ref`
  - fallback gate：`fallback_gate_ref`
  - fallback owner：`fallback_owner_ref`
  - restore steps：`restore_steps_ref`
  - restoration scope：`restoration_scope`

## Suggested Update Command
```bash
./scripts/shared-devnet-blocker-packet.sh \
  --window-id shared-devnet-20260324-06 \
  --candidate-bundle output/release-candidates/shared-devnet-20260324-05.json \
  --candidate-gate-summary output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md \
  --access-out doc/testing/evidence/shared-network-shared-devnet-shared-access-draft-2026-03-24.md \
  --mixed-topology-out doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md \
  --rollback-out doc/testing/evidence/shared-network-shared-devnet-rollback-target-draft-2026-03-24.md \
  --fallback-candidate-bundle <output/release-candidates/fallback.json> \
  --fallback-class bootstrap_restore_ready \
  --fallback-gate-summary <audited fallback gate/checklist ref> \
  --fallback-owner-ref <approval/handoff ref> \
  --restore-steps-ref <restore checklist/log ref> \
  --restoration-scope "runtime build | world snapshot | governance manifest" \
  --rollback-validated-by "<liveops owner / runtime owner>" \
  --rollback-validated-at "<YYYY-MM-DD HH:MM:SS TZ>" \
  --rollback-lane-result pass \
  --rollback-reason "audited fallback contract is pinned for the current shared-devnet window"
```

## Notes
- `pass` if:
  - `fallback_class=formal_pass_candidate` and fallback candidate is a formal previous shared-devnet `pass` candidate; or
  - `fallback_class=bootstrap_restore_ready`, current track is still pursuing the first shared-devnet `pass`, and `restore_steps_ref` + `fallback_owner_ref` + `restoration_scope` are all pinned and audited.
- `partial` if there is only a local/provisional fallback, or a bootstrap fallback is named but the audited restore contract is incomplete.
- `block` if fallback truth is missing, inconsistent, or not restorable.
