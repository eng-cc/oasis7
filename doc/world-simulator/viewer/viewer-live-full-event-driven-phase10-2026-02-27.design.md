# Viewer Live 完全事件驱动 Phase 10 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.project.md`

## 1. 设计定位
定义 viewer 活跃运行链路彻底去 tick/poll 化的收口方案：移除 offline server 的定时回放推进和 web bridge 的轮询间隔配置，把活跃 viewer 路径统一成事件驱动模型。

## 2. 设计结构
- offline server 收敛层：删除 `ViewerServerConfig.tick_interval` 与定时回放推进。
- web bridge 收敛层：删除 `ViewerWebBridgeConfig.poll_interval` 与轮询 sleep。
- mailbox 语义层：只有可处理的外部 mailbox/event 触发才请求 drive；空 mailbox 和未产出 action/event 的 drive 不得制造 snapshot event、logical-time 增量或完成回执。
- 入口文档收敛层：同步清理 `--tick-ms` 与旧轮询示例。
- 阶段合并层：Phase 8/9 增量记录物理合并到 Phase 10 主文档，形成唯一权威入口。

## 3. 关键接口 / 入口
- `ViewerServerConfig`
- `ViewerWebBridgeConfig`
- `viewer/server.rs` / `viewer/web_bridge.rs`
- `site/index.html` / `site/en/index.html`

## 4. 约束与边界
- 本阶段只清理 viewer 活跃链路，不触碰 node/runtime 基础 tick 机制。
- `tick` 不能作为 event-drive 的外部时钟承诺；若当前实现暴露兼容字段，它只可表示 runtime 已观察到的逻辑时间。
- Play 从定时推送改成事件触发批量输出后，前端体验变化需由 viewer 范围内兜底。
- 历史归档文档与 devlog 不在这轮重写范围。
- Phase 8/9 合并后以主文档为唯一活跃入口，避免多个阶段文件继续漂移。

## 5. 设计演进计划
- 先去 offline server tick。
- 再去 web bridge poll 与旧入口参数。
- 最后完成测试、示例和阶段文档收口。
