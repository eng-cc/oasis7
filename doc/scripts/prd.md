# scripts PRD

审计轮次: 7

## 目标
- 建立 scripts 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 scripts 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 scripts 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/scripts/prd.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/scripts/prd.md`
- 项目管理入口: `doc/scripts/prd.md`
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
  - SC-7: Viewer Web 相关脚本文档只允许围绕 canonical `viewer` / `viewer.html` 静态入口、freshness gate 与 browser automation 维护当前真值；`software_safe` 仅作为兼容 alias / legacy regression 命名保留；已移除的 native fallback / 贴图质检 / 视觉基线工具不得继续作为活跃脚本入口保留。
  - SC-8: repo-owned provider real-play helper 文档与脚本（公开 `site/skills/oasis7.md` 以及 `scripts/setup-provider-oasis7-runtime.sh`、`scripts/provider-parity-p0.sh`）中的当前 cargo 运行命令与入口路径必须统一使用 `oasis7` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在兼容说明、历史证据或外部原文引用中。
  - SC-9: `run-launcher-stack.sh`、`run-producer-playtest.sh` 与新的 worktree harness 主入口必须支持“每个 git worktree 一套独立端口、独立 bundle、独立日志 / 产物目录、独立浏览器 session”的隔离执行，不再默认复用全局端口与全局 bundle 目录。
  - SC-10: 仓库必须提供标准化 `git worktree` 创建入口，让每个新需求都能按统一命名、统一路径和统一失败语义落到独立 worktree，而不是依赖人工手写 `git worktree add`。
  - SC-12C: 仓库必须提供同一 PR 内 review comment 收口 helper，能够统一盘点 unresolved review threads、按 thread id 执行显式 resolve，并在每次操作后回报 `reviewDecision` / `mergeStateStatus`，避免 comment 处理继续依赖临时 GraphQL 命令拼装。
  - SC-12D: 仓库必须提供 `.pm` rebase 冲突辅助入口，在 branch 跟进最新 `main` 时统一报告 `.pm/**` 未合并路径；`.pm/inbox/signals.jsonl` 已退休，命中时只能提示删除退休文件或人工归档，git-ignored 本地视图冲突只能提示“保留 `main` 删除并重建”，不能回退到人工恢复共享视图文件。
  - SC-13: 每个 task `worktree` 在 PR 合入后都必须回收，不允许长期保留“已完成但未清理”的 task worktree/branch。
  - SC-13A: 仓库必须提供只读 worktree 生命周期盘点入口，能够统一汇总 prunable worktree、已 closed `.pm` task 对应的 clean worktree 与建议 cleanup 命令，避免回收状态只能靠人工 `git worktree list` + `git status` 拼接判断。
  - SC-14: worktree 治理口径必须明确“文档改动、脚本改动、测试改动、仅改话术”都算新需求；只有用户显式授权复用当前 worktree 时才允许例外，且发现切错 worktree 后必须立即切走。

### 稳定治理承诺

- 每个常见脚本意图只有一个推荐稳定入口；辅助与 fallback 路径必须声明触发条件，且永远不能替代 canonical 路径。
- 对外发布的脚本契约必须说明最小调用、改变验证范围的选项和失败类别；可变参数、默认值与机器可读字段以当前脚本行为、`--help` 和测试为实现权威。`dry-run`、`skip-*`、语法或 help 成功不能被解释为完整门禁通过。
- Worktree harness 的产品承诺是 machine-readable、worktree-scoped 的启动与证据隔离；teardown 终止运行栈并保留证据。其 `ready` / `smoke` 只证明本地 launcher/Viewer reachability 边界，不证明 headed S6、玩法、持久化、replay/recovery、共识或发布就绪。
- Worktree harness 发布 `state.json` 与 `session.meta` 时必须原子替换完整记录，避免并发读取看到部分状态，并保持既有记录格式可兼容读取；旧记录缺少稳定进程身份且对应进程仍存活时，必须拒绝发送信号，确认进程已退出后才可清理陈旧记录。`ready`、`status`、`url` 与重复 `up` 仅在记录的 PID、PGID 和稳定 leader identity 一致时承认进程仍属于该 harness；`down` 发出 TERM 前须核对进程组归属，再使用有界等待和 KILL 升级。端口分配按同一仓库 worktree 家族串行保留，过期保留仅在核对 owner 存活状态后回收。
- 每个请求从绑定单一 task truth 的独立 task worktree 开始；复用必须有用户明确授权。Bootstrap 部分失败时必须保留已创建的 branch/worktree，并返回可执行的 refresh/retry 恢复指令。
- CI package scope 与 exact-integration 语义以 [workflow source of truth 的 canonical clause](../engineering/workflow/source-of-truth.md) 为准；本 PRD 仅保留此链接，不重复定义该规则。

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
  - PRD-SCRIPTS-007: As a `producer_system_designer`, I want a standard task-worktree GitHub PR closure command, so that completed work enters protected `main` with one consistent, auditable path instead of ad hoc local landing.
  - PRD-SCRIPTS-007B: As a `producer_system_designer`, I want a PR review-thread closeout helper, so that same-PR comment maintenance no longer depends on ad hoc `gh api graphql` snippets and can recheck merge state after each resolve batch.
  - PRD-SCRIPTS-007C: As a `producer_system_designer`, I want a `.pm` rebase conflict helper, so that same-PR rebase maintenance can distinguish retired signal inbox conflicts, generated-view conflicts, and canonical task/memory/stage conflicts that still require manual judgment.
  - PRD-SCRIPTS-008: As a `producer_system_designer`, I want every completed task worktree deleted after PR merge or explicit local-only fallback completion, so that the local workspace and branch namespace do not fill with stale finished slices.
  - PRD-SCRIPTS-008A: As a `producer_system_designer`, I want a read-only worktree cleanup report, so that I can identify stale finished slices before they accumulate into branch/worktree drift.
  - PRD-SCRIPTS-009: As a 开发者, I want a worktree-scoped shared cargo development wrapper, so that each git worktree can reuse its own Rust build artifacts without cross-worktree contamination or weakening deterministic wasm/release gates.
- Critical User Flows:
  1. Flow-SCR-001: `调用主入口脚本 -> 执行检查/测试 -> 输出结构化结果`
  2. Flow-SCR-002: `CI 触发脚本 -> 失败定位到参数/环境 -> 修复后重跑`
  3. Flow-SCR-003: `常规链路无法复现 -> 触发 fallback 工具 -> 采集诊断证据`
  4. Flow-SCR-004: `new-task-worktree.sh <module> <task> -> 校验源 worktree 状态 -> 创建 task/<module>-<task> 分支与独立 worktree -> 输出进入新 worktree 的下一步命令`
  6B. Flow-SCR-006B: `pr-review-thread-closeout.sh [pr-number] --unresolved-only -> 盘点 unresolved review threads -> 修复并 push 当前 PR -> pr-review-thread-closeout.sh --resolve-thread <id>|--resolve-all-unresolved -> resolve thread -> 回报 reviewDecision / mergeStateStatus 并继续下一轮 comment closeout`
  6C. Flow-SCR-006C: `git rebase origin/main -> 命中 .pm/** 冲突 -> rebase-conflict-helper.sh --json -> 若命中 retired signal inbox 则删除退休文件或人工归档 -> 若命中 registry/backlog 视图则保留 main 删除并执行 sync-views -> 其余 canonical task/memory/stage 冲突人工处理`
  7. Flow-SCR-007: `用户只说“先写一版 / 先不要提交 / 顺手改一下” -> 仍判定为新需求 -> 先切独立 worktree 再开始编辑；若已在错误 worktree 开工 -> 立即说明并切走`
  8. Flow-SCR-008: `cargo-dev.sh check/test/run -> 解析当前 repo family + worktree source identity 的 target namespace -> 导出稳定 CARGO_TARGET_DIR -> 以 env -u RUSTC_WRAPPER cargo 执行开发态命令`
  9. Flow-SCR-009: `local smoke / regression / drill script -> source cargo-dev-lib.sh -> 调用 oasis7_cargo_dev build/test/run -> 本地复用 shared target；CI 或显式 raw 环境回退原始 cargo target 语义`
  9. Flow-SCR-009: `worktree-gc-report.sh -> 扫描 git worktree + `.pm/github-project-sync/tasks.json` / GitHub issue status -> 标注 prunable / closed clean worktree cleanup 候选 -> 输出只读汇总与建议命令`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 脚本主入口 | 脚本名、参数、返回码、输出路径 | 执行并输出标准化结果 | `idle -> running -> success/failed` | 按命令类型分层执行 | 所有人可执行 |
| 参数契约 | 必填参数、默认值、失败语义 | 参数校验失败即阻断 | `validating -> accepted/rejected` | 必填项优先校验 | 维护者可更新契约 |
| fallback 规则 | 触发条件、替代脚本、产物要求 | 满足条件后才允许 fallback | `normal -> fallback -> diagnosed` | 常规链路优先 | 仅排障场景允许触发 |
| 标题品牌治理 | 标题前缀、适用专题、兼容命名说明 | 将脚本治理专题标题统一切到 `oasis7` | `legacy_title -> oasis7_title -> audited` | 先改治理主入口，再改周边专题 | owner 可改，治理门禁复核 |
| worktree-isolated harness | `worktree_id`、端口组、状态文件、bundle 根目录、artifact 根目录、browser session | 通过单一 harness 入口执行 `up/down/status/url/logs/smoke` | `idle -> booting -> ready -> verifying -> torn_down` | 先按 worktree 生成稳定身份，再为该 worktree 派生 bundle / port / output | `qa_engineer` 维护主入口，runtime/viewer 协同实现 |
| lifecycle hardening | `bootstrap_journal`、`slice_provenance`、`merge_hold`、`pr_gate`、`cleanup_preconditions` | journal resume、exact head/base PR recovery、full PR gate、non-force cleanup | `planned -> remote_partial/resume -> ready -> pr_watch/hold -> merge_ready -> merged -> done -> cleaned` | vacuous verify、self-attested review、active hold、partial comment scan、dirty cleanup 一律 fail closed | `repository_health_engineer` 维护，`qa_engineer` 验证 |
| `.pm` rebase conflict helper | `rebase_in_progress`、`summary.total_conflicted_paths`、`summary.retired_signal_conflicts`、`summary.generated_view_conflicts`、`summary.manual_conflicts`、`conflicts[].category`、`conflicts[].recommended_action`、`resolved_now` | 在 active rebase 中只读分类 `.pm/**` 未合并路径；不自动修复任何 `.pm/**` 路径 | `rebase_conflicted -> classified -> manual_resolution_pending` | `.pm/inbox/signals.jsonl` 已退休，命中时只提示删除退休文件或人工归档；`.pm/registry/tasks.yaml` 与 `.pm/roles/*/backlog/*.yaml` 只提示保留 `main` 删除并执行 `sync-views.sh` | `producer_system_designer` / `qa_engineer` 可读，scripts owner 维护入口 |
| PR review thread closeout | `pr_number`、`thread_id`、`is_resolved`、`is_outdated`、`path`、`line`、`latest_comment`、`review_decision`、`merge_state_status` | 通过统一入口读取当前 PR review threads，并在显式 resolve 时批量关闭指定 unresolved thread | `reported -> patched -> resolved -> rechecked` | 默认 PR 取当前 branch 关联 PR；`--resolve-all-unresolved` 只处理当前 unresolved thread；每次 resolve 后都必须回报最新 PR state | `producer_system_designer` 定流程，scripts owner 维护入口 |
- Acceptance Criteria:
  - AC-1: scripts PRD 明确脚本分类、入口、约束。
  - AC-2: scripts project 文档维护脚本治理任务。
  - AC-3: 与 `doc/scripts/precommit/pre-commit.prd.md`、`testing-manual.md` 口径一致。
  - AC-4: `run-viewer-web.sh`、`viewer-primary-web-entry-regression.sh` 与相关 freshness/browser automation 脚本被明确为当前 Viewer Web 主链路；`viewer-software-safe-*` 脚本只作为兼容/legacy regression 链路保留。
  - AC-5: `doc/scripts/**` 仍可读治理专题标题统一使用 `oasis7` 品牌；旧标题仅允许出现在正文历史上下文中。
  - AC-6: 当前 Viewer UI 与 Web bundle 分别使用 `npm --prefix crates/oasis7_viewer run test:ui`、`test:feedback-contract` 与 `build:viewer`；Bevy bridge 使用实际存在的 `pixel_world_bridge` Rust package，后端协议使用 `oasis7` 的定向测试。各入口的覆盖对象和验证边界以 `testing-manual.md` 为准，不能相互替代；历史 Rust Viewer package 命令只保留在历史证据中。
  - AC-7: scripts 模块不得继续维护已删除的 `capture-viewer-frame`、texture inspector、theme preview 等 Viewer 3D/视觉 QA 工具专题。
  - AC-8: `site/skills/oasis7.md`、`scripts/setup-provider-oasis7-runtime.sh` 与 `scripts/provider-parity-p0.sh` 关联的当前 `cargo run -p` 命令和入口路径必须写为 `oasis7` / `crates/oasis7*`；旧品牌包名与源码路径仅允许保留在兼容说明、历史证据或外部原文引用中。
  - AC-9: 新增 `scripts/worktree-harness.sh` 作为 worktree 级主入口，至少提供 `up/down/status/url/logs/smoke` 六个动作，并把当前 worktree 的运行状态写入稳定 `state.json`。
  - AC-10: `scripts/run-launcher-stack.sh` 必须支持把 `run-id`、`output-dir`、`meta-file` 与 ready payload 交给上层 harness 注入，避免上层通过 grep stdout 猜测 URL/日志路径。
  - AC-11: `scripts/run-producer-playtest.sh` 默认 bundle 根目录必须可按 worktree 隔离，不再强制复用全局 `output/release/game-launcher-producer-local`。
  - AC-12: 新增 `scripts/new-task-worktree.sh`，默认根据 `<module> <task>` 生成稳定分支名与 worktree 路径，并执行 `git worktree add`。
  - AC-13: `scripts/new-task-worktree.sh` 默认在源 worktree 脏时阻断，并给出显式 override；对已存在路径、已被其他 worktree 占用的分支和非法空 slug 提供清晰失败语义。
  - AC-14: `scripts/new-task-worktree.sh --json` 必须输出机器可读摘要，至少包含 `branch`、`worktree_path`、`module`、`task`、`base_ref` 与 `mode`。
  - AC-16: `scripts/new-task-worktree.sh --json --init-docs` 必须输出机器可读 `doc_checks`；加 `--with-harness` 时，stdout 仍保持单个 JSON 对象，并附带 `harness` 摘要字段。
  - AC-18C: 新增 `scripts/pr-review-thread-closeout.sh`，默认按当前 branch 关联的 PR 读取 review threads；`--unresolved-only` 仅返回 unresolved threads，`--resolve-thread <id>` 可重复，`--resolve-all-unresolved` 只在显式传入时执行批量 resolve。
  - AC-18D: `scripts/pr-review-thread-closeout.sh --help` 必须明确列出 `[pr-number]`、`--unresolved-only`、`--resolve-thread`、`--resolve-all-unresolved` 与 `--json`；`--json` 至少输出 `pr.number`、`pr.review_decision`、`pr.merge_state_status`、`summary.total_threads`、`summary.unresolved_threads`、`resolved_now.thread_ids` 与每个 thread 的 `id`、`is_resolved`、`is_outdated`、`path`、`line`、`latest_comment`。
  - AC-18I: 仓库必须提供轻量 Web/UI automation smoke `scripts/viewer-software-safe-step-regression-smoke.sh`；该脚本需在不启动完整 runtime 栈的前提下，通过临时 fixture 页面复用真 `agent-browser` 与 `scripts/viewer-software-safe-step-regression.sh`，并验证 `software-safe-step-summary.json` 与关键 state artifact 的最小契约。
  - AC-19: 当 source worktree 脏、source 分支未被任何 worktree 检出、base ref 不存在、或 `--create` 时 source 分支落后于 comparison ref，脚本必须阻断并给出修复建议。
  - AC-20: PR 合入后，正式流程文档与脚本输出必须明确该 task `worktree` / branch 需要被删除；cleanup 命令不得再被表述为“可选建议”。
  - AC-22: 上述正式文档必须统一列出“复用当前 worktree / 就在这里改 / 不要切新 worktree”为允许例外的显式表述，并明确“先写一版 / 先不要提交 / 顺手改一下”不构成复用授权；若已切错 worktree，必须立即切走。
  - AC-23: 新增 `scripts/cargo-dev.sh`，为本地开发态 `cargo check/test/run/build` 提供 worktree-scoped shared cache 入口，并默认使用 `env -u RUSTC_WRAPPER cargo ...`。
  - AC-23A: 新增 `scripts/cargo-dev-lib.sh`，为本地 smoke / playtest / prewarm / regression / drill / longrun 脚本提供 `oasis7_cargo_dev` 与 shared-target debug binary 解析 helper；默认本地复用 `cargo-dev.sh`，但在 `CI=1`、`OASIS7_CARGO_DEV_SHARED=0` 或 `OASIS7_FORCE_RAW_CARGO=1` 时回退到原始 cargo target 语义。
  - AC-24: `scripts/cargo-dev.sh --print-target-dir` 必须输出当前 worktree 内稳定目录；同一 worktree 重复调用输出一致，默认不同 worktree 输出不同 namespace，且可通过环境变量显式覆盖。
  - AC-25: 正式文档必须明确：`scripts/cargo-dev.sh` / `scripts/cargo-dev-lib.sh` 只服务开发态缓存复用，不适用于要求 `CARGO_TARGET_DIR` 为空的 deterministic wasm / release 构建链路，也不得替代 CI canonical required/full 验收命令。
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
  - `scripts/run-launcher-stack.sh`
  - `scripts/run-producer-playtest.sh`
  - `scripts/worktree-harness.sh`
  - `scripts/cargo-dev.sh`
  - `scripts/new-task-worktree.sh`
  - `scripts/viewer-software-safe-step-regression-smoke.sh`
  - `scripts/pr-review-thread-closeout.sh`
  - `scripts/worktree-gc-report.sh`
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
  - PR review thread closeout：resolve review thread 只代表线程被收口，不代表 PR 已 merge-ready；helper 必须继续单独回报 `reviewDecision`、`mergeStateStatus` 与剩余 unresolved thread 数，避免把“threads 全关掉”和“可以合并”混成同一状态。
  - `.pm` rebase conflict helper：helper 不自动修复任何 `.pm/**` 路径；若冲突来自 retired signal inbox、工作说明或按需 Issue mapping / archive、memory、stage 或其他 canonical 对象，脚本只能分类并提示删除/人工归档/人工处理，不得擅自重写真值。
  - task cleanup：已完成任务的 task `worktree` 若长期不删，会让后续搜索、branch 占用检查与本地磁盘占用持续失真；因此 cleanup 必须成为 PR 合入后的必做步骤。
  - worktree lifecycle report：缺失路径、prunable 记录、dirty worktree 与未绑定 工作说明或按需 Issue mapping 的 worktree 都必须 truthfully 报告，不允许脚本为了“看起来整洁”而隐式删除或跳过。
  - shared cargo dev cache：同一 repo family 的每个 worktree 必须映射到自己的 stable source-identity target namespace；同一 worktree 可复用该 namespace，默认不同 worktree 不得共享编译 artifacts。deterministic wasm / release 脚本若要求 `CARGO_TARGET_DIR` 为空，必须继续走原始 cargo 入口而不是 `cargo-dev.sh`。
- Non-Functional Requirements:
  - NFR-SCR-1: 核心脚本具备可读帮助信息与失败语义说明。
  - NFR-SCR-2: 主入口脚本在 Linux/macOS 环境可执行一致。
  - NFR-SCR-3: CI 脚本接口稳定，破坏性改动需预告与回归。
  - NFR-SCR-4: 脚本默认输出不得包含敏感信息。
  - NFR-SCR-5: fallback 流程必须可追溯到故障诊断记录。
  - NFR-SCR-6: worktree harness 的状态文件必须机器可读，允许 agent 直接拿到 URL、端口组、输出目录与 PID，而不依赖 stdout 文本解析。
  - NFR-SCR-7: 同一仓库下至少两份 worktree 可在默认配置下并行起栈，不因固定端口或全局 bundle 目录直接冲突。
  - NFR-SCR-10: task worktree GitHub PR 收口入口必须默认使用非交互、可审计的 preflight / create 策略；JSON 模式下 stdout 只能输出单个结构化对象。
  - NFR-SCR-11: 已完成 task 的 cleanup 语义必须清晰一致，不允许不同文档同时出现“建议删除”和“必须删除”两套口径；PR 合入后的本地同步/cleanup 与 local-only fallback cleanup 不得混成两套默认流程。
  - NFR-SCR-12: worktree 例外授权与错误 worktree 处置口径在 `AGENTS.md`、模块 PRD 与专题文档之间必须保持一致，不允许根规则更严、模块专题更松。
  - NFR-SCR-12A: worktree 生命周期报告的 JSON 字段必须稳定、只读且不混入人类说明文本，便于 agent 直接消费 cleanup 候选。
  - NFR-SCR-13: 开发态 shared cargo target 目录必须按 worktree source identity 稳定且默认落在工作区外部缓存位置，避免污染仓库源码树或让不同 worktree / repo family 相互踩缓存。
- Security & Privacy: 脚本不得在默认输出中泄漏密钥；涉及网络调用时需要显式参数与最小权限。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化脚本分层与主入口规范。
  - v1.1: 增加高频脚本的契约测试与参数回归。
  - v2.0: 建立脚本治理仪表（稳定性、复用率、故障恢复时间）。
- Technical Risks:
  - 风险-1: 历史脚本行为差异导致切换成本。
  - 风险-2: 入口过多导致文档与实际调用脱节。
  - 风险-3: 若 worktree harness 只包壳而不下沉到 `run-launcher-stack.sh` / `run-producer-playtest.sh` 契约层，后续上层脚本仍会靠 grep stdout 和全局目录工作，隔离性会继续失真。
  - 风险-4: 若 worktree 创建仍停留在口头规范而无标准脚本，团队会继续混用手工 branch/path 命名，导致多任务并行难以搜索、回收与审计。
  - 风险-5: 若 `--with-harness` 破坏 JSON/stdout 纯度，agent 侧自动化会从“稳定入口”退回“半结构化抓取”。
  - 风险-6: 若 GitHub PR 收口仍依赖手工 push / gh 序列，不同人会混用本地 landing、直接 push 与半手工 PR 路径，导致默认保护边界和 task worktree 回收时机失控。
  - 风险-7: 若 landing 成功后不强制 cleanup，仓库会持续累积“已完成但仍挂着”的 task worktree，弱化 branch 占用围栏和任务检索准确性。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-001 | TASK-SCRIPTS-001/002/005/009/011 | `test_tier_required` | 脚本分层与入口清单核验 | 日常开发链路稳定性 |
| PRD-SCRIPTS-002 | TASK-SCRIPTS-002/003/005/viewer-software-safe-step-regression-smoke | `test_tier_required` + `test_tier_full` | 参数契约与失败语义回归、`viewer-software-safe-step-regression-smoke.sh` fixture-driven browser smoke | CI 稳定性与故障定位效率 |
| PRD-SCRIPTS-003 | TASK-SCRIPTS-003/004/005/010 | `test_tier_required` | fallback 使用条件抽样检查 | 排障闭环和风险控制 |
| PRD-SCRIPTS-004 | TASK-SCRIPTS-014 | `test_tier_required` | `bash -n` + `--help` + 双实例并行 smoke + `state.json` / ready payload 检查 + 文档治理检查 | 多 worktree 并行执行稳定性与 agent 可驱动性 |
| PRD-SCRIPTS-005 | TASK-SCRIPTS-015/020 | `test_tier_required` | `bash -n` + `--help` + 真实 create/remove smoke + worktree 例外授权文案一致性检查 + 文档治理检查 | 多任务并行的 worktree/branch 命名一致性与启动成本 |
| PRD-SCRIPTS-006 | TASK-SCRIPTS-016/020 | `test_tier_required` | `--init-docs` / `--with-harness` 真机 create/remove smoke + 错误 worktree 处置文案一致性检查 + 文档治理检查 | 新任务从创建到文档/验证闭环的一跳成本 |
| PRD-SCRIPTS-008 | TASK-SCRIPTS-018/025 | `test_tier_required` | landing/cleanup 文案与脚本输出一致性检查、`worktree-gc-report.sh --json` 结构化字段检查 + 文档治理检查 | task worktree 生命周期收口与本地环境整洁度 |
| PRD-SCRIPTS-009 | TASK-SCRIPTS-021/022 | `test_tier_required` | `bash -n` + `--help` + `--print-target-dir` 跨 worktree 一致性检查 + `AGENTS.md`/scripts/testing 文档口径一致性检查 + 文档治理检查 | 多 worktree Rust 开发回归速度与 deterministic wasm/release 口径隔离 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SCR-001 | 主入口 + fallback 分层治理 | 全脚本平级使用 | 分层更利于稳定维护。 |
| DEC-SCR-002 | 参数契约显式化 | 依赖隐式约定 | 可减少 CI 误用与回归。 |
| DEC-SCR-003 | fallback 仅在受控场景启用 | 默认对所有场景开放 | 可避免过度依赖应急链路。 |
| DEC-SCR-004 | 用独立 `cargo-dev.sh` 包装开发态共享 `CARGO_TARGET_DIR`，而不把共享 target 设成仓库全局默认 | 直接把所有 cargo 流程切到同一个全局 `CARGO_TARGET_DIR` | 能让日常多 worktree 开发复用缓存，同时不破坏 deterministic wasm / release 脚本对空 `CARGO_TARGET_DIR` 的围栏。 |

开发脚本只提供 Git worktree、共享 Cargo 缓存、运行隔离和真实验证。普通 PR 使用 Git、gh、实际 CI 与评审，不依赖本地 PM 身份。资源清理保留用户资料、未提交、未推送及使用中资源。
