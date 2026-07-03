# oasis7: 版本级候选 readiness 扩展（2026-03-11）

- 对应设计文档: `doc/core/release-candidate-version-escalation-2026-03-11.design.md`
- 对应项目管理文档: `doc/core/release-candidate-version-escalation-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: `TASK-CORE-018` 已实例化 task 级候选看板，但当前缺口已经转移到版本级：runtime footprint / GC / soak 联合验证尚未进入统一槽位，导致看板只能停留在 task 级 `conditional`。
- Proposed Solution: 建立版本级候选 readiness 扩展专题，把 task 级看板提升为 version candidate，看板新增 runtime longrun / footprint / soak 槽位，并明确版本级总状态规则。
- Success Criteria:
  - SC-1: 版本级候选看板显式新增 runtime footprint / GC / soak 联合验证槽位。
  - SC-2: task 级 ready 项能被继承到版本级看板，不需重复定义字段。
  - SC-3: 版本级总状态与阻断原因明确可审计。
  - SC-4: core 主项目下一任务推进到“补齐 runtime 候选级联合证据”。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要知道从 task 级到版本级到底还差哪一步。
  - `runtime_engineer`：需要看到自己在版本级候选上的新增证据槽位。
  - `qa_engineer`：需要在同一入口里区分 inherited ready 与新增 blocked/watch 项。
- User Scenarios & Frequency:
  - task 级候选已通过后：提升为版本级候选。
  - runtime 长跑证据逐步补齐时：刷新版本级槽位状态。
  - 版本级 go/no-go 前：读取 version board 而不是 task board。
- User Stories:
  - PRD-CORE-VRC-001: As a `producer_system_designer`, I want a version-level board that inherits task-level ready slots, so that escalation does not restart from zero.
  - PRD-CORE-VRC-002: As a `runtime_engineer`, I want dedicated longrun/footprint/GC slots, so that the remaining release blocker is visible.
  - PRD-CORE-VRC-003: As a `qa_engineer`, I want explicit inherited vs new blocker distinction, so that review focus stays sharp.
- Critical User Flows:
  1. `读取 task 级 board -> 继承已 ready 槽位 -> 新增版本级 runtime 联合验证槽位`
  2. `新增版本级槽位状态 -> 聚合 inherited + new slots -> 输出版本级 conditional/blocked`
  3. `将下一任务推进到 runtime 候选级联合证据补齐`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| inherited slot | 槽位名、来源 board、当前状态 | 从 task board 继承 | `inherited_ready -> confirmed` | 继承项排在新增项前 | core owner 维护 |
| version runtime slot | `footprint`、`GC`、`soak`、owner、状态、路径、阻断 | 版本级新增槽位 | `missing -> watch/blocked/ready` | runtime 三项任一 blocked 即总体不可 ready | `runtime_engineer` 更新 |
| version summary | inherited_ready_count、新增 blocked/watch 数、总状态 | 聚合版本级 board | `unknown -> conditional/blocked/ready` | 先看 P0 blocked，再看新增 watch | `producer_system_designer` 裁定 |
- Acceptance Criteria:
  - AC-1: 新专题定义 inherited slot 与版本级 runtime 槽位。
  - AC-2: 产出首份版本级候选看板。
  - AC-3: 明确当前总状态与仍需补齐的 runtime 候选级联合证据。
  - AC-4: `doc/core/project.md` 下一任务推进到 runtime 联合证据补齐。
- Non-Goals:
  - 不在本任务实际生成新的 soak 运行结果。
  - 不修改既有 task board 结论。
  - 不实现自动汇总脚本。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 版本级 board 复用 task board 作为已确认输入，并新增 runtime 长跑三槽位；最终由 core 统一聚合成版本级总状态。
- Integration Points:
  - `doc/core/reviews/release-candidate-readiness-board-task-game-018-2026-03-11.md`
  - `doc/world-runtime/evidence/runtime-release-gate-metrics-task-game-018-2026-03-10.md`
  - `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
  - `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`
- Edge Cases & Error Handling:
  - runtime 只补齐 footprint 但无 soak：版本级可记 `blocked` 或 `watch`，不得直接 `ready`。
  - 继承项与版本级新项结论冲突：以版本级更严格结论为准，并在备注注明来源。
  - 证据只有专题定义没有实例输出：槽位只能记 `watch`，不得记 `ready`。
- Non-Functional Requirements:
  - NFR-VRC-1: 版本级 board 必须在单页内清晰区分 inherited 与新增项。
  - NFR-VRC-2: 所有 blocked/watch 项必须有下一动作。
  - NFR-VRC-3: 版本级 board 可被 grep 快速检索到 runtime 长跑三槽位。
- Security & Privacy: 仅聚合仓内证据路径与状态。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`VRC-1`): 建立版本级候选扩展专题与首份 board。
  - v1.1 (`VRC-2`): 补齐 runtime footprint / GC / soak 联合证据。
  - v2.0 (`VRC-3`): 若候选稳定，升级为正式版本 go/no-go 入口。
- Technical Risks:
  - 风险-1: 若版本级 board 过早宣称 ready，会掩盖 runtime 长跑缺口。
  - 风险-2: 若不明确 inherited 与新增项，评审会误以为所有证据都需重做。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-VRC-001 | `TASK-CORE-019` | `test_tier_required` | 检查 inherited slot 与版本级 runtime 槽位存在 | 版本级候选入口一致性 |
| PRD-CORE-VRC-002 | `TASK-CORE-019` | `test_tier_required` | 检查 footprint / GC / soak 三槽位与下一动作存在 | runtime 候选缺口可见性 |
| PRD-CORE-VRC-003 | `TASK-CORE-019/020` | `test_tier_required` | 检查下一任务已推进到 runtime 联合证据补齐 | 版本级执行主线一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-VRC-001` | 继承 task 级 board 已 ready 槽位 | 版本级从空白开始重建 | 减少重复维护与认知噪音。 |
| `DEC-VRC-002` | 版本级新增 runtime footprint / GC / soak 三槽位 | 把三项混成一个 runtime 备注 | 细粒度槽位更利于明确缺口与责任。 |
| `DEC-VRC-003` | 当前版本级状态保持 `conditional` | 在无联合长跑证据时标记 `ready` | 保持发布口径保守与可审计。 |
