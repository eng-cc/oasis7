# oasis7 Runtime：无限时长运行的序列号滚动与数值防溢出

- 对应设计文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.design.md`
- 对应项目管理文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md`

审计轮次: 4


## 1. Executive Summary
- 让 Runtime 在超长时间运行场景下保持可持续，不因计数器上溢导致 panic 或静默回绕。
- 对关键数值累加路径增加防溢出策略，避免 release 模式下出现不可见错误。
- 保持现有协议与数据结构兼容，优先做增量增强，不引入大规模类型迁移。

## 2. User Experience & Functionality
### In Scope
- Runtime 内四类序列计数器增强：
  - `next_event_id`
  - `next_action_id`
  - `next_intent_id`
  - `next_proposal_id`
- 为序列计数增加 era（代际）状态，在 `u64::MAX` 时执行“era+1 + seq 重置”。
- snapshot 持久化增加序列 era 字段，并保持对旧快照的反序列化兼容。
- 修复关键未保护数值加法和窄化转换风险（`len as u32`、`u64 as i64`）。

### Out of Scope
- 全仓库引入 `BigInt`。
- 全量把 `u64` ID 类型升级为复合结构并改动外部协议字段。
- 改造历史 journal 结构或追加全量历史压缩系统。


## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（文档迁移任务）。
- Evaluation Strategy: 通过文档治理校验、引用扫描与任务日志检查验证迁移质量。

## 4. Technical Specifications
- `Snapshot` 新增（默认 0）：
  - `event_id_era`
  - `action_id_era`
  - `intent_id_era`
  - `proposal_id_era`
- `World` 内部新增对应 era 状态，并在分配 ID 时执行滚动逻辑：
  - 正常：`seq += 1`
  - 边界：`seq == u64::MAX` 后下一次分配触发 `era = era + 1, seq = 1`
- 关键防溢出改造：
  - 资源与规则聚合采用 `runtime-numeric-safety` 定义的受检错误语义；本专题不再为这些路径定义饱和成功。
  - 模块输出条目数量比较改为 `u32::try_from(len)`。
  - `snapshot.state.time -> i64` 改为 `i64::try_from` 并在越界时报错。

## 5. Risks & Roadmap
- M0：建档与任务拆解完成。
- M1：序列滚动（era + seq）与 snapshot 兼容落地。
- M2：关键数值防溢出修复（累加、窄化转换）。
- M3：定向回归测试通过并文档收口。

### Technical Risks
- 新增 snapshot 字段可能影响旧数据兼容。
  - 缓解：新增字段全部 `serde(default)`，旧快照按默认 era=0 读取。
- 序列跨 era 后，纯 `u64` ID 在极端远期会出现值复用。
  - 缓解：在 runtime 内持久化 era 并持续推进；当前阶段先解决“溢出失效”而非“全链路复合 ID 改造”。
- rollover 仅适用于已持久化 era 的序列；其他关键数值若越界必须遵循 `runtime-numeric-safety` 的显式失败或明确 clamp 契约。

## 当前状态
- 截至 2026-02-22：M0（建档）、M1（序列滚动与快照兼容）、M2（数值防溢出加固）、M3（回归与收口）均已完成。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-006 | 文档内既有任务条目 | `test_tier_required` | `./scripts/doc-governance-check.sh` + 引用可达性扫描 | 迁移文档命名一致性与可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-DOC-MIG-20260303 | 逐篇阅读后人工重写为 `.prd` 命名 | 仅批量重命名 | 保证语义保真与审计可追溯。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章 Executive Summary。
- 原“范围” -> 第 2 章 User Experience & Functionality。
- 原“接口 / 数据” -> 第 4 章 Technical Specifications。
- 原“里程碑/风险” -> 第 5 章 Risks & Roadmap。
