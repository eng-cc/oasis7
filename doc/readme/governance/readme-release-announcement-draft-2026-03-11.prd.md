# oasis7: 版本候选公告 / Changelog 底稿（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-release-announcement-draft-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-announcement-draft-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 已有对外口径简报与模板，但仍缺一份可直接交给运营或外部渠道继续加工的正式公告 / changelog 底稿。若没有底稿，后续每次沟通都要从零开始重组结构，容易重新引入口径漂移。
- Proposed Solution: 基于当前版本候选内部 `go` 结论与对外口径简报，生成一份“未发布、可复审”的 announcement / changelog 底稿，明确事实摘要、已知限制、后续动作与禁用承诺，为后续真实外部发布预留统一基线。
- Success Criteria:
  - SC-1: 底稿明确标注其状态为 `draft`，不等于已发布公告。
  - SC-2: 底稿包含版本摘要、能力亮点、风险边界、后续动作与 FAQ 口径。
  - SC-3: 底稿能回链到 communication brief 与内部 go/no-go 记录。
  - SC-4: `liveops_community -> producer_system_designer` 审核链和 `readme` 追踪都已闭环。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要一份能继续改成公告或 changelog 的底稿。
  - `producer_system_designer`：需要确认底稿中的承诺没有越过当前候选边界。
  - 外部评审者 / 社区成员：未来会消费基于该底稿派生的正式说明。
- User Scenarios & Frequency:
  - 版本候选形成内部 `go` 后：准备公告底稿。
  - 正式对外前：复核底稿并决定是否升级为正式文案。
  - 风险或节奏变化时：更新底稿而不是直接改根 README。
- User Stories:
  - PRD-README-ANN-001: As a `liveops_community`, I want an announcement/changelog draft based on approved messaging, so that final external copy starts from a safe baseline.
  - PRD-README-ANN-002: As a `producer_system_designer`, I want promises and non-promises separated in the draft, so that public-facing claims stay within current release scope.
  - PRD-README-ANN-003: As an external reviewer, I want a concise summary plus known limitations, so that I understand what this candidate means without overreading it.
- Critical User Flows:
  1. `读取 communication brief -> 提取可公开内容 -> 组织成 announcement / changelog 底稿`
  2. `分离“已确认内容”与“仍在推进内容” -> 标记 draft 状态 -> 进入审核`
  3. `通过 liveops -> producer 审核后，作为未来正式外部文案底稿`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| announcement header | 标题、候选 ID、状态、来源文档、最后更新时间 | 生成文案头部 | `draft -> reviewed -> publish_ready` | 状态字段必须显式显示 | `liveops_community` 维护 |
| release summary | 本轮变化、当前结论、适用范围 | 写给外部读者的摘要 | `empty -> drafted` | 先写已确认事实，再写限制 | `producer_system_designer` 审核 |
| known limitations | 当前未覆盖项、风险边界、禁用承诺 | 限制外部理解范围 | `empty -> drafted` | 高风险限制优先写明 | `producer_system_designer` 拍板 |
| faq / next steps | 常见问题、下一步、升级入口 | 供运营答疑复用 | `empty -> drafted` | FAQ 必须与简报一致 | `liveops_community` 维护 |
- Acceptance Criteria:
  - AC-1: 产出 announcement/changelog 底稿专题 PRD / Design / Project。
  - AC-2: 产出一份正式文风的公告 / changelog 底稿，且显式标注 `draft`。
  - AC-3: 底稿明确“当前已确认内容 / 当前未承诺内容 / 下一步动作”三块结构。
  - AC-4: `doc/readme/project.md` 能追踪该任务，且 handoff / devlog 完整。
- Non-Goals:
  - 不直接发布公告。
  - 不修改根 `README.md` 作为正式发布声明。
  - 不新增外部数据、玩家反馈或媒体素材。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题位于 readme/governance，消费 communication brief 与模板，输出一份结构稳定的公告 / changelog 底稿，服务后续外部渠道或站点文案加工。
- Integration Points:
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-communication-template-2026-03-11.md`
  - `doc/core/reviews/release-candidate-go-no-go-version-2026-03-11.md`
- Edge Cases & Error Handling:
  - 内部结论变动：底稿标题和摘要必须同步更新状态，不得沿用旧措辞。
  - 某项信息尚未批准公开：底稿使用抽象描述或直接删除，不允许“先写后删”保留风险表达。
  - 需要正式发布时：必须在底稿基础上生成新文档或新版本，而不是直接覆盖历史底稿。
- Non-Functional Requirements:
  - NFR-ANN-1: 底稿可在 10 分钟内被运营或评审完整阅读。
  - NFR-ANN-2: 所有关键表述都必须可回链到 brief 或 go/no-go 文档。
  - NFR-ANN-3: 必须显式区分 draft 与正式发布态。
- Security & Privacy: 底稿不得暴露内部运行目录、日志路径、命令细节或敏感配置。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`ANN-1`): 形成首份公告 / changelog 底稿。
  - v1.1 (`ANN-2`): 若对外发布节奏稳定，再抽象 announcement 专用模板。
  - v2.0 (`ANN-3`): 评估是否与站点 / README 公开页联动。
- Technical Risks:
  - 风险-1: 若底稿写得像正式发布，会误导读者对当前阶段的判断。
  - 风险-2: 若 FAQ 未固定，运营答疑会重新漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-ANN-001 | `TASK-README-008` | `test_tier_required` | 检查公告底稿与 brief / go-no-go 互链 | 对外发布底稿一致性 |
| PRD-README-ANN-002 | `TASK-README-008` | `test_tier_required` | 检查已确认 / 未承诺 / 下一步三段结构存在 | 对外承诺边界控制 |
| PRD-README-ANN-003 | `TASK-README-008` | `test_tier_required` | 检查 FAQ 与 draft 状态字段存在 | 运营答疑准备度 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-ANN-001` | 先生成 draft 底稿，不直接发布 | 直接把简报当正式公告 | 简报偏治理与边界，不够接近外部文风。 |
| `DEC-ANN-002` | 底稿显式区分已确认与未承诺 | 只写亮点，不写限制 | 限制不写清会导致过度承诺。 |
| `DEC-ANN-003` | 底稿继续走 liveops 起草、producer 审核 | 直接由 producer 单独写公告 | 口径需要运营文风，但承诺边界必须由产品 owner 审核。 |
