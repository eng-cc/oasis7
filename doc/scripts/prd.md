# scripts PRD

审计轮次: 7

## 目标
- 建立 scripts 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 scripts 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 scripts 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/scripts/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/scripts/prd.md`
- 项目管理入口: `doc/scripts/project.md`
- 文件级索引: `doc/scripts/prd.index.md`
- 追踪主键: `PRD-SCRIPTS-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 自动化脚本覆盖构建、测试、发布与调试，但职责边界和使用规范分散，导致脚本重叠、入口混乱和维护成本上升。
- Proposed Solution: scripts PRD 统一定义脚本分层（开发、CI、发布、排障）、调用约束、兼容策略与验证标准。
- Success Criteria:
  - SC-1: 核心脚本均有明确 owner、输入输出约定与失败语义。
  - SC-2: 新增脚本在合并前通过语法/参数最小校验。
  - SC-3: 脚本入口重复率下降并保留稳定主入口。
  - SC-4: 脚本任务 100% 映射到 PRD-SCRIPTS-ID。
  - SC-5: scripts 治理专题标题统一使用 `oasis7` 品牌，不再在脚本治理入口中混用 `oasis7` 标题。
  - SC-6: `doc/scripts/precommit/**` 等活跃脚本手册中的当前 crate 命令、依赖说明与 CI 帮助文案必须统一使用 `oasis7*` 口径；旧品牌包名仅允许保留在历史记录或外部原文引用中。
  - SC-7: `doc/scripts/viewer-tools/capture-viewer-frame.{prd,project}.md` 中当前 native fallback viewer 调试说明必须统一使用 `oasis7_viewer` / `OASIS7_VIEWER_*` 口径；旧品牌 viewer 包名与前缀仅允许保留在历史记录或外部原文引用中。
  - SC-8: repo-owned provider real-play helper 文档与脚本（`.agents/skills/oasis7/**`）中的当前 cargo 运行命令与入口路径必须统一使用 `oasis7` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在兼容说明、历史证据或外部原文引用中。
  - SC-9: `run-game-test.sh`、`run-producer-playtest.sh` 与新的 worktree harness 主入口必须支持“每个 git worktree 一套独立端口、独立 bundle、独立日志 / 产物目录、独立浏览器 session”的隔离执行，不再默认复用全局端口与全局 bundle 目录。
  - SC-10: 仓库必须提供标准化 `git worktree` 创建入口，让每个新需求都能按统一命名、统一路径和统一失败语义落到独立 worktree，而不是依赖人工手写 `git worktree add`。
  - SC-11: 标准化 task worktree bootstrap 入口必须支持“创建后立刻检查模块 PRD / project / 当日 devlog”和“可选预热该 worktree 的隔离 harness”，让新需求能直接进入文档与验证闭环。
  - SC-12: 仓库必须提供标准化 task worktree GitHub PR 收口入口，让已完成需求能够在干净状态下统一执行 PR preflight / create，并把 required checks + review/approval 作为 `main` 的默认保护边界。
  - SC-12A: `scripts/land-task-worktree.sh` 仅保留为 local-only / fallback 兼容工具，不再作为默认最终合流入口；其帮助文案与专题文档必须明确这一边界。
  - SC-12B: 仓库必须提供单命令 task closeout helper，在 commit 前把 `workflow-report --phase close`、`move-task --to-status done|deferred` 与 `.pm` 结构校验收口为一个稳定入口，避免不同 owner 手工串接 close-phase 命令链。
  - SC-12C: 仓库必须提供同一 PR 内 review comment 收口 helper，能够统一盘点 unresolved review threads、按 thread id 执行显式 resolve，并在每次操作后回报 `reviewDecision` / `mergeStateStatus`，避免 comment 处理继续依赖临时 GraphQL 命令拼装。
  - SC-13: 每个 task `worktree` 在 PR 合入后都必须回收，不允许长期保留“已完成但未清理”的 task worktree/branch。
  - SC-13A: 仓库必须提供只读 worktree 生命周期盘点入口，能够统一汇总 prunable worktree、已 closed `.pm` task 对应的 clean worktree 与建议 cleanup 命令，避免回收状态只能靠人工 `git worktree list` + `git status` 拼接判断。
  - SC-14: worktree 治理口径必须明确“文档改动、脚本改动、测试改动、仅改话术”都算新需求；只有用户显式授权复用当前 worktree 时才允许例外，且发现切错 worktree 后必须立即切走。

