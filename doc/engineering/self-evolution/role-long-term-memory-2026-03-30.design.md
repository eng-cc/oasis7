# oasis7：角色长期记忆自建设计（2026-03-30）

- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`

审计轮次: 7

## 1. 设计定位

这份设计文档只保留长期 memory 的文件结构、schema、promotion 规则与 role-topic 边界，不再重复 PRD 的背景、project 的任务拆解或交付清单。

长期 memory 必须继续区别于 task execution log、task registry 和短期 `working_memory`。

## 2. Storage Layout

每个角色:

```text
.pm/roles/<role>/memory/
  active.yaml
  superseded.yaml
```

跨角色共享:

```text
.pm/shared/memory/
  active.yaml
  superseded.yaml
```

## 3. Schema

### 3.1 Active Record

```yaml
- id: MEM-PRODUCER-0001
  role: producer_system_designer
  topic: stage.current
  summary: current stage remains internal_playable_alpha_late
  source_refs:
    - doc/game/project.md
    - .pm/tasks/task_3eb31966906e5ae7b8b8676d756c5510.execution.md
  tags:
    - stage
    - claim_envelope
  effective_at: 2026-03-30T19:00:00+08:00
  last_reviewed_at: 2026-03-30T19:00:00+08:00
  status: active
  confidence: confirmed
  promotion_reason: stage_decision
```

### 3.2 Role Memory Policy Template

```yaml
version: 1
close_phase_memory_questions:
  - 这条结论下个任务还会复用吗？
  - 这条结论如果不沉淀，其他 owner 很可能重复踩坑吗？
  - 这条结论会影响 PRD、实现、测试、阶段判断或对外口径吗？
roles:
  agent_engineer:
    topic_prefix_allowlist:
      - agent.recall.*
      - agent.goal_policy.*
      - agent.execution_policy.*
      - agent.failure_pattern.*
      - agent.context_pollution.*
    allowed_promotion_reasons:
      - agent_behavior
      - engineering_constraint
      - failure_signature
      - repro_pattern
```

### 3.3 Superseded Record

```yaml
- id: MEM-PRODUCER-0001
  role: producer_system_designer
  topic: stage.current
  summary: current stage remains internal_playable_alpha_late
  source_refs:
    - doc/game/project.md
  effective_at: 2026-03-21T10:00:00+08:00
  last_reviewed_at: 2026-03-30T19:00:00+08:00
  status: superseded
  superseded_by: MEM-PRODUCER-0008
  superseded_at: 2026-04-05T11:00:00+08:00
  supersede_reason: stage_upgraded
