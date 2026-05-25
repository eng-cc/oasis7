# Shared Network `shared_devnet` Short-Window Pass Evidence (2026-05-23)

审计轮次: 2

## Meta
- 关联专题:
  - `PRD-P2P-RTMIN-002`
  - `PRD-P2P-RTMIN-003`
  - `PRD-P2P-BENCH-003`
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `qa_engineer`
- 当前结论:
  - `pass`
- 目标:
  - 在当前 live-reset candidate `shared-devnet-live-reset-20260523-01` 上完成真实 S9/S10 short-window rehearsal，把 `short_window_longrun` 从 `partial` 升到 `pass`。

## 执行范围
- `window_id`:
  - `shared-devnet-live-reset-20260523-longrun-fix2`
- `candidate_id`:
  - `shared-devnet-live-reset-20260523-01`
- candidate bundle:
  - `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- git commit:
  - `d59e892ad1deb8cc612a56af67ce08e6c5d7ff97`
- rehearsal root:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/`

## 执行命令
- shared-devnet window:
  - `./scripts/shared-devnet-rehearsal.sh --window-id shared-devnet-live-reset-20260523-longrun-fix2 --candidate-bundle doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json --release-gate-mode skip --web-mode evidence --headless-mode evidence --pure-api-mode evidence --governance-mode evidence --shared-access-pass --fallback-class bootstrap_restore_ready --mixed-topology-baseline-evidence-ref doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md --longrun-mode execute --s9-duration-secs 300 --s10-duration-secs 300 --s9-base-port 7410 --s10-base-port 7610 --out-dir .tmp/shared-devnet-live-reset-20260523-01`

## 修正前提
- 为让 S9 chaos restart 不再把未完整落盘的 reward-runtime 状态直接原样拉起，`scripts/p2p-longrun-soak.sh` 的 `restart` chaos 已改为优先发送 `SIGINT`，等待 runtime 走正常停机路径，再做必要的 fallback kill。
- 为让 longrun 实测真正使用到最新 gap-sync fetch 路由修正，已先重建 `target/debug/oasis7_chain_runtime`。

## 关键产物
- S9 summary:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s9/20260523-231118/summary.md`
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s9/20260523-231118/summary.json`
- S10 summary:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s10/20260523-231621/summary.md`
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s10/20260523-231621/summary.json`
- lane summary:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun-summary.md`
- gate summary:
  - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/gate/shared_devnet-20260523-232124/summary.md`

## QA 结果
- `S9 overall_status=ok`
  - `chaos_events_total=1`
  - `last_error_samples=0`
  - `reward_runtime_available_samples=22`
- `S10 overall_status=ok`
  - `last_error_samples=0`
  - `settlement_apply_attempts=3`
  - `minted_non_empty_samples=3`
- `short_window_longrun=pass`

## 结论
- 当前 `shared-devnet-live-reset-20260523-01` 已拥有 same-window 的真实 S9/S10 short-window evidence。
- 这次升级后，formal gate 只剩 `mixed_topology_baseline=partial`。
