# oasis7：记忆启发式自我进化补强设计（2026-03-31）

- 对应需求文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`

审计轮次: 6

## 目标
- 在不改变 `.pm/` + `doc/` 真值边界的前提下，引入更清晰的记忆分类、预算化召回、task-scoped `working_memory` 与反思提升契约。
- 冻结 `memoryOSS` 与《Hindsight》可借鉴/不可借鉴的边界，防止后续实现时把“工程化记忆”误做成“未受控自治”。

## 外部方案借鉴矩阵
| 来源 | 观察到的模式 | 采用结论 | 映射到 oasis7 | 不采用部分 |
| --- | --- | --- | --- | --- |
| `memoryOSS` | 本地优先、显式 memory mode、namespace、预算化上下文注入、fail-open | adopted | recall profile、role/phase budget、namespace 隔离、离线可运行 | 不接其产品/代理形态为真值，不依赖远程 memory 服务 |
| 《Hindsight》 | `fact/experience/summary/belief` 分层，`retain/recall/reflect` 闭环 | adopted | memory kind、working_memory、reflection signal、belief review gate | 不把论文实验效果直接外推为工程治理结论 |
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
    - .pm/tasks/task_231ca618613d564ca2c9ec758253c7b7.execution.md
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

### 4. Working Memory
```yaml
- task_uid: task_6d7c3d84f6ae5fca8966b69460033552
  role: producer_system_designer
  worktree_hint: engineering-working-memory-conversation-analysis
  entries:
    - entry_id: WM-0001
      entry_kind: hypothesis
      summary: process memory should be task-scoped instead of long-term memory scoped
      source_refs:
        - ~/.codex/session_index.jsonl#id=<session_id>
        - ~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<session_id>.jsonl#L<line>
      captured_at: 2026-03-31T16:30:00+08:00
      expires_at: 2026-04-02T00:00:00+08:00
      promoted_to: []
```

## 会话来源策略
- 对 Codex/engineering task，phase 1 raw evidence 只在 owner 显式指定 `session_id`，或显式传 `--allow-auto-session` 后才读取：
  - `~/.codex/session_index.jsonl`
  - `~/.codex/history.jsonl`
  - `~/.codex/sessions/**/rollout-*.jsonl`（当 `history.jsonl` 无该会话消息时回退）
- `~/.codex/logs_1.sqlite` 仅作为后续可选解析层；在没有稳定 sqlite 解析器前，不作为实施前置条件。
- wrapper 导出的 `output/.../<task_uid>.jsonl` 视为后续可替换输入，不是 phase 1 的 canonical source。
- `source_refs` 需要至少能回指：
  - 原始文件路径；
  - `session_id`；
  - 可定位到具体片段的附加键，例如 `ts` 或等价 offset。

## 流程设计
### Retain
- 原始证据来自 task execution log、runbook、QA failure、community feedback、正式评审结论，或 Codex 本地会话存档。
- 对 Codex/engineering task，phase 1 会话 transcript 只在 owner 显式选择 session 后才从 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl` 读取；若 `history.jsonl` 未命中则回退到 `~/.codex/sessions/**/rollout-*.jsonl`，再与任务过程记录一起提炼为 `working_memory`，不直接写 memory。
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
- `working_memory` 是反思前的临时层，承接：
  - `attempt`
  - `hypothesis`
  - `decision`
  - `open_question`
  - `next_step`
- 对同一 live Codex session，只有在 owner 显式 opt-in 的前提下，抽取才支持“首轮快照 + 后续按水位增量”：
  - 默认 wrapper 需要显式 `--session-id`；若 owner 要让脚本通过 registry/worktree pattern 自动解析当前/最近 session，必须显式传 `--allow-auto-session`；
  - 首轮可全量扫描已存在 transcript；
  - 成功导入后，将 `source_session_id`、`transcript_source`、`last_extracted_ts`、`captured_until_ts` 回写到 `.pm/working_memory/<task_uid>.yaml` header；
  - 后续默认只读取 `after_ts=last_extracted_ts` 之后的新消息，避免“提炼 working_memory 的过程本身”污染同一轮输入；
  - 只有显式 `--full-scan` 才允许回扫整段 transcript。
- 对重复 failure / repeated incident / recurring stage drift，允许创建 `reflection` signal。
- 若反思最终进入 memory：
  - 必须定义 `memory_kind`；
  - 若为 `belief`，必须定义 `review_due_at`；
  - 若推翻旧结论，必须通过 `supersede-memory` 保留历史链。
- phase 1 的 canonical chain 为 `显式 session 选择/显式 opt-in auto-resolution + ~/.codex/session_index.jsonl + ~/.codex/history.jsonl (+ sessions rollout fallback) + task execution log/evidence -> working_memory -> reflection signal`；后续若增加 wrapper artifact，只能替换输入层，不能绕过 `working_memory`。

## 脚本与文件映射
- 现有文件继续作为真值：
  - `.pm/roles/<role>/memory/{active,superseded}.yaml`
  - `.pm/shared/memory/{active,superseded}.yaml`
  - `.pm/inbox/signals.jsonl`
- 会话 raw evidence（非 repo 真值，但为 phase 1 输入）：
  - `~/.codex/session_index.jsonl`
  - `~/.codex/history.jsonl`
  - `~/.codex/sessions/**/rollout-*.jsonl`（fallback）
  - `~/.codex/logs_1.sqlite`（optional）
- 推荐新增/扩展：
  - `.pm/working_memory/<task_uid>.yaml`
  - `.pm/registry/recall_profiles.yaml`
  - `scripts/pm/memory-report.sh --kind ... --profile-id ...`
  - `scripts/pm/working-memory-report.sh --task-uid ...`
  - `scripts/pm/workflow-report.sh --phase ... --role ...` 默认按 profile 输出预算化视图
  - `scripts/pm/codex-transcript-report.sh --session-id ... [--after-ts ...] [--before-ts ...]` 或等价实现，用于把 `~/.codex` JSONL 规范化成 `working_memory` 输入

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
- 风险：working memory 任务结束后无人清理
  - 缓解：close phase 强制转 `promoted/discarded/expired`
- 风险：`.codex` 源格式后续漂移，导致抽取器失配
  - 缓解：phase 1 只锁定 `session_index.jsonl` / `history.jsonl` 的最小字段契约，并允许 `sessions/rollout-*.jsonl` 作为兼容 fallback；`logs_1.sqlite` 与 wrapper artifact 留作后续兼容层
- 风险：当前 live session 做提炼时把新生成消息重新并入本轮 transcript，形成自污染
  - 缓解：默认关闭隐式 auto-resolution；owner 必须显式 `--session-id` 或显式 `--allow-auto-session` 才能读取 `.codex` transcript。在显式 opt-in 的 live-session 场景下，继续采用 `last_extracted_ts/captured_until_ts` 水位；默认增量抽取，只有显式 `--full-scan` 才回扫全量 transcript
- 风险：新 schema 破坏既有 role memory
  - 缓解：字段增量设计、旧数据兼容、lint/report 双向回归

## 交付物
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/project.md`
- `.pm/tasks/task_231ca618613d564ca2c9ec758253c7b7.execution.md`
