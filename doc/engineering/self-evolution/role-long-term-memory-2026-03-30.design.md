# oasis7：角色长期记忆自建设计（2026-03-30）

- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`

审计轮次: 7

## 目标
- 为 `self-evolution` 运行层中的长期 memory 单独冻结文件结构、schema、状态机和脚本契约。
- 保证长期 memory 既不同于 task execution log，也不同于 backlog/task registry。

## 目标完成态
- 每个角色具备：
```text
.pm/roles/<role>/memory/
  active.yaml
  superseded.yaml
```
- 跨角色共享记忆具备：
```text
.pm/shared/memory/
  active.yaml
  superseded.yaml
```

## Schema 设计
### Active Record
```yaml
- id: MEM-PRODUCER-0001
  role: producer_system_designer
  topic: stage.current
  summary: current stage remains internal_playable_alpha_late
  source_refs:
    - doc/game/project.md
    - doc/devlog/2026-03-30.md
  tags:
    - stage
    - claim_envelope
  effective_at: 2026-03-30T19:00:00+08:00
  last_reviewed_at: 2026-03-30T19:00:00+08:00
  status: active
  confidence: confirmed
  promotion_reason: stage_decision
```

### Role Memory Policy Template
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
    disallowed_examples:
      - 单次 prompt 试验记录
      - 未验证的模型主观猜测
```

### Superseded Record
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

## Promotion 规则
- 可提升到长期 memory：
  - 已确认的阶段结论
  - 已确认的失败签名
  - 已确认的对外口径边界
  - 重复出现的稳定模式
  - 关键工程约束
- `promotion_reason` 白名单：
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
- 不可提升到长期 memory：
  - 一次性操作记录
  - 未验证猜测
  - 短期执行细节
  - 纯 task status 更新
- `reject_reason` 白名单：
  - `one_off_operation`
  - `unverified_hypothesis`
  - `short_lived_execution_detail`
  - `task_status_update`

## Role Topic Allowlist Draft
- `producer_system_designer`
  - allowlist：`stage.*`、`claim_envelope.*`、`player_access.*`、`economy.*`、`world_rule.*`、`governance.*`
  - allowed reasons：`stage_decision`、`policy_boundary`、`engineering_constraint`
  - 不允许：一次性版本讨论、未冻结的玩法脑暴、当天执行流水
- `runtime_engineer`
  - allowlist：`runtime.contract.*`、`runtime.replay.*`、`runtime.recovery.*`、`runtime.state_machine.*`、`runtime.failure_signature.*`
  - allowed reasons：`runtime_contract`、`engineering_constraint`、`failure_signature`、`repro_pattern`
  - 不允许：本次改了哪个函数、单次命令结果、临时 debug 过程
- `wasm_platform_engineer`
  - allowlist：`wasm.abi.*`、`wasm.permission.*`、`wasm.manifest.*`、`wasm.hash_contract.*`、`wasm.lifecycle.*`
  - allowed reasons：`abi_contract`、`engineering_constraint`、`failure_signature`
  - 不允许：一次性编译修复、临时兼容 hack、未确认的 ABI 猜想
- `agent_engineer`
  - allowlist：`agent.recall.*`、`agent.goal_policy.*`、`agent.execution_policy.*`、`agent.failure_pattern.*`、`agent.context_pollution.*`
  - allowed reasons：`agent_behavior`、`engineering_constraint`、`failure_signature`、`repro_pattern`
  - 不允许：单轮 prompt 尝试文本、未验证的策略偏好、偶发模型情绪判断
- `viewer_engineer`
  - allowlist：`viewer.ack_semantics.*`、`viewer.observability.*`、`viewer.error_surface.*`、`viewer.usability_pattern.*`、`viewer.web_test_contract.*`
  - allowed reasons：`ux_constraint`、`engineering_constraint`、`failure_signature`、`repro_pattern`
  - 不允许：单次样式偏好、临时布局挪动、个人审美判断
- `qa_engineer`
  - allowlist：`qa.failure_signature.*`、`qa.repro_path.*`、`qa.gate_rule.*`、`qa.regression_scope.*`、`qa.test_strategy.*`
  - allowed reasons：`failure_signature`、`repro_pattern`、`test_strategy`
  - 不允许：一次性执行流水、未稳定复现的瞬时失败、口头测试印象
- `liveops_community`
  - allowlist：`community.messaging_boundary.*`、`community.feedback_pattern.*`、`community.incident_pattern.*`、`community.escalation_rule.*`、`community.channel_runbook.*`
  - allowed reasons：`community_pattern`、`incident_pattern`、`policy_boundary`
  - 不允许：单条评论原文、未聚类的零散抱怨、一次性活动排期记录

## Close-Phase Memory Extraction Checklist Draft
- `workflow-report --phase close` 默认应提示三问：
  - 这条结论下个任务还会复用吗？
  - 这条结论如果不沉淀，其他 owner 很可能重复踩坑吗？
  - 这条结论会影响 PRD、实现、测试、阶段判断或对外口径吗？
- 任一回答为 yes 时，owner 至少执行其一：
  - 写 `signal`
  - 写 `working_memory`
  - 提升到长期 `memory`
- 若三问均为 no，则保留在 task execution log 或 task-scoped `working_memory`，不进入长期 memory
- `shared` 只接收跨角色稳定结论，例如 `gate.claim_envelope`、`release.policy.*`、`cross_role.workflow.*`

## 脚本设计
- `scripts/pm/promote-memory.sh`
  - 输入：`signal_id`、`scope`、`role`、`topic`、`promotion_reason`
  - accepted 输出：写入 active memory，并回写 signal 的 `memory_promotion_state=promoted`、`memory_id`、`memory_scope`、`memory_topic`
  - rejected/deferred 输出：不写 memory，只回写 signal 的 `memory_promotion_state` 与对应 reason
  - 约束：`--scope shared` 仅允许 `producer_system_designer`
- `scripts/pm/supersede-memory.sh`
  - 输入：`memory_id`、`new_memory_id`、`supersede_reason`
  - 输出：旧记录移动到 superseded
- `scripts/pm/memory-report.sh`
  - 输出：按 role/topic/status 生成 active/stale/superseded 报表
  - 默认 stale 阈值：`7` 天；支持 `--stale-after-days` 覆盖
  - 支持 `--role <role>` 与 `--no-shared` 过滤
- `scripts/pm/memory-lint.sh`
  - 检查字段完整性、active 冲突、source ref 可达性、superseded 链
- `.pm/templates/role-memory-policy.yaml`
  - 记录 base `promotion_reason` 白名单、close-phase 三问和 7 个标准角色的 `topic_prefix_allowlist` 草案
- `scripts/pm/workflow-report.sh`
  - close phase checklist 中必须包含记忆抽取三问，避免 owner 只写 execution log 不做结构化沉淀

## 查询与消费
- role report：
  - 每个角色当前 active memory
  - `needs_review` 清单
  - 最新 superseded 链
- stage report：
  - 只读取 `producer_system_designer` 和 `shared` 的相关 active memory
- backlog/report：
  - 通过 `memory_refs` 字段引用 memory，不复制摘要

## 风险与缓解
- 风险：同 topic 同时存在多条 active
  - 缓解：lint 阻断
- 风险：memory 过期无人 review
  - 缓解：report 标 `needs_review`
- 风险：signal 噪声污染 memory
  - 缓解：promotion_reason / reject_reason 白名单与 signal 决策回写

## 交付物
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- `.pm/templates/role-memory-policy.yaml`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/devlog/2026-03-30.md`
