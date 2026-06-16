# testing PRD Project

审计轮次: 10

## 当前执行窗口
- 当前状态: active
- 当前权威入口: `doc/testing/prd.md`、`testing-manual.md`、`doc/testing/prd.index.md`
- 当前阻断摘要: `doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`
- 活跃任务:
  - [ ] `test-coverage-gate-fill` (PRD-TESTING-002/003) [test_tier_required] + [test_tier_full]: 补齐 Rust CI 测试覆盖缺口，让 `full-support` 直接触达 workspace support crates，并为 `required-gate` changed-path planner 增加 regression，防止未分类代码路径绕过 full fallback。Trace: `.pm/tasks/task_ce44b8a269824fbcb718febd2140c425.yaml`
  - [ ] token genesis allocation audit follow-up: 等待 `producer_system_designer` / `runtime_engineer` 提供真实创世账户表后，用 `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.project.md` 与对应模板执行首轮正式审计。
- 当前门禁/治理摘要:
  - `required-gate-ondemand-launcher-web-build`、`rust-required-gate-ondemand-scope` 与 `wasm-determinism-gate-ondemand-scope` 已把 GitHub required gate 收口为 stable context + changed-path on-demand 执行；launcher/shared runtime 命中时会额外补跑 launcher Web `trunk build`。
  - `engineering-code-quality-performance-baselines` 已为 `required-gate` 增补 Viewer changed-path perf smoke scope；`viewer-performance-probe.sh --profile smoke` 当前是 report-only scoped gate，不作为 blocking failure。
  - `playability-governance-stack-2026-05-06` 已把好玩性证据栈、标准角色 subagent 评审系统、模拟玩家 persona panel，以及 `L4A synthetic` / `L4B embodied-agent` / `L5` 真实人类与线上验证边界收口为单个 bundle 视图。
  - `playability-player-leverage-evidence-rubric` 已为 trust/playability 证据补单独的 `player leverage` 审查层，避免再用 world activity 代替玩家有效参与。
  - `shared-network-ecs-triad-chain-status-metrics-rollout` 已冻结本机 observer + 两台阿里云 ECS 的 same-window triad snapshot、最近 `10` 分钟 traffic window，以及 `/v1/chain/status` 新增 live contract 证据。

## 历史追溯索引
本页不再按时间顺序手工追加完成项长表；历史 trace 仍保留在对应 topic `*.project.md`、证据文件与 `.pm/tasks/task_<32hex>.yaml` / `.execution.md` 中。

| 历史批次 / 专题 | 追溯入口 |
| --- | --- |
| 基础 testing PRD、S0~S10、证据包、趋势 baseline 与 strict schema 迁移 (`TASK-TESTING-001` 至 `TASK-TESTING-034`) | `doc/testing/prd.index.md` 的专题三件套清单；对应 `.pm/tasks/task_<32hex>.*` |
| Archive / CI / wasm determinism / required-gate 保护批次 (`TASK-TESTING-035` 至 `TASK-TESTING-040`) | `doc/testing/ci/*.project.md`、`doc/testing/governance/*.project.md` |
| Launcher / Web UI / release gate hardening 批次 (`TASK-TESTING-041` 至 `TASK-TESTING-065`) | `doc/testing/launcher/*.project.md`、`doc/testing/manual/web-ui-agent-browser-closure-manual.project.md`、相关 `.pm/tasks/task_<32hex>.*` |
| 2026-03-02 / 2026-03-03 / 2026-03-06 专题任务映射 | `doc/testing/prd.index.md` 按文件名检索对应 `*.project.md`；旧 `SUBTASK-TESTING-*` 映射保留在专题 project 文档与 task evidence 中 |
| Playability evidence stack / L4A-L4B / model visual review | `doc/testing/governance/playability-*.project.md`、`doc/testing/manual/model-visual-review-sop-2026-05-29.manual.md`、`doc/testing/templates/model-visual-review-card-template.md` |
| Shared network / hosted-world / release evidence | `doc/testing/evidence/README.md` 先分流，再进入具体 evidence 文件 |
| Performance coverage and baselines | `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`、`testing-manual.md` 的 required-gate / Viewer performance probe 段落 |

## 最近高价值完成摘要
- `engineering-code-quality-performance-baselines` (Trace: `.pm/tasks/task_35657c5f0a5543dda5d57f51fc4b8841.execution.md`): Viewer changed-path perf smoke 已接入 required-gate report-only scope；runtime module routing perf harness 已有首个 dev/release baseline。
- `required-gate-ondemand-launcher-web-build` (Trace: `.pm/tasks/task_3778b0e747b249bc85b92b942a32b3fd.yaml`): launcher Web `trunk build` 已按 changed-path 注入 required gate，避免 release `build-web-dist` 才暴露编译错误。
- `release-web-*` hardening 系列 (Trace examples: `.pm/tasks/task_b7097f476e674a429469a98d6ae36794.execution.md`、`.pm/tasks/task_bca86dc8b045420baf35b7e28414818c.execution.md`、`.pm/tasks/task_f59a3d14ebcd47dcacbee3a7aa725675.execution.md`): release Web gate 已收口 preflight shadow、独立端口、software-safe live-control 语义与 headed/headless evidence 边界。
- `TASK-TESTING-065/064/063/062`: 已依次收口 launcher `build-web-dist` wasm 兼容性、`release-gate-soak` S10 样本修复、Web UI 闭环 canonical manual，以及 Token 创世分配 QA 审计清单；详细验收命令回到对应 `.pm/tasks/task_<32hex>.execution.md`。

## 依赖
- 模块设计总览: `doc/testing/design.md`
- 文件级索引: `doc/testing/prd.index.md`
- 标准测试手册: `testing-manual.md`
- Web UI 闭环 manual: `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- Web UI 闭环 PRD: `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- CI 入口: `scripts/ci-tests.sh`
- GitHub workflow: `.github/workflows/*`
- PRD governance check: `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-06-16
- 当前状态: active
- 阶段收口优先级: `P0`
- 阶段 owner: `qa_engineer`（联审: `producer_system_designer`）
- 当前阻断条件: 在 `test-coverage-gate-fill` 完成前，跨模块发布评审不得声称 workspace support crates 与 required-gate changed-path planner 覆盖已经完整。
- 承接约束: `test-coverage-gate-fill` 是当前唯一 active 测试治理任务；历史 `TASK-TESTING-*` 完成项回到上方追溯索引、专题 project 与 `.pm` execution log 查询。
- headless-runtime 长稳门禁联动: 已通过 `doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md` 约定证据包字段映射。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- 模块进展补充（2026-03-11）: 已新增 `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`，以 launcher / game / runtime 三个近期样本建立首次通过率、阶段内逃逸率与修复时长 baseline。
- 说明: 本文档仅维护 testing 模块当前执行窗口和高价值追溯入口；更细的历史完成项、旧阶段交接与专题收口请回到对应 topic `*.project.md`、`doc/testing/evidence/*.md` 与 `.pm/tasks/task_<32hex>.execution.md` 追溯。
