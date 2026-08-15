# Viewer 发行体验总改造设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- 历史阶段实施与验收记录：GitHub task issue evidence。

## 1. 设计定位
本文是 Viewer-domain 的视觉/交互配对设计；产品语义仍以
[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](../../product/agents-world-simulation/player-readable-world-stage.prd.md)
为上游权威，终端 shell 的 Viewer 边界以配对 PRD 的
`Terminal Viewer Authority` 为准。本文不把目标 shell 当作已发货或发行放行事实。

## 2. 设计结构
- **Player shell（目标默认）**：全舞台世界棋盘、紧凑权威 HUD、低存在感 edge dock、按需 contextual console。
- **Director（目标显式 opt-in）**：可保留密集工具，但不改变玩法语义、进度或回执因果。
- **信息层级**：`World / Targets / Command` 为主路径；`Search` 是 Targets 内的
  `#entity-search` 兼容过滤/导航能力，`Diagnostics` 次级；`Quote` 属于 Command/Console 上下文。
- **反馈**：`Action Receipt` 负责玩家因果，World Feed 仅是待实现的环境上下文投影。

## 3. 当前实现与目标边界
- 当前实现保留 focus/right-panel 兼容 hooks，且 `#entity-search` 仍是 Targets
  过滤入口；这些是实现基线/债务，不是第二产品模式或终端 authority。
- 当前页面仍以 Recent Events/Gameplay Feedback 等现有投影为准；它们不能被文档重命名为已发货 World Feed。
- 目标契约暂不声称存在 `OASIS7_VIEWER_EXPERIENCE_MODE`、`ViewerExperienceMode`
  或 `RightPanel*` 运行时资源；需要这些字段时另开 runtime/viewer slice。
- 目标实现必须在既有语义数据上收敛 shell，并由 Viewer/QA 以真实桌面、移动、键盘和空/不可用状态验证。

## 4. 约束与边界
- 本设计只重排 Viewer 的视觉/交互，不改网络协议、仿真核心逻辑和 `third_party`。
- Search/selection 只导航、居中和打开上下文；执行只能从已授权 Command/Console 路径发生。
- queued/accepted 不等于完成；无 receipt 时显示明确的 `No action receipt yet`。
- 所有实现必须保留键盘焦点、关闭/`Escape`、CJK/长文案和无横向溢出的可读性契约。

## 5. 设计演进计划
1. 先冻结配对 PRD 的 shell、dock、console、HUD、receipt/feed 语义和稳定锚点。
2. 再由 `viewer_engineer` 在不新增协议字段前提下收敛当前实现的可见性/响应式布局。
3. 最后由 `qa_engineer` 以真实浏览器截图、键盘、长文案和空/不可用状态验证；本设计不构成发行结论。

## 6. 终端交互与实施边界
- Fresh load/reload/new tab/session 一律进入 Player。Director 只能由 Player 的
  capability-gated Diagnostics/operator action 显式打开，且仅改变可见性/密度、
  不持久化、不改变 command/auth/ownership/runtime 或 receipt 因果。
- Director entry 无效、过期、撤销或未授权时 fail closed 回 Player，清理 Director
  surfaces，保留 world/selection，并给出恢复文案。
- `World / Targets / Command` 是同一概念路径；Search 留在 Targets，Quote 留在
  contextual Command；选择只导航/高亮，不执行。
- Escape 先关闭局部 surface/drawer，再退出 Focus/presentation；IME composition
  期间由 IME 消费 Escape，关闭后焦点回到 invoker。
- 实施顺序固定为 anchors/tests → shell layout → console/receipt → focus/a11y →
  World Feed v1 → headed QA；真实截图、浏览器 smoke 和 runtime/QA verdict 仍是后续证据。

World Feed 仅作为 `world_feed/v1` additive ambient projection：identity/dedup 为
`(world_id,reorg_epoch,event_seq)`，source order ascending；nullable receipt link
只能来自显式 runtime causal identity。loading、empty、replay、gap/reorg 和
unavailable 不可合并，当前 Recent Events/Feedback 名称保持不变。
