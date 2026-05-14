# Shared Network Shared Access Check Template

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

## Shared Endpoint
- `viewer_url`:
  - `<https://... | http://...>`
- `live_addr`:
  - `<host:port>`
- `operator_contact_ref`:
  - `<runbook | handoff doc | chat log | ticket>`
- `independent_operator_ref`:
  - `<operator name / duty roster / handoff evidence>`

## Access Validation
- `access_mode`:
  - `shared_multi_operator`
- `validated_by`:
  - `<qa operator / runtime operator>`
- `validated_at`:
  - `<YYYY-MM-DD HH:MM:SS TZ>`
- `validation_steps`:
  - `<independent operator opened viewer endpoint>`
  - `<independent operator reached live endpoint>`
  - `<candidate_id matched bundle truth>`
- `candidate_bundle_ref`:
  - `<output/release-candidates/current.json>`
- `candidate_gate_summary_ref`:
  - `<output/shared-network/.../gate/.../summary.md>`
- `evidence_ref`:
  - `<screenshots | logs | duty record>`

## Verdict
- `lane_result`:
  - `pass | partial | block`
- `reason`:
  - `<why this is pass/partial/block>`

## Notes
- `pass` only if access is not single-owner local-only rehearsal and the endpoint, operator handoff, and independent access evidence refs are all pinned.
- `partial` if endpoint exists but still depends on one local operator or one private machine.
- `block` if endpoint is unreachable, candidate truth mismatches, or owner handoff is missing.
