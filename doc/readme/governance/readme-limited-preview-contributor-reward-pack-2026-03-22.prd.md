# oasis7 Limited Preview Early Contributor Reward Pack（2026-03-22）

- 对应设计文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: oasis7 已经有 Token 分配口径、创世参数草案与 QA 审计清单，但 `liveops_community` 还没有一套可直接执行的 early contributor reward 模板。没有这个模板，团队容易把“早期贡献奖励”做成临场判断，甚至滑向 `play-to-earn`、`airdrop for players` 或依赖 invite-only 的错误叙事。
- Proposed Solution: 建立一份 limited preview early contributor reward pack，固定贡献类型、评分规则、证据字段、奖励建议档位与禁语清单，并明确“不依赖 invite-only、也不公开固定 token/point 汇率”。
- Success Criteria:
  - SC-1: 模板明确区分可计分贡献与不可计分行为，单纯登录/在线时长/试玩不计分。
  - SC-2: 每条奖励建议都必须附带 `Reward Account`、证据字段与 reviewer。
  - SC-3: 奖励输出只使用 `eligible-small / eligible-medium / eligible-large / no-token-recommendation`，不公开固定 token 数额。
  - SC-4: producer 审批真实发放金额时，普通 merged PR 默认 ceiling 为 `150 OC`；若超过该值，必须显式写明 exceptional case 理由，`1500 OC` 仅能作为极少数 exceptional row 的 round-specific 决策。
  - SC-5: 对外禁语清单明确阻断 `play-to-earn`、`login reward`、`time played = token` 等说法。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要把 early contributor reward 变成可执行、可审计、可复用的运营动作。
  - `producer_system_designer`：需要知道 liveops 侧如何评估贡献，而不是让奖励完全凭感觉发放。
  - 早期 builder / contributor：需要清楚什么类型的贡献才有可能进入奖励建议池。
- User Scenarios & Frequency:
  - 每轮 limited preview 结束时汇总一次贡献评分。
  - 每次出现高价值 bug、PR、长时样本或内容贡献时追加评分记录。
  - 每次贡献者希望让某个 GitHub PR 直接进入奖励审核时，在 PR intake 中补齐 `Reward Account`。
  - 每次 `liveops_community` 需要把 PR intake 批量导入台账时，通过仓库脚本解析 PR body，而不是手工抄字段。
  - 每次准备对外说明奖励机制时，先用禁语清单复核文案。
- User Stories:
  - PRD-README-LTPR-001: As a `liveops_community`, I want a contribution scoring template, so that reward review is auditable.
  - PRD-README-LTPR-002: As a `producer_system_designer`, I want reward recommendation bands instead of raw marketing copy, so that I can approve distribution conservatively.
  - PRD-README-LTPR-003: As a builder, I want contribution criteria to be explicit, so that I know effort quality matters more than raw playtime.
- Critical User Flows:
  1. Flow-LTPR-001: `收集 bug / PR / 长时样本 / 内容贡献 -> 填写证据字段 -> 按评分表打分 -> 输出建议档位`
  2. Flow-LTPR-002: `对外准备奖励说明 -> 用禁语清单复核 -> 删除任何 P2E / login reward 表述 -> 再交 producer 审核`
  3. Flow-LTPR-003: `producer 审核奖励建议档位 -> 决定是否批准与真实金额 -> 普通 merged PR 默认不超过 150 OC；若超过则必须写明 exceptional case 理由 -> 若批准则进入后续链上/多签执行`
  4. Flow-LTPR-004: `贡献者提交 GitHub PR -> 如需进入奖励审核则填写可选 reward intake block -> liveops 从 PR 中读取 Reward Account / evidence link -> 再进入评分模板`
  5. Flow-LTPR-005: `liveops 运行导入脚本 -> 脚本输出 ready/deferred/no_reward_review_requested 与 ledger-ready 字段 -> 再决定是否进入 reward ledger`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 贡献评分 | `reward_account`、`contribution_type`、`base_score`、`quality_modifier`、`duplicate_flag` | 按模板打分 | `captured -> scored -> reviewed` | `Reward Account` 仅作执行字段；重复低价值反馈降权 | `liveops_community` 记录，producer 审核 |
