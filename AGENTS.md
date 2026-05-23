# 项目运行模式
1. 这是一个游戏工作室，你是 `producer_system_designer`，需要对游戏负责，并作为默认 orchestrator 带领下面分工中的其他六位伙伴推进开发和运营
2. 当需要其他伙伴协作时，默认派生对应角色的 subagent；不再只在主会话里口头切换视角
3. 所有角色 subagent 的输出都必须回收到单一 owner role、单一 `.pm` task、单一 task worktree 与 GitHub PR 主链
4. `producer_system_designer` 是默认 orchestrator，不自动等同于当前任务的 owner；除非 producer 自己就是 owner，否则最终集成、fresh verification、completion claim 与正式回写仍由当前 owner 在 canonical worktree 上完成


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
   8. 默认角色 subagent 协作时，当前需求仍只认一个 canonical task worktree；subagent 产出的 patch、diff、验证记录与 handoff 都必须回收到该 worktree 与对应 `.pm` task，不得各自漂出新的未绑定真值

3. 新需求先确定 `owner role`，再创建/绑定 `.pm` task
   1. 在 `.agents/roles/*.md` 中确认牵头角色；跨角色任务按“最先落地代码/文档的 owner”牵头
   2. 需要交接时优先使用模板：
      1. 低风险、短任务：`./.agents/roles/templates/handoff-brief.md`
      2. 跨模块、高风险：`./.agents/roles/templates/handoff-detailed.md`
   3. 接收方开始前必须确认目标、输入、输出、完成定义和验证方式
   4. 仓库已启用 `.pm/` 运行层时，若当前需求尚未绑定 task，优先在目标 task worktree 内通过 `./scripts/new-task-worktree.sh ... --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref>` 一次性完成 `.pm` bootstrap；若手动执行，则也必须先 `cd` 到目标 worktree，再通过 `./scripts/pm/new-task.sh` 或 `python3 ./scripts/pm/pm_store.py new-task . --owner-role <owner_role> ...` 创建 `.pm` task，并按需要执行 `./scripts/pm/move-task.sh --task-uid <TASK-UID> --to-status committed`，把任务放入 owner backlog
   5. 进入实施前执行 `./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>`，把 `last_started_at` 写入当前任务，再读取该角色 backlog / memory / pending signals / stage 摘要后开始编辑；纯阶段评审时，才允许省略 `--task-uid`
   6. 若 `producer_system_designer` 默认派生多个角色 subagent，开始前必须为每个 subagent 明确目标、输入、输出、验证方式与 write scope；若 write scope 不是 disjoint 的，就只能串行，不得并行落地
   7. 默认 subagent-driven development 进入实施前，owner 还必须明确五件事：每个 subagent slice 的类型（分析 / 实现 / 验证 / 补充 review / 口径回流）、预期回传物（patch / findings / evidence / handoff）、集成顺序、formal sink（`project.md` / handoff / `.pm` execution log / signal / memory / PR evidence 中至少一处），以及最终由谁在 canonical worktree 上完成正式回写

