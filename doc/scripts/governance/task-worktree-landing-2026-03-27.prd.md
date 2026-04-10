# oasis7 task worktree landing PRD

- 对应设计文档: `doc/scripts/governance/task-worktree-landing-2026-03-27.design.md`
- 对应项目管理文档: `doc/scripts/governance/task-worktree-landing-2026-03-27.project.md`

审计轮次: 1

## 0. Meta
- Owner: `producer_system_designer`
- Collaborators: `qa_engineer`
- Scope: `scripts/land-task-worktree.sh`、`AGENTS.md`、`doc/scripts/**`
- PRD-ID: `PRD-SCRIPTS-WTL-001`
- Related Module PRD: `doc/scripts/prd.md` (`PRD-SCRIPTS-007`)
- 当前定位: 兼容 / fallback 专题；默认最终合流已迁到 `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.prd.md`

## 1. Executive Summary
- Problem Statement: 仓库已经要求“每个需求默认新开一个 task worktree”，但任务完成后如何把结果稳定回流到本地 `main` 仍停留在人工 `git checkout` / `git rebase` / `git merge` 组合拳。不同执行者容易混用 merge 策略、跳过 clean-state 检查、在错误 worktree 上操作，导致 `main` 真值不稳定，也让后续 task worktree 的回收时机缺少统一口径。
- Proposed Solution: 保留 `scripts/land-task-worktree.sh` 作为 local-only / fallback 兼容入口。默认最终合流迁到 GitHub PR；本脚本仅在用户显式要求本地合流、离线应急或 PR 路径不可用时使用，仍提供 clean-state 检查、线性历史整理与 cleanup 命令输出。
- Success Criteria:
  - SC-1: task branch 可通过单一入口而非手写 git 序列合入本地 `main`。
  - SC-2: source / target 任一 worktree 脏时，脚本快速失败并给出修复建议。
  - SC-3: local-only landing 策略保持“rebase target -> fast-forward target”，确保线性历史与可审计性。
  - SC-4: agent 可通过 `--json` 读取 landing 前后提交、source/target worktree 路径与结果状态。
  - SC-5: landing 成功后，source task `worktree` / branch 必须被回收，不再以“可选 cleanup”表述。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要稳定回流任务结果到本地 `main`，避免流程分叉。
  - `qa_engineer`: 需要 landing 失败时有明确失败签名和修复建议。
  - agent executor: 需要结构化输出，自动驱动 cleanup 或下一步验证。
- User Stories:
  - PRD-SCRIPTS-WTL-001: As a `producer_system_designer`, I want one landing command, so that every completed task returns to the local `main` through the same audited workflow.
  - PRD-SCRIPTS-WTL-002: As a `qa_engineer`, I want clean-state and fast-forward guards, so that landing failures surface before the local `main` is mutated.
  - PRD-SCRIPTS-WTL-003: As an agent executor, I want JSON output for landing results, so that I can automate follow-up cleanup or verification.
- Critical User Flows:
  1. `scripts/land-task-worktree.sh -> 读取当前 branch 作为 source -> 检查 source/本地 main worktree 干净 -> source rebase 本地 main -> 本地 main fast-forward merge source -> 输出 cleanup 命令并执行回收`
  2. `scripts/land-task-worktree.sh task/<module>-<task> --target main --json -> 上层读取 source/target worktree、commit 前后变化与 landing 状态 -> 决定是否继续 cleanup`
  3. `source 或 target worktree 脏 / target branch 未被任何 worktree 检出 / rebase 冲突 -> 立即失败 -> 输出修复建议`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| source 解析 | `source_branch`、`source_worktree` | 默认取当前 branch；也允许显式传入 source branch | `input -> source_resolved` | source branch 必须被某个 worktree 检出 | 执行者可用 |