```

## 4. Promotion Contract

- 可提升:
  - 已确认的阶段结论
  - 已确认的失败签名
  - 已确认的对外口径边界
  - 重复出现的稳定模式
  - 关键工程约束
- `promotion_reason` 白名单:
  - `stage_decision`
  - `failure_signature`
  - `policy_boundary`
  - `stable_pattern`
  - `engineering_constraint`
  - `runtime_contract`
  - `abi_contract`
  - `agent_behavior`
  - `ux_constraint`
  - `repro_pattern`
  - `community_pattern`
  - `incident_pattern`
  - `test_strategy`
- 不可提升:
  - 一次性操作记录
  - 未验证猜测
  - 短期执行细节
  - 纯 task status 更新
- `reject_reason` 白名单:
  - `one_off_operation`
  - `unverified_hypothesis`
  - `short_lived_execution_detail`
  - `task_status_update`

## 5. Role Topic Boundary

| role | allowlist family | allowed reasons | disallowed examples |
| --- | --- | --- | --- |
| `producer_system_designer` | `stage.*` `claim_envelope.*` `player_access.*` `economy.*` `world_rule.*` `governance.*` | `stage_decision` `policy_boundary` `engineering_constraint` | 一次性版本讨论、未冻结玩法脑暴、当天执行流水 |
| `runtime_engineer` | `runtime.contract.*` `runtime.replay.*` `runtime.recovery.*` `runtime.state_machine.*` `runtime.failure_signature.*` | `runtime_contract` `engineering_constraint` `failure_signature` `repro_pattern` | 本次改了哪个函数、单次命令结果、临时 debug 过程 |
| `wasm_platform_engineer` | `wasm.abi.*` `wasm.permission.*` `wasm.manifest.*` `wasm.hash_contract.*` `wasm.lifecycle.*` | `abi_contract` `engineering_constraint` `failure_signature` | 一次性编译修复、临时兼容 hack、未确认 ABI 猜想 |
| `agent_engineer` | `agent.recall.*` `agent.goal_policy.*` `agent.execution_policy.*` `agent.failure_pattern.*` `agent.context_pollution.*` | `agent_behavior` `engineering_constraint` `failure_signature` `repro_pattern` | 单轮 prompt 尝试文本、未验证策略偏好、偶发模型情绪判断 |
| `viewer_engineer` | `viewer.ack_semantics.*` `viewer.observability.*` `viewer.error_surface.*` `viewer.usability_pattern.*` `viewer.web_test_contract.*` | `ux_constraint` `engineering_constraint` `failure_signature` `repro_pattern` | 单次样式偏好、临时布局挪动、个人审美判断 |
| `qa_engineer` | `qa.failure_signature.*` `qa.repro_path.*` `qa.gate_rule.*` `qa.regression_scope.*` `qa.test_strategy.*` | `failure_signature` `repro_pattern` `test_strategy` | 一次性执行流水、未稳定复现的瞬时失败、口头测试印象 |
| `liveops_community` | `community.messaging_boundary.*` `community.feedback_pattern.*` `community.incident_pattern.*` `community.escalation_rule.*` `community.channel_runbook.*` | `community_pattern` `incident_pattern` `policy_boundary` | 单条评论原文、未聚类零散抱怨、一次性活动排期记录 |

## 6. Close-Phase Extraction Rule

- `workflow-report --phase close` 默认提示三问:
  - 这条结论下个任务还会复用吗？
  - 这条结论如果不沉淀，其他 owner 很可能重复踩坑吗？
  - 这条结论会影响 PRD、实现、测试、阶段判断或对外口径吗？
- 任一回答为 yes 时，owner 至少执行其一:
  - 写 `signal`
  - 写 `working_memory`
  - 提升到长期 `memory`
- 若三问均为 no，则保留在 task execution log 或 task-scoped `working_memory`
- `shared` 只接收跨角色稳定结论，如 `gate.claim_envelope`、`release.policy.*`、`cross_role.workflow.*`

## 7. Script Surface

- `scripts/pm/promote-memory.sh`
  - 输入: `signal_id`、`scope`、`role`、`topic`、`promotion_reason`
  - `--scope shared` 仅允许 `producer_system_designer`
- `scripts/pm/supersede-memory.sh`
  - 输入: `memory_id`、`new_memory_id`、`supersede_reason`
- `scripts/pm/memory-report.sh`
  - 输出 active / stale / superseded 报表
  - 默认 stale 阈值 `7` 天
- `scripts/pm/memory-lint.sh`
  - 校验字段完整性、active 冲突、source ref 可达性、superseded 链
- `.pm/templates/role-memory-policy.yaml`
  - 记录 base `promotion_reason` 白名单、close-phase 三问和标准角色 `topic_prefix_allowlist`
- `scripts/pm/workflow-report.sh`
  - close phase 必须包含记忆抽取三问

## 8. 查询与消费

- role report:
  - 当前 active memory
  - `needs_review` 清单
  - 最新 superseded 链
- stage report:
  - 只读取 `producer_system_designer` 和 `shared` 的相关 active memory
- backlog / report:
  - 通过 `memory_refs` 引用 memory，不复制摘要

## 9. 风险控制

- 同 topic 同时存在多条 active: 用 lint 阻断
- memory 过期无人 review: 报表标记 `needs_review`
- signal 噪声污染 memory: 用 `promotion_reason` / `reject_reason` 白名单和 signal 决策回写限制

## 10. 使用方式

- 看正式规格与验收: `role-long-term-memory-2026-03-30.prd.md`
- 看 rollout / follow-up: `role-long-term-memory-2026-03-30.project.md`
- 看长期 memory 的 schema、promotion 与 role-topic 边界: 本文档
