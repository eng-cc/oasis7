# oasis7 shared network / release train 最小执行形态（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- 对应运行手册: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`

审计轮次: 10
## 任务拆解（含 PRD-ID 映射）
- [x] RTMIN-0 (PRD-P2P-RTMIN-001/002/003/004) [test_tier_required]: 新建 shared network / release train minimum 专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪与 `testing-manual`。
- [x] RTMIN-1 (PRD-P2P-RTMIN-001/002) [test_tier_required]: `runtime_engineer` 落地 `release_candidate_bundle` 真值、版本 pinning 与 drift blocker，并把 bundle 校验接入 `release-gate` 前置步骤。
- [x] RTMIN-2 (PRD-P2P-RTMIN-003) [test_tier_required]: `qa_engineer` 冻结 `shared_devnet/staging/canary` 的 `pass/partial/block` 证据模板与 gate 表，并落地统一 `summary.json/md` scaffold。
- [x] RTMIN-3 (PRD-P2P-RTMIN-004) [test_tier_required]: `liveops_community` 冻结 promotion/freeze/rollback/run window/public claims runbook。
- [x] RTMIN-4 (PRD-P2P-RTMIN-002/003) [test_tier_required + test_tier_full]: 执行 first shared-devnet dry run，落下 candidate/evidence/incident 产物。
- [x] RTMIN-4A (PRD-P2P-RTMIN-002/003) [test_tier_required]: 新增 `shared-devnet` rehearsal orchestration，把 same-candidate 多入口复跑、lane scaffold 与 gate 聚合收敛成单命令入口。
- [ ] RTMIN-5 (PRD-P2P-RTMIN-003/004) [test_tier_required + test_tier_full]: 执行 first staging rehearsal 与 first canary rehearsal，并做 freeze/rollback 演练。

### RTMIN-1 产物
- `scripts/release-candidate-bundle.sh`
- `scripts/release-candidate-bundle-smoke.sh`
- `scripts/release-gate.sh`
- `scripts/release-gate-smoke.sh`
- `testing-manual.md`
- `doc/devlog/2026-03-24.md`

### RTMIN-2 产物
- `scripts/shared-network-track-gate.sh`
- `scripts/shared-network-track-gate-smoke.sh`
- `doc/testing/templates/shared-network-track-gate-template.md`
- `doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv`
- `doc/testing/templates/shared-network-track-gate-lanes.staging.template.tsv`
- `doc/testing/templates/shared-network-track-gate-lanes.canary.template.tsv`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- `testing-manual.md`
- `doc/devlog/2026-03-24.md`

