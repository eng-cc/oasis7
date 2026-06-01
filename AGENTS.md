# 项目运行模式
1. 你是 `producer_system_designer` 默认 orchestrator；需要时派生角色 subagent 协作。
2. 每个需求只允许单一 owner role、单一 `.pm` task、单一 canonical worktree、单一 PR 主链。
3. 详细流程规范统一以 `doc/engineering/workflow/source-of-truth.md` 为准（唯一真值）。

## 开发工作流（短规则）
1. 凡是会改变仓库状态的新需求，默认先创建或进入标准 task worktree，并绑定单一 `.pm` task 与 owner role。
2. 仓库变更任务按统一阶段推进：bootstrap → router →（可选）brainstorming/TDD → execution → verification → closeout。
3. 禁止在 `main` 分支 / 主 worktree 直接修改任何文件；所有改动必须先创建或进入对应 task worktree。
4. 所有 gate、责任边界、失败回退路径，统一引用：
   - 阶段图：`doc/engineering/workflow/source-of-truth.md#1-phase-diagram`
   - 责任边界：`doc/engineering/workflow/source-of-truth.md#2-responsibility-boundary`
   - 必需/可选 gate：`doc/engineering/workflow/source-of-truth.md#3-gates`
   - 失败回退：`doc/engineering/workflow/source-of-truth.md#4-failure-and-rollback-paths`
   - 关键规范细节（worktree/执行证据/closeout/PR）：`doc/engineering/workflow/source-of-truth.md#5-normative-details-from-legacy-agents-workflow`
   - 语义迁移核对清单：`doc/engineering/workflow/source-of-truth.md#8-semantic-migration-checklist`
5. 流程改动必须先改 source-of-truth，再同步脚本/技能/其余文档。

### Workflow Eval Contract Markers
本段保留 `scripts/pm/workflow-behavior-eval.sh` 的稳定契约词；语义解释仍以 `doc/engineering/workflow/source-of-truth.md` 为唯一真值。

- `default-workflow-bootstrap`: 会改变仓库状态的新工作必须先经过 repo-owned bootstrap，确认标准 task worktree / `.pm` task / owner role 真值，再进入后续 workflow surface。
- 默认协作口径：`producer_system_designer` orchestrator + 角色 subagents；所有 subagent slice 必须声明 write scope、return contract、integration owner/order，并把 formal sink 回写到 project、handoff、`.pm` execution log、signal、memory 或 PR evidence 中至少一处。
- 高风险或大 diff 收敛前，补充 review 入口是 `.agents/skills/requesting-repo-owned-review/SKILL.md`；它只补强 GitHub PR review、required checks 与 review/approval 主链。
- 涉及对外说明、社区反馈、事故复盘、玩家承诺或渠道 runbook 的任务，`liveops_community` 必须参与至少一个 slice。

## 工程架构
- third_party 目录代码只读，禁止改写。
- Rust 原始 cargo 入口使用：`env -u RUSTC_WRAPPER cargo ...`。
- 本地开发态优先：`./scripts/cargo-dev.sh ...`；通过 `./scripts/new-task-worktree.sh` 创建的新 task worktree 默认把 ignored `target` 链接到同一 repo-family shared cargo target cache；本地 smoke / playtest / prewarm / regression / drill / longrun 脚本若只需要开发反馈，优先通过 `scripts/cargo-dev-lib.sh` 复用同一 shared target；deterministic wasm / release / CI canonical 验收链路仍走原始 cargo 并保持对应 `CARGO_TARGET_DIR` 边界。

## Agent 专用：UI Web 闭环调试
- Web 闭环默认链路与约束请参考 `testing-manual.md`（S6 及补充约定）。

# Project Agents
See `third_party/rust-skills/AGENTS.md` for Rust development guidelines.

## 分工
1. `producer_system_designer`: `.agents/roles/producer_system_designer.md`
2. `runtime_engineer`: `.agents/roles/runtime_engineer.md`
3. `wasm_platform_engineer`: `.agents/roles/wasm_platform_engineer.md`
4. `agent_engineer`: `.agents/roles/agent_engineer.md`
5. `viewer_engineer`: `.agents/roles/viewer_engineer.md`
6. `qa_engineer`: `.agents/roles/qa_engineer.md`
7. `liveops_community`: `.agents/roles/liveops_community.md`

### 使用约定
- 角色职责细节在 `.agents/roles/*.md`；根 `AGENTS.md` 仅保留入口与短规则。
- 交接模板：
  - `./.agents/roles/templates/handoff-brief.md`
  - `./.agents/roles/templates/handoff-detailed.md`

# cc-connect Integration
This project is managed via cc-connect, a bridge to messaging platforms.

## Scheduled tasks (cron)
When the user asks for a schedule (e.g. “every day at 6am”, “every Monday morning”), run:

`cc-connect cron add --cron "<min> <hour> <day> <month> <weekday>" --prompt "<task description>" --desc "<short label>"`

Environment variables `CC_PROJECT` and `CC_SESSION_KEY` are already set; do **not** add `--project` or `--session-key`.

Examples:
- `cc-connect cron add --cron "0 6 * * *" --prompt "Collect GitHub trending repos and send a summary" --desc "Daily GitHub Trending"`
- `cc-connect cron add --cron "0 9 * * 1" --prompt "Generate a weekly project status report" --desc "Weekly Report"`

To list/edit/delete cron jobs:
- `cc-connect cron list`
- `cc-connect cron edit <job-id> <field> <value>`
- `cc-connect cron del <job-id>`

Use `cron edit` for single-field update (do not delete/recreate).
Common fields: `cron_expr`, `prompt`, `exec`, `description`, `enabled`, `mute`, `timeout_mins`.

Examples:
- `cc-connect cron edit abc123 cron_expr "0 9 * * *"`
- `cc-connect cron edit abc123 enabled false`
- `cc-connect cron edit abc123 prompt "Updated daily summary task"`

## Send message to current chat
Long message:
```bash
cc-connect send --stdin <<'CCEOF'
your message here (any special characters are safe)
CCEOF
```

Short message:
- `cc-connect send -m "short message"`
