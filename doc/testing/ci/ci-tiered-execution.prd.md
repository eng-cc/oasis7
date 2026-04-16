# oasis7: CI 与提交钩子测试分级

- 对应设计文档: `doc/testing/ci/ci-tiered-execution.design.md`
- 对应项目管理文档: `doc/testing/ci/ci-tiered-execution.project.md`

审计轮次: 4


## ROUND-002 口径归属（2026-03-05）
- 本文档是 `commit` / `required` / `full` 分层触发策略的权威定义入口。
- 其它文档（含 `doc/scripts/precommit/pre-commit.prd.md`）仅引用本口径，不再重复定义分层规则。

## 1. Executive Summary
- Problem Statement: 本地 `pre-commit` 与 PR 门禁若共用同一套较重测试，会显著拉长提交反馈周期并影响开发迭代效率。
- Proposed Solution: 建立 `commit` / `required` / `full` 分级执行模型，让默认提交路径只跑轻量 commit baseline，PR/CI required gate 保持单一稳定 check context，但可在 job 内按 changed paths 剪裁 viewer/runtime/support 重型分量，每日定时跑 full，保持“快反馈 + 全覆盖”平衡。
- Success Criteria:
  - SC-1: `scripts/ci-tests.sh` 支持 `commit` / `required` / `full` 分级参数并统一入口。
  - SC-2: `pre-commit` 默认执行 `commit` baseline，开发反馈时间下降。
  - SC-3: GitHub Actions 实现 push/PR 跑 stable-context `required-gate`，并在 job 内先规划 changed-path scope；schedule 继续跑 full。
  - SC-4: 分级策略在脚本、workflow、文档三端口径一致。
  - SC-5: docs-only / `.pm` / 纯元数据 PR 不再实际执行 viewer/runtime/support 重型测试，但仍保留 `required-gate` check context。

## 2. User Experience & Functionality
- User Personas:
  - 开发者：希望提交前获得更快反馈。
  - CI 维护者：希望门禁策略清晰且不漂移。
  - 发布负责人：希望 full 回归仍覆盖主干风险。
- User Scenarios & Frequency:
  - 本地提交：每次提交执行 `commit` baseline。
  - 显式本地重门禁：需要补跑 runtime/simulator 核心 shard 或 viewer Rust 长跑时，手动执行 `required`。
  - PR 门禁：每次 push/PR 先规划 required scope，再执行命中的 required 组件。
  - 每日回归：schedule 执行 full。
- User Stories:
  - PRD-TESTING-CI-TIERED-001: As a 开发者, I want pre-commit to run the lightweight `commit` baseline only, so that commit feedback is fast.
  - PRD-TESTING-CI-TIERED-002: As a CI 维护者, I want one unified test entrypoint with tier flags, so that policy drift is reduced.
  - PRD-TESTING-CI-TIERED-003: As a 发布负责人, I want daily full regression preserved, so that deep regressions are still caught.
- Critical User Flows:
  1. Flow-TIERED-001: `本地提交 -> pre-commit 调用 commit -> 快速返回结果`
  2. Flow-TIERED-002: `push/PR -> planner 基于 changed paths 规划 required scope -> workflow 执行命中的 required 组件 -> 决定是否可合入`
  3. Flow-TIERED-003: `每日定时 -> workflow 执行 full -> 生成重型回归结果`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 分级入口脚本 | `commit` / `required` / `full` 参数 | 调用 `scripts/ci-tests.sh` | `idle -> running -> passed/failed` | commit 优先速度，required 负责较重核心门禁，full 负责低频高覆盖 | CI/开发者可触发 |
| pre-commit 接线 | 默认等级、执行命令 | `pre-commit.sh` 默认跑 `commit` | `hooked -> executed -> reported` | 以提交速度优先 | 本地开发默认执行 |
| workflow 分流 | 触发器类型、执行等级 | push/PR 跑 required，schedule 跑 full | `triggered -> running -> archived` | 时间敏感路径优先 required | 维护者可调整触发策略 |
| required-gate scope planner | `changed_paths`, `scope`, `run_*` 布尔量 | `required-gate` 先规划 `minimal / targeted / full`，再只执行命中的重型组件 | `planned -> pruned -> executed` | docs-only / 无关元数据走 governance/fmt-only；共享 CI/脚本输入命中时必须回退 full | workflow 自动执行；本地显式 `required` 不受影响 |
- Acceptance Criteria:
  - AC-1: `scripts/ci-tests.sh` 分级参数行为明确并可复现。
  - AC-2: `scripts/pre-commit.sh` 默认执行 `commit`，且不再触发 `cargo test -p oasis7 --tests --features test_tier_required`。
  - AC-3: `.github/workflows/rust.yml` 按触发器分流 required/full。
  - AC-4: 文档与任务日志回写完整。
  - AC-5: `.github/workflows/rust.yml` 的 `required-gate` 在 push/PR 上先执行 changed-path planner；docs-only / `.pm` / 无关元数据改动只跑 governance/fmt，且 `required-gate` check context 名称保持不变。
