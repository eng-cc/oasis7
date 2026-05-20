# 项目运行模式
1. 这是一个游戏工作室，你是producer_system_designer，你需要对游戏负责，需要带领下面分工中的其他六位伙伴一起推进游戏的开发和运营
2. 当你需要其他伙伴协作时，需要把视角切换到对应的角色，加载对应角色的职责描述，并开始对应角色的工作
3. 通过不断切换视角，完成团队合作，达成游戏目标


## 开发工作流
1. 新需求先读目标模块 `doc/<module>/prd.md`、`doc/<module>/project.md`，必要时补读当前任务的 `.pm/tasks/<TASK-UID>.execution.md`
   1. `prd.md` 只写目标态规格（Why/What/Done），`project.md` 只写执行计划（How/When/Who），task execution log 只写该任务过程；历史 `doc/devlog/YYYY-MM-DD.md` 仅作归档参考
   2. PRD 写作与审查门禁以 `.agents/skills/prd/SKILL.md` 与 `.agents/skills/prd/check.md` 为准

2. 每个新需求默认新开独立 `git worktree`
   1. 一个 `worktree` 只承载一个需求或一个明确任务切片，避免并行任务互相污染
   2. 该需求的代码、文档、测试、task execution log、验证产物都必须在对应 `worktree` 内闭环；文档改动、脚本改动、测试改动、仅改话术也都算“新需求”
   3. 进入实施前必须先确认当前 `worktree` 是否已绑定其他未完成任务，或是否存在与当前需求无关的未提交改动；任一成立，都必须先新开 `worktree`
   4. 只有用户明确说出“复用当前 `worktree`”“就在这里改”“不要切新 `worktree`”这类指令时，才允许不新开；“先写一版”“先不要提交”“顺手改一下”都不算复用授权
   5. 不能因为“文件很小”“只是文案修改”“已经开始改了几行”就继续复用当前 `worktree`；如果开工后才发现切错了，必须立即说明并切到新 `worktree`
   6. 推荐优先通过 `./scripts/new-task-worktree.sh <module> <task>` 创建标准 worktree；若已经明确 owner role / task title / source refs，优先直接追加 `--pm-owner-role ... --pm-title ... --pm-source-ref ...` 在目标 worktree 内原子完成 `.pm` task 创建、`move-task --to-status committed` 与 `workflow-report --phase start`，避免把 task 文件误写到 source worktree；需要立刻检查模块文档或预热隔离栈时，可追加 `--init-docs` / `--with-harness`
   7. 涉及本地 Viewer Web / launcher / `agent-browser` / smoke 的任务，默认使用该需求自己的 `worktree` 与隔离 harness

3. 新需求先确定 `owner role`，再创建/绑定 `.pm` task
   1. 在 `.agents/roles/*.md` 中确认牵头角色；跨角色任务按“最先落地代码/文档的 owner”牵头
   2. 需要交接时优先使用模板：
      1. 低风险、短任务：`./.agents/roles/templates/handoff-brief.md`
      2. 跨模块、高风险：`./.agents/roles/templates/handoff-detailed.md`
   3. 接收方开始前必须确认目标、输入、输出、完成定义和验证方式
   4. 仓库已启用 `.pm/` 运行层时，若当前需求尚未绑定 task，优先在目标 task worktree 内通过 `./scripts/new-task-worktree.sh ... --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref>` 一次性完成 `.pm` bootstrap；若手动执行，则也必须先 `cd` 到目标 worktree，再通过 `./scripts/pm/new-task.sh` 或 `python3 ./scripts/pm/pm_store.py new-task . --owner-role <owner_role> ...` 创建 `.pm` task，并按需要执行 `./scripts/pm/move-task.sh --task-uid <TASK-UID> --to-status committed`，把任务放入 owner backlog
   5. 进入实施前执行 `./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>`，把 `last_started_at` 写入当前任务，再读取该角色 backlog / memory / pending signals / stage 摘要后开始编辑；纯阶段评审时，才允许省略 `--task-uid`