| 证据字段 | `proof_link`、`build_id`、`repro_steps`、`duration_sample`、`reviewer` | 填充证据并核验完整性 | `missing -> complete` | 缺关键证据不得进入奖励建议池 | `liveops_community` 维护 |
| GitHub PR Intake | `reward_review_request`、`reward_account`、`evidence_link`、`notes` | 在 PR 模板中按需保留 reward intake block | `deleted -> submitted -> imported` | 不申请 reward review 的作者应删除整个区块；不得在该区块使用 raw `public_key` 作为名称 | PR 作者填写，`liveops_community` 导入 |
| PR Intake Import Script | `import_status`、`validation_error`、`missing_fields`、`ledger_row` | 解析 PR body 并输出 ledger-ready 结构化结果 | `parsed -> ready/deferred/no_reward_review_requested/invalid_intake` | `Request reward review` 必须显式为 `yes`；`Reward Account` 缺失时返回 `deferred`，未请求或 intake 无效时不建 row | `liveops_community` 执行 |
| 奖励建议档位 | `eligible-small`、`eligible-medium`、`eligible-large`、`no-token-recommendation` | 根据总分给出建议档位 | `scored -> recommended` | 只给档位，不给固定 token 数额 | `producer_system_designer` 决定是否批准 |
| 禁语清单 | `forbidden_phrase`、`safe_phrase` | 审核对外 copy | `draft -> safe/block` | 命中禁语即阻断 | `liveops_community` 起草，producer 审核 |
- Acceptance Criteria:
  - AC-1: 操作包至少包含 `Reward Account` 字段、`贡献类型表`、`评分模板`、`证据字段`、`奖励建议档位` 与 `禁语清单`。
  - AC-2: 以下行为必须显式标记为 `no-token-recommendation` 默认项：登录、注册、浏览帖子、单纯试玩、在线时长、挂机时长。
  - AC-3: 对外模板必须明确“不依赖 invite-only，贡献审核也不等于公开发币活动”。
  - AC-4: 对外模板不得出现 `play-to-earn`、`login reward`、`time played = token`、`come play to earn`、`airdrop for players`。
  - AC-5: 奖励建议档位只允许使用 `eligible-small / eligible-medium / eligible-large / no-token-recommendation`，不得公开固定 token 数额或固定 token/point 比率。
  - AC-5A: 若贡献来源是普通 merged PR，producer 审批的默认真实发放 ceiling 为 `150 OC`；任何 `>150 OC` 的 row 都必须在 approval note 中写明 exceptional case 理由，且不得把 `1500 OC` 重新表述成常规 MR 档位。
  - AC-6: 若 GitHub PR 被视作贡献证据入口，默认 PR 模板必须提供可选 reward intake block，字段至少包含 `Reward Account`，不得把 raw `public key` 当作 claimant 名称字段。
  - AC-7: 仓库必须提供可执行导入脚本，至少支持 `--body-file` 离线解析，并能输出 `ready / deferred / no_reward_review_requested / invalid_intake` 四类状态。
- Non-Goals:
  - 本专题不决定具体 token 发放数量。
  - 不替代 `qa_engineer` 的创世配置审计职责。
  - 不建设产品级 invite-only 访问控制。
  - 不把普通 merged PR 的 ceiling 写成公开 bounty 报价表。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该操作包位于 liveops 层，负责把 limited preview 的贡献信号转成结构化奖励建议，再回流给 producer；不直接触发链上分发，也不对外承诺固定 token 兑换规则。
- Integration Points:
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
  - `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
  - `doc/readme/governance/readme-limited-preview-invite-pack-2026-03-22.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
  - `scripts/readme-reward-pr-intake-import.py`
- Edge Cases & Error Handling:
  - 多人重复提交同一 bug：只给首个高质量提交 full 分，后续重复只保留低分或不计分。
  - 只有情绪反馈、没有证据：记录但不进入 token 建议池。
  - PR 未合并但价值高：可给 `eligible-small` 或 `eligible-medium` 建议，但必须说明状态。
  - PR 作者删除了 reward intake block：视为该 PR 未申请 reward review；若后续要进入奖励审核，必须补齐 `Reward Account` 或由 liveops 补录后标记来源。
  - PR 作者保留 intake block 但 `Request reward review` 不是显式 `yes`：导入脚本应返回 `invalid_intake`，不得默认为已申请。
  - 若贡献者只提供 raw `public key` 派生材料，进入奖励模板前必须先收口为 `Reward Account`，不得把 raw `public key` 直接当作领取名称展示。
  - 普通 merged PR 即使证据完整、价值较高，也不得默认滑到 `1500 OC` 这一层；若 producer 认为必须高于 `150 OC`，必须把安全关键、release-blocking 或其他 exceptional 理由写进审批备注。
  - 对外问“玩多久能拿多少 token”：必须明确回答“没有固定时长换算，不按在线时长发放”。
- Non-Functional Requirements:
  - NFR-LTPR-1: 每条奖励建议记录都必须能追溯到至少 1 条证据链接。
  - NFR-LTPR-2: 对外 copy 中禁语命中率必须为 `0`。
  - NFR-LTPR-3: 模板必须可在一个 limited preview round 内重复使用，不依赖 invite-only 工具链。
  - NFR-LTPR-4: 普通 merged PR 的 producer 审批默认 ceiling 为 `150 OC`；任何 `>150 OC` 的审批都必须带 exceptional case note，且 `1500 OC` 不得作为常规 merged PR 预期传播。
- Security & Privacy: 贡献模板只记录必要的公开标识、证据链接与链上账户，不要求暴露私密身份信息；raw `public key` 仅保留在底层签名/账户绑定流程中，不作为奖励领取名称对外展示。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 固定评分模板、证据字段、建议档位与禁语清单。
  - v1.1: 跑完首轮真实贡献复盘后再调权重。
  - v2.0: 若 future governance 确定固定档位额度，再补链上执行映射。
