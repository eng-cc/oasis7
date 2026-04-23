# oasis7 task worktree GitHub PR closure PRD

- 对应设计文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.design.md`
- 对应项目管理文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.project.md`

审计轮次: 1

## 0. Meta
- Owner: `producer_system_designer`
- Collaborators: `qa_engineer`
- Scope: `scripts/prepare-task-pr.sh`、`AGENTS.md`、`.pm/README.md`、`doc/scripts/**`
- PRD-ID: `PRD-SCRIPTS-GHPR-001`
- Related Module PRD: `doc/scripts/prd.md` (`PRD-SCRIPTS-007/008`)

## 1. Executive Summary
- Problem Statement: 本地 `land-task-worktree.sh` 能约束单机线性历史，但它无法替代 GitHub PR 的 required checks、review/approval 与受保护 `main` 分支边界。继续把“本地 landing 到 local main”当默认最终合流，会让默认保护边界停留在单机流程，而不是服务端门禁；同时，owner 在 commit 后进入 PR preflight 时，仍缺少一份与 changed-path planner 对齐的“本地最小 required 校验建议”，容易退回人工猜测。
- Proposed Solution: 新增 `scripts/prepare-task-pr.sh` 作为标准 GitHub PR 收口入口。脚本负责校验 task worktree 干净状态、比较 source branch 与 base ref、输出或执行 `gh pr create` 命令，并给出 PR 合入后的本地同步/cleanup 命令；同时复用现有 changed-path planner，为当前 diff 输出只读的本地 required-gate 建议命令。旧 `land-task-worktree.sh` 降级为 local-only / fallback 兼容工具。
- Success Criteria:
  - SC-1: 已完成任务的默认最终合流入口切到 `scripts/prepare-task-pr.sh`，而不是 `scripts/land-task-worktree.sh`。
  - SC-2: `prepare-task-pr.sh --json` 输出 source/base/comparison/create/cleanup 结构化字段，便于 agent 消费。
  - SC-3: source worktree 脏、base ref 缺失、`--create` 时 source 落后于 comparison ref 等情况会在本地 preflight 阶段被阻断。
  - SC-4: 正式文档统一说明“GitHub PR + required checks + review/approval”是默认最终保护边界。
  - SC-5: PR 合入后的 task worktree / branch cleanup 仍是必做项，而不是“可选建议”。
  - SC-6: `prepare-task-pr.sh` 必须在 preflight 阶段输出与当前 changed paths 对齐的本地 required-gate 建议命令，但只负责推荐，不自动执行，也不改变 `./scripts/ci-tests.sh required/full` 的既有语义。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要统一、可审计、与受保护 `main` 一致的最终合流路径。
  - `qa_engineer`: 需要在 PR 打开前就看到 branch 落后、source 脏状态、缺少 comparison ref 等明确失败签名。
  - agent executor: 需要 machine-readable JSON 输出，自动驱动 PR create 或 post-merge cleanup。
- User Stories:
  - PRD-SCRIPTS-GHPR-001: As a `producer_system_designer`, I want one PR preflight/create command, so that completed task branches enter protected `main` through one consistent path.
  - PRD-SCRIPTS-GHPR-002: As a `qa_engineer`, I want preflight failure signatures before PR creation, so that branch hygiene issues are blocked before remote review starts.
  - PRD-SCRIPTS-GHPR-003: As an agent executor, I want JSON output for PR closure preparation, so that I can automate create/sync/cleanup follow-ups.
  - PRD-SCRIPTS-GHPR-004: As an owner preparing a PR, I want one changed-path-based local required validation recommendation, so that I can补齐最小本地验证而不必手工猜测该跑 `required` 的哪一部分。
- Critical User Flows:
  1. `prepare-task-pr.sh -> 读取当前 branch 作为 source -> 检查 source worktree 干净 -> 比较 source 与 origin/main 或本地 main -> 输出 changed-path local required validation recommendation -> 输出 gh pr create 命令与 post-merge cleanup 命令`
  2. `prepare-task-pr.sh --create -> 必要时 fetch base -> push source branch -> 执行 gh pr create -> 返回 PR URL`
  3. `source worktree 脏 / source 分支未被任何 worktree 检出 / base ref 不存在 / --create 时 source 落后于 comparison ref -> 立即失败 -> 输出修复建议`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| source 解析 | `source_branch`、`source_worktree`、`source_head` | 默认取当前 branch；也允许显式传入 source branch | `input -> source_resolved` | source branch 必须被某个 worktree 检出 | 执行者可用 |
