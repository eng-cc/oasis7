# scripts 文档索引

审计轮次: 10

## 从这里开始
- 想先理解脚本模块的边界、门禁与维护口径：`doc/scripts/prd.md`
- 想看当前脚本治理任务与最近完成项：`doc/scripts/project.md`
- 想按专题文件名精确查 precommit / wasm / governance 文档：`doc/scripts/prd.index.md`
- 想直接为新需求开独立 worktree：`scripts/new-task-worktree.sh` + `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`
- 想把 task 的 `.pm` close-phase 连同 fresh verification 一步收口：`scripts/pm/task-closeout.sh` + `.pm/README.md`
- 想本地验证默认 workflow 主链是否仍可回放：`scripts/pm/workflow-behavior-eval.sh`
- 想处理当前 PR 的 review comments / thread resolve：`scripts/pr-review-thread-closeout.sh`
- 想把已完成任务标准化通过 GitHub PR 合入 `main`，并先看 changed-path 对齐的本地 required 建议与 planner 原因摘要：`scripts/prepare-task-pr.sh` + `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.prd.md`
- 想在 rebase 时先看 `.pm/**` 冲突哪些能自动修、哪些必须手工处理：`scripts/pm/rebase-conflict-helper.sh` + `.pm/README.md`
- 想盘点哪些 task worktree 已可回收、哪些 worktree 仍有重型 `target` / `node_modules` 缓存：`scripts/worktree-gc-report.sh --footprint`
- 想在不启动完整 runtime 栈的前提下先快查 Web/UI automation 链路：`scripts/viewer-software-safe-step-regression-smoke.sh`
- 想启动本地真实 LetAI bridge + runtime/game 测试链路：`scripts/run-local-letai-game-test.sh`
- 想预热隔离 harness 或理解 worktree 栈约束：`doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`
- 想让多个 worktree 复用 Rust 开发态编译缓存：默认用 `scripts/cargo-dev.sh`

## 入口
- PRD: `doc/scripts/prd.md`
- 设计总览: `doc/scripts/design.md`
- 标准执行入口: `doc/scripts/project.md`
- 文件级索引: `doc/scripts/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者决定是先看规则、看项目状态，还是直接去某个高频脚本入口。
- `prd.md` 是脚本模块的权威规格入口，适合先理解主入口分层、参数契约、稳定性趋势与隔离约束。
- `project.md` 是执行台账，适合确认当前 worktree、landing、harness 等治理任务的完成状态。
- `prd.index.md` 是精确检索索引，适合已知专题名后按文件名直达，不适合作为第一次进入 scripts 模块时的首读入口。
- 高频脚本与治理专题承担主题真值：`new-task-worktree.sh` 负责新任务 bootstrap，`scripts/pm/task-closeout.sh` 负责带 fresh verification 的 `.pm` close-phase 收口，`scripts/pm/workflow-behavior-eval.sh` 负责默认 workflow 主链 eval，`scripts/pr-review-thread-closeout.sh` 负责 same-PR review thread 盘点与 resolve，`prepare-task-pr.sh` 负责默认 GitHub PR 收口，`scripts/pm/rebase-conflict-helper.sh` 负责 `.pm` rebase 冲突分类，`worktree-gc-report.sh` 负责 worktree 生命周期盘点，`land-task-worktree.sh` 只保留给 local-only / fallback，`worktree-isolated-harness` 负责隔离栈与状态文件约束。

## 模块职责
- 维护仓内高频脚本的主入口、参数契约与 fallback 围栏口径。
- 维护 worktree 级隔离 harness，让 agent / QA 能并行起栈并读取稳定状态文件。
- 维护标准化 task worktree bootstrap 与 GitHub PR 收口入口，让每个新需求按统一 branch/path 命名落到独立 worktree，并可选直接检查模块文档、预热 harness、标准化通过 PR 合入 `main`。
- 维护 repo-family 共享的 cargo 开发态缓存入口，减少多 worktree 并行时的重复编译。
- 维护本地真实 LetAI provider bridge + runtime/game 测试入口，避免把底层 launcher/bootstrap 脚本误当作人工试玩主入口。
- 汇总 precommit、wasm 与治理专题文档。
- 承接脚本稳定性趋势、文档门禁与运行约束收口。

## 主题文档
- `precommit/`：提交前检查与门禁策略。
- `wasm/`：WASM 构建脚本与环境约束。
- `governance/`：脚本分层、参数契约、稳定性趋势、worktree harness 与 task worktree bootstrap 专题。

## 高频专题
- 脚本治理基线：`doc/scripts/governance/script-entry-layering-2026-03-11.prd.md`、`doc/scripts/governance/script-parameter-contracts-2026-03-11.prd.md`、`doc/scripts/governance/script-stability-trend-tracking-2026-03-11.prd.md`
- task worktree / PR 收口：`doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`、`doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.prd.md`、`doc/scripts/governance/task-worktree-landing-2026-03-27.prd.md`
- 隔离栈：`doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`
- 其他高频入口：`doc/scripts/precommit/pre-commit.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `precommit/`、`viewer-tools/`、`wasm/`、`governance/`。

