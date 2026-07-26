# 玩家可读表面连续性迁移追踪

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/player-readable-surface-continuity.prd.md`](player-readable-surface-continuity.prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-surface-continuity.design.md`](player-readable-surface-continuity.design.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 追踪范围：产品文档语义迁移
- Owner role：`repository_health_engineer`

本文只记录产品语义归位和删除边界，不维护执行任务、PR、CI、发布或当前能力状态。

## 迁移映射

| 已吸收源专题 | 归位语义 | 未提升为产品承诺 |
| --- | --- | --- |
| `viewer-right-panel-module-visibility` 三件套 | 次级内容可收起且主要决策面可恢复 | EGUI module、可见性资源、缓存路径与固定右栏 |
| `viewer-player-ui-declutter-2026-02-24` 三件套 | 降低密度和遮挡时保留目标、blocker、反馈与下一步 | 具体 panel、宽度、布局、hit boundary 与旧测试 |
| `viewer-web-fullscreen-panel-toggle` 三件套 | 可用空间变化不切断关键决策链 | fullscreen toggle、DOM/WASM 行为与历史兼容实现 |
| `viewer-i18n` 三件套 | 支持语言具有明确选择、fallback 与关键文本覆盖 | locale 自动选择、字体资产、翻译键、缓存和同步 |
| `viewer-web-usability-hardening-2026-02-22` 三件套 | 连接状态真实可读且存在恢复路径 | websocket、callback、backoff、toast 与实现命令 |
| `viewer-step-completion-ack-2026-02-28` 三件套 | 产品语义归入 gameplay 分册：接受与观察到的完成分离，无进展保留下一决策 | request_id、ack enum/字段、delta、时序、兼容与测试 API |

## 删除收据

- 本批吸收并删除源文件：18。
- surface 连续性由本专题 PRD/design 承载；step acknowledgement 的玩家承诺由 `indirect-control-agency-and-continuation.prd.md` 和 `first-session-and-continuation.prd.md` 承载。
- 当前 Viewer 手册、Web 专业合同、proto/runtime 代码与测试继续拥有实现、协议、兼容和验证真值。
- `viewer-egui-right-panel` 明确保留为 legacy EGUI 专业追溯，不属于本批。

## 完成条件

仅在六组三件套全部删除、活跃索引和 incoming references 修复、负向历史引用 guard 保留、产品文档未承诺旧控件或协议字段时，本迁移收据有效。Git history 与 GitHub task evidence 保留历史任务追溯。

## 任务拆解

不适用。任务状态只进入 GitHub task issue evidence。

## 依赖

- [`doc/product/README.md`](../README.md) 的产品与专业 authority 边界。
- [`doc/world-simulator/viewer/viewer-manual.manual.md`](../../world-simulator/viewer/viewer-manual.manual.md) 的当前玩家/操作 surface。
- [`doc/world-simulator/prd.md`](../../world-simulator/prd.md) 与 [`doc/testing/prd.md`](../../testing/prd.md) 的 Viewer、协议和验证真值。

## 状态

- 文档生命周期：`active`
- 迁移收据：`finalized`