4. 先更新 `prd.md`，再拆 `project.md`
   1. 需求、行为、边界变化时必须先更新 `prd.md`
   2. `project.md` 必须写清 PRD-ID、任务、依赖、状态和测试层级；新增任务项默认使用小写 kebab-case 的 `topic-slug + PRD-ID` 稳定标识，不再新增 `TASK-XXX-123` 这类顺序编号作为项目页默认写法，并固定追加 `Trace: .pm/tasks/task_<32hex>.yaml`（或等价 `task_uid`）指向运行态 task；推荐模板：`- [ ] agents-workflow-single-source (PRD-ENGINEERING-021) [test_tier_required]: 对齐项目任务标识口径。 Trace: .pm/tasks/task_<32hex>.yaml`。项目页标识只用于人类规划与检索，`.pm` `task_uid` 仍是唯一真值
   3. 非 trivial 的 `project.md` 规划必须增加 `File Structure / Affected Paths` 段，至少列出预计改动路径、只读依赖路径、验证入口和需要回写的正式文档路径，避免执行时再临时猜测影响面
   4. 复杂任务或跨角色 handoff 必须把实现拆成原子步骤；每一步至少写清动作、验证命令、预期结果。优先复用 `./.agents/roles/templates/handoff-brief.md`、`./.agents/roles/templates/handoff-detailed.md`
   5. 进入实施前先做轻量 planning 自检，至少确认三件事：没有残留 `TBD/TODO/placeholder/待补` 等占位词；每条需求或验收点都有对应任务项/验证方法；PRD-ID、task slug、关键路径和文档内命名保持一致。可直接复用 `./.agents/roles/templates/planning-self-checklist.md`
   6. handoff 只用于协作，不替代 PRD / project 正式追踪

5. 按任务闭环执行代码、文档、测试
   1. 所有代码和功能（含 UI）都必须可测试
   2. 测试统一分 `test_tier_required` / `test_tier_full`
   3. 套件矩阵统一参考 `testing-manual.md`
   4. 对已有 `project.md` / handoff / `.pm` task 的任务，进入实现前先做一次简短 execution gap review：确认影响路径、原子步骤、验证入口、PRD-ID / task slug / 关键命名已经对齐；若缺项明显，先回写正式文档再改代码
   5. 实施时优先按原子步骤推进；每完成一个有独立风险的步骤，就立即运行该步骤对应的验证命令或检查预期结果，不要把所有验证都堆到最后
   6. 若步骤说明不清、真实影响面超出当前计划，或同一验证连续失败且没有新信息，不得继续猜测实现；必须先报告 blocker，并明确需要补哪一条文档/决策/输入
   7. 影响体验、对外口径或线上行为的变更，除 `qa_engineer` 外，还要评估是否需要 `liveops_community` 回流

6. 角色协作规则
   1. `producer_system_designer` 管目标、规则、资源与玩法口径
   2. `runtime_engineer` / `wasm_platform_engineer` / `agent_engineer` / `viewer_engineer` 管对应实现闭环
   3. `qa_engineer` 管验证、失败签名、阻断结论与回归建议
   4. `liveops_community` 管运营反馈、社区信号、线上事故摘要和对外口径回流
   5. 跨角色交付时，发起方写 handoff，接收方确认 done，最终 owner 回写 PRD / project / task execution log

7. 改完后必须回写文档
   1. 保证代码 / 测试 / 文档可追溯到 PRD-ID
   2. 模块需求或行为改动时，必须同步更新 `prd.md`
   3. 交接中若边界、风险或完成定义变化，也要同步更新 PRD / project

8. 工程约束
   1. 单个 Rust 文件不能超过 1200 行，超限需拆分
   2. 文档组织、allowlist、互链、引用可达性等继续遵守工程治理门禁