| base 比较 | `base_branch`、`comparison_ref`、`ahead_count`、`behind_count`、`rebase_required` | 优先比较 `origin/<base>`，缺失时退回本地 `<base>` | `source_resolved -> compared` | `origin/<base>` 优先于本地 `<base>` | 执行者可用 |
| 本地 required 推荐 | `local_required_validation.scope`、`local_required_validation.changed_path_count`、`local_required_validation.recommended_required_command`、`local_required_validation.recommended_extra_commands[]` | 在 preflight 摘要里输出 changed-path 对齐的本地 required 校验建议 | `compared -> locally_recommended` | 复用现有 changed-path planner 推导 scope；只推荐，不自动执行 | 执行者可用 |
| PR create | `remote_name`、`create_command`、`pr_url` | `--create` 时先 push，再执行 `gh pr create` | `compared -> pr_ready -> pr_opened` | 默认 remote=`origin`，title 缺省时走 `--fill` | `producer_system_designer` 定流程 |
| post-merge cleanup | `cleanup_commands[]` | 输出本地 `main` 同步与 source worktree/branch 删除命令 | `pr_opened -> merged -> cleaned_up` | cleanup 是合流后的必做项 | 人类 / agent 皆可读 |
- Acceptance Criteria:
  - AC-1: `scripts/prepare-task-pr.sh --help` 明确列出 `--base`、`--remote`、`--create`、`--draft`、`--title`、`--body-file`、`--json`。
  - AC-2: 默认 source 为当前 branch，默认 base 为 `main`，默认 remote 为 `origin`。
  - AC-3: source worktree 脏、source 分支未被任何 worktree 检出、comparison ref 不存在时，脚本必须阻断。
  - AC-4: `--json` 至少输出 `source_branch`、`source_worktree`、`base_branch`、`comparison_ref`、`ahead_count`、`behind_count`、`create_command`、`cleanup_commands` 与 `local_required_validation.scope` / `local_required_validation.recommended_required_command`。
  - AC-5: `--create` 时若 source 分支落后于 comparison ref，脚本必须阻断并要求先 rebase。
  - AC-6: 正式流程文档必须明确：PR 合入后仍要同步本地 `main` 并回收 task worktree/branch。
  - AC-7: changed-path local required 推荐必须沿用现有 `scripts/plan-rust-required-scope.sh` 与 `./scripts/ci-tests.sh required` 的边界；脚本不得在 preflight 阶段自动执行推荐命令，也不得改写 `required/full` 的既有测试语义。
- Non-Goals:
  - 不自动 merge PR。
  - 不自动等待 GitHub required checks 完成。
  - 不在本轮把 planner `reason_summary`、wasm gate 解释层或自动执行逻辑并入 `prepare-task-pr.sh`。
  - 不删除旧 `land-task-worktree.sh`；它保留给 local-only / fallback 场景。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: Bash、`git worktree`、`gh` CLI、GitHub PR / branch protection。
- Evaluation Strategy: 以“默认最终合流是否经由 GitHub PR、preflight 阻断是否清晰、JSON 是否稳定”评估落地效果。

## 4. Technical Specifications
- Architecture Overview: PR 收口入口复用当前仓库 `git-common-dir` 作为 truth，定位 source worktree，选择 `origin/<base>` 或本地 `<base>` 作为 comparison ref，再将“本地 preflight / 可选 gh create / post-merge cleanup”拆成三个明确阶段。
- Integration Points:
  - `scripts/prepare-task-pr.sh`
  - `scripts/land-task-worktree.sh`
  - `AGENTS.md`
  - `.pm/README.md`
  - `doc/scripts/prd.md`
  - `doc/scripts/project.md`
  - `.github/workflows/*`
- Edge Cases & Error Handling:
  - detached HEAD：拒绝默认 source 解析，要求显式传入 branch。
  - source=base：拒绝 PR preflight。
  - comparison ref 缺失：阻断并要求先 fetch 或补本地 base。
  - source worktree 脏：阻断并要求先 commit/stash。
  - `gh` 不存在：`--create` 时阻断。
  - PR create 失败：保留当前 branch/worktree，不擅自 cleanup。
- Non-Functional Requirements:
  - NFR-GHPR-1: 默认模式必须非交互，适合 agent 调用。
  - NFR-GHPR-2: `--json` stdout 契约必须保持纯净。
  - NFR-GHPR-3: 默认最终保护边界必须落在 GitHub PR，而不是本地 main。
  - NFR-GHPR-4: 失败提示必须指出需要修复的 branch/worktree/ref。
- Security & Privacy:
  - 不在默认输出中泄漏 token。
  - 不默认 merge 或删除远端 branch。

## 5. Risks & Roadmap
- Technical Risks:
  - 风险-1: 若 `gh` 未登录或远端不可达，`--create` 会失败；因此脚本必须先把本地 preflight 与 create 分开。
  - 风险-2: 若旧 `land-task-worktree.sh` 仍被误当默认入口，正式文档会重新分叉。
  - 风险-3: 若 PR 合入后不强制 cleanup，task worktree 生命周期仍会失真。
- Roadmap:
  - v1: 标准化 PR preflight / create。
  - v1.1: 视需要补充 PR 模板、自动 wait/verify 或 merge queue 辅助，但不在本轮实现。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-GHPR-001/002/003/004 | GPR-1/GPR-2 | `test_tier_required` | `bash -n` + `--help` + 当前 task worktree `--json` + JSON 字段断言 + compatibility 文案检查 + 文档治理检查 | task worktree 经由 GitHub PR 收口的一致性、可审计性与本地最小验证推荐 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-GHPR-001 | 默认最终合流切到 GitHub PR，local landing 仅保留兼容/应急 | 继续默认本地 landing 到 local main | 默认保护边界应该落在服务端 required checks + review/approval，而不是单机流程。 |
| DEC-GHPR-002 | 把 PR preflight 与 `gh pr create` 合到一个脚本里，但 `--create` 作为显式动作 | 只输出文档说明，不提供标准入口 | 治理变更必须有可执行主入口，否则默认流程仍会退回口头约定。 |
