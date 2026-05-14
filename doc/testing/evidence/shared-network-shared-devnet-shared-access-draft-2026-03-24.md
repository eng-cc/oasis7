# Shared Network Shared Access Check

审计轮次: 1

## Meta
- `window_id`:
  - `shared-devnet-20260324-06`
- `track`:
  - `shared_devnet`
- `candidate_id`:
  - `shared-devnet-20260324-05`
- `owner`:
  - `qa_engineer`

## Shared Endpoint
- `viewer_url`:
  - `<https://... | http://...>`
- `live_addr`:
  - `<host:port>`
- `operator_contact_ref`:
  - `<pending>`
- `independent_operator_ref`:
  - `<pending>`

## Access Validation
- `access_mode`:
  - `shared_multi_operator`
- `validated_by`:
  - `<qa operator / runtime operator>`
- `validated_at`:
  - `<YYYY-MM-DD HH:MM:SS TZ>`
- `validation_steps`:
  - `independent operator opened viewer endpoint`
  - `independent operator reached live endpoint`
  - `candidate_id matched bundle truth`
- `candidate_bundle_ref`:
  - `output/release-candidates/shared-devnet-20260324-05.json`
- `candidate_gate_summary_ref`:
  - `output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md`
- `evidence_ref`:
  - `<pending>`

## Verdict
- `lane_result`:
  - `partial`
- `reason`:
  - shared access input is still draft; convert to pass only after independent operator access is verified

## Pending Checklist
- [ ] 固定真实 `viewer_url`
- [ ] 固定真实 `live_addr`
- [ ] 固定 `operator_contact_ref`
- [ ] 固定 `independent_operator_ref`
- [ ] 固定至少一条 `evidence_ref`（截图、访问日志或 duty record）
- [ ] 回填 `validated_by`
- [ ] 回填 `validated_at`

## Pass Closure Rule
- 只有以下三类字段同时齐全时，`shared_access` 才能升到 `pass`：
  - 真实 shared endpoint：`viewer_url` + `live_addr`
  - 独立 operator/handoff：`operator_contact_ref` + `independent_operator_ref`
  - 独立访问证据：`evidence_ref`

## Suggested Update Command
```bash
./scripts/shared-devnet-blocker-packet.sh \
  --window-id shared-devnet-20260324-06 \
  --candidate-bundle output/release-candidates/shared-devnet-20260324-05.json \
  --candidate-gate-summary output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md \
  --access-out doc/testing/evidence/shared-network-shared-devnet-shared-access-draft-2026-03-24.md \
  --mixed-topology-out doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md \
  --rollback-out doc/testing/evidence/shared-network-shared-devnet-rollback-target-draft-2026-03-24.md \
  --viewer-url <https://shared-viewer.example/...> \
  --live-addr <host:port> \
  --operator-contact-ref <handoff/oncall/runbook ref> \
  --independent-operator-ref <independent operator proof ref> \
  --access-evidence-ref <screenshot/log/duty record ref> \
  --access-validated-by "<qa operator / runtime operator>" \
  --access-validated-at "<YYYY-MM-DD HH:MM:SS TZ>" \
  --access-lane-result pass \
  --access-reason "independent operator access is verified for this shared-devnet window"
```

## Notes
- `pass` only if access is not single-owner local-only rehearsal and the endpoint, operator handoff, and independent access evidence refs are all pinned.
- `partial` if endpoint exists but still depends on one local operator or one private machine.
- `block` if endpoint is unreachable, candidate truth mismatches, or owner handoff is missing.