## 2. User Experience & Functionality
- User Personas:
  - 开发者：需要可预期的脚本入口与错误提示。
  - CI 维护者：需要稳定脚本接口，减少流水线波动。
  - 排障人员：需要区分常规链路与 fallback 工具链路。
- User Scenarios & Frequency:
  - 日常开发执行：开发者每次本地验证时使用主入口脚本。
  - CI 流水线运行：每次合并与 nightly 执行。
  - 故障排查：出现异常时按 fallback 规则执行诊断脚本。
  - 脚本契约更新：每周巡检并同步参数文档。
- User Stories:
  - PRD-SCRIPTS-001: As a 开发者, I want stable script entry points, so that daily workflows are reliable.
  - PRD-SCRIPTS-002: As a CI 维护者, I want deterministic script contracts, so that pipeline changes are controlled.
  - PRD-SCRIPTS-003: As a 排障人员, I want explicit fallback tooling rules, so that issue triage is faster.
  - PRD-SCRIPTS-004: As a `qa_engineer`, I want a worktree-isolated harness for Viewer Web / launcher stack, so that multiple agent tasks can boot, verify, and tear down isolated stacks without port, artifact, or browser-session collisions.
  - PRD-SCRIPTS-005: As a `producer_system_designer`, I want a standard task-worktree bootstrap script, so that every new requirement starts from one isolated branch/worktree with consistent naming and minimal manual git ceremony.
  - PRD-SCRIPTS-006: As a `qa_engineer`, I want the task-worktree bootstrap command to optionally inspect module docs and prewarm the worktree harness, so that a new task can move from creation to “read docs + boot isolated stack” in one hop.
  - PRD-SCRIPTS-007: As a `producer_system_designer`, I want a standard task-worktree GitHub PR closure command, so that completed work enters protected `main` with one consistent, auditable path instead of ad hoc local landing.
  - PRD-SCRIPTS-007A: As a `producer_system_designer`, I want a one-command PM task closeout helper, so that close-phase bookkeeping no longer depends on manually chaining `workflow-report`、`move-task` 与 `pm lint` before commit/PR.
  - PRD-SCRIPTS-007B: As a `producer_system_designer`, I want a PR review-thread closeout helper, so that same-PR comment maintenance no longer depends on ad hoc `gh api graphql` snippets and can recheck merge state after each resolve batch.
  - PRD-SCRIPTS-008: As a `producer_system_designer`, I want every completed task worktree deleted after PR merge or explicit local-only fallback completion, so that the local workspace and branch namespace do not fill with stale finished slices.
  - PRD-SCRIPTS-008A: As a `producer_system_designer`, I want a read-only worktree cleanup report, so that I can identify stale finished slices before they accumulate into branch/worktree drift.
  - PRD-SCRIPTS-009: As a 开发者, I want a repo-family shared cargo development wrapper, so that multiple git worktrees can reuse Rust build artifacts without weakening deterministic wasm/release gates.
