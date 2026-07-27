# oasis7 Runtime：无限时长运行的序列号滚动与数值防溢出设计

- 对应需求文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md`
- 对应项目管理文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md`

## 1. 设计定位
定义无限时长运行中的序列号滚动与数值防溢出设计，确保长期运行场景下序列、游标与计数器保持单调和可恢复。

## 2. 设计结构
- 序列管理层：为长期运行的 sequence/cursor 定义滚动策略。
- 溢出防护层：在接近边界前执行窄化、重置或迁移，避免整数溢出。
- 恢复兼容层：让滚动后的序列仍可被重放、同步与校验。
- 观测回归层：暴露边界接近、已滚动和恢复状态。

## 3. 关键接口 / 入口
- sequence rollover 策略
- 边界检测与切换入口
- 重放/同步兼容接口
- 滚动回归用例

## 4. 约束与边界
- 序列语义必须保持单调和可验证。
- 滚动不得破坏重放或同步兼容性。
- 不在本专题扩展新的全局 ID 体系。
- 本专题仅拥有已持久化 era 的 rollover 语义；余额、票权、高度、slot、lease 与 membership 时间计算遵循 `runtime-numeric-safety` 的受检失败或显式 clamp 契约。

## 5. 设计演进计划
- 先明确边界与滚动规则。
- 再接入恢复与兼容校验。
- 最后沉淀长期运行回归。
