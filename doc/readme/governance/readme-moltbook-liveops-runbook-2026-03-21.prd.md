# oasis7: Moltbook 运营 Runbook（2026-03-21）

- 对应设计文档: `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: `liveops_community` 已有 Moltbook 推广方案与首批帖文草案，但一旦开始真实发帖与持续跟帖，团队仍缺一份日常运营 SOP；如果只靠角色卡或临场判断，很容易在检查频率、评论分级、回复边界、升级条件和回写方式上出现漂移。
- Proposed Solution: 在 `readme/governance` 新增一份 Moltbook 运营 runbook，把发帖前检查、发帖后巡检、通知/评论分级、回复边界、升级路径、GitHub 回流和 `devlog` 记录要求固化成可重复执行的 how-to 文档，并由 `liveops_community` 使用。
- Success Criteria:
  - SC-1: runbook 明确日常检查入口、检查节奏和证据来源，而不是停留在抽象运营原则。
  - SC-2: runbook 明确哪些评论可以直接回复、哪些必须升级，以及回复时必须坚持的 claim boundary。
  - SC-3: runbook 明确 GitHub `issue` / `PR` 回流与 `doc/devlog/YYYY-MM-DD.md` 回写方式。
  - SC-4: 角色卡只保留职责入口，具体运营动作下沉到专题 runbook。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`: 需要一份拿来就能执行的 Moltbook 日常运营 SOP。
  - `producer_system_designer`: 需要确信 LiveOps 在真实互动时不会越过技术预览与合作承诺边界。
  - `qa_engineer` / 工程 owner: 需要从渠道反馈中拿到结构化、可升级的问题输入。
- User Scenarios & Frequency:
  - 发帖前：复核文案、链接、claim boundary 与资产。
  - 发帖后 0-24 小时：高频检查 `/home`、通知、评论与互动信号。
  - 常规运营日：按固定节奏处理未读、回复、升级和回写。
  - 周复盘：聚合本周信号，决定继续 world proof、agent diary 还是 builder hook。
- User Stories:
  - PRD-README-MOLTRUN-001: As a `liveops_community`, I want a day-2 Moltbook operations runbook, so that post-publish monitoring and replies are consistent.
  - PRD-README-MOLTRUN-002: As a `producer_system_designer`, I want clear escalation triggers and forbidden-reply boundaries, so that community interactions do not create unsafe promises.
  - PRD-README-MOLTRUN-003: As a `qa_engineer` or engineering owner, I want Moltbook feedback triaged into product, quality, and collaboration buckets, so that external signals can enter backlog without ambiguity.
- Critical User Flows:
  1. `发帖前复核主贴 / 首评 / 资产 / 链接 -> 发布 -> 记录发帖时间与 post id`
  2. `检查 /home 与 notifications -> 定位互动所属帖子 -> 判断是否需要直接回复 / GitHub 回流 / owner 升级`
  3. `整理当日外部信号 -> 回写 devlog -> 必要时回流 producer_system_designer / qa_engineer / 工程 owner`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 日常检查节奏 | `check_window`、`endpoint`、`goal` | 固化发帖后 24h 与常规日巡检动作 | `pending -> checked -> logged` | 先看 `home`，再看通知，再看帖子评论 | `liveops_community` 执行 |
| 评论/通知分级 | `signal_bucket`、`severity`、`owner`、`reply_mode` | 判断直接回复、GitHub 回流或升级 | `new -> triaged -> replied/escalated` | 先处理 overclaim 风险，再处理真实问题，再处理合作线索 | `liveops_community` 分级，相关 owner 接收 |
| 回复边界 | `allowed_claim`、`forbidden_claim`、`safe_redirect` | 限制所有公开回复措辞 | `draft -> approved -> reused` | 先重申技术预览，再给证据或回流口 | `producer_system_designer` 定边界 |
| 回写闭环 | `post_id`、`summary`、`signal_tags`、`follow_up` | 把运营动作和信号沉淀到 devlog / 项目文档 | `captured -> documented -> handed_off` | 高风险与高价值线索优先记录 | `liveops_community` 维护 |
- Acceptance Criteria:
  - AC-1: 产出 Moltbook liveops runbook 的 PRD / Design / Project / Runbook 文档。
  - AC-2: runbook 必须包含发帖前、发帖后 24h、常规日、周复盘四类动作。
  - AC-3: runbook 必须明确 `/home`、通知、帖子评论与 GitHub 回流的使用顺序。
  - AC-4: runbook 必须明确回复边界、升级条件和 `devlog` 回写要求。
