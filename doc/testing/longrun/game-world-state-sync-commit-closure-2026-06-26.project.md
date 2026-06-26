# oasis7: Game World State Sync and Commit Closure Project

- 对应需求文档: `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`
- 对应设计文档: `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] gwsc (PRD-TESTING-LONGRUN-GWSC-001/002/003) [test_tier_required]: 建立 PRD/design/project 三件套，写明 claim boundary 和多节点测试矩阵。 Trace: .pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.yaml
- [x] gwsc (PRD-TESTING-LONGRUN-GWSC-001/002) [test_tier_required]: 增加一键 `module_required` wrapper 或 testing-manual 命令块，避免手工漏跑 node/net/libp2p。 Trace: .pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.yaml
- [x] gwsc (PRD-TESTING-LONGRUN-GWSC-002) [test_tier_full]: 增加 state-sync closure evidence packet 模板。 Trace: .pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.yaml
- [x] gwsc (PRD-TESTING-LONGRUN-GWSC-003) [test_tier_full]: 扩展 S10 summary，显式记录 API/viewer projection 对账字段。 Trace: .pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.yaml
- [x] gwsc (PRD-TESTING-LONGRUN-GWSC-003) [test_tier_full]: 将 real-env/public_testnet readiness lane 与同窗口 projection evidence 绑定。 Trace: .pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.yaml

## 依赖
- `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`
- `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md`
- `testing-manual.md`
- `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md`
- `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`
- `doc/testing/templates/state-sync-closure-evidence-packet-template.md`
- `scripts/game-world-state-sync-commit-module-required.sh`
- `scripts/p2p-mixed-topology-matrix.sh`
- `scripts/p2p-longrun-soak.sh`
- `scripts/p2p-export-state-sync-bundle.sh`
- `scripts/p2p-verify-state-sync-closure.sh`
- `scripts/s10-five-node-game-soak.sh`
- `scripts/state-sync-closure-evidence-template.test.sh`
- `scripts/s10-five-node-game-soak-summary.test.sh`
- `scripts/network-tier-public-testnet-readiness.sh`
- `scripts/network-tier-manifest-smoke.sh`
- `doc/testing/templates/public-testnet-readiness-lanes.example.tsv`
- `.pm/tasks/task_b52dc268bc394b0eb05a139eabc00307.execution.md`

## 状态
- 更新日期: 2026-06-26
- 当前阶段: active implementation
- owner role: `qa_engineer`
- 联审角色: `runtime_engineer`, `blockchain_ops_engineer`, `viewer_engineer`
- 当前阻塞项: 无；GWSC-1 到 GWSC-5 的轻量实现和 smoke gate 已完成。
- 下一步: 按需执行重型非 dry-run module/full/integration/release 验证，刷新真实 evidence。

## Current Evidence
- `testing-manual.md` 已有 S9A claim boundary 与命令入口。
- `scripts/game-world-state-sync-commit-module-required.sh` 已提供一键 `module_required` wrapper，并输出 `summary.json` / `summary.md` 与 claim boundary。
- `doc/testing/templates/state-sync-closure-evidence-packet-template.md` 已提供 `module_full` state-sync closure 证据包模板，并明确 blob closure 与 observer catch-up 不可互相替代。
- `scripts/s10-five-node-game-soak.sh` 的 `summary.json` / `summary.md` 已输出 `api_viewer_projection` 契约字段，默认 `status=not_collected`，并明确不声明 API/viewer projection verified、`release_full` 或 public_testnet ready。
- `scripts/network-tier-public-testnet-readiness.sh` 已要求 public_testnet active lanes 包含 `api_viewer_projection_ready`；`scripts/network-tier-manifest-smoke.sh` 覆盖 10 条 required lanes 的 ready/block/template-pass 拒绝路径。
- `p2p-mixed-topology-matrix` 已有 required/full exact/proxy 区分。
- `p2p-longrun-soak` 已有 triad/triad_distributed 长跑入口。
- `p2p-verify-state-sync-closure` 已有 blob closure verifier。
- `s10-five-node-game-soak` 已有五节点真实游戏长跑入口。

## Known Gaps
- 当前实现提供 entrypoints、templates、summary contracts 和 readiness lane binding；尚未执行重型非 dry-run S10 / public_testnet 真实同窗口采样。
- state-sync closure 与 observer 自动追高是两个不同证据，不能互相替代。
- macOS 默认 Bash 3.2 不能直接执行 S9/S10 longrun 脚本；执行环境需 Bash 4+。

## Next Validation
推荐下一轮验证任务按以下顺序刷新证据：
1. 执行 `./scripts/game-world-state-sync-commit-module-required.sh`。
2. 执行 `./scripts/p2p-mixed-topology-matrix.sh --tier full`。
3. 执行一条短窗 triad soak。
4. 从 healthy node 导出 state-sync bundle，执行 closure verifier，并按 `doc/testing/templates/state-sync-closure-evidence-packet-template.md` 回填 evidence packet。
5. 执行 S10 five-node short soak，并附 API/viewer projection 同窗口证据。