### RTMIN-3 产物
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- `doc/testing/templates/shared-network-promotion-record-template.md`
- `doc/testing/templates/shared-network-incident-template.md`
- `doc/testing/templates/shared-network-incident-review-template.md`
- `doc/testing/templates/shared-network-exit-decision-template.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `doc/p2p/README.md`
- `testing-manual.md`
- `doc/devlog/2026-03-24.md`

### RTMIN-4 产物
- `output/release-candidates/shared-devnet-dry-run-20260324-01.json`
- `output/shared-network/shared-devnet-dry-run-20260324-01/release-gate/20260324-150030/release-gate-summary.md`
- `output/shared-network/shared-devnet-dry-run-20260324-01/gate/shared_devnet-20260324-150230/summary.md`
- `doc/testing/evidence/shared-network-shared-devnet-dry-run-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-promotion-record-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-incident-2026-03-24.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.design.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.project.md`
- `doc/p2p/project.md`
- `testing-manual.md`
- `doc/devlog/2026-03-24.md`

### RTMIN-4A 产物
- `scripts/shared-devnet-rehearsal.sh`
- `scripts/shared-devnet-rehearsal-smoke.sh`
- `scripts/shared-devnet-blocker-packet.sh`
- `scripts/shared-devnet-blocker-packet-smoke.sh`
- `doc/testing/templates/shared-network-mixed-topology-gate-template.md`
- `doc/testing/templates/shared-network-shared-access-check-template.md`
- `doc/testing/templates/shared-network-rollback-target-template.md`
- `doc/testing/evidence/shared-network-shared-devnet-shared-access-draft-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md`
- `doc/testing/evidence/shared-network-shared-devnet-rollback-target-draft-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-follow-up-window-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-follow-up-promotion-record-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-follow-up-incident-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-short-window-promotion-record-2026-03-24.md`
- `doc/testing/evidence/shared-network-shared-devnet-short-window-incident-2026-03-24.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- `doc/p2p/project.md`
- `testing-manual.md`
- `doc/devlog/2026-03-24.md`

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - shared network verdict: `pass`
- 当前缺口:
  - `shared_devnet` 已到 `pass`
  - 当前 formal gate 已更新到 `candidate_id=shared-devnet-live-reset-20260523-01`，见 `doc/testing/evidence/generated-shared-network-gates/shared_devnet-20260524-101652/summary.md`；aggregate 结论为 `pass / eligible_for_promotion`
  - `shared_access` 已基于同窗云上 shared endpoint + 独立 workstation/browser 证据 + cloud storage host 访问证据升到 `pass`
  - `multi_entry_closure` 已基于同窗 `headed web + no-ui + pure_api` 证据升到 `pass`
  - `governance_live_drill` 已基于同窗 post-reset finality drill 升到 `pass`
  - 2026-05-23 的 live triad 卡死事件已通过 `doc/testing/evidence/shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md` 收口：旧链历史被放弃，三节点已在 fresh shared-devnet chain 上恢复推进；因此当前 `shared_devnet` 不再被“observer retention window / sequencer predecessor-gap”这组运行时故障阻断
  - `rollback_target_ready` 已通过 `doc/testing/evidence/shared-network-shared-devnet-rollback-contract-2026-05-23.md` 升到 first-pass 允许的 `bootstrap_restore_ready` `pass`：当前 live-reset candidate 的 fallback bundle/gate、owner、restore steps 与 restoration scope 都已固定
  - `P2PARCH-6` matrix baseline 已成为 shared-network required lane，但它当前只足以阻止 claims 越界，不等价于 shared-window `pass`
  - `shared_access / rollback_target_ready / short_window_longrun` 已全部在当前 candidate 窗口内转为 `pass`
  - `mixed_topology_baseline` 现已通过 `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-2026-05-23.md` 升到 `pass`：同窗 live repair 后，本地 workstation validator 与两台 ECS validator 已在 `2026-05-24 10:13:55 CST` 收敛到 `committed_height=1280 / network_committed_height=1280 / last_execution_height=1280`，并已补齐 producer/QA `pass_uplift_decision_ref`
  - `shared_access` 的 endpoint / operator handoff / access evidence 现在已在同窗 candidate 上闭环；后续不再需要围绕 shared access 本身补结构
  - mixed-topology 的 `pass_uplift_decision_ref` 已被落实到本轮 execution log，并已进入正式 lane / gate 真值
  - `rollback_target_ready` 的 first-pass 语义已收口为：`staging/canary` 仍要求最近一次 formal `pass` candidate，但首条 `shared_devnet pass` 可接受受审计 `bootstrap_restore_ready` fallback；若 `restore_steps_ref/fallback_owner_ref/restoration_scope` 不完整，仍只能记 `partial`
  - `rollback_target_ready` 的脚本合同也已收口为：只有 fallback bundle + fallback gate + fallback owner + restore steps + restoration scope 全部齐全时，repo 才允许把该 lane 记为 `pass`
  - 没有正式 `staging/canary`

## 依赖
- `testing-manual.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`

## 验收命令（RTMIN-4）
- `./scripts/release-gate.sh --dry-run --candidate-bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json --out-dir output/shared-network/shared-devnet-dry-run-20260324-01/release-gate`
- `./scripts/shared-network-track-gate.sh --track shared_devnet --candidate-bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json --lanes-tsv output/shared-network/shared-devnet-dry-run-20260324-01/lanes.shared_devnet.tsv --out-dir output/shared-network/shared-devnet-dry-run-20260324-01/gate`
- `rg -n "partial|hold_promotion|shared-devnet-dry-run-20260324-01|local-only" doc/testing/evidence/shared-network-shared-devnet-dry-run-2026-03-24.md doc/testing/evidence/shared-network-shared-devnet-promotion-record-2026-03-24.md doc/testing/evidence/shared-network-shared-devnet-incident-2026-03-24.md doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.design.md doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.project.md testing-manual.md`
- `./scripts/shared-devnet-rehearsal-smoke.sh`
- `./scripts/shared-devnet-blocker-packet-smoke.sh`
- `./scripts/shared-network-track-gate-smoke.sh`
- `./scripts/release-candidate-bundle-smoke.sh`
- `./scripts/release-gate-smoke.sh`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: active
- 下一步: `shared_devnet` 当前 candidate 已正式转绿；后续从“补 shared-devnet blocker”切换为 `RTMIN-5`：
  - 执行 first `staging` rehearsal
  - 执行 first `canary` rehearsal
  - 继续保持 `shared_devnet -> staging -> canary` 单向 promotion，不把当前 `shared_devnet pass` 误记成 staging/canary 已验证
- 最近更新: 2026-05-24
