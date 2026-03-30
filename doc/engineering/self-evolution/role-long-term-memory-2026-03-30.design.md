# oasis7：角色长期记忆自建设计（2026-03-30）

- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`

审计轮次: 6

## 目标
- 为 `self-evolution` 运行层中的长期 memory 单独冻结文件结构、schema、状态机和脚本契约。
- 保证长期 memory 既不同于 `devlog`，也不同于 backlog/task registry。

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
- 不可提升到长期 memory：
  - 一次性操作记录
  - 未验证猜测
  - 短期执行细节
  - 纯 task status 更新

## 脚本设计
- `scripts/pm/promote-memory.sh`
  - 输入：`signal_id`、`role`、`topic`、`promotion_reason`
  - 输出：写入 active memory 或返回 reject
- `scripts/pm/supersede-memory.sh`
  - 输入：`memory_id`、`new_memory_id`、`supersede_reason`
  - 输出：旧记录移动到 superseded
- `scripts/pm/memory-report.sh`
  - 输出：按 role/topic/status 生成 active/stale/superseded 报表
- `scripts/pm/memory-lint.sh`
  - 检查字段完整性、active 冲突、source ref 可达性、superseded 链

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
  - 缓解：promotion_reason 白名单与 reject 规则

## 交付物
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/devlog/2026-03-30.md`
