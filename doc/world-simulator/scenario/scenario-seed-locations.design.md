# oasis7 Simulator：场景种子化地点生成（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/scenario-seed-locations.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义场景种子化地点生成设计，让地点数量、命名和布局由 seed 驱动并保持稳定可复现。

## 2. 设计结构
- seed 驱动层：先确定场景级有效 seed；地点数量来自 `location_generator.count`，位置由有效 seed 与序号派生。
- 地点生成层：生成地点 ID、名称与位置表达。
- 场景接线层：把 seed 逻辑纳入场景文件与初始化。
- 回归校验层：验证不同 seed 下地点生成可复现且有区分度。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [普通地点确定性](scenario-seed-locations.prd.md#location-determinism) | 普通地点与 chunk trigger order 承诺分开 | [设计约束](#seed-input-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |
| [有效 seed](scenario-seed-locations.prd.md#location-effective-seed) | override 在生成前生效，显式坐标不重生成 | [设计约束](#seed-input-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |

## 3. 关键接口 / 入口
- 场景 seed 字段
- location_generator 配置
- 地点生成入口
- seed 回归用例

<a id="seed-input-design"></a>
## 4. 约束与边界
- 普通地点仅在有效 seed、空间与生成配置相同时保持一致；Agent 出生另依赖相同可用地点集合与枚举顺序。
- 内置场景的 seed override 在 `into_init_config` 生成地点之前应用；显式 `init.locations` 保留调用者坐标，不随 seed 重生成，配置 sanitization 仍适用。
- chunk/fragment 不继承无条件顺序无关性：其 world seed、effective config、trigger order 与历史 committed delta 规则以[专业合同](chunked-fragment-generation.prd.md)为准。
- 地点命名与布局需保持可读性。
- 不在本专题引入复杂随机分布 DSL。

## 5. 设计演进计划
- 先固化 seed 语义。
- 再接地点生成配置。
- 最后维护稳定性回归。
- 离线用例分别验证 override 前置、模板默认、显式坐标、Agent/fragment 派生和 chunk 触发顺序边界；不把文档补齐作为 runner 接线已修证据。


## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [普通地点确定性](scenario-seed-locations.prd.md#location-determinism) | [设计约束](#seed-input-design) | 普通地点与 chunk trigger order 承诺分开 | 候选版本的[验证入口](../../../crates/oasis7/src/simulator/tests/init.rs)：固定 seed/空间/配置和 Agent 可用地点；不声称跨 chunk 顺序无关；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
| [有效 seed](scenario-seed-locations.prd.md#location-effective-seed) | [设计约束](#seed-input-design) | override 在生成前生效，显式坐标不重生成 | 候选版本的[验证入口](../../../crates/oasis7/src/simulator/tests/init.rs)：无 override/不同 seed/显式坐标/Agent 与 fragment 派生；runner helper 回归仍待实现；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