| target 解析 | `target_branch`、`target_worktree` | 默认 target=本地 `main`；读取其当前 worktree | `input -> target_resolved` | target branch 必须被某个 worktree 检出 | 执行者可用 |
| clean-state 围栏 | `source_clean`、`target_clean` | 任一 worktree 脏则阻断 | `resolved -> guarded/rejected` | 用 `git status --short` 判定 | `qa_engineer` 定义失败语义 |
| landing 执行 | `source_head_before`、`source_head_after`、`target_head_after`、`result` | 在 source 执行 `git rebase <target>`，在 target 执行 `git merge --ff-only <source>` | `guarded -> rebased -> landed` | 目标历史保持线性；若已 landing 则返回 no-op | `producer_system_designer` 定流程 |
| 摘要输出 | `cleanup_commands`、`result` | 输出人类摘要或 JSON | `landed -> cleanup_required -> cleaned_up` | `--json` 时 stdout 仅输出单对象；landing 成功后 cleanup 为必做项 | 人类 / agent 皆可读 |
- Acceptance Criteria:
  - AC-1: `scripts/land-task-worktree.sh --help` 明确列出 source branch、`--target`、`--json`、`--dry-run`。
  - AC-2: 默认 source 为当前 branch，默认 target 为本地 `main`。
  - AC-3: source / target 任一 worktree 脏、source/target branch 未被任何 worktree 检出时，脚本必须阻断。
  - AC-4: 脚本必须先在 source worktree 上执行 `git rebase <target>`，再在 target worktree 上执行 `git merge --ff-only <source>`。
  - AC-5: `--json` 至少输出 `source_branch`、`source_worktree`、`target_branch`、`target_worktree`、`source_head_before`、`source_head_after`、`target_head_after`、`result`。
  - AC-6: landing 成功后必须输出 cleanup 命令，并明确该 task worktree / branch 需要被删除；默认仍不自动删除，避免脚本在 source worktree 内自删当前目录。
- Non-Goals:
  - 不负责默认最终合流到受保护 `main`；该职责已迁到 `prepare-task-pr.sh`。
  - 不自动解决 rebase 冲突。
  - 不默认自动 cleanup。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: Bash、`git worktree`、`git rebase`、`git merge --ff-only`
- Evaluation Strategy: 以“landing 到本地 `main` 的步骤数、失败语义清晰度、是否保持线性历史”评估落地效果。

## 4. Technical Specifications
- Architecture Overview: local-only landing 入口复用当前仓库 `git-common-dir` 作为真值，通过 branch -> worktree 映射定位 source/target，再把“rebase source”与“ff merge target”拆成两个明确阶段。
- Integration Points:
  - `scripts/land-task-worktree.sh`
  - `scripts/new-task-worktree.sh`
  - `AGENTS.md`
  - `doc/scripts/prd.md`
  - `doc/scripts/project.md`
- Edge Cases & Error Handling:
  - detached HEAD：拒绝默认 source 解析，要求显式传入 branch。
  - source=target：拒绝 landing。
  - target 未被任何 worktree 检出：阻断并要求先挂载本地 `main` worktree。
  - rebase 冲突：立即失败并保留冲突现场，不做 reset。
  - target 已经包含 source：返回 `already_landed`，不重复 merge。
  - cleanup 延迟：视为流程未完成，而不是“可选后续动作”。
- Non-Functional Requirements:
  - NFR-WTL-1: 流程必须非交互，适合 agent 调用。
  - NFR-WTL-2: `--json` stdout 契约必须保持纯净。
  - NFR-WTL-3: local-only landing 策略必须保持线性历史。
  - NFR-WTL-4: 失败提示必须指出哪一个 worktree/branch 需要修复。
- Security & Privacy:
  - 不泄漏敏感配置。
  - 不自动 push 到远端。

## 5. Risks & Roadmap
- Technical Risks:
  - 风险-1: 若 target=本地 `main` 未在本地 worktree 中常驻，脚本会频繁因缺少 target worktree 失败。
  - 风险-2: 若团队仍绕过脚本手写 git 命令，本地 `main` 合入路径会再次分叉。
  - 风险-3: 若 landing 成功后 task worktree 不及时删除，本地 branch/worktree 清单会逐渐失真。
  - 风险-4: 若未来需要 squash / merge-commit 策略，本轮线性历史假设需要重新评估。
- Roadmap:
  - v1: local-only 合入本地 `main` 的 landing 标准化。
  - v1.1: 视需要补 `--cleanup` 或远端 push/PR 辅助，但不在本轮实现。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-WTL-001/002/003 | WTL-1 | `test_tier_required` | `bash -n` + `--help` + dry-run JSON + 临时 source/target worktree landing smoke + 文档治理检查 | task worktree 回流本地 `main` 的工程闭环 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WTL-001 | local-only 兼容路径采用 `rebase target -> merge --ff-only source` | 直接 `merge --no-ff` 或手工要求用户自己选策略 | 当前仓库强调一任务一 worktree/branch，线性 landing 更利于审计与回滚。 |
| DEC-WTL-002 | local-only 兼容路径仍不自动 cleanup，但 cleanup 仍是 landing 成功后的必做步骤 | landing 成功后立刻删除 source worktree/branch | 脚本常从 source worktree 内执行，自动删除当前工作目录风险过高；因此保留手动删除，但不允许长期跳过。 |
