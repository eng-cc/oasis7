# Shared Network First `shared_devnet` Dry-Run Evidence (2026-03-24)

审计轮次: 1

## Meta
- 关联专题:
  - `PRD-P2P-RTMIN-002`
  - `PRD-P2P-RTMIN-003`
  - `PRD-P2P-BENCH-003`
- 关联任务:
  - `RTMIN-4`
- 责任角色:
  - `qa_engineer`
- 协作角色:
  - `runtime_engineer`
  - `liveops_community`
- 当前结论:
  - `partial`
- 目标:
  - 执行 first `shared_devnet` dry run，并为同一 `candidate_id` 留下 candidate / promotion / gate / incident 产物。

## 执行范围
- candidate bundle:
  - `output/release-candidates/shared-devnet-dry-run-20260324-01.json`
- git commit:
  - `f5437acc7ce76722a46fb99c41cd216b67ec395b`
- runtime build:
  - `output/release/game-launcher-local/bin/oasis7_chain_runtime`
- world snapshot:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- governance manifest:
  - `output/governance-drills/20260324-finality-live-world-signer04/manifests/rotated_pass_manifest.json`
- shared-devnet dry-run root:
  - `output/shared-network/shared-devnet-dry-run-20260324-01/`

## 执行步骤
1. 生成并冻结同一份 `release_candidate_bundle`。
2. 对该 candidate 运行 `release-gate --dry-run`，确认 shared-devnet 编排链路可被候选包驱动。
3. 填写 `shared_devnet` 六条 required lanes 的首轮 dry-run 证据：
   - `candidate_bundle_integrity`
   - `shared_access`
   - `multi_entry_closure`
   - `governance_live_drill`
   - `short_window_longrun`
   - `rollback_target_ready`
4. 运行 `shared-network-track-gate.sh` 生成正式 `summary.json/md`。
5. 由 `liveops_community` 记录 promotion / hold / incident 边界，不做 staging promotion。

## 执行命令
- candidate bundle:
  - `./scripts/release-candidate-bundle.sh create --bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json --candidate-id shared-devnet-dry-run-20260324-01 --track shared_devnet --runtime-build-ref output/release/game-launcher-local/bin/oasis7_chain_runtime --world-snapshot-ref output/chain-runtime/viewer-live-node/reward-runtime-execution-world --governance-manifest-ref output/governance-drills/20260324-finality-live-world-signer04/manifests/rotated_pass_manifest.json --evidence-ref doc/testing/evidence/governance-registry-live-world-drill-finality-2026-03-24.md --evidence-ref doc/testing/evidence/governance-registry-live-world-drill-foundation-ops-2026-03-24.md --note 'first shared_devnet local-only dry run'`
- release gate dry-run:
  - `./scripts/release-gate.sh --dry-run --candidate-bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json --out-dir output/shared-network/shared-devnet-dry-run-20260324-01/release-gate`
- shared-network gate:
  - `./scripts/shared-network-track-gate.sh --track shared_devnet --candidate-bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json --lanes-tsv output/shared-network/shared-devnet-dry-run-20260324-01/lanes.shared_devnet.tsv --out-dir output/shared-network/shared-devnet-dry-run-20260324-01/gate`

## 关键产物
- candidate bundle:
  - `output/release-candidates/shared-devnet-dry-run-20260324-01.json`
- release-gate dry-run:
  - `output/shared-network/shared-devnet-dry-run-20260324-01/release-gate/20260324-150030/release-gate-summary.md`
- lane evidence:
  - `output/shared-network/shared-devnet-dry-run-20260324-01/access-check.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/multi-entry-summary.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/longrun-summary.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/rollback-target.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/lanes.shared_devnet.tsv`
- shared-network gate:
  - `output/shared-network/shared-devnet-dry-run-20260324-01/gate/shared_devnet-20260324-150230/summary.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/gate/shared_devnet-20260324-150230/summary.json`
- liveops records:
  - `doc/testing/evidence/shared-network-shared-devnet-promotion-record-2026-03-24.md`
  - `doc/testing/evidence/shared-network-shared-devnet-incident-2026-03-24.md`

## 结果摘要
- `release_candidate_bundle`:
  - `validation=ok`
  - `git_worktree_dirty=false`
  - runtime/world/governance 都已固定路径和 hash
- `release-gate --dry-run`:
  - `overall=PASS`
  - candidate bundle / ci / sync / web strict / S9 / S10 编排入口都已接上
- `shared-network-track-gate`:
  - `gate_result=partial`
  - `promotion_recommendation=hold_promotion`
- lane verdict:
  - `candidate_bundle_integrity=pass`
  - `shared_access=partial`
  - `multi_entry_closure=partial`
  - `governance_live_drill=partial`
  - `short_window_longrun=partial`
  - `rollback_target_ready=partial`
- liveops:
  - `promotion_decision=hold`
  - `freeze_decision=no`
  - `rollback_required=no`

## QA 结论
- 本轮 first `shared_devnet` dry run 已经真实执行并且可审计，因此 `RTMIN-4` 的“candidate/evidence/incident 产物落地”目标可以记完成。
- 但这次执行只到 `partial`：
  - 共享访问仍是 local-only
  - multi-entry closure 还没在同一 candidate 上重跑
  - governance / short-window longrun 还没有 shared-devnet 窗口内的新样本
  - rollback target 还没有可审计的 shared-grade fallback；即便首条 `shared_devnet pass` 允许 `bootstrap_restore_ready` fallback，本轮也还没补齐 restore steps / owner ref / scope
- 因此当前不能 promotion 到 `staging`，更不能升级 public claims。

## 边界与遗留
1. 本轮使 shared-network 从 `specified_not_executed` 升到 `partial`，但绝不等于 `shared network validated`。
2. benchmark `L5` 可以从 `missing` 调整为 `partial`，因为已有首轮可审计 dry run；但 shared execution 仍未达 `pass`。
3. 下一步不是直接做 `staging`，而是先把 `shared_devnet` 从 `partial` 提升到 `pass`，至少补齐：
  - 真实 shared access
  - 同一 candidate 的 multi-entry closure
  - shared-devnet short-window longrun
  - 可追溯 fallback candidate 或受审计 `bootstrap_restore_ready` fallback
