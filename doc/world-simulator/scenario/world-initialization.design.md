# oasis7 Simulator：世界初始化（设计分册）设计

- 对应需求文档: `doc/world-simulator/scenario/world-initialization.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义世界初始化设计分册，统一场景、默认实体、设施与启动状态在初始化阶段的装配顺序。

## 2. 设计结构
- 初始化装配层：按场景与配置生成世界初始模型。
- 显式装配层：遵循 origin.enabled、agents.count 和场景资源配置；设施只遍历 `power_plants`，没有按区域/碎片场景名称补设施的默认分支。
- 配置覆盖层：允许场景与启动参数对初始化细节做有限覆盖。
- 验证回归层：通过 init 测试和 demo 校验世界初始化稳定。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [显式设施](world-initialization.prd.md#init-explicit-facilities) | 默认区域/碎片场景设施为 0，power_bootstrap 显式正例，单一矩阵 | [设计约束](#init-state-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |
| [时间与初始事件](world-initialization.prd.md#init-time-events) | 时间 0 可有 Init 事件，snapshot 与 journal 覆盖范围分开 | [设计约束](#init-state-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |

## 3. 关键接口 / 入口
- World 初始化入口
- 场景到 world model 装配
- 默认实体/设施注入点
- 初始化回归用例

<a id="init-state-design"></a>
## 4. 约束与边界
- 初始化顺序需保持确定性。
- 场景能力只在[主矩阵](scenario-files.prd.md#场景测试覆盖矩阵2026-02-06)维护；`twin_region_bootstrap` 与 `asteroid_fragment_bootstrap` 默认设施数为 0，设施正例使用 `power_bootstrap`。
- `initialize_kernel` 保持时间 0；enabled fragment 的已生成 chunk 可追加 `ChunkGenerated(Init)`，不清空合法事件。snapshot 承载全部初始状态，journal 只按[分块生成合同](chunked-fragment-generation.prd.md)承载已声明部分。
- 初始化产物交给不同消费者时按[入口矩阵](scenario-files.prd.md#场景初始化与消费入口矩阵)验收；named runtime-live 的默认 Agent 与 seed model 投影不能冒充 Simulator 原模型保留。
- 场景覆盖不得绕过核心一致性校验。
- 不在本专题扩展运行时动态重建世界。

## 5. 设计演进计划
- 先收敛初始化装配顺序。
- 再对齐场景覆盖和默认注入。
- 最后固化 init 回归。
- 验收读取模型/快照中的设施、Agent、origin、chunk 与 Init 事件，不只检查解析成功；使用已有 init/persist 离线回归，独立记录实际执行范围。


## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [显式设施](world-initialization.prd.md#init-explicit-facilities) | [设计约束](#init-state-design) | 默认区域/碎片场景设施为 0，power_bootstrap 显式正例，单一矩阵 | 候选版本的[验证入口](../../../crates/oasis7/src/simulator/tests/init.rs)：场景模型数目与设施配置对照；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
| [时间与初始事件](world-initialization.prd.md#init-time-events) | [设计约束](#init-state-design) | 时间 0 可有 Init 事件，snapshot 与 journal 覆盖范围分开 | 候选版本的[验证入口](../../../crates/oasis7/src/simulator/tests/persist.rs)：启用 chunk、关闭 fragment 的初始事件与持久快照对照；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
