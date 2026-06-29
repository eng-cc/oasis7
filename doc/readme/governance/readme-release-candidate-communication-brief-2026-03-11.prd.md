# oasis7: 版本候选对外口径简报（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 内部版本候选已经形成正式 `go` 结论，但目前缺一份面向社区/外部沟通的统一口径简报，容易导致对外说明与内部证据链、风险边界和回滚口径脱节。
- Proposed Solution: 在 readme/governance 建立版本候选对外口径简报，统一说明“当前能说什么、不能说什么、已知风险如何表述、如有回退如何说明”，并明确 `liveops_community` 的承接输出。
- Success Criteria:
  - SC-1: 简报明确引用内部版本级 go/no-go 记录与候选 ID。
  - SC-2: 简报包含可对外复用的状态摘要、风险说明、禁用表述与回滚口径。
  - SC-3: `liveops_community` 与 `producer_system_designer` 的交接边界在同一主题下可追溯。
  - SC-4: readme 模块能把该专题作为“对外口径主控层”的增量能力承接。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要快速拿到不会越界的外部沟通草案。
  - `producer_system_designer`：需要确保对外承诺不超出内部证据与版本目标。
  - 外部评审者 / 社区观察者：需要简洁、稳定、不夸大的状态说明。
- User Scenarios & Frequency:
  - 内部 `go` 形成后：生成对外口径简报。
  - 对外说明前：复核禁用表述、残余风险与回滚说明。
  - 版本风险变化后：增量更新简报而不是重写 README 全文。
- User Stories:
  - PRD-README-COMM-001: As a `liveops_community`, I want a communication brief anchored to internal release evidence, so that external messaging stays accurate.
  - PRD-README-COMM-002: As a `producer_system_designer`, I want forbidden claims and risk boundaries stated, so that public promises do not exceed current evidence.
  - PRD-README-COMM-003: As an external reviewer, I want a concise status summary and rollback note, so that I can understand the current release posture without reading all internal docs.
- Critical User Flows:
  1. `读取内部 go/no-go -> 提取可公开状态摘要 -> 生成对外简报`
  2. `校验禁用表述与残余风险 -> 确认不会超出内部证据`
  3. `通过 liveops -> producer 交接冻结当前版本口径`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| external status brief | 候选 ID、当前状态、适用范围、证据来源 | 生成对外摘要 | `draft -> reviewed -> approved` | 先写事实，再写限制，再写后续动作 | `liveops_community` 起草，`producer_system_designer` 审核 |
| forbidden claims | 禁止用语、原因、替代表述 | 约束外部表述 | `defined -> adopted` | 高风险承诺优先列出 | `producer_system_designer` 拍板 |
| rollback note | 触发条件、对外说明模板、升级入口 | 供异常时直接复用 | `prepared -> referenced` | 与内部 go/no-go 回滚口径一致 | `liveops_community` 维护 |
| role evidence record | 发起角色、接收角色、输入、输出、done | 通过正式评审记录、readme 专题与 `.pm` execution log 固化口径承接链 | `sent -> reviewed -> closed` | owner 结论优先 | 发起方填写，接收方确认 |
- Acceptance Criteria:
  - AC-1: 产出简报专题 PRD / Design / Project。
  - AC-2: 产出一份可直接引用的版本候选对外口径简报。
  - AC-3: 明确禁用表述、残余风险、回滚口径与升级入口。
  - AC-4: `doc/readme/project.md` 能追踪该任务与 owner 边界。
- Non-Goals:
  - 不直接改写根 `README.md` 大段正文。
  - 不替代正式外部公告或 changelog。
  - 不虚构玩家反馈或线上事故数据。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题位于 readme/governance，消费 core 版本级 go/no-go 记录，输出一份面向 `liveops_community` 的外部沟通简报，并通过正式评审记录、readme 专题与 `.pm` execution log 回流 `producer_system_designer` 审核。
- Integration Points:
  - `doc/core/reviews/release-candidate-go-no-go-version-2026-03-11.md`
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.md`
  - `doc/readme/prd.md`
  - `README.md`
- Edge Cases & Error Handling:
  - 内部结论变化：简报必须回写版本号或日期，旧版不得继续复用。
  - 对外提问超出简报范围：统一回退到“当前只确认仓内候选级证据，不做超范围承诺”。
  - 回滚触发：必须优先引用内部记录中的回滚条件，不得临时发明新口径。
- Non-Functional Requirements:
  - NFR-COMM-1: 单页简报可在 5 分钟内完成阅读与复述。
  - NFR-COMM-2: 所有对外状态表述都必须能回链到内部 go/no-go 或 readiness 文档。
  - NFR-COMM-3: 禁用表述必须至少覆盖“已正式发布”“长期稳定已验证”“所有风险已清零”三类高风险承诺。
- Security & Privacy: 对外口径不得暴露内部运行目录、敏感配置或非必要工程细节。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`COMM-1`): 形成首份版本候选对外口径简报。
  - v1.1 (`COMM-2`): 与后续 changelog / announcement 模板联动。
  - v2.0 (`COMM-3`): 若发布节奏稳定，抽象为固定 release communication 模板。
- Technical Risks:
  - 风险-1: 若禁用表述不明确，外部承诺容易超前于内部证据。
  - 风险-2: 若回滚口径未预先准备，异常时沟通会失真。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-COMM-001 | `TASK-README-006` | `test_tier_required` | 检查简报与内部 go/no-go 记录互链 | 对外口径一致性 |
| PRD-README-COMM-002 | `TASK-README-006` | `test_tier_required` | 检查禁用表述、风险边界、替代表述存在 | 对外承诺边界控制 |
| PRD-README-COMM-003 | `TASK-README-006` | `test_tier_required` | 检查 rollback note 与升级入口存在 | 异常沟通准备度 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-COMM-001` | 用专题简报承接内部 `go` 结论 | 直接修改根 `README.md` 作为一次性说明 | 简报更轻量，便于迭代与复用。 |
| `DEC-COMM-002` | 明确禁用表述与替代表述 | 只给正向摘要，不写边界 | 对外口径必须同时说明能说与不能说。 |
| `DEC-COMM-003` | liveops 起草、producer 审核 | 仅技术 owner 自行写对外说明 | 运营沟通应由 `liveops_community` 牵头，但不能越过产品承诺边界。 |
