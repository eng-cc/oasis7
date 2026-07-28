# Runtime 数值安全与原子状态转移契约

- 对应设计文档：`doc/world-runtime/runtime/runtime-numeric-safety.design.md`
- 对应项目文档：`doc/world-runtime/runtime/runtime-numeric-safety.project.md`
- 专业权威：`runtime_engineer`
- 吸收范围：已完成的 numeric-correctness phase 1–15 与 infinite-sequence rollover 专题。

## 1. 目标

Runtime、consensus 与 node 的权威状态不得因整数越界、隐式窄化或部分写入产生不可重放、不可恢复或节点间不一致的结果。玩家侧连续性与恢复承诺由 `doc/product/world-infrastructure/world-continuity-governance-and-recovery.prd.md` 承载；本文只定义专业数值语义。

## 2. 范围

### 永久契约

1. 权威计数、余额、票权、高度、slot、epoch、sequence、term 与过期时间的推进必须采用以下一种显式策略：
   - 受检算术：越界返回领域错误；
   - 有协议定义的 rollover：代际与序列一起持久化并保持兼容；
   - 有明确调用约定的 clamp：边界值可观测、可测试，且不得被描述成“成功计算了精确值”。
2. 禁止依赖 release 模式回绕或无说明的饱和算术掩盖错误。
3. 可能失败的状态转移必须先读取、校验并预计算所有后继值，再一次性提交。失败不得留下部分余额、proposal、head、lease、schedule 或 durable record。
4. replay 可见的 action 成本溢出必须形成明确拒绝结果；不得中断主循环后留下半提交。
5. snapshot 恢复、复制补洞或 writer 位置若无法表示下一个高度、slot、epoch 或 sequence，必须拒绝启动或拒绝本轮处理，不得构造第二条权威历史。
6. 错误必须保留足以定位字段和现场值的上下文，并由调用链显式传播或映射为稳定的领域错误。

## 3. 接口 / 数据

### 已覆盖的权威路径

| 子系统 | 数值安全语义 |
| --- | --- |
| World 资源与事件 | `ResourceStock`、`ResourceDelta` 和资源余额使用受检累加；资源转移、power redeem、经济结算等高风险事件先预计算再提交；tick 成本失败记录 `ActionRejected`。 |
| Node points 与 PoS | epoch 结算的 award/cumulative/total/epoch 后继值在写入前全部校验；stake 与 slot 越界失败不得污染 proposal 或投票状态。 |
| Node 高度、恢复与复制 | height/slot 后继值、gap-fill 进度、proposal 摄取及 snapshot 恢复使用受检语义；不可表示的恢复状态拒绝启动。 |
| Replication writer | 本地 writer 的 epoch/sequence 位置计算可失败，local commit 构造必须透传错误；同 writer、writer 切换和无 guard 三种边界均须覆盖。 |
| Sequencer 与 lease | proposal slot/height、lease term/expiry 的推进使用受检语义；失败保留原 proposal/head/lease。 |
| Membership 与时钟转换 | in-memory 与 store-backed coordinator 在插入或持久化前计算 expiry；可失败接口使用受检转换，明确采用 clamp 的辅助函数必须稳定返回边界值并有测试。 |
| PoS 比率与 required stake | 超多数配置使用无乘法溢出的 `> 1/2` 判定，并在 proto、consensus、node 三层保持一致；required-stake 使用加宽计算和可失败窄化，无法表示时拒绝。 |
| Membership recovery/replay | retry attempt、backoff timestamp、调度间隔、policy/rollback cooldown 与计数/容量聚合采用受检算术；阈值和比率比较不得以饱和乘法改变数学判断，失败不更新 replay、pending、dead-letter 或 policy 状态。 |
| Governance archive/audit | recovery drill、audit retention、tiered offload、rollback streak 与 alert window 的时间和计数运算采用受检语义；时间回拨或边界异常在 archive、governance、alert 状态写入前失败。 |
| Membership reconciliation / sync / mempool | 调度与 revocation-dedup 时间差、reconciliation/sync report 计数、mempool batch/zone payload 字节数采用 checked 算术；溢出以带现场值的 `WorldError::DistributedValidationFailed` 返回，schedule、dedup、report 与 batch 状态不部分更新。 |
| Federated replay archive | 聚合扫描、复合游标 offset 与 archive 查询计数采用 checked helper；溢出不污染 cursor 或聚合结果。 |
| Runtime sequence rollover | event/action/intent/proposal 四类 `next_*_id` 在 `u64::MAX` 后以持久化的 `era + 1, sequence = 1` 滚动；snapshot 的 era 字段以 `serde(default)` 兼容旧快照（era=0），恢复与 replay 保持同一 era。 |

## 4. 验收

- 每条可失败推进路径至少有一个边界测试，验证错误类型与状态未变。
- 涉及 durable write 的路径必须验证失败前未写入或写入事务整体回滚。
- 涉及 replay/recovery 的路径必须验证拒绝结果可观测，且不会生成新的 committed history。
- rollover 必须验证边界分配、snapshot roundtrip 与缺失 era 字段的 legacy load；它只消除溢出失效，纯 `u64` ID 在不同 era 极端远期仍可能复用，不能表述为全链路复合 ID 唯一性。
- 文档或实现若改变受检失败、rollover、clamp 三者之一，必须同步更新本文、对应 design/project 以及相关测试。

## 5. 里程碑

- M1：phase 1–12 的永久语义与当前代码/测试 evidence 完成核对。
- M2：稳定 PRD/design/project 三件套建立并接管专业权威。
- M3：phase 1–12 的 36 个阶段源文件、活动索引和历史入口分两批完成原子迁移。
- M4：文档治理、陈旧引用、定向测试和 frozen-head review 通过。

## 6. 风险与边界

- 本文不要求全仓库立即采用 BigInt 或统一 newtype。
- 本文不要求将纯 `u64` 外部 ID 立即迁为复合 ID。
- 本文不把实现路径、测试任务状态或运维阈值迁入产品 PRD。
- 本文不宣称模块、集成、长稳或发布就绪；发布证据仍由对应测试与任务 evidence 承载。

## 7. Validation & Decision Record

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-041 | runtime numeric safety | `test_tier_required` + 定向边界测试 | 文档治理、陈旧引用扫描、受检算术与原子性测试 | world、consensus、node、replication、membership |

| 决策 ID | 选定方案 | 否决方案 | 依据 |
| --- | --- | --- | --- |
| DEC-WR-NUMERIC-001 | 以稳定专业契约吸收已完成的 phase 1–6 | 长期保留六组三件套作为并列权威 | 阶段文档重复、设计模板化且项目状态已完成；稳定契约更利于防漂移。 |
| DEC-WR-NUMERIC-002 | 明确区分 checked failure、rollover 与 observable clamp | 统一写成“全部越界都失败” | 当前接口签名与兼容要求不同，clamp/rollover 是受约束的显式策略。 |
| DEC-WR-NUMERIC-003 | 以同一稳定专业契约吸收已完成的 phase 7–12 | 继续保留六组三件套与稳定契约并列 | phase 7–12 均是同一 numeric-safety 专业链中的完成记录；合并后仍以模块和边界测试保留精确证据。 |
