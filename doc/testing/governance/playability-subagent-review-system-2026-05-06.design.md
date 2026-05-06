# oasis7: 好玩性 subagent 评审系统（2026-05-06）设计

- 对应需求文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
- 对应项目管理文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.project.md`

审计轮次: 1

## 1. 设计定位
把“可以用多角色 subagent 补齐内部人工评审”从治理原则推进到可执行系统设计，提供标准角色 subagent 清单、统一输入包、统一输出卡和调度顺序。

## 2. 系统结构
- Orchestrator:
  - `producer_system_designer` 负责总装与最终内部结论。
- Gatekeeper:
  - `qa_engineer` 负责阻断、证据质量和必跑验证。
- Surface reviewers:
  - `viewer_engineer`
  - `agent_engineer`
  - `runtime_engineer`
  - `wasm_platform_engineer`
- Claim reviewer:
  - `liveops_community`

## 3. 数据契约
- Input:
  - `review packet`
- Output:
  - `role review card`
- Aggregation:
  - `final internal playability review summary`

## 4. 调度策略
- 先做 packet 完整性检查。
- 再按 trigger matrix 并行拉起 surface reviewers。
- 最后由 `qa_engineer` 与 `producer_system_designer` 收口，并在需要时交 `liveops_community` 做对外边界复核。

## 5. 约束
- 不新增非标准正式角色。
- 不让任一角色 subagent 越权替代其它角色的最终职责。
- 不让内部 review 直接替代 L5 真实外部验证。
