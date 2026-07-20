# public_testnet exit review template

- tier: `public_testnet`
- source_manifest_ref: `<manifest path>`
- review_window: `<YYYY-MM-DD HH:MM CST>`
- candidate_for: `<continue_public_testnet|hold|mainnet_gating_input_only>`

## Promotion prerequisites
- [ ] `public_testnet_exit_review`
- [ ] public RPC remained reachable during rehearsal window
- [ ] explorer evidence is public and current
- [ ] guarded faucet policy stayed within announced boundary
- [ ] reset policy was announced and rehearsed without claim drift
- [ ] same-window API/viewer projection evidence is linked and matches S10 `api_viewer_projection`
- [ ] no evidence claims crossed into `mainnet_live` / `production_oc_settlement`

## Mainnet gating handoff
- [ ] `MAINNET-1` ready
- [ ] `MAINNET-2` ready
- [ ] `MAINNET-3` ready
- [ ] 当前 mainnet gating 已由对应专业权威和同一证据窗口复核，并重新对照根 `README.md`；不得用历史任务完成状态或本模板勾选升级公开口径
- [ ] frozen genesis candidate identified
- [ ] no-reset commitment drafted

## Final verdict
- exit_review_verdict: `<eligible_for_mainnet_gating_input|hold_public_testnet|block>`
- summary: `<one-line conclusion>`
- blockers:
  - `<blocker-1>`

## Readiness review input
- `./scripts/network-tier-public-testnet-readiness.sh --manifest <manifest> --lanes-tsv <lanes.tsv>`
