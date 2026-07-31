# oasis7 Simulator：场景种子化地点生成（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/scenario-seed-locations.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义场景种子化地点生成设计，让地点数量、命名和布局由 seed 驱动并保持稳定可复现。

## 2. 设计结构
- seed 驱动层：以统一随机根驱动地点数量和布局。
- 地点生成层：生成地点 ID、名称与位置表达。
- 场景接线层：把 seed 逻辑纳入场景文件与初始化。
- 回归校验层：验证不同 seed 下地点生成可复现且有区分度。

## 3. 关键接口 / 入口
- 场景 seed 字段
- location_generator 配置
- 地点生成入口
- seed 回归用例

## 4. 约束与边界
- 同 seed 必须产出相同地点集。
- 地点命名与布局需保持可读性。
- 不在本专题引入复杂随机分布 DSL。

## 5. 设计演进计划
- 先固化 seed 语义。
- 再接地点生成配置。
- 最后维护稳定性回归。