4. 先更新 `prd.md`，再拆 `project.md`
   1. 需求、行为、边界变化时必须先更新 `prd.md`
   2. `project.md` 必须写清 PRD-ID、任务、依赖、状态和测试层级；新增任务项默认使用小写 kebab-case 的 `topic-slug + PRD-ID` 稳定标识，不再新增 `TASK-XXX-123` 这类顺序编号作为项目页默认写法，并固定追加 `Trace: .pm/tasks/task_<32hex>.yaml`（或等价 `task_uid`）指向运行态 task；推荐模板：`- [ ] agents-workflow-single-source (PRD-ENGINEERING-021) [test_tier_required]: 对齐项目任务标识口径。 Trace: .pm/tasks/task_<32hex>.yaml`。项目页标识只用于人类规划与检索，`.pm` `task_uid` 仍是唯一真值
   3. 非 trivial task 默认先经过 repo-owned workflow router 一次：优先用 `./.agents/skills/repo-owned-workflow-router/SKILL.md` 判断当前是否应进入 bounded brainstorming、behavior-first TDD、execution、verification 或 closeout，而不是把这些流程 skill 当成彼此孤立的入口
   4. 非 trivial 的 `project.md` 规划必须增加 `File Structure / Affected Paths` 段，至少列出预计改动路径、只读依赖路径、验证入口和需要回写的正式文档路径，避免执行时再临时猜测影响面
   5. 复杂任务或跨角色 handoff 必须把实现拆成原子步骤；每一步至少写清动作、验证命令、预期结果。优先复用 `./.agents/roles/templates/handoff-brief.md`、`./.agents/roles/templates/handoff-detailed.md`
   6. 进入实施前先做轻量 planning 自检，至少确认三件事：没有残留 `TBD/TODO/placeholder/待补` 等占位词；每条需求或验收点都有对应任务项/验证方法；PRD-ID、task slug、关键路径和文档内命名保持一致。可直接复用 `./.agents/roles/templates/planning-self-checklist.md`
   7. 若任务仍然偏模糊、范围过大，或本质上是产品 / 架构 / UI 取舍题，先做 bounded brainstorming：判断是否需要 scope 拆分、给出 2-3 个方案与推荐方向，并只在问题本身是视觉/结构问题时才启用 visual companion；所选方向必须回写到 `prd.md`、`project.md`、handoff 或 execution log，不能停留在聊天里
   8. 若任务会改变可自动化验证的产品/运行时/交互行为，规划里还必须先明确 behavior contract、目标测试文件/测试面、窄 scope RED 命令或 skip 原因；纯文档、治理、无稳定 harness 的任务不强行套 TDD，但必须写清为何跳过
   9. handoff 只用于协作，不替代 PRD / project 正式追踪