- Non-Goals:
  - 不做 case-level / flaky-aware 的动态测试选择，也不把本地显式 `./scripts/ci-tests.sh required` 改成 changed-path 按需运行。
  - 不做缓存、并行矩阵、runner 基础设施优化。
  - 不变更业务测试断言。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本任务为测试执行策略治理）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 通过统一入口脚本承载分级策略，把 pre-commit commit baseline、CI required gate 与定时 full 回归都收敛到同一执行面。
- Integration Points:
  - `scripts/ci-tests.sh`
  - `scripts/pre-commit.sh`
  - `.github/workflows/rust.yml`
  - `doc/scripts/precommit/pre-commit.prd.md`
  - `doc/testing/ci/ci-test-coverage.prd.md`
- Edge Cases & Error Handling:
  - commit 覆盖过窄：可能把 runtime/simulator 回归延后到显式 required 或 CI required gate 暴露，需结合缺陷复盘补齐。
  - commit 误挂重型 viewer Rust 校验：会重新拉长默认提交耗时，需把 `oasis7_viewer` 全量单测与 wasm32 编译检查限制在显式 `required` / CI required gate。
  - required 覆盖过窄：可能延后发现问题，需结合每日 full 和缺陷复盘补齐。
  - required planner 漏判：若 diff base 不可解析、命中共享 CI / gate 脚本输入、或落入未分类代码路径，必须回退 full，不能静默少跑。
  - docs-only / `.pm` 元数据 PR：允许 `required-gate` 退化为 governance/fmt-only，但 check context 仍必须存在，避免分支保护漂移。
  - full 仅定时执行：发现延迟增加，需保留手动触发路径。
  - 旧命令调用习惯：不带参数调用时需定义默认行为避免误解。
  - 策略漂移：脚本与 workflow 不一致时，以统一入口脚本为基线回收。
- Non-Functional Requirements:
  - NFR-TIERED-1: 本地提交门禁时延显著下降且无关键漏检。
  - NFR-TIERED-2: full 回归覆盖范围不低于迁移前。
  - NFR-TIERED-3: 分级策略变更具备可追溯证据。
- Security & Privacy: 不涉及新数据采集，仅为执行策略调整。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (T1): 文档与项目管理落地。
  - v1.1 (T2): 脚本分级改造与 pre-commit 接线。
  - v2.0 (T3/T4): workflow 分流、文档回写、验证收口。
- Technical Risks:
  - 风险-1: required 覆盖下降导致回归延后暴露。
  - 风险-2: full 定时执行带来发现延迟。
  - 风险-3: 默认参数语义不清导致团队误用。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-CI-TIERED-001 | T1/T2 | `test_tier_required` | pre-commit `commit` 路径验证 | 本地提交反馈效率 |
| PRD-TESTING-CI-TIERED-002 | T2/T3/rust-required-gate-ondemand-scope | `test_tier_required` | 脚本参数、changed-path planner 与 workflow 分流检查 | CI 门禁一致性 |
| PRD-TESTING-CI-TIERED-003 | T3/T4/rust-required-gate-ondemand-scope | `test_tier_required` + `test_tier_full` | required-gate scope 剪裁验证 + schedule full 回归与结果审查 | 发布前深度回归覆盖 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-TIERED-001 | `commit` / `required` / `full` 分级执行 | 每次全量执行 | 分级更符合反馈效率目标。 |
| DEC-TIERED-002 | pre-commit 默认 `commit` | pre-commit 跑 `required` / `full` | 较重门禁会显著阻塞开发节奏。 |
| DEC-TIERED-003 | schedule 承担 full 回归 | 取消 full | 无法保证主干深度质量。 |
| DEC-TIERED-004 | `required-gate` 保持单一 check context，但在 CI job 内按 changed paths 剪裁重型组件 | 把 docs-only / 元数据 PR 继续送进全量 required，或把 `required-gate` 拆成新 check 名称 | 前者无法改善平均时长，后者会引入分支保护漂移与 required context 迁移成本。 |

## 原文约束点映射（内容保真）
- 原“目标（降低提交耗时 + 保留主干覆盖 + 统一入口）” -> 第 1 章与第 2 章 AC。
- 原“In/Out of Scope” -> 第 2 章 AC 与 Non-Goals。
- 原“接口/数据（scripts/workflow）” -> 第 4 章 Integration Points。
- 原“里程碑 T1~T4” -> 第 5 章 Phased Rollout。
- 原“风险（覆盖下降、发现延迟、默认参数误解）” -> 第 4 章 Edge Cases + 第 5 章 Risks。
