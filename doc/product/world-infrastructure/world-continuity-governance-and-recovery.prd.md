# 世界连续性、治理与恢复（已退休，语义迁移中）

## 文档身份

- 所属产品模块：大世界基础设施
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`retired`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

## 迁移状态

本页保留完整历史语义和验收，不能作为大世界基础设施的 active authority、路线图或验收入口。原第 2–3 节的唯一权威历史、确定性结果、checkpoint/replay/state-sync 与恢复语义，已分别映射到 [`分布式共识与状态可用性`](distributed-consensus-and-state-availability.prd.md) 的 DC-1/DC-2 和 [`确定性世界执行`](deterministic-world-execution.prd.md) 的 DE-1/DE-3；其实现/证据仍由 P2P/runtime authority 拥有。原第 5 节的系统性危机 containment、同一世界恢复项目与无选择性 bailout 语义，已由 [`区域冲突、软赛季与可恢复损失`](../world-rules-core-gameplay/chartered-conflict-soft-seasons-and-recovery.prd.md) 第 6 节和 AC-6/AC-7 接收。原第 4–6 节其余玩家治理、惩罚/申诉和长期经济/发行产品语义尚未被接收模块完整吸收，仍是待迁移语义，接收 authority 为 [`世界规则与核心玩法`](../world-rules-core-gameplay/prd.md)。在对应 owner 回填剩余语义、验收和活跃引用前不得删除本页。


本文是长期产品分册，承载统一持久世界在分布式运行、治理变化、故障与恢复中的产品承诺。它不定义证书字段、共识算法、拓扑、阈值、测试命令、运行手册或历史候选结论。

## 1. 产品目标

玩家始终面对同一个权威世界：节点传播、网络分区、治理变化、故障处置或恢复不会产生第二条玩家历史，也不会静默丢失、重复或改写已确认结果。服务退化时进入受控保护与可解释恢复，恢复后能够证明世界连续性。

## 2. 权威世界不分叉

- 只有权威裁决并提交的状态可以成为玩家世界结果；传播、副本、缓存或本地推测不能取得最终写入权。
- 同一世界、执行版本、逻辑时间和已排序输入必须产生一致结果；本地墙钟或非权威随机源不能改变状态迁移。
- 冲突、验签失败或非权威写入必须原子拒绝并留下可审计原因，不能进入玩家可见历史。
- 节点切换、落后追赶或网络分区不能让玩家观察到两条均被包装为有效的世界时间线。

## 3. 回放、恢复与玩家结果连续

- 已确认行动、资源变化、设施状态、治理结果与 receipt 必须能通过专业域定义的 snapshot、checkpoint、replay、replication 或 state sync 恢复。
- 异构或局部退化的分布式状态底座应保持玩家可见世界结果可恢复、可连续，不以某个特权的全量数据节点作为唯一前提；这不是对具体拓扑、容量、SLA、readiness 或所有故障均可自主恢复的承诺，也不在产品层规定实现方式。
- 漂移或损坏必须定位到首个不一致边界，按最小影响范围回滚、重放并完成对账后，才能重新对玩家暴露为已恢复。
- 恢复后，同一结果身份与主要因果保持连续；无法保持时必须明确阻断、说明受影响范围和下一恢复步骤。
- 重试、重复投递、重复惩罚或重复结算不得产生第二次世界效果。
- transport 或节点存活 green 不能代签世界状态、玩家结果与历史连续性已经恢复。

## 4. 治理变化可预期且不可旁路

- 改变世界规则、关键参数、经济边界或高影响权限的操作必须来自授权、可审计的治理流程，并在专业合同定义的安全边界生效。
- 提前、越权、证据不足或绕过治理的应用必须原子拒绝，不能通过运维便利静默改写世界规则。
- 玩家和审计者能够理解变化的对象、授权来源、生效边界与结果；产品层不冻结具体 epoch、timelock、投票权重或签名阈值。

## 5. 紧急处置、惩罚与申诉

