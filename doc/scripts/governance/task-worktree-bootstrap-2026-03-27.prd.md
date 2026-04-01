# oasis7: task worktree bootstrap 标准入口（2026-03-27）

- 对应设计文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.design.md`
- 对应项目管理文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.project.md`

审计轮次: 2

## 1. Executive Summary
- Problem Statement: 根 `AGENTS.md` 已明确“每个新需求默认新开独立 `git worktree`”，但仓库内仍缺少统一执行入口。继续依赖人工手写 `git worktree add` 会导致 branch/path 命名不一致、脏源 worktree 被误当作 base truth、已有 branch 被重复检出，以及 agent 无法稳定拿到新 worktree 的机器可读摘要。
- Proposed Solution: 新增 `scripts/new-task-worktree.sh` 作为标准入口，接收 `<module> <task>` 并派生稳定 `task/<module>-<task>` 分支名与默认 worktree 路径；默认阻断脏源 worktree，检测 branch/path 冲突，并提供可选 JSON 摘要、模块文档检查和 harness 预热能力供 agent 或上层脚本消费。同时在正式规则口径中明确：文档/脚本/测试/话术改动也属于“新需求”，只有用户明确授权“复用当前 worktree”才允许例外，且发现切错 worktree 后必须立即切走，不得因为“已经开始改了几行”继续留在错误 worktree。
- Success Criteria:
  - SC-1: 每个新需求都能通过统一入口创建独立 worktree，而不是依赖手写 `git worktree add`。
  - SC-2: 默认 branch/path 命名稳定、可搜索、可回收。
  - SC-3: 源 worktree 脏、目标路径已存在或目标 branch 已在其他 worktree 检出时，脚本会快速失败并给出修复建议。
  - SC-4: agent 可通过 `--json` 直接读取新建 worktree 的 `branch`、`path`、`base_ref` 与创建模式。
  - SC-5: `--init-docs` 可立即报告 `doc/<module>/prd.md`、`doc/<module>/project.md` 与当日 `doc/devlog/YYYY-MM-DD.md` 的存在性。
  - SC-6: `--with-harness` 可在新 worktree 中后台预热 `./scripts/worktree-harness.sh up --no-llm`，且 `--json` 仍保持纯 JSON 输出。
  - SC-7: worktree 治理口径必须明确“文档改动、脚本改动、测试改动、仅改话术”都算新需求，不得因改动看起来“小”就继续复用当前 worktree。
  - SC-8: worktree 治理口径必须明确只有用户显式说出“复用当前 worktree / 就在这里改 / 不要切新 worktree”时才算例外；“先写一版”“先不要提交”“顺手改一下”都不算复用授权。
  - SC-9: worktree 治理口径必须明确若开工后才发现切错 worktree，需要立即说明并切走，不能把“已经开始改了几行”当成继续复用的理由。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要把“新需求先开新 worktree”从口头规范变成低摩擦默认流程。
  - `qa_engineer`: 需要让并行验证任务能快速落到独立 worktree，再接上已有 harness。
  - agent 执行者: 需要稳定得到 `branch` / `worktree_path` / `base_ref`，便于自动进入新任务环境。
- User Scenarios & Frequency:
  - 新需求启动：高频。
  - 多任务并行推进：高频。
  - 中断后恢复已有任务 branch：中频。
  - 口头需求澄清：高频；尤其是“先写一版”“先不要提交”“顺手改一下”这类表述必须有明确歧义围栏。
- User Stories:
  - PRD-SCRIPTS-WTB-001: As a `producer_system_designer`, I want a standard task worktree bootstrap command, so that every new requirement starts with the same branch and path conventions.
  - PRD-SCRIPTS-WTB-002: As a `qa_engineer`, I want dirty-source and branch-collision guards, so that parallel tasks do not accidentally inherit ambiguous source truth.
  - PRD-SCRIPTS-WTB-003: As an agent executor, I want a JSON summary for the new worktree, so that I can automate the next step without scraping prose.
  - PRD-SCRIPTS-WTB-004: As a `qa_engineer`, I want the bootstrap command to optionally inspect module docs and prewarm harness, so that a new task can move from creation to validation with one command.
- Critical User Flows:
  1. `scripts/new-task-worktree.sh scripts task-worktree-bootstrap -> 校验源 worktree clean -> 创建 task/scripts-task-worktree-bootstrap 分支与默认路径 -> 输出下一步命令`
  2. `scripts/new-task-worktree.sh <module> <task> --json -> 上层读取 branch/path/base_ref -> 进入新 worktree 执行 PRD / project 工作流`
  3. `目标 branch 已存在但未被检出 -> 复用该 branch 附着到新路径 -> 输出 mode=attach_existing_branch`
  4. `scripts/new-task-worktree.sh <module> <task> --init-docs -> 输出模块 PRD / project / 当日 devlog 的存在性与建议下一步`
  5. `scripts/new-task-worktree.sh <module> <task> --with-harness -> 创建新 worktree 后异步触发 worktree-harness.sh up --no-llm -> 立即返回 bootstrap_log / state.json / 当前状态摘要`
  6. `用户只说“先写一版 / 先不要提交 / 顺手改一下” -> 仍视为新需求 -> 先切独立 worktree，再开始编辑`
  7. `agent 已在错误 worktree 开始改动 -> 发现后立即说明 -> 停止继续扩写 -> 切到新 worktree 再继续`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 输入规范 | `module`、`task`、`base_ref`、`path`、`branch` | 校验参数、生成 slug | `raw -> validated/rejected` | 默认 slug 化并折叠非法字符 | 当前仓库执行者可用 |
