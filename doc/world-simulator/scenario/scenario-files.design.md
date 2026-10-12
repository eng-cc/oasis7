# oasis7 Simulator：场景文件化（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/scenario-files.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义场景文件化设计，把内置 `WorldScenario` 迁移为 JSON 场景文件并作为单一权威来源。

## 2. 设计结构
- 文件模型层：用场景 JSON 描述 seed、地点生成与可选碎片配置。
- 加载接线层：内置 `include_str!` 修改需重编译；demo `--scenario-file` 在初始化时读取给定 JSON。两者共用解析与校验，不提供生产 runtime 热更新。
- 生成表达层：用 `location_generator`、origin 等字段驱动初始化。
- 有效配置层：初始化时复制 `WorldConfig.asteroid_fragment` 后仅应用 JSON 显式 override；不修改全局默认值，也不提供运行期 hot configuration。
- 测试矩阵层：维护场景到测试目标的映射与稳定性校验。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [加载与 demo 例外](scenario-files.prd.md#scenario-loading-contract) | include_str 重编译与 demo 显式文件初始化；同解析与校验 | [设计约束](#scenario-loading-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |
| [消费入口矩阵](scenario-files.prd.md#场景初始化与消费入口矩阵) | Simulator/named runtime-live/generated-world 的模型保留与默认 Agent 分开 | [设计约束](#scenario-loading-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |

## 3. 关键接口 / 入口
- `crates/oasis7/scenarios/*.json`
- `WorldInitConfig::from_scenario`
- `WorldScenario::parse`
- `oasis7_init_demo --scenario-file`

<a id="scenario-loading-design"></a>
## 4. 约束与边界
- 内置场景文件需保持单一来源。
- 普通地点按相同有效 seed、空间和生成配置可复现；chunk/fragment 另外固定 effective config 与 trigger order。
- 消费者的模型保留与默认 Agent 语义以[初始化入口矩阵](scenario-files.prd.md#场景初始化与消费入口矩阵)为准；纯碎片场景当前使用 Simulator 离线入口验收，不推荐普通 named runtime-live Web 回归作为零 Agent 模型证明。
- `asteroid_fragment` 是唯一规范字段名；`min_fragment_spacing_cm` 缺省继承 `50_000` cm，`<= 0` 关闭额外 spacing。effective config 必须进入固定 seed/触发序列的确定性上下文，replay 只应用 committed delta。
- 只有显式 `power_bootstrap` 数据可注入 `power_plants`；其他内置和 `asteroid_fragment*` 场景无隐式设施。该注入不改变 runtime 电力规则、storage/动作 ABI、链上经济或治理边界。
- 不在本专题扩展复杂 DSL 与版本迁移工具。

## 5. 设计演进计划
- 先迁移内置场景为 JSON。
- 再接入加载逻辑与调试入口。
- 最后维护场景测试覆盖矩阵。


## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [加载与 demo 例外](scenario-files.prd.md#scenario-loading-contract) | [设计约束](#scenario-loading-design) | include_str 重编译与 demo 显式文件初始化；同解析与校验 | 候选版本的[验证入口](../../../crates/oasis7/tests/oasis7_init_demo.rs)：核对自定义文件与内置场景入口、不可读/解析失败，不宣称热更新；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
| [消费入口矩阵](scenario-files.prd.md#场景初始化与消费入口矩阵) | [设计约束](#scenario-loading-design) | Simulator/named runtime-live/generated-world 的模型保留与默认 Agent 分开 | 候选版本的[验证入口](../../../crates/oasis7/src/simulator/tests/init.rs)：核对纯碎片四 chunks、无 origin/Agent；Web 保留入口未实现则不宣称通过；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