- 冻结、否决、降权、惩罚或其他高影响保护动作必须具备明确权限、理由、证据、审计和恢复/申诉路径。
- 紧急机制用于限制故障扩散，不得成为绕过治理、永久改写历史或取消申诉的旁路。
- 恢复或复核结果必须持久化并可重放；重复证据或请求不能重复生效。
- 安全与反滥用策略的算法、阈值和运营处置由 P2P、runtime、gameplay 与运维专业权威共同拥有，产品层只约束可解释、公平和可恢复结果。
- 系统性危机的 containment 只可保护连续性、限制扩散并维持基本权利；恢复应通过同一世界内可审计的玩家/Agent/组织项目推进，不得 reset 世界、选择性抹去已确认因果或以临时偏袒性 bailout 取代公开规则。

## 6. 长期经济与发行健康

- 权威资源 source/sink、关键经济变化和治理影响必须可审计；异常不能被单节点、单专题或短时 green 掩盖。
- 长期世界候选放行需要 runtime、P2P、恢复与 QA 证据绑定同一候选版本；必要时包含多节点长跑、故障注入和恢复对账。
- 任一世界连续性硬门失败或缺证时，保持阻断或较低承诺，不把“服务仍在线”包装成世界状态健康。
- 具体 SLO、告警阈值、演练频率、值守时序与 runbook 留在测试和运维权威。

## 7. 组合验收

- CR-1：非权威写入、冲突状态或验签失败被原子拒绝，且玩家不会看到第二条有效世界历史。
- CR-2：同版本、同输入回放保持权威结果一致；故障、snapshot/state sync 和重连后，已确认行动、资源、治理结果与 receipt 保持连续。
- CR-3：治理变化只能通过授权、可审计流程生效；提前、越权或证据不足的应用失败且无部分副作用。
- CR-4：紧急处置、惩罚与申诉形成权限、证据、执行、恢复/复核的持久闭环，重复请求不产生重复效果。
- CR-5：长期世界放行将权威执行、共识复制、恢复、经济审计与 QA 证据绑定同一候选；单节点或局部 green 不得代签。
- CR-6：系统性危机样例证明 containment、恢复项目、审计和申诉在同一世界历史中运行；不存在 reset、历史重写或选择性 bailout。

### 7.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| CR-1 | runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-P2P-002 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 权威写入、冲突拒绝、确定性及多节点分歧对账证据；必须包含匹配拓扑和故障场景的 S9/S10，单节点 full 不得代签 | test_tier_full |
| CR-2 | runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-003 / PRD-WORLD_RUNTIME-014 / PRD-P2P-002 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | replay、checkpoint、replication/state sync、故障注入、重连和恢复后结果对账；必须包含匹配恢复场景的 S9/S10，单节点 replay 不得代签 | test_tier_full |
| CR-3 | gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 授权治理流程、越权/提前应用拒绝、回放一致与多节点最终性证据 | test_tier_full |
| CR-4 | runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 紧急权限、惩罚/申诉、重复拒绝与持久恢复状态机证据 | test_tier_full |
| CR-5 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-006 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-P2P-002 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 同候选 gameplay 经济规则、runtime、P2P 与恢复证据的完整 release gate，必须同时包含 test_tier_full、S8、S9 和 S10 | test_tier_full |
| CR-6 | producer_system_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | containment、恢复项目、审计/申诉、同一世界连续性与无 reset/bailout 负例 | test_tier_full |

## 8. Non-Goals

- 不定义 TickBlock、certificate、hash、state root、签名或协议 schema。
- 不定义 validator、observer、relay、bootstrap、拓扑或 signer custody 实现。
- 不冻结 epoch、timelock、抵押、信誉、惩罚、经济或反女巫阈值。
- 不保存测试命令、run ID、告警指标、SLO、MTTA/MTTR、演练频率或历史 pass。
- 不替代 incident、rollback、灾备、节点恢复或发布 runbook。