| 命名派生 | `module_slug`、`task_slug`、`branch_name`、`repo_name`、`worktrees_root` | 默认生成 branch/path | `validated -> named` | 默认 `task/<module>-<task>` 与 `<worktrees_root>/<repo_name>-<module>-<task>`；若当前 worktree 已记录 family 配置则继承之 | scripts owner 可维护 |
| worktree 创建 | `mode`、`branch_exists`、`path_exists` | 新建 branch 或附着已有 branch | `named -> created/attached/failed` | 先检查 path，再检查 branch 是否已被其他 worktree 占用 | 当前仓库执行者可用 |
| worktree 例外授权 | `reuse_authorized`、`authorization_phrase`、`source_worktree_dirty` | 只有用户显式授权复用当前 worktree 时才允许跳过 bootstrap | `unknown -> authorized/rejected` | 仅接受“复用当前 worktree / 就在这里改 / 不要切新 worktree”等明确表述；其他模糊表达一律视为未授权 | 仅用户可授予复用例外 |
| 摘要输出 | `worktree_path`、`branch`、`base_ref`、`mode` | 打印人类可读摘要或 JSON | `created -> ready` | `--json` 时仅输出机器可读单对象 | agent / 人类均可读 |
| 文档检查 | `doc_checks.prd`、`doc_checks.project`、`doc_checks.today_devlog` | `--init-docs` 时报告存在性与路径 | `ready -> docs_checked` | 只读检查，不自动创建文档 | `producer_system_designer` / `qa_engineer` 可读 |
| harness 预热 | `harness.bootstrap_log`、`harness.state_file`、`harness.status`、`harness.viewer_url` | `--with-harness` 时在新 worktree 后台触发 `worktree-harness.sh up --no-llm` | `ready -> harness_booting -> harness_ready` | 默认 no-llm；立即返回，不阻塞完整冷启动 | `qa_engineer` / agent 可触发 |
- Acceptance Criteria:
  - AC-1: `scripts/new-task-worktree.sh --help` 明确列出 `<module> <task>`、`--base`、`--branch`、`--path`、`--json`、`--allow-dirty-source`、`--init-docs`、`--with-harness`。
  - AC-2: 默认 branch 名为 `task/<module>-<task>`，默认 worktree 根目录位于仓库同级 `worktrees/` 下。
  - AC-3: 源 worktree 脏时默认阻断，只有显式 `--allow-dirty-source` 才允许继续。
  - AC-4: 若目标 branch 已在其他 worktree 检出，脚本必须阻断并打印对应 worktree 路径。
  - AC-5: `--json` 至少输出 `module`、`task`、`branch`、`worktree_path`、`base_ref`、`mode`。
  - AC-6: `--init-docs` 至少输出 `doc/<module>/prd.md`、`doc/<module>/project.md`、当日 `doc/devlog/YYYY-MM-DD.md` 的路径与存在性。
  - AC-7: `--with-harness` 必须在新 worktree 中异步执行 `./scripts/worktree-harness.sh up --no-llm`，并在摘要中输出 `bootstrap_log`、`state_file` 与当前 `status`；若 `viewer_url` 已就绪则一并输出。
  - AC-8: `--json --with-harness` 仍只输出单个 JSON 对象；harness 子命令的人类输出不得混入 stdout。
  - AC-9: 与 `AGENTS.md` 对齐的正式口径必须明确“文档/脚本/测试/话术改动也算新需求”，并列出“不算复用授权”的模糊表达示例。
  - AC-10: 与 `AGENTS.md` 对齐的正式口径必须明确：若 agent / owner 开工后才发现切错 worktree，必须立即切走，而不是继续把错误 worktree 用完。
- Non-Goals:
  - 不在本轮自动创建模块 PRD / project 骨架。
  - 不接管 `git worktree remove` 或 branch 回收生命周期。
  - 不替代已有 harness / producer playtest 脚本。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `git`、`python3`、POSIX shell 环境。
- Evaluation Strategy: 通过 create/remove smoke、失败语义清晰度和 JSON 摘要完整率评估脚本可用性。

## 4. Technical Specifications
- Architecture Overview: `scripts/new-task-worktree.sh` 作为最上游任务初始化入口，负责解析 repo root、校验源 worktree 状态、派生命名、检查 branch/path 冲突，并调用 `git worktree add`。脚本不改业务源码，只负责把新需求稳定地放到独立 worktree。
- Integration Points:
  - `AGENTS.md`
  - `scripts/new-task-worktree.sh`
  - `scripts/worktree-harness.sh`
  - `doc/scripts/project.md`