## 维护约定
- 脚本行为变化需同步更新对应文档、测试口径与参数契约说明。
- 新增专题后，需同步回写 `doc/scripts/prd.index.md` 与本目录索引。
- `scripts/new-task-worktree.sh` 为新需求默认入口；`--init-docs` 用于检查模块 PRD / project，启用 PM bootstrap 时任务证据写入对应 GitHub task issue evidence comments；`--with-harness` 用于在新 worktree 中后台预热 `./scripts/worktree-harness.sh up`。
- `scripts/pm/task-closeout.sh` 为默认 close-phase helper；负责在 task 已 start 且 execution log 已回写后，若收口到 `done` 则先跑 fresh verification，再执行 `workflow-report close -> move-task done|deferred -> pm lint`，但不替代 commit 或 `prepare-task-pr.sh`。
- `scripts/pm/workflow-behavior-eval.sh` 为默认 workflow behavior eval 入口；负责把 task-worktree bootstrap、可选/必需 routing scenarios、subagent contract surface、PM closeout/claim gate、PR preflight 与 review-thread closeout 串成一条可重复的本地验证链。
- `scripts/pr-review-thread-closeout.sh` 为当前 PR 的 review-thread closeout helper；默认只读盘点 review threads，显式传 `--resolve-thread` 或 `--resolve-all-unresolved` 时才会调用 GitHub resolve mutation，且每次都会重新回报 `reviewDecision` / `mergeStateStatus`。
- `scripts/prepare-task-pr.sh` 为任务完成后的默认 GitHub PR 收口入口；负责在干净 task worktree 上执行 PR preflight / create，并基于 changed-path planner 输出本地 required 验证建议、planner 原因摘要，以及 PR 合入后的本地同步与回收命令。
- `scripts/pm/rebase-conflict-helper.sh` 为 `.pm` rebase 冲突辅助入口；默认只读分类 `.pm/**` 未合并路径。`.pm/inbox/signals.jsonl` 已退休，历史冲突只提示删除/人工归档，不再自动修复。若冲突命中 `.pm/registry/tasks.yaml` 或 role backlog 这类 git-ignored 本地视图，应保留 `main` 删除，再执行 `./scripts/pm/sync-views.sh`。
- `scripts/worktree-gc-report.sh` 为 worktree 生命周期盘点入口；默认只读汇总 prunable worktree、已 closed `.pm` task 对应的 clean worktree 与建议 cleanup 命令，不自动删除任何 worktree/branch；加 `--footprint` 时会额外统计每个 worktree 的 `target` 与 `crates/oasis7_viewer/node_modules` 体量，用于优先清理高成本缓存。
- `scripts/viewer-software-safe-step-regression-smoke.sh` 为轻量 Web/UI automation smoke；通过临时 fixture 页面复用真 `agent-browser` 和 `viewer-software-safe-step-regression.sh`，在不启动完整 runtime/build 的前提下先验证浏览器自动化链路与 summary/state 产物契约。
- `scripts/run-local-letai-game-test.sh` 为完全本地的真实 LetAI provider bridge + runtime/game 测试主入口；它不连接 formal/public testnet。日常人工试玩使用 `./scripts/run-local-letai-game-test.sh --local-world-playtest`，不要再手工拼一长串端口、detach、reuse、smoke 与 chain 参数。该 preset 固化 `playtest` startup profile、跳过 provider smoke、复用已有 source build、后台启动、手动推进、标准端口 `48420/48421/48422`、`--json-ready`，并保留 wrapper 默认的 local standalone chain。脚本同时负责 token config 规范化、默认 Rust direct provider bridge、Rust bridge chat probe/provider contract smoke 与 launcher stack 串联。Python `check-letai-chat-completions.sh` 只保留作 legacy adapter 兼容诊断。直接跑 `run-launcher-stack.sh` 只适合作为底层 bootstrap、wrapper 内部调用或定向回归，不应再作为本地真实 LLM 游戏测试的人工入口。默认 launcher stack 会启用 chain runtime，并透传 `--chain-local-standalone-test`，确保本地 submit -> commit -> snapshot 闭环可在单节点试玩栈内完成；若绕过该 wrapper 手动启动 lower stack，必须显式带上同等 chain-local standalone 配置，否则 gameplay action 可能停在 pending consensus queue，页面会显示“认领已提交，正在等待链上 committed 快照同步”。chain-enabled ready 还要求 `output/chain-runtime/<node-id>/reward-runtime-execution-world/snapshot.json` 与 `journal.json` 出现；若该 gate 超时，浏览器可能只表现为 `viewer.ws` / WebSocket 断线。透传 `--chain-disable` 只能作为本地页面试玩或排查 mitigation，不能作为本地 standalone chain-enabled 世界通过证据，更不能作为 public_testnet 证据。
- `scripts/land-task-worktree.sh` 仅保留给用户显式要求的 local-only / fallback 场景，不再是默认最终合流入口。
- `scripts/cargo-dev.sh` 是本地开发态 `cargo check/test/run/build` 的默认 shared target 入口；`scripts/cargo-dev-lib.sh` 是本地 smoke / playtest / prewarm / regression / drill / longrun 脚本复用同一 target 的 shell helper。要求 deterministic wasm / release / CI canonical 验收链路继续走原始 cargo 入口，保持对应 `CARGO_TARGET_DIR` 边界。
- 若默认高频脚本入口变化，需同步回写本目录“从这里开始”，避免 README 退化回纯专题目录页。
