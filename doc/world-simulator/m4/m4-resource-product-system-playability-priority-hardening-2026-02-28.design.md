# M4 资源产业链可玩性优先强化（2026-02-28）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.project.md`

## 1. 设计定位
定义 M4 资源产业链可玩性优先强化设计，优先解决最影响游玩感受的产业链摩擦点。

## 2. 设计结构
- 优先问题层：定位最影响可玩性的产业链阻塞。
- 快速强化层：优先修正关键反馈、节奏和可理解性问题。
- 排程报价层：把维护 sink、稀缺延迟和折旧压力转成玩家确认前可比较信息。
- 维护 runway 层：把高负载折旧从“压力档位”补成剩余可生产时间、停机临界点和推荐动作。
- 保护约束层：确保强化不破坏系统合理性。
- 回归验收层：用重点场景复核体验改善。

## 3. 关键接口 / 入口
- 可玩性优先问题清单
- 快速强化入口
- `schedule_quote` 展示入口：维护预览、额外 tick、压力档位、`runway_before_ticks`、`runway_after_ticks`、`downtime_threshold_ppm`、推荐预备动作
- 保护性约束
- 体验复核场景

## 4. 约束与边界
- 优先强化应聚焦少数高影响点。
- 不得以牺牲系统一致性换取短期顺滑。
- 不得只用执行后日志解释维护/稀缺成本；排程前 quote 是玩家策略判断的一部分。
- 不得只用 `depreciation_pressure_class` 替代维护 runway；玩家需要知道继续排产后离 critical / 停机还有多远。
- `maintenance_runway_missing` 是 quote 可读性缺口，不代表需要重做维修系统或调整维护数值。
- 不在本专题扩展完整工业重做。

## 5. 设计演进计划
- 先排序高影响问题。
- 再逐项强化。
- 最后用体验场景复核。
