# Viewer WebSocket/HTTP Bridge 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md`
- 历史 bridge 实施与验证记录：GitHub task issue evidence。

## 1. 设计定位
定义 Web Viewer 通过 WebSocket bridge 接入 live server 的协议、桥接生命周期与错误恢复策略。

## 2. 设计结构
- 服务端层：`oasis7_viewer_live --web-bind` 启动 bridge。
- 协议层：WebSocket text frame 与 TCP line protocol 互转。
- 客户端层：viewer wasm 接入 WebSocket 客户端与错误状态呈现。

## 3. 关键接口 / 入口
- `--bind` / `--web-bind` 参数
- `ViewerRequest/ViewerResponse` JSON 协议
- WebSocket 地址配置与 UI 状态接口

## 4. 约束与边界
- 浏览器断连时必须主动释放 upstream socket。
- Web 端只承担 Viewer + 网关接入，不承担完整分布式节点职责。
- 协议错误必须显式反馈到 UI 状态。

## 5. 设计演进计划
- 先完成 Design 补齐与互链回写。
- 后续实现与验证以新建 GitHub-backed task、当前代码和本合同为准。
