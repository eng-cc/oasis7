# Local Provider 本地 HTTP Provider 接入 world-simulator 首期方案（2026-03-12）设计

- 对应需求文档: `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.prd.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.project.md`

## 1. 设计定位
定义“安装在用户机器上的 `Local Provider` 如何通过本地 HTTP 参与 world-simulator 的 agent 决策”的首期工程方案，覆盖本地发现、握手、配置、决策请求、反馈回写、状态可观测与失败回退。

## 2. 设计结构
- 用户侧本地 provider 层：`Local Provider` 以独立本地服务运行，仅监听 `127.0.0.1`。
- Adapter 层：world-simulator 内新增 `Local ProviderAdapter`，把 `DecisionRequest/FeedbackEnvelope` 与本地 HTTP API 互转。
- 配置与发现层：launcher 负责 provider 模式选择、base URL/token 配置、发现与 health-check。
- 运行与裁决层：runtime/kernel 继续负责动作白名单、规则校验、状态演化与事件产出。
- 观测层：viewer 与 launcher 展示 provider 连接状态、最近延迟、最后错误、最近动作与 trace 摘要。
- 测试层：用 mock local HTTP server 替代真实 `Local Provider`，保证 required 回归可离线执行。

## 3. 关键接口 / 入口
- `GET /v1/provider/info`
- `GET /v1/provider/health`
- `POST /v1/world-simulator/decision`
- `POST /v1/world-simulator/feedback`
- launcher provider 设置入口
- viewer provider 状态与 trace 调试入口

## 4. 约束与边界
- provider 服务只允许监听本地回环地址，不允许默认对局域网/公网开放。
- adapter 只传输结构化决策，不允许 provider 直接调用 runtime 内部写接口。
- 首期不支持反向 callback 与复杂 streaming；所有决策通过单次 request/response 完成。
- 本地 HTTP 失败时必须可回退到内置 provider 或禁用该 provider，不得阻断游戏主流程。
- 首期动作集严格白名单，只覆盖低频、低破坏性动作。
- required 回归不依赖真实用户安装的 `Local Provider`，必须使用 mock 服务即可跑通。

## 5. 设计演进计划
- 先落地 launcher provider 配置、发现和 health-check。
- 再落地 `Local ProviderAdapter` 与 mock local HTTP contract tests。
- 再接 viewer 状态/trace 面板与低频 NPC 闭环。
- 最后基于单 NPC 试点结果决定是否扩展动作集、是否引入双向 callback 或更底层 IPC。