- Critical User Flows:
  1. Flow-SCR-001: `调用主入口脚本 -> 执行检查/测试 -> 输出结构化结果`
  2. Flow-SCR-002: `CI 触发脚本 -> 失败定位到参数/环境 -> 修复后重跑`
  3. Flow-SCR-003: `常规链路无法复现 -> 触发 fallback 工具 -> 采集诊断证据`
  4. Flow-SCR-004: `new-task-worktree.sh <module> <task> -> 校验源 worktree 状态 -> 创建 task/<module>-<task> 分支与独立 worktree -> 输出进入新 worktree 的下一步命令`
  5. Flow-SCR-005: `new-task-worktree.sh <module> <task> --init-docs --with-harness -> 检查 doc/<module>/{prd,project}.md 与当日 devlog -> 在新 worktree 中后台预热 worktree-harness.sh up --no-llm -> 输出文档检查与 harness 摘要`
  6. Flow-SCR-006: `prepare-task-pr.sh [task/<module>-<task>] -> 检查 source task worktree 干净状态与 base 分支对齐情况 -> 输出或执行 GitHub PR create 命令 -> 通过 required checks + review/approval 合入 `main` -> 同步本地 `main` 并删除已完成 task worktree/branch`
  6A. Flow-SCR-006A: `task-closeout.sh --role <owner_role> --task-uid <TASK-UID> -> 校验 task 已有 last_started_at -> 执行 workflow-report close -> move-task done|deferred -> pm lint -> 输出下一步 prepare-task-pr 提示`
  6B. Flow-SCR-006B: `pr-review-thread-closeout.sh [pr-number] --unresolved-only -> 盘点 unresolved review threads -> 修复并 push 当前 PR -> pr-review-thread-closeout.sh --resolve-thread <id>|--resolve-all-unresolved -> resolve thread -> 回报 reviewDecision / mergeStateStatus 并继续下一轮 comment closeout`
  7. Flow-SCR-007: `用户只说“先写一版 / 先不要提交 / 顺手改一下” -> 仍判定为新需求 -> 先切独立 worktree 再开始编辑；若已在错误 worktree 开工 -> 立即说明并切走`
  8. Flow-SCR-008: `cargo-dev.sh check/test/run -> 解析当前 repo family 的 shared target namespace -> 导出稳定 CARGO_TARGET_DIR -> 以 env -u RUSTC_WRAPPER cargo 执行开发态命令`
  9. Flow-SCR-009: `worktree-gc-report.sh -> 扫描 git worktree + `.pm/tasks/*.yaml` -> 标注 prunable / closed clean worktree cleanup 候选 -> 输出只读汇总与建议命令`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 脚本主入口 | 脚本名、参数、返回码、输出路径 | 执行并输出标准化结果 | `idle -> running -> success/failed` | 按命令类型分层执行 | 所有人可执行 |
| 参数契约 | 必填参数、默认值、失败语义 | 参数校验失败即阻断 | `validating -> accepted/rejected` | 必填项优先校验 | 维护者可更新契约 |
| fallback 规则 | 触发条件、替代脚本、产物要求 | 满足条件后才允许 fallback | `normal -> fallback -> diagnosed` | 常规链路优先 | 仅排障场景允许触发 |
| 标题品牌治理 | 标题前缀、适用专题、兼容命名说明 | 将脚本治理专题标题统一切到 `oasis7` | `legacy_title -> oasis7_title -> audited` | 先改治理主入口，再改周边专题 | owner 可改，治理门禁复核 |
| worktree-isolated harness | `worktree_id`、端口组、状态文件、bundle 根目录、artifact 根目录、browser session | 通过单一 harness 入口执行 `up/down/status/url/logs/smoke` | `idle -> booting -> ready -> verifying -> torn_down` | 先按 worktree 生成稳定身份，再为该 worktree 派生 bundle / port / output | `qa_engineer` 维护主入口，runtime/viewer 协同实现 |
| task worktree bootstrap | `module_slug`、`task_slug`、`branch_name`、`worktree_path`、`base_ref` | 通过统一入口创建或附着任务 worktree，并输出下一步命令 / JSON 摘要 | `draft -> validated -> created/attached -> ready` | 默认派生 `task/<module>-<task>` 分支与 `../worktrees/<repo>-<module>-<task>` 路径 | `producer_system_designer` 定流程，scripts owner 维护入口 |
| task bootstrap followups | `doc_checks`、`today_devlog_path`、`harness_mode`、`harness_state_file`、`viewer_url` | 通过 `--init-docs` / `--with-harness` 补齐文档检查与 harness 预热 | `ready -> doc_checked -> harness_booted` | `--init-docs` 只读检查模块文档；`--with-harness` 默认调用 `worktree-harness.sh up --no-llm` | `qa_engineer` 与 scripts owner 协同维护 |
| task worktree PR closure | `source_branch`、`source_worktree`、`base_branch`、`comparison_ref`、`ahead_count`、`behind_count`、`create_command`、`cleanup_commands` | 通过统一入口校验任务分支、输出或执行 GitHub PR create 命令，并给出 PR 合入后的本地同步/cleanup 命令 | `ready_to_pr -> preflighted -> pr_opened -> merged -> cleaned_up` | 默认源分支取当前 branch，base 默认 `main`、remote 默认 `origin`；PR 合入后必须执行 cleanup | `producer_system_designer` 定流程，scripts owner 维护入口 |
| PR review thread closeout | `pr_number`、`thread_id`、`is_resolved`、`is_outdated`、`path`、`line`、`latest_comment`、`review_decision`、`merge_state_status` | 通过统一入口读取当前 PR review threads，并在显式 resolve 时批量关闭指定 unresolved thread | `reported -> patched -> resolved -> rechecked` | 默认 PR 取当前 branch 关联 PR；`--resolve-all-unresolved` 只处理当前 unresolved thread；每次 resolve 后都必须回报最新 PR state | `producer_system_designer` 定流程，scripts owner 维护入口 |
| worktree lifecycle report | `worktree_path`、`branch`、`prunable_reason`、`dirty`、`pm_task_uid`、`pm_task_status`、`cleanup_candidate`、`cleanup_commands[]` | 通过统一入口只读盘点当前 repo 的 worktree 生命周期状态，并给出建议 cleanup 命令 | `discovered -> classified -> cleanup_candidate/retained` | 默认同时看 `git worktree list --porcelain` 与 `.pm/tasks/*.yaml`；prunable 和 closed clean worktree 优先暴露 | `producer_system_designer` 定流程，scripts owner 维护入口 |
| shared cargo dev cache | `shared_target_dir`、`cache_namespace`、`host_triple`、`rustc_release` | 通过 `cargo-dev.sh` 为开发态 `cargo` 命令注入稳定共享 `CARGO_TARGET_DIR` | `idle -> cache_ready -> cargo_running -> success/failed` | 默认按 `git-common-dir` 派生 repo-family namespace，并按 host/toolchain 拆分目录；deterministic wasm/release 流程继续要求 `CARGO_TARGET_DIR` 为空 | 开发者可执行，scripts owner 维护入口 |
- Acceptance Criteria:
  - AC-1: scripts PRD 明确脚本分类、入口、约束。
  - AC-2: scripts project 文档维护脚本治理任务。
  - AC-3: 与 `doc/scripts/precommit/pre-commit.prd.md`、`testing-manual.md` 口径一致。
  - AC-4: `capture-viewer-frame.sh` 被明确为 fallback 链路使用。
  - AC-5: `doc/scripts/**` 仍可读治理专题标题统一使用 `oasis7` 品牌；旧标题仅允许出现在正文历史上下文中。
  - AC-6: `doc/scripts/precommit/pre-commit.{prd,project}.md` 中当前 viewer wasm 编译门禁、依赖说明与 CI 帮助文案必须写为 `oasis7_viewer` / `cargo check -p oasis7_viewer`；旧品牌 viewer 包名仅允许保留在历史记录或外部原文引用中。
  - AC-7: `doc/scripts/viewer-tools/capture-viewer-frame.{prd,project}.md` 中当前 native fallback viewer 调试说明必须写为 `oasis7_viewer` / `OASIS7_VIEWER_*`；旧品牌 viewer 包名与前缀仅允许保留在历史记录或外部原文引用中。
  - AC-8: `.agents/skills/oasis7/SKILL.md`、`.agents/skills/oasis7/references/real-play-config.md` 与 `.agents/skills/oasis7/scripts/oasis7-run.sh` 中当前 `cargo run -p` 命令和入口路径必须写为 `oasis7` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在兼容说明、历史证据或外部原文引用中。
  - AC-9: 新增 `scripts/worktree-harness.sh` 作为 worktree 级主入口，至少提供 `up/down/status/url/logs/smoke` 六个动作，并把当前 worktree 的运行状态写入稳定 `state.json`。
  - AC-10: `scripts/run-game-test.sh` 必须支持把 `run-id`、`output-dir`、`meta-file` 与 ready payload 交给上层 harness 注入，避免上层通过 grep stdout 猜测 URL/日志路径。
  - AC-11: `scripts/run-producer-playtest.sh` 默认 bundle 根目录必须可按 worktree 隔离，不再强制复用全局 `output/release/game-launcher-producer-local`。
  - AC-12: 新增 `scripts/new-task-worktree.sh`，默认根据 `<module> <task>` 生成稳定分支名与 worktree 路径，并执行 `git worktree add`。
  - AC-13: `scripts/new-task-worktree.sh` 默认在源 worktree 脏时阻断，并给出显式 override；对已存在路径、已被其他 worktree 占用的分支和非法空 slug 提供清晰失败语义。
  - AC-14: `scripts/new-task-worktree.sh --json` 必须输出机器可读摘要，至少包含 `branch`、`worktree_path`、`module`、`task`、`base_ref` 与 `mode`。
  - AC-15: `scripts/new-task-worktree.sh --help` 必须列出 `--init-docs` 与 `--with-harness`；前者输出 `doc/<module>/prd.md`、`doc/<module>/project.md` 和当日 `doc/devlog/YYYY-MM-DD.md` 的存在性摘要，后者在新 worktree 中后台预热 `./scripts/worktree-harness.sh up --no-llm`。
  - AC-16: `scripts/new-task-worktree.sh --json --init-docs` 必须输出机器可读 `doc_checks`；加 `--with-harness` 时，stdout 仍保持单个 JSON 对象，并附带 `harness` 摘要字段。
  - AC-17: 新增 `scripts/prepare-task-pr.sh`，默认以当前 task branch 为 source、以 `origin/main`（若存在）或本地 `main` 为对齐基线，执行“source clean 检查 -> base 对齐检查 -> 输出或执行 GitHub PR create 命令 -> 输出 PR 合入后的本地同步/cleanup 命令”。
  - AC-18: `scripts/prepare-task-pr.sh --help` 必须明确列出 `--base`、`--remote`、`--create`、`--draft` 与 `--json`；`--json` 至少输出 `source_branch`、`source_worktree`、`base_branch`、`comparison_ref`、`ahead_count`、`behind_count`、`create_command` 与 `cleanup_commands`。
  - AC-18A: 新增 `scripts/pm/task-closeout.sh`，默认目标状态为 `done`，执行 `workflow-report close -> move-task done|deferred -> pm lint` 的标准 close-phase 链；若 task 缺少 `last_started_at` 或已经处于 `done/deferred`，脚本必须在写入前失败退出。
  - AC-18B: `scripts/pm/task-closeout.sh --help` 必须明确列出 `--role`、`--task-uid`、`--to-status`、`--no-lint` 与 `--json`；`--json` 至少输出 `task_uid`、`previous_status`、`final_status`、`last_started_at`、`last_closed_at`、`pm_lint.status`、`workflow_close` 与 `move_task`。
  - AC-18C: 新增 `scripts/pr-review-thread-closeout.sh`，默认按当前 branch 关联的 PR 读取 review threads；`--unresolved-only` 仅返回 unresolved threads，`--resolve-thread <id>` 可重复，`--resolve-all-unresolved` 只在显式传入时执行批量 resolve。
  - AC-18D: `scripts/pr-review-thread-closeout.sh --help` 必须明确列出 `[pr-number]`、`--unresolved-only`、`--resolve-thread`、`--resolve-all-unresolved` 与 `--json`；`--json` 至少输出 `pr.number`、`pr.review_decision`、`pr.merge_state_status`、`summary.total_threads`、`summary.unresolved_threads`、`resolved_now.thread_ids` 与每个 thread 的 `id`、`is_resolved`、`is_outdated`、`path`、`line`、`latest_comment`。
  - AC-19: 当 source worktree 脏、source 分支未被任何 worktree 检出、base ref 不存在、或 `--create` 时 source 分支落后于 comparison ref，脚本必须阻断并给出修复建议。
  - AC-20: PR 合入后，正式流程文档与脚本输出必须明确该 task `worktree` / branch 需要被删除；cleanup 命令不得再被表述为“可选建议”。
  - AC-20A: `scripts/land-task-worktree.sh` 的帮助文案与正式专题文档必须明确它只是 local-only / fallback 兼容工具，不再是默认最终合流入口。
  - AC-20B: 新增 `scripts/worktree-gc-report.sh`，默认只读输出当前 repo 的 worktree 生命周期报告，并在 `--json` 模式下至少包含 `repo_root`、`current_worktree`、`summary.total_worktrees`、`summary.cleanup_candidates`、以及每个 worktree 的 `path`、`branch`、`prunable`、`dirty`、`pm_task_uid`、`pm_task_status`、`cleanup_candidate` 与 `cleanup_commands`。
  - AC-21: `AGENTS.md`、`doc/scripts/prd.md` 与 task-worktree bootstrap 专题必须统一写明：文档/脚本/测试/话术改动也算新需求，不能因为改动小而复用已有 worktree。
  - AC-22: 上述正式文档必须统一列出“复用当前 worktree / 就在这里改 / 不要切新 worktree”为允许例外的显式表述，并明确“先写一版 / 先不要提交 / 顺手改一下”不构成复用授权；若已切错 worktree，必须立即切走。
  - AC-23: 新增 `scripts/cargo-dev.sh`，为本地开发态 `cargo check/test/run/build` 提供 repo-family 共享缓存入口，并默认使用 `env -u RUSTC_WRAPPER cargo ...`。
  - AC-24: `scripts/cargo-dev.sh --print-target-dir` 必须输出稳定共享目录；同一 repo family 下不同 worktree 输出一致，且可通过环境变量覆盖。
  - AC-25: 正式文档必须明确：`scripts/cargo-dev.sh` 只服务开发态缓存复用，不适用于要求 `CARGO_TARGET_DIR` 为空的 deterministic wasm / release 构建链路。
  - AC-26: 根 `AGENTS.md` 的 cargo 规则必须与 `scripts/cargo-dev.sh` / `testing-manual.md` 对齐，明确“原始 cargo 命令走 `env -u RUSTC_WRAPPER cargo ...`，开发态共享缓存可走 `./scripts/cargo-dev.sh ...`，但 deterministic wasm / release 仍必须保持 `CARGO_TARGET_DIR` 为空”。
- Non-Goals:
  - 不在 scripts PRD 中替代业务功能设计。
  - 不承诺所有历史脚本长期向后兼容。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: Bash 校验、脚本帮助文档、CI 调用链路。
- Evaluation Strategy: 以脚本失败定位时长、重复脚本数量、CI 脚本稳定性趋势评估。

## 4. Technical Specifications
- Architecture Overview: scripts 模块是工程自动化执行层，向开发、测试、发布提供可组合命令入口，强调“单一职责 + 明确输出”。
- Integration Points:
  - `scripts/`
  - `doc/scripts/precommit/`
  - `doc/scripts/viewer-tools/`
  - `doc/scripts/wasm/`
  - `scripts/run-game-test.sh`
  - `scripts/run-producer-playtest.sh`
  - `scripts/worktree-harness.sh`
  - `scripts/cargo-dev.sh`
  - `scripts/new-task-worktree.sh`
  - `scripts/prepare-task-pr.sh`
  - `scripts/pm/task-closeout.sh`
  - `scripts/pr-review-thread-closeout.sh`
  - `scripts/worktree-gc-report.sh`
  - `scripts/land-task-worktree.sh`
  - `scripts/build-wasm-module.sh`
  - `testing-manual.md`
  - `.github/workflows/*`
- Edge Cases & Error Handling:
  - 参数缺失：立即失败并打印最小可执行示例。
  - 依赖缺失：输出依赖安装提示与环境检查命令。
  - 超时：长脚本超时后输出中间进度并建议重试策略。
  - 权限不足：不可写目录或权限异常时给出路径修复建议。
  - 并发冲突：同产物目录并发执行时强制隔离输出。
  - fallback 误用：未满足触发条件时拒绝 fallback。
  - worktree 并行：同一分支或同一用户同时开多个 worktree 时，端口、bundle、日志、browser session 与 chain node id 必须按 worktree 隔离，避免互相踩踏。
  - worktree bootstrap：源 worktree 脏、目标路径已存在、目标分支已在其他 worktree 检出或 `<module>/<task>` 为空时，必须阻断并打印修复建议。
  - worktree 例外授权：用户若仅说“先写一版”“先不要提交”“顺手改一下”，仍必须先新开 worktree；只有显式授权复用当前 worktree 才可例外。
  - 错误 worktree：若任务开始后才发现 worktree 用错，必须立即说明并切走；不允许把“已经开始改了几行”当作继续复用的理由。
  - bootstrap followups：`--json` 模式下即便开启 `--with-harness`，也不得把 harness 子命令的人类输出混入 JSON；模块文档不存在时只报告缺失，不替用户静默创建空文档。
  - task PR closure：若 base branch 缺少本地/远端 ref、source 分支落后于 comparison ref、`gh` 不可用，或 `--create` 时 push/PR create 失败，脚本只中断并保留现场，不擅自修改 `main` 或删除 branch/worktree。
  - task closeout helper：helper 只负责 `.pm` close-phase bookkeeping，不替代 execution log 回写、commit 或 `prepare-task-pr`；若 task 尚未 start 或已处于关闭态，脚本必须在改动前失败，不允许留下半收口状态。
  - PR review thread closeout：resolve review thread 只代表线程被收口，不代表 PR 已 merge-ready；helper 必须继续单独回报 `reviewDecision`、`mergeStateStatus` 与剩余 unresolved thread 数，避免把“threads 全关掉”和“可以合并”混成同一状态。
  - local-only landing compatibility：`land-task-worktree.sh` 仍可用于用户显式要求的本地合流或离线应急，但帮助文案和正式文档必须明确它不是默认最终合流入口。
  - task cleanup：已完成任务的 task `worktree` 若长期不删，会让后续搜索、branch 占用检查与本地磁盘占用持续失真；因此 cleanup 必须成为 PR 合入后的必做步骤。
  - worktree lifecycle report：缺失路径、prunable 记录、dirty worktree 与未绑定 `.pm` task 的 worktree 都必须 truthfully 报告，不允许脚本为了“看起来整洁”而隐式删除或跳过。
  - shared cargo dev cache：同一 repo family 的多个 worktree 必须映射到同一 shared target namespace，但 deterministic wasm / release 脚本若要求 `CARGO_TARGET_DIR` 为空，必须继续走原始 cargo 入口而不是 `cargo-dev.sh`。
- Non-Functional Requirements:
  - NFR-SCR-1: 核心脚本具备可读帮助信息与失败语义说明。
  - NFR-SCR-2: 主入口脚本在 Linux/macOS 环境可执行一致。
  - NFR-SCR-3: CI 脚本接口稳定，破坏性改动需预告与回归。
  - NFR-SCR-4: 脚本默认输出不得包含敏感信息。
  - NFR-SCR-5: fallback 流程必须可追溯到故障诊断记录。
  - NFR-SCR-6: worktree harness 的状态文件必须机器可读，允许 agent 直接拿到 URL、端口组、输出目录与 PID，而不依赖 stdout 文本解析。
  - NFR-SCR-7: 同一仓库下至少两份 worktree 可在默认配置下并行起栈，不因固定端口或全局 bundle 目录直接冲突。
  - NFR-SCR-8: task worktree bootstrap 入口必须生成稳定默认分支名 / 路径，并支持 JSON 摘要，便于 agent 或上层脚本直接消费。
  - NFR-SCR-9: task worktree bootstrap 入口在开启 followup 选项后，仍需保证 stdout 契约稳定；JSON 模式下所有附加说明必须写入结构化字段或 stderr。
  - NFR-SCR-10: task worktree GitHub PR 收口入口必须默认使用非交互、可审计的 preflight / create 策略；JSON 模式下 stdout 只能输出单个结构化对象。
  - NFR-SCR-11: 已完成 task 的 cleanup 语义必须清晰一致，不允许不同文档同时出现“建议删除”和“必须删除”两套口径；PR 合入后的本地同步/cleanup 与 local-only fallback cleanup 不得混成两套默认流程。
  - NFR-SCR-12: worktree 例外授权与错误 worktree 处置口径在 `AGENTS.md`、模块 PRD 与专题文档之间必须保持一致，不允许根规则更严、模块专题更松。
  - NFR-SCR-12A: worktree 生命周期报告的 JSON 字段必须稳定、只读且不混入人类说明文本，便于 agent 直接消费 cleanup 候选。
  - NFR-SCR-13: 开发态 shared cargo target 目录必须稳定且默认落在工作区外部缓存位置，避免污染仓库源码树或让不同 repo family 相互踩缓存。
- Security & Privacy: 脚本不得在默认输出中泄漏密钥；涉及网络调用时需要显式参数与最小权限。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化脚本分层与主入口规范。
  - v1.1: 增加高频脚本的契约测试与参数回归。
  - v2.0: 建立脚本治理仪表（稳定性、复用率、故障恢复时间）。
- Technical Risks:
  - 风险-1: 历史脚本行为差异导致切换成本。
  - 风险-2: 入口过多导致文档与实际调用脱节。
  - 风险-3: 若 worktree harness 只包壳而不下沉到 `run-game-test.sh` / `run-producer-playtest.sh` 契约层，后续上层脚本仍会靠 grep stdout 和全局目录工作，隔离性会继续失真。
  - 风险-4: 若 worktree 创建仍停留在口头规范而无标准脚本，团队会继续混用手工 branch/path 命名，导致多任务并行难以搜索、回收与审计。
  - 风险-5: 若 `--with-harness` 破坏 JSON/stdout 纯度，agent 侧自动化会从“稳定入口”退回“半结构化抓取”。
  - 风险-6: 若 GitHub PR 收口仍依赖手工 push / gh 序列，不同人会混用本地 landing、直接 push 与半手工 PR 路径，导致默认保护边界和 task worktree 回收时机失控。
  - 风险-7: 若 landing 成功后不强制 cleanup，仓库会持续累积“已完成但仍挂着”的 task worktree，弱化 branch 占用围栏和任务检索准确性。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-001 | TASK-SCRIPTS-001/002/005/009/011 | `test_tier_required` | 脚本分层与入口清单核验 | 日常开发链路稳定性 |
| PRD-SCRIPTS-002 | TASK-SCRIPTS-002/003/005 | `test_tier_required` + `test_tier_full` | 参数契约与失败语义回归 | CI 稳定性与故障定位效率 |
| PRD-SCRIPTS-003 | TASK-SCRIPTS-003/004/005/010 | `test_tier_required` | fallback 使用条件抽样检查 | 排障闭环和风险控制 |
| PRD-SCRIPTS-004 | TASK-SCRIPTS-014 | `test_tier_required` | `bash -n` + `--help` + 双实例并行 smoke + `state.json` / ready payload 检查 + 文档治理检查 | 多 worktree 并行执行稳定性与 agent 可驱动性 |
| PRD-SCRIPTS-005 | TASK-SCRIPTS-015/020 | `test_tier_required` | `bash -n` + `--help` + 真实 create/remove smoke + worktree 例外授权文案一致性检查 + 文档治理检查 | 多任务并行的 worktree/branch 命名一致性与启动成本 |
| PRD-SCRIPTS-006 | TASK-SCRIPTS-016/020 | `test_tier_required` | `--init-docs` / `--with-harness` 真机 create/remove smoke + 错误 worktree 处置文案一致性检查 + 文档治理检查 | 新任务从创建到文档/验证闭环的一跳成本 |
| PRD-SCRIPTS-007 | TASK-SCRIPTS-017/024/task-closeout-helper/pr-review-thread-closeout-helper | `test_tier_required` | `bash -n` + `prepare-task-pr` `--help/--json` + `task-closeout.sh --help` + `required-tier-smoke` closeout helper 断言 + `pr-review-thread-closeout.sh --help/--json` + fake-`gh` review-thread 测试 + `land-task-worktree` compatibility 文案检查 + 文档治理检查 | 多 task worktree 向受保护 `main` 回流的一致性与可审计性 |
| PRD-SCRIPTS-008 | TASK-SCRIPTS-018/025 | `test_tier_required` | landing/cleanup 文案与脚本输出一致性检查、`worktree-gc-report.sh --json` 结构化字段检查 + 文档治理检查 | task worktree 生命周期收口与本地环境整洁度 |
| PRD-SCRIPTS-009 | TASK-SCRIPTS-021/022 | `test_tier_required` | `bash -n` + `--help` + `--print-target-dir` 跨 worktree 一致性检查 + `AGENTS.md`/scripts/testing 文档口径一致性检查 + 文档治理检查 | 多 worktree Rust 开发回归速度与 deterministic wasm/release 口径隔离 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SCR-001 | 主入口 + fallback 分层治理 | 全脚本平级使用 | 分层更利于稳定维护。 |
| DEC-SCR-002 | 参数契约显式化 | 依赖隐式约定 | 可减少 CI 误用与回归。 |
| DEC-SCR-003 | fallback 仅在受控场景启用 | 默认对所有场景开放 | 可避免过度依赖应急链路。 |
| DEC-SCR-004 | 用独立 `cargo-dev.sh` 包装开发态共享 `CARGO_TARGET_DIR`，而不把共享 target 设成仓库全局默认 | 直接把所有 cargo 流程切到同一个全局 `CARGO_TARGET_DIR` | 能让日常多 worktree 开发复用缓存，同时不破坏 deterministic wasm / release 脚本对空 `CARGO_TARGET_DIR` 的围栏。 |
