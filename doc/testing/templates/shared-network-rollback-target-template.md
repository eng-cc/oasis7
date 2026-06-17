# Legacy Shared-Network Rollback Target Template

审计轮次: 1

## Meta
- `window_id`:
  - `<legacy-public-testnet-rehearsal-window-id | test-environment-candidate-id>`
- `track`:
  - `public_testnet_rehearsal`
- `candidate_id`:
  - `<current-candidate-id>`
- `owner`:
  - `liveops_community`

## Current Candidate
- Note: this template is legacy rehearsal only. It must not be used for active product/player-facing unified-world claims.
- `candidate_bundle_ref`:
  - `<output/release-candidates/current.json>`
- `candidate_gate_ref`:
  - `<output/network-rehearsal/.../gate/.../summary.md>`

## Fallback Candidate
- `fallback_candidate_id`:
  - `<previous-pass-candidate-id>`
- `fallback_class`:
  - `formal_pass_candidate | bootstrap_restore_ready`
- `fallback_candidate_bundle_ref`:
  - `<output/release-candidates/fallback.json>`
- `fallback_gate_ref`:
  - `<output/network-rehearsal/.../gate/.../summary.md>`
- `fallback_track_result`:
  - `pass`
- `fallback_owner_ref`:
  - `<promotion record | incident review | approval record>`

## Rollback Readiness
- `restore_steps_ref`:
  - `<runbook | command log | operator checklist>`
- `validated_by`:
  - `<liveops owner / runtime owner>`
- `validated_at`:
  - `<YYYY-MM-DD HH:MM:SS TZ>`
- `restoration_scope`:
  - `<runtime build | world snapshot | governance manifest>`

## Verdict
- `lane_result`:
  - `pass | partial | block`
- `reason`:
  - `<why this is pass/partial/block>`

## Notes
- `pass` if:
  - `fallback_class=formal_pass_candidate` and the fallback candidate is a formal previous public-testnet-rehearsal `pass` candidate; or
  - `fallback_class=bootstrap_restore_ready`, the current track is still pursuing the first public-testnet-rehearsal `pass`, and `restore_steps_ref` + `fallback_owner_ref` + `restoration_scope` are all pinned and audited.
- `partial` if there is only a local/provisional fallback, or a bootstrap fallback is mentioned but the audited restore contract is incomplete.
- `block` if fallback truth is missing, inconsistent, or not restorable.
