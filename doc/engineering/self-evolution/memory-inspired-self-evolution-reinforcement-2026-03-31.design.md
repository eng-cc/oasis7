# oasis7：记忆启发式自我进化补强设计（2026-03-31）

- 对应需求文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`

审计轮次: 6

## 1. 设计定位

这份设计文档只保留记忆分层、会话来源与 retain/recall/reflect 流程的唯一设计信息，不再重复 PRD 的 adopted/rejected 结论或 project 的 rollout 任务。

## 2. Borrowing Boundary

- adopted:
  - `memoryOSS` 的本地优先、显式 memory mode、namespace、预算化上下文注入、fail-open
  - 《Hindsight》 的 `fact / experience / summary / belief` 分层，以及 `retain / recall / reflect` 闭环
- rejected:
  - 向量库 / 图数据库 / 云托管统一记忆真值
  - agent 自主无限制召回历史上下文
- repo-owned translation:
  - recall profile
  - role / phase budget
  - task-scoped `working_memory`
  - reflection signal
  - belief review gate

## 3. Object Model Delta

### 3.1 Memory Record Extension

```yaml
- id: MEM-AGENT-0012
  role: agent_engineer
  topic: recall.policy.default
  summary: start phase agents should prefer fact and experience memories first
  source_refs:
    - doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md
    - .pm/tasks/task_231ca618613d564ca2c9ec758253c7b7.execution.md
  status: active
  memory_kind: summary
  confidence: confirmed
  review_due_at: 2026-04-30T00:00:00+08:00
  recall_priority: 80
  promotion_reason: engineering_constraint
```

### 3.2 Belief Record Constraint

```yaml
- id: MEM-PRODUCER-0015
  role: producer_system_designer
  topic: stage.risk.memory_budget
  summary: current workflow may need tighter budget_chars before online agent rollout
  status: active
  memory_kind: belief
  confidence: hypothesis
  review_due_at: 2026-04-07T00:00:00+08:00
  recall_priority: 40
```

### 3.3 Recall Profile

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

### 3.4 Working Memory

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
      captured_at: 2026-03-31T16:30:00+08:00
      expires_at: 2026-04-02T00:00:00+08:00
      promoted_to: []
```

## 4. Session Source Policy

- phase 1 raw evidence 只在 owner 显式指定 `session_id`，或显式传 `--allow-auto-session` 后才读取:
  - `~/.codex/session_index.jsonl`
  - `~/.codex/history.jsonl`
  - `~/.codex/sessions/**/rollout-*.jsonl` 作为 fallback
- `~/.codex/logs_1.sqlite` 仅作为后续可选解析层
- wrapper 导出的 `output/.../<task_uid>.jsonl` 只是后续可替换输入，不是 phase 1 canonical source
- `source_refs` 至少要能回指:
  - 原始文件路径
  - `session_id`
  - 可定位片段的键，如 `ts` 或等价 offset

## 5. Retain / Recall / Reflect

### 5.1 Retain

- 原始证据来自 task execution log、runbook、QA failure、community feedback、正式评审结论，或 Codex 本地会话存档
- transcript 先提炼成 `working_memory`，不直接写长期 memory
- “反思”先进入 `signal(source_type=reflection)`，由 owner 决定 promotion / rejection / defer

### 5.2 Recall

- `workflow-report` / `memory-report` 读取 recall profile
- 召回顺序:
  1. `fact`
  2. `summary`
  3. `experience`
  4. `belief`
- 同类内按 `recall_priority desc`、`effective_at desc`
- 超出 `max_items` 或 `budget_chars` 时显式截断并报告原因

### 5.3 Reflect

- `working_memory` 承接:
  - `attempt`
  - `hypothesis`
  - `decision`
  - `open_question`
  - `next_step`
- live session 抽取只在显式 opt-in 下支持“首轮快照 + 后续按水位增量”
- 成功导入后回写:
  - `source_session_id`
  - `transcript_source`
  - `last_extracted_ts`
  - `captured_until_ts`
- 默认只读 `after_ts=last_extracted_ts` 之后的新消息
- 只有显式 `--full-scan` 才允许回扫整段 transcript
- 若最终提升为 memory:
  - 必须定义 `memory_kind`
  - `belief` 必须定义 `review_due_at`
  - 推翻旧结论必须走 `supersede-memory`

## 6. File / Script Surface

- 现有真值:
  - `.pm/roles/<role>/memory/{active,superseded}.yaml`
  - `.pm/shared/memory/{active,superseded}.yaml`
  - `.pm/inbox/signals.jsonl`
- phase 1 会话输入:
  - `~/.codex/session_index.jsonl`
  - `~/.codex/history.jsonl`
  - `~/.codex/sessions/**/rollout-*.jsonl`
  - `~/.codex/logs_1.sqlite` 仅 optional
- 推荐新增/扩展:
  - `.pm/working_memory/<task_uid>.yaml`
  - `.pm/registry/recall_profiles.yaml`
  - `scripts/pm/working-memory-report.sh --task-uid ...`
  - `scripts/pm/memory-report.sh --kind ... --profile-id ...`
  - `scripts/pm/codex-transcript-report.sh --session-id ... [--after-ts ...]`

## 7. 风险控制

- `belief` 过多污染 recall: 默认排最后，且必须设置 `review_due_at`
- reflection signal 重复: 按 `source_ref + candidate_topic + summary hash` 去重
- working memory 无人清理: close phase 强制转 `promoted / discarded / expired`
- `.codex` 源格式漂移: 锁最小字段契约，并保留 rollout fallback
- live session 自污染: 默认关闭隐式 auto-resolution，显式 opt-in 后仍走水位增量
- 新 schema 破坏旧 memory: 采用字段增量设计和 lint/report 回归

## 8. 使用方式

- 看正式 adopted/rejected 和验收: `memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- 看 rollout / follow-up: `memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`
- 看 working memory、recall profile 与 transcript policy 的唯一设计约束: 本文档
