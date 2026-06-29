# oasis7: 版本候选 go/no-go 裁决入口（2026-03-11）

- 对应设计文档: `doc/core/release-candidate-go-no-go-entry-2026-03-11.design.md`
- 对应项目管理文档: `doc/core/release-candidate-go-no-go-entry-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 当前版本级 readiness board 已达到 `ready`，但仓内仍缺一个正式的版本候选 go/no-go 裁决入口来回答“谁拍板、按什么表拍板、最终结论是什么、风险是否接受”。如果只停留在 readiness 状态，发布仍会回到口头确认。
- Proposed Solution: 在 core 建立版本候选 go/no-go 裁决入口，复用 readiness board 与既有 evidence bundle，产出正式评审记录、结论状态与后续角色交接，使版本级 `ready` 可以升级为正式 `go` / `conditional-go` / `no-go` 结论。
- Success Criteria:
  - SC-1: 版本级 go/no-go 记录明确引用同一候选的 readiness board 与核心证据路径。
  - SC-2: 评审记录明确 `P0/P1/P2`、最终结论、风险接受情况与后续动作。
  - SC-3: `producer_system_designer`、`qa_engineer`、`liveops_community` 的职责边界在同一入口里可见。
  - SC-4: core 主项目能够把该专题作为 readiness 完成后的下一步正式承接项。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要从 readiness 进入正式版本裁决，而不是无限停留在“准备好了”。
  - `qa_engineer`：需要知道证据是否足够支持 `go`，以及还有哪些风险只是跟踪项。
  - `liveops_community`：需要在正式裁决后接到统一口径，用于后续对外说明与事故回流。
- User Scenarios & Frequency:
  - 版本候选达到 `ready` 后：创建正式 go/no-go 裁决记录。
  - 风险接受或拒绝时：在同一记录中落最终结论与回滚条件。
  - 发布前复核：按固定入口而不是再读多份模块文档。
- User Stories:
  - PRD-CORE-GNG-001: As a `producer_system_designer`, I want a version-level go/no-go record, so that release approval is explicit and auditable.
  - PRD-CORE-GNG-002: As a `qa_engineer`, I want `P0/P1/P2` evidence and risks summarized in one page, so that I can review the release decision without oral补充.
  - PRD-CORE-GNG-003: As a `liveops_community`, I want the final decision, residual risks, and rollback note linked, so that external messaging stays aligned.
- Critical User Flows:
  1. `读取版本级 readiness board -> 确认全部 P0 已 ready -> 填写正式 go/no-go 记录`
  2. `汇总 P1 风险与接受条件 -> 给出 go / conditional-go / no-go -> 记录后续动作`
  3. `通过正式评审记录、.pm execution log 与 role review evidence 串起 producer / QA / LiveOps 责任边界 -> 保证发布口径可回流`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| version decision header | 候选 ID、日期、评审人、结论、来源 board | 创建正式版本裁决记录 | `draft -> reviewed -> decided` | 同一候选只保留一份主记录 | `producer_system_designer` 发起 |
| P0 evidence summary | 槽位、owner、证据路径、当前状态、阻断原因 | 从 readiness board 聚合并复核 | `ready -> go_input` / `blocked -> no_go_input` | 任一 P0 非 `ready` 不得给出 `go` | `qa_engineer` 复核 |
| P1/P2 risk note | 风险摘要、接受情况、缓解措施、回收点 | 记录非阻断项 | `tracked -> accepted/deferred` | P1 先于 P2 审议 | `producer_system_designer` 裁定 |
| role evidence linkage | 发起角色、接收角色、输入、输出、done | 将结论通过正式评审记录、`.pm` execution log 与 role review evidence 交接给 QA / LiveOps | `prepared -> reviewed -> acknowledged` | 先 QA 后 LiveOps | 发起方填写，接收方确认 |
- Acceptance Criteria:
  - AC-1: 产出版本候选 go/no-go 专题 PRD / Design / Project。
  - AC-2: 产出一份正式版本级 go/no-go 评审记录，并引用 readiness board。
  - AC-3: 评审记录明确最终结论、残余风险、回滚口径与后续角色责任边界。
  - AC-4: `doc/core/project.md` 能追踪该任务的完成与主项目状态。
- Non-Goals:
  - 不新增新的 runtime / gameplay / testing 实测样本。
  - 不修改 task 级 go/no-go 原始结论。
  - 不替代未来的线上运营公告或外部发布文案。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该入口位于 core，消费 `release-candidate-readiness-board-version` 与各模块 evidence bundle，输出唯一的版本级 go/no-go 裁决记录和角色交接入口。
- Integration Points:
  - `doc/core/reviews/release-candidate-readiness-board-version-2026-03-11.md`
  - `doc/core/templates/stage-closure-go-no-go-template.md`
  - `doc/testing/evidence/release-evidence-bundle-task-game-018-2026-03-10.md`
  - `doc/playability_test_result/evidence/playability-release-evidence-bundle-task-game-018-2026-03-10.md`
  - `doc/world-runtime/evidence/runtime-version-candidate-evidence-2026-03-11.md`
  - `doc/world-runtime/evidence/runtime-version-candidate-soak-evidence-2026-03-11.md`
- Edge Cases & Error Handling:
  - readiness board 为 `ready` 但证据路径失效：go/no-go 只能记 `blocked`，不得沿用旧结论。
  - P1 风险已知但无 owner：不得记为已接受风险。
  - `liveops_community` 尚未接收口径：可以先形成内部 `go`，但必须显式标记后续 handoff。
- Non-Functional Requirements:
  - NFR-GNG-1: 正式 go/no-go 记录单页可在 10 分钟内审完。
  - NFR-GNG-2: 每条 P0/P1/P2 项都必须可追溯到仓内证据或既有 review。
  - NFR-GNG-3: 版本候选达到 `ready` 后 1 个工作日内必须形成正式 go/no-go 记录或明确阻断原因。
- Security & Privacy: 仅聚合仓内证据、风险和角色交接信息，不引入额外敏感数据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`GNG-1`): 定义版本级 go/no-go 入口与正式评审记录。
  - v1.1 (`GNG-2`): 将 QA / LiveOps handoff 固化为标准后续动作。
  - v2.0 (`GNG-3`): 评估是否需要版本候选 registry / cadence 机制。
- Technical Risks:
  - 风险-1: 若只有 readiness 没有正式裁决，发布仍会退化成口头拍板。
  - 风险-2: 若不把 LiveOps 纳入后续口径链，外部说明会与内部证据脱节。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-GNG-001 | `TASK-CORE-022` | `test_tier_required` | 检查 go/no-go 专题与版本级评审记录存在且互链 | 版本候选正式裁决入口 |
| PRD-CORE-GNG-002 | `TASK-CORE-022` | `test_tier_required` | 检查 P0/P1/P2、最终结论、风险接受字段存在 | 发布评审可审计性 |
| PRD-CORE-GNG-003 | `TASK-CORE-022` | `test_tier_required` | 检查 QA / LiveOps 责任边界已在正式评审记录和 evidence 中落档 | 发布口径交接完整性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-GNG-001` | 在 version readiness 之后追加正式 go/no-go 入口 | 将 readiness board 直接视为最终放行记录 | readiness 解决“是否齐备”，go/no-go 解决“是否拍板”，两者职责不同。 |
| `DEC-GNG-002` | 继续复用既有模板与 evidence bundle | 重新发明一套 release 审批模板 | 保持口径连续、降低维护成本。 |
| `DEC-GNG-003` | QA 后续复核、LiveOps 口径回流都纳入同一任务 | 只做内部评审，不处理发布口径承接 | 角色交接必须在正式裁决时闭环。 |