- Edge Cases & Error Handling:
  - 当前目录不在 git worktree：立即失败。
  - `module` / `task` 为空或 slug 化后为空：立即失败。
  - 源 worktree 脏：默认阻断，并提示 `--allow-dirty-source`。
  - 目标路径已存在：立即失败，避免写入未知目录。
  - 目标 branch 已在其他 worktree 检出：立即失败并指明路径。
  - 目标 branch 已存在但未被检出：允许附着已有 branch。
  - 用户只说“先写一版”“先不要提交”“顺手改一下”：仍视为未授权复用当前 worktree，必须先切新 worktree。
  - 已在错误 worktree 开始改动：视为执行偏差，必须立即说明并切走；“已经开始改了几行”不是例外。
  - `--init-docs` 对缺失模块文档只报告缺失，不静默创建空文件。
  - `--with-harness` 若后台启动器都没拉起，立即失败；若 harness 后续冷启动失败，失败签名写入 `bootstrap_log` / `state.json`，由后续命令读取。
- Non-Functional Requirements:
  - NFR-WTB-1: 默认 branch/path 命名必须稳定可预测。
  - NFR-WTB-2: `--json` 输出必须稳定，便于 agent 和上层脚本消费。
  - NFR-WTB-3: 失败提示必须包含下一步修复建议，而不仅是 git 原始报错。
  - NFR-WTB-4: `--json` 模式下 stdout 契约必须保持纯净，即使同时启用 `--with-harness`。
  - NFR-WTB-5: worktree 治理口径与 `AGENTS.md` 的一致率 100%，不得出现脚本专题默认更松、根规则默认更严的双重解释空间。
- Security & Privacy: 脚本仅操作本地 git metadata 与目录，不输出敏感凭证。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (WTB-1): 提供 branch/path 派生、clean guard、JSON 摘要。
  - v1.1 (WTB-2): 叠加模板化命名约束或自动打开新 shell/编辑器提示。
  - v2.0 (WTB-3): 视需要补模块级 PRD/project 骨架生成。
- Technical Risks:
  - 风险-1: 若命名规则太宽松，不同任务可能 slug 冲突。
  - 风险-2: 若不检查“branch 已在其他 worktree 检出”，会产生并行任务踩 branch 的隐性冲突。
  - 风险-3: 若 harness 预热的人类输出泄漏到 stdout，agent 将无法稳定消费 JSON 摘要。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-WTB-001 | WTB-BOOT-1/2 | `test_tier_required` | `--help` + 默认 branch/path 命名 smoke | 新需求初始化一致性 |
| PRD-SCRIPTS-WTB-002 | WTB-BOOT-1/2 | `test_tier_required` | dirty/branch/path 围栏与 create/remove smoke | 并行任务隔离与 source truth 清晰度 |
| PRD-SCRIPTS-WTB-003 | WTB-BOOT-1/3 | `test_tier_required` | `--json` 字段核验 | agent 自动进入新 worktree 的稳定性 |
| PRD-SCRIPTS-WTB-004 | WTB-BOOT-4 | `test_tier_required` | `--init-docs` / `--with-harness` create/remove smoke + JSON 纯度检查 | 新任务从创建到文档/验证闭环的一跳成本 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WTB-001 | 新增统一 `new-task-worktree.sh` 入口 | 继续仅靠口头规范 + 手写 `git worktree add` | 手工流程无法保证命名、围栏和 agent 可消费性一致。 |
| DEC-WTB-002 | 默认阻断脏源 worktree，只允许显式 override | 默认忽略脏源状态 | 新任务若悄悄基于未提交改动语义展开，后续追溯会失真。 |
| DEC-WTB-003 | 允许附着未被占用的已有 branch，但阻断已被其他 worktree 检出的 branch | 一律强制新建 branch | 恢复历史任务时复用 branch 更实用，但必须保留 branch 占用围栏。 |
| DEC-WTB-004 | `--with-harness` 默认后台触发 `worktree-harness.sh up --no-llm` 并立即返回摘要 | 只打印推荐命令，不执行；或同步阻塞到 ready | 对 QA / agent 而言，真正降低一跳成本的是“创建后立即预热”，而不是再次人工拼命令或把创建命令卡在一次完整冷启动上。 |
| DEC-WTB-005 | worktree 例外只接受用户的显式复用授权 | 将“先写一版”“先不要提交”“顺手改一下”等模糊话术视作默认复用许可 | 模糊表述会把“是否切新 worktree”重新变回主观判断，破坏单需求单 worktree 的默认围栏。 |
| DEC-WTB-006 | 发现切错 worktree 后必须立即切走 | 允许因为“已经开始改了几行”继续在错误 worktree 内完成任务 | 错误 worktree 一旦继续承载新需求，会把 source truth、task 追踪和后续 landing 全部混在一起。 |
