# oasis7：记忆启发式自我进化补强设计（2026-03-31）

- 对应需求文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`

审计轮次: 6

## 目标
- 在不改变 `.pm/` + `doc/` 真值边界的前提下，引入更清晰的记忆分类、预算化召回与反思提升契约。
- 冻结 `memoryOSS` 与《Hindsight》可借鉴/不可借鉴的边界，防止后续实现时把“工程化记忆”误做成“未受控自治”。

## 外部方案借鉴矩阵
| 来源 | 观察到的模式 | 采用结论 | 映射到 oasis7 | 不采用部分 |
| --- | --- | --- | --- | --- |
| `memoryOSS` | 本地优先、显式 memory mode、namespace、预算化上下文注入、fail-open | adopted | recall profile、role/phase budget、namespace 隔离、离线可运行 | 不接其产品/代理形态为真值，不依赖远程 memory 服务 |
| 《Hindsight》 | `fact/experience/summary/belief` 分层，`retain/recall/reflect` 闭环 | adopted | memory kind、reflection signal、belief review gate | 不把论文实验效果直接外推为工程治理结论 |
| 外部 memory 产品常见做法 | 向量库/图数据库/云托管统一记忆真值 | rejected | 暂不纳入 | 破坏 Git/worktree 审计与离线自治 |
| 完全自由检索历史上下文 | agent 自主决定召回多少、召回什么 | rejected | 暂不纳入 | 噪声不可控、难审计、难复现 |

## 对象模型增量
### 1. Memory Record 扩展
```yaml
- id: MEM-AGENT-0012
  role: agent_engineer
  topic: recall.policy.default
  summary: start phase agents should prefer fact and experience memories first
  source_refs:
    - doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md
    - doc/devlog/2026-03-31.md
  tags:
    - recall
    - memory
  effective_at: 2026-03-31T14:10:00+08:00
  last_reviewed_at: 2026-03-31T14:10:00+08:00
  status: active
  memory_kind: summary
  confidence: confirmed
  review_due_at: 2026-04-30T00:00:00+08:00
  recall_priority: 80
  promotion_reason: engineering_constraint
```

### 2. Belief Record 约束
```yaml
- id: MEM-PRODUCER-0015
  role: producer_system_designer
  topic: stage.risk.memory_budget
  summary: current workflow may need tighter budget_chars before online agent rollout
  source_refs:
    - doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md
  effective_at: 2026-03-31T14:10:00+08:00
  last_reviewed_at: 2026-03-31T14:10:00+08:00
  status: active
  memory_kind: belief
  confidence: hypothesis
  review_due_at: 2026-04-07T00:00:00+08:00
  recall_priority: 40
```

### 3. Recall Profile
```yaml
- profile_id: RECALL-PRODUCER-START-001
  role: producer_system_designer
  phase: start
  kind_allowlist:
    - fact
    - summary
    - experience
  topic_filters:
    - stage.*
    - claim_envelope.*
    - workflow.*
  max_items: 8
  budget_chars: 2400
  freshness_days: 30
  status: active
```

## 流程设计
### Retain
- 原始证据来自 `devlog`、runbook、QA failure、community feedback 或正式评审结论。
- 如果产出的是“反思”，先进入 `signal(source_type=reflection)`，不直接写 memory。
- owner 决定把反思提升为：
  - 新 memory；
  - 新/更新 task；
  - rejected / deferred。

### Recall
- `workflow-report` / `memory-report` 读取 recall profile。
- 召回顺序：
  1. `fact`
  2. `summary`
  3. `experience`
  4. `belief`
- 同类内按 `recall_priority desc`、`effective_at desc` 排序。
- 超出 `max_items` 或 `budget_chars` 时显式截断并报告原因。

### Reflect
- 反思不是“自动改真值”，而是“生成待审查候选结论”。
- 对重复 failure / repeated incident / recurring stage drift，允许创建 `reflection` signal。
- 若反思最终进入 memory：
  - 必须定义 `memory_kind`；
  - 若为 `belief`，必须定义 `review_due_at`；
  - 若推翻旧结论，必须通过 `supersede-memory` 保留历史链。

## 脚本与文件映射
- 现有文件继续作为真值：
  - `.pm/roles/<role>/memory/{active,superseded}.yaml`
  - `.pm/shared/memory/{active,superseded}.yaml`
  - `.pm/inbox/signals.jsonl`
- 推荐新增/扩展：
  - `.pm/registry/recall_profiles.yaml`
  - `scripts/pm/memory-report.sh --kind ... --profile-id ...`
  - `scripts/pm/workflow-report.sh --phase ... --role ...` 默认按 profile 输出预算化视图

## 采用与拒绝的原因
- 采用 `memoryOSS` 的原因：
  - 它强调本地优先、显式模式和有限预算，这与 oasis7 的 worktree/审计思路兼容。
- 拒绝 `memoryOSS` 产品形态的原因：
  - oasis7 当前不是在造一个通用 LLM 记忆代理，而是在强化仓库内治理运行层。
- 采用《Hindsight》对象分层的原因：
  - 现有单一 memory 语义难以区分“已证事实”“经验模式”“综合摘要”“暂定判断”。
- 拒绝直接照搬论文执行闭环的原因：
  - 工程治理比会话智能更重视可追踪、可回放、owner 审批和阶段裁决。

## 风险与缓解
- 风险：`belief` 过多，污染 recall 结果
  - 缓解：默认 `belief` 排序最低，且必须设置 `review_due_at`
- 风险：reflection signal 大量重复
  - 缓解：按 `source_ref + candidate_topic + summary hash` 去重
- 风险：新 schema 破坏既有 role memory
  - 缓解：字段增量设计、旧数据兼容、lint/report 双向回归

## 交付物
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/project.md`
- `doc/devlog/2026-03-31.md`