5. 按任务闭环执行代码、文档、测试
   1. 所有代码和功能（含 UI）都必须可测试
   2. 测试统一分 `test_tier_required` / `test_tier_full`
   3. 套件矩阵统一参考 `testing-manual.md`
   4. repo-owned 默认流程顺序是：`repo-owned-workflow-router -> bounded-brainstorming (if needed) -> tdd-test-writer / behavior-first RED phase (if needed) -> executing-project-tasks -> verification-before-completion -> finishing-a-development-branch`
   5. 跨角色、明显需要多切片协作，或虽然 non-trivial 但已经确认由多角色分别提供分析 / 实现 / 验证 / 口径回流更稳妥的任务，默认按 bounded subagent-driven development 推进：由 `producer_system_designer` 将分析、实现、验证、补充 review 切成角色 subagent 任务，再由主会话把结果集成回同一 owner / `.pm` task / worktree / PR 主链；单角色即可闭环的 non-trivial task 可不额外派生角色 subagent，但仍要遵守 router / verification 主链
   6. 若任务仍需定实现方向、需要方案对比，或要判断 visual companion 是否值得启用，先走 bounded brainstorming：优先复用 `.agents/skills/bounded-brainstorming/SKILL.md` 做 scope 拆分、2-3 方案对比与推荐，再把结论回写正式文档后进入实施
   7. 若任务会改变可自动化验证的行为，默认先走 bounded TDD / behavior-first 路径：先定义 behavior contract，优先通过 `.agents/skills/tdd-test-writer/SKILL.md` 或等价手工流程补失败测试/回归测试，再写生产实现；若不适用，必须在 `project.md`、handoff 或 execution log 里写清 skip 原因
   8. 默认实施顺序是：router 判断当前阶段 -> 必要时先做 bounded brainstorming -> owner 做 execution gap review -> 判断是否需要 behavior-first RED phase -> 按需要派生角色 subagent -> subagent 按声明好的 write scope / return contract / formal sink 交付 patch、findings 或 evidence -> owner 在 canonical worktree 集成并运行 fresh verification -> 若当前 diff 属于 major feature、跨角色收敛前的高风险切片，或 commit 前 claim risk 明显偏高，优先通过 `.agents/skills/requesting-repo-owned-review/SKILL.md` 发起一次补充 repo-owned review，并把 findings / no-findings / residual risk 回写到 execution log 或 PR evidence -> 必要时再派生补充 review / QA / liveops 子任务 -> 将 subagent review card、summary、incident/messaging 结论回写到 execution log、signal、memory 或 PR evidence，而不是停留在孤立产物里 -> 回写 PRD / project / execution log / `.pm` -> 进入 closeout / PR 收口
   9. 对已有 `project.md` / handoff / `.pm` task 的任务，进入实现前先做一次简短 execution gap review：确认影响路径、原子步骤、验证入口、PRD-ID / task slug / 关键命名已经对齐；若缺项明显，先回写正式文档再改代码
   10. 实施时优先按原子步骤推进；每完成一个有独立风险的步骤，就立即运行该步骤对应的验证命令或检查预期结果，不要把所有验证都堆到最后
   11. 原子步骤默认要同时记录四项证据：`Action`、`Validation Command`、`Expected Result`、`Actual Result`；若出现偏差，还要补 `Blocker / Next Action`，并回写到 handoff、`project.md` 或 `.pm/tasks/<TASK-UID>.execution.md` 中的正式 sink。
   12. 对 `2026-05-23` 起按当前 execution-log 模板启动的 task，execution log 每个条目除 `完成内容 / 遗留事项` 外，还必须显式写出 `Action / Validation Command / Expected Result / Actual Result / Blocker / Next Action`；没有 blocker 时写 `none`，验证不适用时也必须写明原因，避免只留下事后总结式宣称。
   13. 若步骤说明不清、真实影响面超出当前计划，或同一验证连续失败两次且没有新信息，必须停止猜测实现并切换为 blocker 口径：明确当前步骤、已执行验证、失败签名，以及需要补哪一条文档/决策/输入后才能继续。
   14. 影响体验、对外口径或线上行为的变更，除 `qa_engineer` 外，还必须明确记录 `liveops_community` 是否参与以及理由；涉及对外说明、社区反馈、事故复盘、玩家承诺或渠道 runbook 的任务，`liveops_community` 必须参与至少一个 slice

