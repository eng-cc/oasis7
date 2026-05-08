# oasis7: worktree-isolated harness 主入口（2026-03-27）

- 对应设计文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.design.md`
- 对应项目管理文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 `run-game-test.sh` 与 `run-producer-playtest.sh` 仍默认使用固定端口、时间戳散落产物目录与全局 bundle 路径，agent 若在多个 git worktree 中并行起栈，会互相争抢端口、复用旧 bundle、污染浏览器 session，并迫使上层脚本通过 grep stdout 猜测 URL/日志路径。
- Proposed Solution: 新增 `scripts/worktree-harness.sh` 作为 worktree 级主入口，为当前 git worktree 派生稳定 `worktree_id`、端口组、bundle 根目录、artifact 根目录与浏览器 session，并将这些状态落盘到 `output/harness/<worktree_id>/state.json`。同时扩展 `run-game-test.sh` / `run-producer-playtest.sh` 契约，允许上层显式注入 `run-id`、`output-dir`、`meta-file` 与 ready payload。
- Success Criteria:
  - SC-1: 每个 worktree 都能通过统一入口执行 `up/down/status/url/logs/smoke`。
  - SC-2: 不同 worktree 默认不会因固定端口或共享 bundle 目录互相冲突。
  - SC-3: `state.json` 提供机器可读的 URL、端口组、PID、bundle 与 artifact 路径。
  - SC-4: `run-game-test.sh` 不再要求上层通过 grep stdout 才能获取 ready 信息。
  - SC-5: `run-producer-playtest.sh` 默认 bundle 目录可按 worktree 隔离。

## 2. User Experience & Functionality
- User Personas:
  - `qa_engineer`: 需要一条命令起当前 worktree 的 Web 验证栈并留下稳定证据目录。
  - `viewer_engineer`: 需要并行比较多个分支/修复 worktree 的 UI 行为而不抢端口。
  - agent 执行者: 需要从仓库内直接获取当前 worktree 的 URL、日志与状态，而不是依赖终端人工摘取。
- User Scenarios & Frequency:
  - 多 worktree 并行回归：高频。
  - PR / 分支修复后的局部 smoke：高频。
  - 制作人或 QA 在 bundle-first 路径上复核当前 worktree 构建：中频。
- User Stories:
  - PRD-SCRIPTS-HARNESS-001: As a `qa_engineer`, I want one harness command per worktree, so that parallel validation runs do not collide.
  - PRD-SCRIPTS-HARNESS-002: As an agent executor, I want machine-readable harness state, so that I can drive the current stack without scraping human-oriented logs.
  - PRD-SCRIPTS-HARNESS-003: As a `viewer_engineer`, I want producer/bundle playtest paths to inherit worktree isolation, so that manual and automated checks share one isolated substrate.
- Critical User Flows:
  1. `git worktree 中执行 worktree-harness.sh up -> 生成 worktree_id / ports / state.json -> 拉起 launcher stack`
  2. `agent 读取 state.json -> 获取 viewer_url / logs / bundle_dir -> 驱动 agent-browser smoke`
  3. `worktree-harness.sh down -> 关闭 launcher pid / 浏览器 session -> 保留 artifacts`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| worktree 身份 | `worktree_id`、`worktree_path`、`git_head` | 进入 harness 时自动解析 | `unknown -> identified` | 基于当前 git worktree 路径生成稳定短 id | 当前 worktree 执行者可读写 |
| 端口组分配 | `viewer_port`、`web_bind`、`live_bind`、`chain_status_bind` | `up` 时一次性分配并回写状态 | `unassigned -> reserved -> released` | 优先 loopback；单次写入同一状态文件 | harness owner 可维护 |
| 隔离目录 | `bundle_dir`、`runtime_dir`、`artifacts_dir`、`browser_dir` | `up` 时创建，`down` 时保留证据 | `missing -> provisioned -> archived` | 固定在 `output/harness/<worktree_id>/` 下 | 执行者可读 |
| ready payload | `viewer_url`、`launcher_pid`、`launch_mode`、`meta_file` | 底层 stack ready 后写入 JSON | `booting -> ready` | 上层通过 JSON 读取，不依赖 grep stdout | agent / QA 可读 |
| smoke 入口 | `smoke_timeout`、`session_name`、`open_headed` | 复用当前 stack 执行最小 smoke | `ready -> verifying -> pass/fail` | 默认优先复用 formal gameplay 的 active LLM stack；`--no-llm` 只保留给 direct viewer diagnostics | QA / agent 可触发 |
- Acceptance Criteria:
  - AC-1: `scripts/worktree-harness.sh --help` 明确列出 `up/down/status/url/logs/smoke`。
  - AC-2: `scripts/run-game-test.sh` 支持 `--output-dir`、`--run-id`、`--meta-file` 与 `--json-ready`。
  - AC-3: `scripts/run-producer-playtest.sh` 支持按 worktree 指定 bundle 根目录与启动日志路径。
  - AC-4: `state.json` 至少包含 `worktree_id`、`viewer_url`、`launcher_pid`、`viewer_port`、`web_bind`、`bundle_dir`、`artifact_dir`。
  - AC-5: 并行 worktree 运行时，默认端口与 bundle 根目录不共享。
- Non-Goals:
  - 不在本轮引入完整 LogQL/PromQL/TraceQL 观测栈。
  - 不重写 `agent-browser` CLI。
  - 不改动业务 runtime / Viewer 功能语义。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `git`、`python3`、`agent-browser`（用于 `smoke`）、现有 `run-game-test.sh` / `run-producer-playtest.sh`。
- Evaluation Strategy: 通过双实例并行成功率、状态文件完整率、URL/日志自动发现成功率评估 harness 质量。

## 4. Technical Specifications
- Architecture Overview: `worktree-harness.sh` 作为 worktree 级包装器，负责解析当前 git worktree 身份、准备隔离目录、分配端口、调用 `run-game-test.sh` / `run-producer-playtest.sh`，并将最终状态汇总到稳定 `state.json`。底层启动逻辑仍由既有脚本负责。
- Integration Points:
  - `scripts/worktree-harness.sh`
  - `scripts/worktree-harness-lib.sh`
  - `scripts/run-game-test.sh`
  - `scripts/run-producer-playtest.sh`
  - `testing-manual.md`
  - `doc/scripts/project.md`
- Edge Cases & Error Handling:
  - 当前目录不是 git worktree：立即失败并提示需要在仓库 worktree 内执行。
  - 端口探测失败：阻断 `up`，不得回退到共享默认端口。
  - 上次状态文件残留但 PID 已失效：允许复用目录，但必须先清理失效状态。
  - bundle 不 fresh：继续沿用现有 freshness gate；producer path 允许自动重建。
  - `down` 时子进程已退出：返回成功并清理浏览器 session / 状态文件中的运行态字段。
- Non-Functional Requirements:
  - NFR-WTH-1: 状态文件必须机器可读且字段命名稳定。
  - NFR-WTH-2: `up` 默认只绑定 loopback，不扩大网络暴露面。
  - NFR-WTH-3: 至少两份 worktree 可在默认配置下并行 smoke。
- Security & Privacy: 状态文件默认不记录密钥，只记录端口、路径、PID 与 URL。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (WTH-1): 新增 harness 主入口与状态文件。
  - v1.1 (WTH-2): 将更多 Web/launcher 回归脚本切到消费 harness `state.json`。
  - v2.0 (WTH-3): 为每个 worktree 叠加临时观测侧车与更强 agent 自治回路。
- Technical Risks:
  - 风险-1: 若端口预占逻辑不稳，仍可能出现竞态冲突。
  - 风险-2: 若 `run-game-test.sh` 继续保留过多 stdout 约定，上层仍会偷偷回到 grep 驱动。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-HARNESS-001 | WTH-1/2 | `test_tier_required` | `up/down/status/url` 命令 smoke + 双实例并行抽样 | 多 worktree 并行起栈稳定性 |
| PRD-SCRIPTS-HARNESS-002 | WTH-1/2 | `test_tier_required` | `state.json` / ready payload 字段核验 | agent 自动发现 URL / 日志能力 |
| PRD-SCRIPTS-HARNESS-003 | WTH-2/3 | `test_tier_required` | producer/bundle path 隔离目录与 help/契约检查 | 手工与自动化路径一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WTH-001 | 新增 worktree 级主入口，并下沉到底层脚本契约 | 仅新增外层 wrapper，不改底层脚本接口 | 不改底层契约就无法消除 grep stdout 与共享目录依赖。 |
| DEC-WTH-002 | 状态文件固定写到 `output/harness/<worktree_id>/state.json` | 继续沿用时间戳散点输出 + 人工 grep | 稳定路径更利于 agent 和后续脚本消费。 |
| DEC-WTH-003 | bundle-first / producer path 也继承 worktree 隔离 | 仅源码模式 worktree 化 | 手工验收与自动化若隔离模型不同，会继续产生假通过与脏状态。 |
