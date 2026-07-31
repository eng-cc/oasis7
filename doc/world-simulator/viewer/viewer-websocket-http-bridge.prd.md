# Viewer WebSocket/HTTP Bridge 设计文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-websocket-http-bridge.design.md`
- 历史 bridge 实施与验证记录：GitHub task issue evidence。

审计轮次: 5

## 1. Executive Summary
- 为 `oasis7_viewer` 的 Web 端补齐在线连接能力，使浏览器可以观察 `oasis7_viewer_live` 的实时世界演化（含 `llm_bootstrap`）。
- 在不破坏现有 TCP viewer 协议的前提下，增加一条浏览器可用的桥接通道（WebSocket over HTTP Upgrade）。
- 建立最小在线闭环：`oasis7_viewer_live --web-bind` 启动后，Web Viewer 能收到 `snapshot/event/metrics` 并可发送 `control/prompt_control`。
- 明确角色边界：Web 端是 Viewer + 网关接入客户端，不承担完整分布式节点职责。

## 2. User Experience & Functionality
- 范围内：
  - 在 `oasis7` 增加 WebSocket bridge：`ws <-> tcp line protocol` 双向转发。
  - `oasis7_viewer_live` 增加 bridge 启停参数，支持同进程启动 bridge。
  - `oasis7_viewer` wasm 路径接入 WebSocket 客户端，替代“固定 offline”。
  - 更新使用手册与闭环策略文档，补充 `llm_bootstrap + web` 命令。
- 范围外：
  - 不在本阶段新增浏览器端重型 E2E 套件（保持 agent-browser CLI 最小闭环）。
  - 不改造 Viewer 业务协议字段（沿用现有 `ViewerRequest/ViewerResponse` JSON 协议）。
  - 不引入多租户会话管理或鉴权（默认本地开发地址）。
  - 不在浏览器端实现 `oasis7_node` 的完整分布式能力（gossip/replication/共识）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 服务端参数（`oasis7_viewer_live`）
- 新增：`--web-bind <addr>`
  - 例：`--web-bind 127.0.0.1:5011`
  - 含义：在同进程启动 WebSocket bridge，桥接到 `--bind` 指定的 TCP live 服务。
- 现有：`--bind <addr>`
  - 例：`--bind 127.0.0.1:5023`

### 2) Bridge 传输协议
- 浏览器侧：WebSocket text frame，payload 为单条 `ViewerRequest`/`ViewerResponse` JSON。
- 上游侧：现有 TCP 行协议（每行一条 JSON）。
- 规则：
  - `ws -> tcp`：text payload 追加 `\n` 后写入 upstream。
  - `tcp -> ws`：按行读取后去掉换行，作为 text payload 回推。
  - `ws disconnect -> upstream cleanup`：当浏览器连接关闭时，bridge 必须主动 `shutdown` upstream socket，确保上游 live server 会话立即释放并允许后续刷新重连。

### 3) Web Viewer 连接约定
- wasm 默认连接地址：`ws://127.0.0.1:5011`（可被 URL 参数覆盖）。
- URL 参数建议：`?ws=ws://127.0.0.1:5011`。
- 失败回退：连接错误时在 UI 状态区显示错误信息，可选择手动 offline。

## 5. Risks & Roadmap
- `WLB1`：文档建模（设计 + 项目管理 + 总项目入口）。
- `WLB2`：后端 bridge 与 `oasis7_viewer_live --web-bind` 落地。
- `WLB3`：web viewer wasm WebSocket 接入并通过 wasm 编译回归。
- `WLB4`：文档口径迁移 + Web 端闭环验证（agent-browser）+ 状态收口。

### Technical Risks
- 协议阻塞风险：bridge 任一侧阻塞可能导致另一侧堆积。
  - 缓解：采用读写分离与短周期轮询，连接断开时显式关闭 upstream socket 并回收读线程。
- 浏览器兼容风险：wasm 侧 WebSocket 错误处理不足会导致“假在线”。
  - 缓解：显式上报 `onerror/onclose` 到 `ViewerState.status`。
- 配置误用风险：`--bind` 与 `--web-bind` 混淆导致连错端口。
  - 缓解：手册给出固定推荐端口组合与一键命令。

## 6. Validation & Decision Record
- 当前 bridge 传输、断连清理和可见错误合同由本文及配对设计维护；历史实施与 Web 闭环证据由 GitHub task issue evidence 追溯。
