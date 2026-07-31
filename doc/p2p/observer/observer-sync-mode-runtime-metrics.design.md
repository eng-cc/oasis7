# oasis7 Runtime：Observer 同步源运行态统计设计

- 对应需求文档: `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## 1. 设计定位
定义 Observer 同步源运行态统计设计，把同步源选择、追平进度与回退状态纳入统一运行时指标。

## 2. 设计结构
- 状态采集层：采集同步源选择、追平高度和延迟。
- 模式报告层：非 DHT/DHT observed report 同时保留 mode、原始同步报告与真实 `fallback_used`。
- 自动记录桥接层：单轮成功报告记录一次，follow 按轮记录且复用原聚合/终止语义；metrics 继续由调用方持有。
- 指标输出层：沉淀 runtime metrics 与聚合字段。
- 告警信号层：为异常 lag、切源与失败建立告警基础。
- 回归验证层：通过场景回归校验指标准确性。

## 3. 关键接口 / 入口
- 同步源运行时状态
- `HeadSyncModeReport` / `HeadSyncModeWithDhtReport`
- 单轮/follow metrics bridge
- runtime metrics 字段
- 告警聚合入口
- 指标回归场景

## 4. 约束与边界
- 指标必须来源于真实运行态，不得手工拼装。
- 字段命名需与 observer 其他文档保持一致。
- fallback 标识必须在模式分发层计算，桥接不得引入隐式全局 metrics 或改变 `HeadFollowReport`。
- 不在本专题定义完整告警策略。

## 5. 设计演进计划
- 先固化运行态字段。
- 再接 metrics 输出。
- 最后补告警口径与回归。
