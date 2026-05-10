# Viewer WebSocket/HTTP Bridge（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-websocket-http-bridge.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）

### WLB1 文档建模
- [x] WLB1.1 输出设计文档（`doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md`）
- [x] WLB1.2 输出项目管理文档（本文件）
- [x] WLB1.3 在总项目文档挂载任务入口

### WLB2 后端 bridge
- [x] WLB2.1 实现 WebSocket <-> TCP line protocol 双向桥接
- [x] WLB2.2 `oasis7_viewer_live` 增加 `--web-bind` 参数并接入 bridge 生命周期
- [x] WLB2.3 补充测试并通过 `test_tier_required` 最小回归

### WLB3 Web Viewer 接入
- [x] WLB3.1 wasm 路径接入 WebSocket 客户端（替代固定 offline）
- [x] WLB3.2 支持 WebSocket 地址配置（默认 + URL 参数）
- [x] WLB3.3 通过 wasm 编译回归与 viewer 相关最小测试

### WLB4 文档与闭环收口
- [x] WLB4.1 更新 AGENTS/手册/运行路径文档（含 llm_bootstrap Web 命令）
- [x] WLB4.2 执行 Web 端闭环验证（live server + web viewer + agent-browser）
- [x] WLB4.3 更新项目状态、开发日志并收口

### WLB5 连接稳定性修复
- [x] WLB5.1 修复 websocket 刷新后无法重连（断连时主动释放 upstream socket）
- [x] WLB5.2 增加 bridge 重连回归测试（覆盖“首连断开后二次连接可用”）

### WLB6 口径澄清（Viewer + 网关）
- [x] WLB6.1 修订 `README.md`，明确 Web 端为 Viewer + 网关接入，不承担完整分布式节点职责
- [x] WLB6.2 修订 `doc/world-simulator/viewer/viewer-manual.md` 与本设计文档，统一“方案1”边界描述

## 依赖
- `doc/world-simulator/viewer/viewer-websocket-http-bridge.design.md`
- `crates/oasis7/src/viewer/live.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7_viewer/src/main.rs`
- `doc/world-simulator/viewer/viewer-manual.md`
- `AGENTS.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`
- `doc/world-simulator.project.md`

## 状态
- 当前阶段：WLB1~WLB6 全部完成。
- 下一步：按 Viewer + 网关口径维持回归，必要时补充 bridge 稳定性压力测试（慢消费者/并发连接）。
- 最近更新：2026-02-19（完成方案1边界口径修订）。
