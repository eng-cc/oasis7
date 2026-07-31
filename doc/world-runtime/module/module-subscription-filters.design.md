# oasis7 Runtime：模块订阅过滤器（设计分册）设计

- 对应需求文档: `doc/world-runtime/module/module-subscription-filters.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义模块订阅过滤器设计，统一事件/动作路由前的过滤表达、匹配语义与拒收边界。

## 2. 设计结构
- 过滤表达层：用 JSON Pointer、等值/非等值、数值比较与正则规则描述订阅条件。
- 路由执行层：在模块调用前执行事件/动作过滤，避免无关模块被触发。
- 校验拒收层：对非法 filters 在 schema/shadow/apply 阶段直接拒绝。
- 回归验证层：覆盖事件过滤、动作过滤、OR 逻辑与数值/正则规则。

## 3. 关键接口 / 入口
- `ModuleSubscription.filters`
- 事件/动作路由过滤入口
- filters schema 校验
- 过滤器回归用例

## 4. 约束与边界
- 过滤规则必须保持确定性、可回放。
- 过滤失败不得产生额外副作用。
- 不在本专题支持复杂脚本型过滤逻辑。

## 5. 设计演进计划
- 先固化 filters 结构。
- 再补路由执行与非法配置拒收。
- 最后扩展 OR/数值/正则并固化测试。