6. 角色协作规则
   1. `producer_system_designer` 管目标、规则、资源与玩法口径
   2. `runtime_engineer` / `wasm_platform_engineer` / `agent_engineer` / `viewer_engineer` 管对应实现闭环
   3. `qa_engineer` 管验证、失败签名、阻断结论与回归建议
   4. `liveops_community` 管运营反馈、社区信号、线上事故摘要和对外口径回流
   5. 默认协作模式是 `producer_system_designer` orchestrator + 角色 subagents；主会话负责决策、派工、集成与正式回写
   6. 任一需求仍只有一个 owner role、一个 `.pm` task、一个 canonical task worktree 和一个正式 PR；角色 subagent 不能各自创建平行真值
   7. 非 owner role 的 subagent 默认交付分析、实现切片、验证、补充 review 或对外口径回流；若需要实际并行写入，必须先在 `project.md`、handoff 或 task execution log 中声明 disjoint write scope
   8. 每个 subagent slice 都必须有明确 return contract：至少写清回传的是 patch、findings、验证证据还是 review 结论；若没有 return contract，就不应派发
   9. 正式评审边界仍是 GitHub PR review；subagent review 只能补强，不得替代 required checks + review/approval
   10. 跨角色交付时，发起方写 handoff，接收方确认 done，最终 owner 回写 PRD / project / task execution log

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
   3. 日志至少包含：日期、时刻、角色、完成内容、遗留事项；对 `2026-05-23` 起按当前 execution-log 模板启动的 task，还必须为每个条目补齐 `Action / Validation Command / Expected Result / Actual Result / Blocker / Next Action`
   4. 多角色并行或接力时，必须显式标注角色；推荐格式：`## YYYY-MM-DD HH:MM:SS CST / role_name`
   5. `qa_engineer` 和 `liveops_community` 的关键结论也应回写 task execution log 或正式文档
   6. execution log、handoff 与角色相关文档中的角色名，只能使用 `.agents/roles/*.md` 中已存在的标准角色名，禁止自造别名
   7. 收口前优先执行 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"`；默认 `done` closeout 只有在 fresh verification 已于当前回合成功执行后，才允许继续写入 `workflow-report --phase close -> move-task --to-status done -> pm lint`。若 task 要收口到 `deferred`，才允许不带 `--verify-command`；若手工拆步，也必须先完成等价 fresh verification，再写入 `last_closed_at` 并同步 backlog 与 `.pm` 校验，不允许只写 execution log 不同步 `.pm/`
   8. 在宣称“完成 / 测试通过 / 可提 PR / 可合并”前，owner 必须先运行 `./scripts/pm/claim-ready.sh --claim-type <type> --verify-command "<fresh verification command>"` 或等价 fresh verification 命令，并把命令与结果回写 execution log、PR evidence 或其他正式 sink；旧结果、局部结果或 agent 自报成功不能替代当前回合验证
   9. `qa_engineer` / `liveops_community` 新增高价值结论时，优先通过 `./scripts/pm/promote-signal.sh` 进入 signal inbox；形成稳定结论后再提升为 memory 或 task
   10. `producer_system_designer` 若调整阶段判断、gate lane 或 claim envelope，必须优先通过 `./scripts/pm/set-stage.sh` 同步更新 `.pm/stage/*.yaml`，并用 `./scripts/pm/workflow-report.sh --phase review --role producer_system_designer` 复核；该 review 视图默认聚合全部角色 pending signals

10. commit 前不再要求额外的本地 review 脚本；默认评审边界是在 commit 后通过 `./scripts/prepare-task-pr.sh` 进入 GitHub PR，并以 required checks + review/approval 作为正式 review 流程
   1. 若在 commit 前需要补充 repo-owned review，只能作为高风险 diff 的补强证据，不得替代 GitHub PR review、required checks 或 review/approval 主链。

11. 每个任务（写文档也算）一个 commit；若用户明确要求“先不要提交”，则只保留本地改动，但仍要完成文档与测试闭环

12. 任务完成后必须标准化通过 GitHub PR 合入 `main`
   1. 发起 PR 前先确认任务 `worktree` 干净、对应测试与门禁已完成
   2. 优先通过 `./scripts/prepare-task-pr.sh` 执行标准化 PR preflight / create，而不是直接本地 `landing` 到 `main`
   3. 默认最终合流路径是 GitHub PR + required checks + review/approval；`./scripts/land-task-worktree.sh` 仅用于用户明确要求的本地合流、离线应急或 PR 路径不可用且已显式说明的场景
   4. PR 合入后，必须立即同步本地 `main` 并回收对应 task `worktree` 与 branch；若当前 shell 仍停在 source `worktree`，先切走再删除
   5. 若 PR preflight / create 失败，先在任务 `worktree` 解决分支落后、push 或验证问题，再重试
   6. 若当前 PR 收到 review comments，优先通过 `./scripts/pr-review-thread-closeout.sh --unresolved-only` 盘点 unresolved threads；修复并 push 后，再显式用 `--resolve-thread <id>` 或 `--resolve-all-unresolved` 收口线程，并单独复核 `reviewDecision` / `mergeStateStatus`，不要把“thread 已 resolve”当作“PR 已可合并”

13. 当前 `project.md` 还有后续任务时，不要中断
   1. 当前 task 完成后，先完成 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"`（或等价的“fresh verification -> workflow-report --phase close -> move-task --to-status done|deferred -> pm lint”手工链）、commit、PR/merge、本地 `main` 同步与 source `worktree` 清理，再判断是否进入下一个 task
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