- Technical Risks:
  - 风险-1: 若过早公开具体 token 数额，会让 early contributor reward 被外界理解成发币营销。
  - 风险-2: 若评分规则过粗，会让高质量贡献和低质量噪音无法区分。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-LTPR-001 | LTPR-1/2 | `test_tier_required` | 评分模板、证据字段与重复贡献处理规则检查 | liveops 贡献记录一致性 |
| PRD-README-LTPR-002 | LTPR-2/3 | `test_tier_required` | 奖励建议档位、producer 审核边界与不公开固定数额检查 | producer 审批与对外口径安全性 |
| PRD-README-LTPR-003 | LTPR-1/2/3 | `test_tier_required` | 可计分/不可计分行为表、禁语清单与邀请机制解耦检查 | builder 预期管理与风控 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LTPR-001 | 奖励模板按贡献评分，不按游玩时长 | time-play mining | 当前阶段仍是技术预览，不适合做时长挖矿叙事。 |
| DEC-LTPR-002 | 只输出奖励建议档位，不公开固定 token 数额 | 直接公布每种贡献对应多少 token | 先控制承诺，再决定实际分发。 |
| DEC-LTPR-003 | 模板不依赖 invite-only | 把奖励资格绑定到 invite-only 名单 | 用户已经确认不做 product-level invite-only，模板必须与此一致。 |
| DEC-LTPR-004 | reward intake 与执行层统一只要求 `Reward Account` | 在 PR intake 里继续保留一套独立 claimant 字段 | 这条链路的目标是可执行收款；名称层已有 `Contributor / Public Handle / GitHub` 可追溯，额外 claimant 字段会增加填写负担。 |
| DEC-LTPR-005 | GitHub PR 如要直接进入奖励审核，使用可选 reward intake block 收集 `Reward Account` | 继续要求 liveops 在评论/私聊里二次追问账户字段，或在 PR 模板里直接索要 raw `public_key` | PR 已是公开贡献面；把 reward intake 做成可选结构化区块，既方便导入台账，也能保持模板最小必填面。 |
| DEC-LTPR-006 | 用仓库脚本解析 PR intake block 并输出结构化导入状态 | 继续靠 liveops 人工抄 PR body、口头判断 `ready/deferred/no_reward_review_requested` | 脚本化导入能把重复劳动和判定歧义收掉，同时把必填 contract 收口到单一 `Reward Account`。 |
| DEC-LTPR-007 | 普通 merged PR 的默认真实发放 ceiling 收紧到 `150 OC`，`1500 OC` 只保留给极少数 exceptional case | 继续让单个 merged PR 在没有 exceptional note 时也能默认落到 `1500 OC` | 当前总量口径改为 `10B` 后，普通 MR 的激励应继续保持保守，不应把 `1500 OC` 变成常规 merged PR 预期。 |

## 7. 执行模板（执行版）

### 7.1 贡献类型与基础分
| Type ID | 贡献类型 | 基础分 | 说明 |
| --- | --- | --- | --- |
| C-01 | 可复现 `blocking` bug + 完整复现路径 | `40` | 必须含 build/env/steps/result |
| C-02 | 高质量 `non-blocking` bug / UX friction | `20` | 必须有清晰前后对比 |
| C-03 | 合并 PR / patch | `40` | 普通 merged PR 默认按保守口径记分；未合并但高质量可临时记 `20` |
| C-04 | 长时有效游玩样本 + 结构化总结 | `25` | 至少含时长、路径、关键问题 |
| C-05 | 高质量内容/文档/翻译贡献 | `20` | 必须已公开可验证 |
| C-06 | 生态帮助 / builder onboarding 协助 | `15` | 必须能证明实际帮助了他人 |
| C-07 | 仅登录/注册/试玩/在线时长 | `0` | 默认 `no-token-recommendation` |

### 7.2 质量修正
- `+20`: 直接推动修复、合并或关键决策
- `+10`: 证据非常完整，便于直接执行
- `0`: 正常质量
- `-10`: 重复、证据不足或价值偏低
- `-20`: 误报、刷屏、不可验证

### 7.3 奖励建议档位
| 总分 | 建议档位 | 说明 |
| --- | --- | --- |
| `<20` | `no-token-recommendation` | 感谢即可，不建议 token |
| `20-49` | `eligible-small` | 可进入小额贡献奖励候选 |
| `50-89` | `eligible-medium` | 可进入中档贡献奖励候选 |
| `>=90` | `eligible-large` | 可进入高价值贡献奖励候选 |

注：本表只给建议档位，不公开固定 token 数额，也不形成固定 token/point 汇率。

### 7.4 Producer 审批金额护栏
- `eligible-small / medium / large` 只是审阅档位，不等于公开 bounty 表。
- 对于 `Source Type=PR` 且 `Contribution Type=C-03` 的普通 merged PR，producer 审批的默认真实发放 ceiling 为 `150 OC`。
- 若某条 merged PR 需要高于 `150 OC`，producer 必须在 approval note 中写明为什么它属于 exceptional case，而不是普通 merged PR。
- `1500 OC` 只能用于极少数 exceptional row，且必须写成 round-specific 决策，不得外推为全局 `eligible-large=1500 OC` 映射。