- Non-Goals:
  - 不在本专题中新增 Moltbook 集成承诺或平台合作承诺。
  - 不把 token、凭据路径或任何敏感配置写入正式文档。
  - 不替代首批帖文草案包本身；runbook 只定义执行方法，不重写主贴文案。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - `https://www.moltbook.com/skill.md`
  - `https://www.moltbook.com/api/v1/home`
  - `https://www.moltbook.com/api/v1/notifications`
  - `https://www.moltbook.com/api/v1/posts/:id`
  - `https://www.moltbook.com/api/v1/posts/:id/comments`
  - `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
  - `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.md`
- Evaluation Strategy:
  - 抽样检查 runbook 是否给出明确检查入口与执行频率。
  - 抽样检查 runbook 是否将评论分到产品 / 质量 / 合作 / 口径风险桶。
  - 若出现凭据路径、明文 token 或未批准的产品承诺，判为不通过。

## 4. Technical Specifications
- Architecture Overview: 本专题位于 `readme/governance`，承接 Moltbook 从“有方案、有草案”到“进入持续运营”的执行层流程。它依赖已有推广方案与帖文草案，但额外定义日常检查、回复、升级和回写 SOP。
- Integration Points:
  - `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
  - `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.md`
  - `.agents/roles/liveops_community.md`
  - `doc/devlog/YYYY-MM-DD.md`
  - `README.md`
- Edge Cases & Error Handling:
  - 若 `/home` 显示未读通知但 `activity_on_your_posts` 为空：仍需检查 `notifications`，因为可能只是新关注，而不是评论。
  - 若评论问的是“已上线吗 / 能玩吗 / 已集成 Moltbook 吗”：优先回到既有回复模板，不自行扩写承诺。
  - 若评论含 bug、friction、缺文档反馈：优先回流 GitHub `issue`，并在内部标注为 `qa_engineer` / 对应工程 owner follow-up。
  - 若评论涉及合作、身份、上链、provider 联动：只记录为线索并升级，不公开承诺时间和交付。
  - 若平台接口结构变化或网页不可读：回退到“人工查看 profile/post page + 记录绝对时间”的保守流程，并更新 runbook。
- Non-Functional Requirements:
  - NFR-MOLTRUN-1: runbook 必须能在 10 分钟内帮助运营同学完成一次基础巡检。
  - NFR-MOLTRUN-2: 所有升级路径必须映射到标准角色名。
  - NFR-MOLTRUN-3: 文档不得包含任何敏感凭据、私有 token 或不应公开的本地路径。
  - NFR-MOLTRUN-4: 回写格式必须满足 `doc/devlog/YYYY-MM-DD.md` 的时间 / 角色 / 完成内容 / 遗留事项约束。
- Security & Privacy: 凭据只允许存在于受控本地存储或 owner 管理面；正式文档、截图和 `devlog` 中不得记录 token、cookie 或邮箱验证码等敏感信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`MOLTRUN-1`): 固化 Moltbook 发帖后巡检、评论分级和回写流程。
  - v1.1 (`MOLTRUN-2`): 根据真实互动把高频问题沉淀成更完整的回复模板索引。
  - v2.0 (`MOLTRUN-3`): 若后续进入更多平台，抽象成通用 third-party liveops skeleton，再保留 Moltbook 特化附录。
- Technical Risks:
  - 风险-1: 只有推广方案没有日常 SOP，执行时会重新回到临场 improvisation。
  - 风险-2: 若运营只看网页摘要而不看 `home/notifications/comments`，容易误判“有未读 = 有评论”。
  - 风险-3: 若不把合作与缺陷信号分桶，外部反馈很难进入项目闭环。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| `PRD-README-MOLTRUN-001` | `TASK-README-024` | `test_tier_required` | 检查 runbook 含发帖前、发帖后 24h、常规日与周复盘 SOP | Moltbook 持续运营一致性 |
| `PRD-README-MOLTRUN-002` | `TASK-README-024` | `test_tier_required` | 检查回复边界、升级条件与 owner 路径存在 | 对外口径风险控制 |
| `PRD-README-MOLTRUN-003` | `TASK-README-024` | `test_tier_required` | 检查信号分桶、GitHub 回流与 `devlog` 回写要求存在 | 外部反馈回流效率 |
| `PRD-README-MOLTRUN-001/002/003` | `TASK-README-024` | `test_tier_required` | `./scripts/doc-governance-check.sh` | 文档互链与治理一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-MOLTRUN-001` | 把 Moltbook 日常运营动作写成独立 runbook | 继续只靠角色卡 + 推广方案执行 | 角色卡应稳定，runbook 才适合持续迭代。 |
| `DEC-MOLTRUN-002` | 先看 `/home`，再查通知和目标帖子评论 | 每次都手动刷 profile 页面猜互动 | `home` 已提供账号级汇总，更适合作为日常检查入口。 |
| `DEC-MOLTRUN-003` | 统一把 bug/friction 导向 GitHub `issue`，把 fix 意向导向 GitHub `PR` | 在 Moltbook 评论区长期来回追 bug 细节 | 仓库已有正式协作面，渠道评论区不应承担缺陷跟踪职责。 |
