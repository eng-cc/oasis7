# Observer 子域入口

## 从这里开始
- 想理解 Observer 同步源策略化的基线：先读 `observer-sync-source-mode.prd.md`。
- 想理解 DHT 组合链路差异：先读主文档后，再读 `observer-sync-source-dht-mode.prd.md`。
- 想查同步模式 metrics / runtime bridge / observability：按专题名进入对应 PRD / design / project 三件套。

## 阅读面边界
- 本页只做 Observer 子域分流，不复制 `doc/p2p/prd.index.md` 的完整三件套长表。
- `observer-sync-source-mode` 是 source-mode 组的默认主入口；`observer-sync-source-dht-mode` 只承载 DHT 组合链路差异。
- metrics / runtime bridge / observability 组仍以 `doc/p2p/prd.index.md` 的 ROUND-002 主从说明为准。
- 完整文件级检索仍回到 `doc/p2p/prd.index.md`。

## 主从/增量组
| 组 | 默认入口 | 增量入口 |
| --- | --- | --- |
| Observer sync source mode | `observer-sync-source-mode.prd.md` | `observer-sync-source-dht-mode.prd.md` |
| Observer sync mode metrics | `observer-sync-mode-runtime-metrics.prd.md` | `observer-sync-mode-metrics-runtime-bridge.prd.md`, `observer-sync-mode-observability.prd.md` |

## 维护规则
- 新增 Observer 专题时，先判断是否是现有同步模式的增量差异；属于增量时只补本页和 `doc/p2p/prd.index.md` 的主从说明。
- 如果增量文档开始定义新的默认同步语义，应先提升为新的主入口，再更新本页分流。
