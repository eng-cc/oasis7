# oasis7 shared network / release train 最小执行形态（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- 对应运行手册: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`

审计轮次: 8
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
  - shared network verdict: `partial`
- 当前缺口:
  - `shared_devnet` 仍未到 `pass`
  - 剩余 blocker 已收敛到 `shared_access / rollback_target_ready / mixed_topology_baseline`
  - `P2PARCH-6` matrix baseline 已成为 shared-network required lane，但它当前只足以阻止 claims 越界，不等价于 shared-window `pass`
  - `shared_access / rollback_target_ready` draft 已生成；`mixed_topology_baseline` 现在也已有正式 `partial` 证据文档，但仍等待更强 same-window mixed-topology 真值或 dedicated lab 裁决，并需要 producer/QA pass-uplift decision ref，才能升到 `pass`
  - `shared_access` 的 endpoint / operator handoff / access evidence 现在也已被提升为模板/脚本/编排输出的正式字段；后续若要在这个 PR 里继续推进，重点不再是“补结构”，而是拿到真实 shared endpoint 与独立 operator/access proof
  - mixed-topology 的 `pass_uplift_decision_ref` 现在已被提升为模板/脚本/编排输出的正式字段；后续若要在这个 PR 里继续推进，重点不再是“补结构”，而是决定 repo 已有 proxy/shared-window 证据是否足以支撑真实 `pass` 裁决
  - `rollback_target_ready` 的 first-pass 语义已收口为：`staging/canary` 仍要求最近一次 formal `pass` candidate，但首条 `shared_devnet pass` 可接受受审计 `bootstrap_restore_ready` fallback；若 `restore_steps_ref/fallback_owner_ref/restoration_scope` 不完整，仍只能记 `partial`
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
- 下一步: 继续用 `./scripts/shared-devnet-rehearsal.sh` 保持既有 `pass` lanes，不再重复 dry-run；在 mixed-topology lane 已有正式 `partial` 证据的基础上，优先补齐真实 `shared_access`、formal `rollback_target_ready`，并决定当前 proxy/shared-window 证据是否足以把 `mixed_topology_baseline` 提升到 `pass`；在此之前继续维持 preview claims，再进入 `staging/canary` rehearsal。
- 最近更新: 2026-04-07
