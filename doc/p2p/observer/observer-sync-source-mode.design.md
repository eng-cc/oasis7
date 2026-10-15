# oasis7 Runtime：Observer 同步源策略化设计

- 对应需求文档: `doc/p2p/observer/observer-sync-source-mode.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 1. 设计定位
保留 Observer 同步源策略化的历史设计意图和当前负向边界。相关 source 当前未由 `oasis7_net` facade 暴露，本设计不把路径索引或 DHT 组合回退描述成 active runtime 能力。


## 当前基线、目标与风险

- Owner role：`runtime_engineer`；`qa_engineer` 负责验证范围与证据判读。
- 源码审读基线：`edf61e801e4888087c2e65a402011088bd1c50b2`（2026-10-15）。当前状态是 fixed-tree / facade 静态事实，不是 Cargo、S9A 或运行验收。
- 问题与目标：保留旧模式合同的完整义务，同时让维护者能区分 dormant 接口与仍在使用的基础 flow；只有对应实现重新接线且取得实际证据后才更新能力说明。
- 完整 tracked tree 中，`crates/oasis7_net/src/` 下旧 `execution_storage.rs`、`head_follow.rs`、`head_validation.rs`、`observer.rs`、`observer_metrics.rs`、`observer_replay.rs` 与 `bootstrap.rs` 均不存在；此结论不是由稀疏目录缺文件推得。
- 当前 [lib.rs](../../../crates/oasis7_net/src/lib.rs) 仍经 `observer_flow` 导出 `HeadFollowReport`、`HeadSyncReport`、`HeadSyncResult`、`compose_head_sync_report` 与 `follow_head_sync`，经 `observer_replay_flow` 导出 `load_manifest_and_segments`。[head_sync.rs](../../../crates/oasis7_net/src/head_sync.rs) 的基础报告/循环与 [replay_flow.rs](../../../crates/oasis7_net/src/replay_flow.rs) 的加载流程仍在模块图中；它们不提供本文旧模式/metrics 接口，也不能据此声称所有 Observer 或恢复功能已删除。
- [Cargo.toml](../../../crates/oasis7_net/Cargo.toml) 的 `runtime_bridge = []` 是兼容 feature；feature 名称或基础 flow 可编译不能证明旧模式重接、fallback 或 metrics 测试已执行。
- 产品适用性：N/A；理由：本次承接的是 dormant 旧接口的专业维护合同和静态能力边界，本身不新增玩家承诺；范围：本文旧模式/metrics 接口的保存与重新激活映射。真实世界恢复、共识连续性和玩家可用性继续由现行 P2P/runtime 与产品 authority 承担，本 N/A 不否认或免除它们。
- 剩余风险：旧报告名称与 active flow 同名部分可能造成误读；未来接线须重新核对兼容、调用方持有状态及错误传播，不能把本轮文档检查当运行完成。

## 2. 设计结构
- 选源策略层（重新激活目标）：`HeadSyncSourceMode` 定义 `NetworkOnly`（仅网络，失败返回）、`PathIndexOnly`（仅路径索引）、`NetworkThenPathIndex`（网络错误后才回退）。
- 接口接线（重新激活目标）：`ObserverClient::sync_heads_with_mode` 及对应报告、结果和循环须由 facade 明确导出；这里不新增名称或替代实现。
- DHT 模式集合（重新激活目标）：`NetworkWithDhtOnly` 只走网络+DHT；`PathIndexOnly` 只走路径索引；`NetworkWithDhtThenPathIndex` 在前段报错后才回退。
- DHT 组合层（重新激活目标）：以 `HeadSyncSourceModeWithDht` 表达网络+DHT、路径索引及二者有界回退，不改写基础同步语义。
- 切换状态层：维护同步源切换、恢复和失败状态机。
- 健康判定层：依据 lag、可达性和一致性判断同步源健康。
- 治理观测层：把策略口径、指标和日志沉淀为运维入口。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [SC-1](observer-sync-source-mode.prd.md#sc-1) | 准确保留 dormant 负向边界，未来恢复须保留错误与一致性。 | [设计定位](#1-设计定位) | runtime / blockchain ops / QA；上游各自 owning module | 不证明当前模式 API 或端到端恢复。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-1](observer-sync-source-mode.prd.md#ac-1) | 重新激活时提供非 DHT 策略枚举。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-2](observer-sync-source-mode.prd.md#ac-2) | 重新激活时实现 ObserverClient 模式接口、报告/结果/循环及 facade 导出。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-3](observer-sync-source-mode.prd.md#ac-3) | 保留历史目标模式集合，不把列表标题误作额外运行模式。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-4](observer-sync-source-mode.prd.md#ac-4) | NetworkOnly 仅走网络，失败直接返回。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-5](observer-sync-source-mode.prd.md#ac-5) | PathIndexOnly 仅走路径索引。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-6](observer-sync-source-mode.prd.md#ac-6) | NetworkThenPathIndex 仅在网络错误后回退。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-7](observer-sync-source-mode.prd.md#ac-7) | DHT 组合三模式保留原同步与一致性语义。 | [设计结构](#2-设计结构) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |
| [AC-8](observer-sync-source-mode.prd.md#ac-8) | 网络+DHT 报错才回退，二次失败保留两段诊断。 | [约束与边界](#4-约束与边界) | runtime / blockchain ops / QA；上游各自 owning module | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。  产品适用性：N/A；理由：本行是旧接口维护或 operator 技术观测合同，本身不新增玩家承诺；范围：本行所列专业机制，不免除现行产品恢复、经济或可用性义务。 |

## 3. 关键接口 / 入口

下列全部为未来重新激活目标，当前不提供这些旧模式接口：
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

以下是未来重新激活的历史顺序，不是当前实施排期或完成声明：
- 先冻结主策略和优先级。
- 再补切换与健康判定。
- 最后联动统一 metrics/observability 权威。

## 11. 验证与证据
### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 验证方法 / 精确来源 / candidate 与环境 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [SC-1](observer-sync-source-mode.prd.md#sc-1) | [设计定位](#1-设计定位) | 准确保留 dormant 负向边界，未来恢复须保留错误与一致性。 | 静态核对固定完整 tree、lib.rs 模块/facade 及 Cargo.toml，再对照本文当前基线；文档链接/内容检查仅验证文本关系。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 不证明当前模式 API 或端到端恢复。 |
| [AC-1](observer-sync-source-mode.prd.md#ac-1) | [设计结构](#2-设计结构) | 重新激活时提供非 DHT 策略枚举。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-2](observer-sync-source-mode.prd.md#ac-2) | [设计结构](#2-设计结构) | 重新激活时实现 ObserverClient 模式接口、报告/结果/循环及 facade 导出。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-3](observer-sync-source-mode.prd.md#ac-3) | [设计结构](#2-设计结构) | 保留历史目标模式集合，不把列表标题误作额外运行模式。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-4](observer-sync-source-mode.prd.md#ac-4) | [设计结构](#2-设计结构) | NetworkOnly 仅走网络，失败直接返回。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-5](observer-sync-source-mode.prd.md#ac-5) | [设计结构](#2-设计结构) | PathIndexOnly 仅走路径索引。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-6](observer-sync-source-mode.prd.md#ac-6) | [设计结构](#2-设计结构) | NetworkThenPathIndex 仅在网络错误后回退。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-7](observer-sync-source-mode.prd.md#ac-7) | [设计结构](#2-设计结构) | DHT 组合三模式保留原同步与一致性语义。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
| [AC-8](observer-sync-source-mode.prd.md#ac-8) | [约束与边界](#4-约束与边界) | 网络+DHT 报错才回退，二次失败保留两段诊断。 | 现在逐项核对配对 PRD 历史策略与本文映射；未来 runtime 重接后，按 `../../../testing-manual.md#s9a-gwsc-qa-receiver`（S9A） 选择适用 tier，新增并实际运行模式分发、成功/失败/回退与双错误的定向 Cargo 回归；当前无可执行旧模式 target，不借基础 flow 测试代签。  验证选择按 [testing manual](../../../testing-manual.md#s9a-gwsc-qa-receiver)；源码审读与未来实际执行分别记录。 | 同一候选的静态核对记录；未来实际命令、退出码、断言与适用环境分项记录 | 未实现的旧接口与行为仍待重新激活；不声明任何 tier 或 release 通过。 |
