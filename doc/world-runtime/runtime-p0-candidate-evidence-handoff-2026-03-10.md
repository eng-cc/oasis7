# Role Handoff Brief

## Meta
- Handoff ID: `HO-RUNTIME-20260310-P0-001`
- Date: `2026-03-10`
- From Role: `producer_system_designer`
- To Role: `runtime_engineer`
- Related PRD-ID: `PRD-WORLD_RUNTIME-001/002/003`
- Related Task ID: `TASK-WORLD_RUNTIME-033`
- Priority: `P0`

## Goal
- 把 runtime P0 实测证据绑定到同一候选级 core go/no-go 记录，解除当前仅因 runtime 证据缺失导致的 `blocked`。

## Why Now
- `TASK-GAME-018` 已完成，playability / testing / game 三段证据链已经 ready。
- 当前 `doc/core/reviews/stage-closure-go-no-go-task-game-018-2026-03-10.md` 仍为 `blocked`，唯一未绑定的 P0 项是 runtime 核心边界验收实测证据。

## Inputs
- 代码 / 文档入口：`doc/core/reviews/stage-closure-go-no-go-task-game-018-2026-03-10.md`、`doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md`、`doc/world-runtime/templates/runtime-release-gate-metrics-template.md`、`doc/world-runtime/project.md`
- 已完成内容：runtime 边界清单、回归模板、门禁指标模板均已建好；game / playability / testing 证据已 ready
- 已知约束：不得虚构 `ready`；必须绑定真实 runtime 命令、日志、metrics/summary 路径
- 依赖前置项：需要从 `TASK-WORLD_RUNTIME-033` 或等价 runtime 样本中抽取可复用的候选级实测证据

## Expected Output
- 接收方交付物 1：runtime 候选级证据记录（可直接被 core go/no-go 引用）
- 接收方交付物 2：`doc/world-runtime/project.md` 回写当前候选证据路径与结论
- 需要回写的文档 / 日志：`doc/core/reviews/stage-closure-go-no-go-task-game-018-2026-03-10.md`、`doc/world-runtime/project.md`、`doc/devlog/README.md`

## Done Definition
- [ ] 绑定至少 1 组 runtime 实测证据到 core 记录
- [ ] 明确 runtime P0 当前是 `ready` / `not_ready` / `blocked`
- [ ] 补齐测试 / 验证证据

## Risks / Blockers
- 风险：`TASK-WORLD_RUNTIME-033` 当前更偏长期 footprint/GC/soak，若直接套用可能与当前候选不完全等价
- 阻断项：没有真实 runtime 指标或日志路径时，core 总评不得提升为 `go`
- 需要升级给谁：`producer_system_designer`

## Validation
- 建议测试层级：`test_tier_required`（必要时补 `test_tier_full`）
- 建议验证命令：沿用 runtime 定向回归 / metrics / soak 相关命令，并回填真实日志路径

## Notes
- 当前不是要求 runtime 新做一轮大范围功能实现，而是优先完成候选级证据绑定。