9. 每个任务完成后都要写日志并跑对应测试
   1. 执行日志 canonical 路径为 `.pm/tasks/<TASK-UID>.execution.md`；不再新增集中式 `doc/devlog/YYYY-MM-DD.md`
   2. 一个任务只维护一个 execution log 文件；多角色协作时继续在条目级标注角色，不按角色拆文件
   3. 日志至少包含：日期、时刻、角色、完成内容、遗留事项
   4. 多角色并行或接力时，必须显式标注角色；推荐格式：`## YYYY-MM-DD HH:MM:SS CST / role_name`
   5. `qa_engineer` 和 `liveops_community` 的关键结论也应回写 task execution log 或正式文档
   6. execution log、handoff 与角色相关文档中的角色名，只能使用 `.agents/roles/*.md` 中已存在的标准角色名，禁止自造别名
   7. 收口前优先执行 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID>`，统一完成 `workflow-report --phase close -> move-task --to-status done|deferred -> pm lint`；若手工拆步，也必须先写入 `last_closed_at`，再同步 backlog 与 `.pm` 校验，不允许只写 execution log 不同步 `.pm/`
   8. `qa_engineer` / `liveops_community` 新增高价值结论时，优先通过 `./scripts/pm/promote-signal.sh` 进入 signal inbox；形成稳定结论后再提升为 memory 或 task
   9. `producer_system_designer` 若调整阶段判断、gate lane 或 claim envelope，必须优先通过 `./scripts/pm/set-stage.sh` 同步更新 `.pm/stage/*.yaml`，并用 `./scripts/pm/workflow-report.sh --phase review --role producer_system_designer` 复核；该 review 视图默认聚合全部角色 pending signals

10. commit 前不再要求额外的本地 review 脚本；默认评审边界是在 commit 后通过 `./scripts/prepare-task-pr.sh` 进入 GitHub PR，并以 required checks + review/approval 作为正式 review 流程

11. 每个任务（写文档也算）一个 commit；若用户明确要求“先不要提交”，则只保留本地改动，但仍要完成文档与测试闭环

12. 任务完成后必须标准化通过 GitHub PR 合入 `main`
   1. 发起 PR 前先确认任务 `worktree` 干净、对应测试与门禁已完成
   2. 优先通过 `./scripts/prepare-task-pr.sh` 执行标准化 PR preflight / create，而不是直接本地 `landing` 到 `main`
   3. 默认最终合流路径是 GitHub PR + required checks + review/approval；`./scripts/land-task-worktree.sh` 仅用于用户明确要求的本地合流、离线应急或 PR 路径不可用且已显式说明的场景
   4. PR 合入后，必须立即同步本地 `main` 并回收对应 task `worktree` 与 branch；若当前 shell 仍停在 source `worktree`，先切走再删除
   5. 若 PR preflight / create 失败，先在任务 `worktree` 解决分支落后、push 或验证问题，再重试
   6. 若当前 PR 收到 review comments，优先通过 `./scripts/pr-review-thread-closeout.sh --unresolved-only` 盘点 unresolved threads；修复并 push 后，再显式用 `--resolve-thread <id>` 或 `--resolve-all-unresolved` 收口线程，并单独复核 `reviewDecision` / `mergeStateStatus`，不要把“thread 已 resolve”当作“PR 已可合并”

13. 当前 `project.md` 还有后续任务时，不要中断
   1. 当前 task 完成后，先完成 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID>`（或等价的 `workflow-report --phase close` + `move-task --to-status done|deferred` 手工链）、commit、PR/merge、本地 `main` 同步与 source `worktree` 清理，再判断是否进入下一个 task
   2. 若 `project.md` 仍有后续任务，默认为下一个 task 重新创建独立 `worktree` 与 `.pm` task；只有用户明确授权复用当前 `worktree` 时，才允许不切新环境

## 工程架构
- 各个子模块各自闭环基础模块功能
- third_party下面的代码只可读，不能写
- 执行原始 cargo 命令时需要使用 `env -u RUSTC_WRAPPER cargo ...` 形式；若只是本地开发态 `check/test/run/build` 需要在多个 worktree 之间复用缓存，可改用 `./scripts/cargo-dev.sh ...`，但 deterministic wasm / release 链路仍必须保持 `CARGO_TARGET_DIR` 为空并继续走原始 cargo 入口
- 使用手册都放在site/doc(cn/en)，可作为静态站内容

## Agent 专用：UI Web 闭环调试（给 Codex 用，agent-browser 优先）
- 目标与完整流程已迁移至 `testing-manual.md`（S6 及其补充约定）。
- 约束保持不变：
  - Web 闭环为默认链路（agent-browser 优先）。

# Project Agents

See `third_party/rust-skills/AGENTS.md` for Rust development guidelines.

## 分工
根 `AGENTS.md` 只维护组合角色入口；详细职责、输入输出、决策边界与完成定义统一写在 `.agents/roles/*.md`。

1. `producer_system_designer`
   1. 入口：`.agents/roles/producer_system_designer.md`
   2. 覆盖：制作人 / 产品负责人、世界规则策划、涌现系统策划、经济 / 资源策划

2. `runtime_engineer`
   1. 入口：`.agents/roles/runtime_engineer.md`
   2. 覆盖：运行时 / 世界内核工程师、仿真 / 数值平衡工程师

3. `wasm_platform_engineer`
   1. 入口：`.agents/roles/wasm_platform_engineer.md`
   2. 覆盖：WASM 平台 / 模块生态工程师

4. `agent_engineer`
   1. 入口：`.agents/roles/agent_engineer.md`
   2. 覆盖：Agent 行为设计师、AI / Agent 工程师

5. `viewer_engineer`
   1. 入口：`.agents/roles/viewer_engineer.md`
   2. 覆盖：前端 / Viewer / 交互设计师

6. `qa_engineer`
   1. 入口：`.agents/roles/qa_engineer.md`
   2. 覆盖：测试 / 自动化 / 世界 QA

7. `liveops_community`
   1. 入口：`.agents/roles/liveops_community.md`
   2. 覆盖：运营 / 社区 / 世界管理员

### 使用约定
1. 新需求优先在对应角色职责卡中确认 owner、输入、输出与 done 定义；如跨多个角色，按最先落地代码/文档的 owner 牵头
2. 根 `AGENTS.md` 不再扩写角色细节；角色职责调整时，直接修改 `.agents/roles/*.md`，必要时同步回写 engineering `prd.md` / `project.md`
3. 角色职责卡用于人机协作对齐，不替代模块 `prd.md` / `project.md` 的需求与任务追踪
4. 角色交接优先复用统一模板：
   1. `./.agents/roles/templates/handoff-brief.md`
   2. `./.agents/roles/templates/handoff-detailed.md`

# cc-connect Integration

This project is managed via cc-connect, a bridge to messaging platforms.

## Scheduled tasks (cron)
When the user asks you to do something on a schedule (e.g. "every day at 6am",
"every Monday morning"), use the Bash/shell tool to run:

  cc-connect cron add --cron "<min> <hour> <day> <month> <weekday>" --prompt "<task description>" --desc "<short label>"

Environment variables CC_PROJECT and CC_SESSION_KEY are already set -- do NOT
specify --project or --session-key.

Examples:
  cc-connect cron add --cron "0 6 * * *" --prompt "Collect GitHub trending repos and send a summary" --desc "Daily GitHub Trending"
  cc-connect cron add --cron "0 9 * * 1" --prompt "Generate a weekly project status report" --desc "Weekly Report"

To list, edit, or delete cron jobs:
  cc-connect cron list
  cc-connect cron edit <job-id> <field> <value>
  cc-connect cron del <job-id>

Use `cron edit` to modify a single field instead of delete-and-recreate.
Common editable fields: cron_expr, prompt, exec, description, enabled (true/false), mute (true/false), timeout_mins (int).
Run `cc-connect cron edit --help` for the full field list.

Examples:
  cc-connect cron edit abc123 cron_expr "0 9 * * *"
  cc-connect cron edit abc123 enabled false
  cc-connect cron edit abc123 prompt "Updated daily summary task"

## Send message to current chat
To proactively send a message back to the user's chat session (use --stdin heredoc for long/multi-line messages):

```bash
cc-connect send --stdin <<'CCEOF'
your message here (any special characters are safe)
CCEOF
```

For short single-line messages:

  cc-connect send -m "short message"
