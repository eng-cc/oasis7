# oasis7: worktree-isolated harness 主入口（2026-03-27）设计

- 对应需求文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`
- 对应项目管理文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.project.md`

审计轮次: 1

## 1. 设计定位
把 Viewer Web / launcher 起栈流程升级为“以 git worktree 为边界的隔离运行单元”，为后续 agent-first 回归与并行执行提供稳定入口。

## 2. 设计结构
- 身份层：解析当前 `git worktree` 并生成稳定 `worktree_id`。
- 资源层：为该 worktree 分配独立端口组、bundle 根目录、runtime/artifact/browser 目录。
- 执行层：通过 `worktree-harness.sh up/down/status/url/logs/smoke` 驱动底层脚本。
- 契约层：`run-launcher-stack.sh` 对外暴露 `--output-dir`、`--run-id`、`--meta-file`、`--json-ready`，`run-producer-playtest.sh` 接受 worktree 级 bundle / log 路径。
- 状态层：统一写 `output/harness/<worktree_id>/state.json`，供 agent 与其他脚本消费。

## 3. 关键接口 / 入口
- 主入口：`scripts/worktree-harness.sh`
- 状态库：`scripts/worktree-harness-lib.sh`
- 底层启动器：`scripts/run-launcher-stack.sh`
- producer 入口：`scripts/run-producer-playtest.sh`
- 测试手册入口：`testing-manual.md`

## 4. 约束与边界
- 继续沿用既有 bundle freshness gate。
- 仅做 worktree 级隔离和 ready/state 结构化，不引入新的业务逻辑。
- 默认只使用 loopback 端口，不扩大暴露面。
- `smoke` 只覆盖最小 stack readiness + 页面最小探活，不等于 release gate。

## 5. 设计演进计划
- 先把 harness 主入口和状态文件落地。
- 再让更多 Web 回归脚本改为消费 `state.json`。
- 最后再叠加观测侧车与更丰富的 agent 自治能力。
