# oasis7 Runtime：Observer 同步源策略化设计

- 对应需求文档: `doc/p2p/observer/observer-sync-source-mode.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 1. 设计定位
保留 Observer 同步源策略化的历史设计意图和当前负向边界。相关 source 当前未由 `oasis7_net` facade 暴露，本设计不把路径索引或 DHT 组合回退描述成 active runtime 能力。

## 2. 设计结构
- 选源策略层：定义 observer 在不同环境下的同步源优先级。
- DHT 组合层（重新激活目标）：以 `HeadSyncSourceModeWithDht` 表达网络+DHT、路径索引及二者有界回退，不改写基础同步语义。
- 切换状态层：维护同步源切换、恢复和失败状态机。
- 健康判定层：依据 lag、可达性和一致性判断同步源健康。
- 治理观测层：把策略口径、指标和日志沉淀为运维入口。

### 2.1 需求承接与分配表

| 上游需求（path#fragment） | 具体 obligation 与适用条件 | 本设计片段（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [PRD SC-1](observer-sync-source-mode.prd.md#1-executive-summary) | 保留同步源策略的历史设计目标，并明确当前负向能力边界。 | [设计定位](observer-sync-source-mode.design.md#1-设计定位) | `runtime_engineer` 维护模块图/facade 状态；`qa_engineer` 审核相关证据边界。 | 不声明当前 Observer source-mode API、恢复能力或测试通过。 |
| [PRD AC-1–AC-7](observer-sync-source-mode.prd.md#2-user-experience-functionality) | 将策略枚举、Observer 接口/facade 和目标模式列为未来重新激活时的设计目标。 | [设计结构](observer-sync-source-mode.design.md#2-设计结构) | 未来由 `runtime_engineer` 实现并接入 facade，`qa_engineer` 验证对应测试。 | 本设计不实现接口、不改变现行 crate facade，也不提供当前调用能力。 |
| [PRD AC-8](observer-sync-source-mode.prd.md#2-user-experience-functionality) | 未来激活时仅在网络+DHT 链路报错后回退，并保留两段错误诊断上下文。 | [约束与边界](observer-sync-source-mode.design.md#4-约束与边界) | 未来 runtime 实现和定向回归共同验证错误及回退边界。 | 不扩展其他触发条件，也不宣称当前存在 fallback 实现。 |

## 3. 关键接口 / 入口
- 同步源策略配置
- DHT 组合策略与双错误上下文
- 切换状态机
- 健康判定信号
- observer 运维读数

## 4. 约束与边界
- 同步源切换必须保持数据一致性优先。
- 状态机要可回放、可解释。
- 回退只由前置网络/DHT 错误触发；二次失败必须保留两段诊断信息。
- 不在本专题扩展新的 observer 身份体系。
- Git history 中的历史实现、completed 状态或空兼容 feature 均不构成当前 API；重新激活必须先由 runtime owner 重新实现并接入 crate facade，再补定向回归。

## 5. 设计演进计划
- 先冻结主策略和优先级。
- 再补切换与健康判定。
- 最后联动统一 metrics/observability 权威。

## 11. 验证与证据
### 11.1 验证映射表

| 上游需求（path#fragment） | 本设计片段（path#anchor） | 独立 obligation 与适用条件 | 验证方法、test/manual source、scenario/layer | Evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [PRD SC-1](observer-sync-source-mode.prd.md#1-executive-summary) | [设计定位](observer-sync-source-mode.design.md#1-设计定位) | 本次只记录当前 source-mode 边界；未来启用前须由 runtime 任务复核模块图和 facade。 | N/A: reason=Observer source-mode implementation is not in the active crate module graph; scope=documentation of historical intent and current boundary only; owner_role=runtime_engineer; evidence_ref=../../../testing-manual.md#s9a-gwsc-qa-receiver; re-evaluate=when a runtime task reimplements and exposes the source modes | 未来 runtime task 的模块/facade 检查及适用 S9A 验证证据。 | 不证明当前 source-mode API、observer recovery、replay、tier 通过或 release readiness。 |
| [PRD AC-1–AC-7](observer-sync-source-mode.prd.md#2-user-experience-functionality) | [设计结构](observer-sync-source-mode.design.md#2-设计结构) | 本次只映射未来策略和接口目标；未来启用时须实际实现并按适用层级验证。 | N/A: reason=the source-mode interfaces are future reactivation targets and have no active implementation in oasis7_net; scope=documentation mapping only; owner_role=runtime_engineer; evidence_ref=../../../testing-manual.md#s9a-gwsc-qa-receiver; re-evaluate=when a runtime task implements and connects the documented modes | 未来 runtime task 的定向 Cargo 测试和适用 S9A tier 证据。 | 不证明历史接口存在于当前 crate、当前测试覆盖这些接口或任一 tier 已通过。 |
| [PRD AC-8](observer-sync-source-mode.prd.md#2-user-experience-functionality) | [约束与边界](observer-sync-source-mode.design.md#4-约束与边界) | 本次只保留未来回退触发与双错误上下文的验证义务；当前无行为实现可测。 | N/A: reason=the documented fallback contract has no active observer implementation to exercise; scope=documentation mapping of the existing future contract only; owner_role=runtime_engineer; evidence_ref=../../../testing-manual.md#s9a-gwsc-qa-receiver; re-evaluate=when a runtime task implements the fallback paths | 未来 runtime task 的错误路径回归及适用 S9A tier 证据。 | 不证明当前 fallback 行为、错误上下文、恢复能力或 S9A tier 通过。 |
