# Local Provider 本地 HTTP Provider 接入 world-simulator 首期方案（2026-03-12）设计

- 对应需求文档: `doc/world-simulator/llm/provider-loopback-http-contract.prd.md`
- 专题入口与权威边界: `doc/world-simulator/llm/README.md`

## 1. 设计定位
定义“安装在用户机器上的 `Local Provider` 如何通过本地 HTTP 参与 world-simulator 的 agent 决策”的首期工程方案，覆盖本地发现、握手、配置、决策请求、反馈回写、状态可观测与失败回退。

## 2. 设计结构
- 用户侧本地 provider 层：`Local Provider` 以独立本地服务运行，仅监听 `127.0.0.1`。
- Adapter 层：world-simulator 内新增 `Local ProviderAdapter`，把 Continuous Harness outer request/response/feedback wrappers（其中保留 `DecisionRequest/DecisionResponse/FeedbackEnvelope` inner DTO）与本地 HTTP API 互转。
- 配置与发现层：launcher 负责 provider 模式选择、base URL/token 配置、发现与 health-check。
- 运行与裁决层：runtime/kernel 继续负责动作白名单、规则校验、状态演化与事件产出。
- 观测层：viewer 与 launcher 展示 provider 连接状态、最近延迟、最后错误、最近动作与 trace 摘要。
- 测试层：用 mock local HTTP server 替代真实 `Local Provider`，保证 required 回归可离线执行。

## 3. 关键接口 / 入口
- `GET /v1/provider/info`
- `GET /v1/provider/health`
- `POST /v1/world-simulator/decision`：target lane 使用 `ContinuousAgentRequestContextV1` / `ContinuousAgentResponseContextV1` outer wrappers；旧 `DecisionRequest/DecisionResponse` body 仅在显式 `compatibility_lane=legacy_v1` 使用，并记录 `legacy_no_cognition_proof`。
- `POST /v1/world-simulator/feedback`：target lane 使用 Harness target `FeedbackEnvelope` outer contract；旧 `FeedbackEnvelope` DTO 仅在显式 `compatibility_lane=legacy_v1` 使用，并且不能关闭 target turn 或进入 target memory/continuation。
- launcher provider 设置入口
- viewer provider 状态与 trace 调试入口

## 4. 约束与边界
- provider 服务只允许监听本地回环地址，不允许默认对局域网/公网开放。
- adapter 只传输结构化决策，不允许 provider 直接调用 runtime 内部写接口。
- 首期不支持反向 callback 与复杂 streaming；所有决策通过单次 request/response 完成。
- 本地 HTTP 失败时必须可回退到内置 provider 或禁用该 provider，不得阻断游戏主流程。
- 首期动作集严格白名单，只覆盖低频、低破坏性动作。
- required 回归不依赖真实用户安装的 `Local Provider`，必须使用 mock 服务即可跑通。

### 4.1 Harness contract alignment

Loopback is an adapter/compatibility lane for the Continuous Harness. Its tagged `decision` response
preserves `wait/wait_ticks/act/query/module_command/module_command_response` variants; `query` is
read-only against the frozen world snapshot, while `module_command_response` carries the complete
host-bound `AgentCommandResponse` and requires catalog/context/nonce validation before Runtime.
Its response may carry `module_command?`, but the field is only a typed candidate and follows host schema encoding plus
Runtime validation. The target loopback decision endpoint carries the outer
`ContinuousAgentRequestContextV1`/`ContinuousAgentResponseContextV1` wrappers; the old
`DecisionRequest`/`DecisionResponse` DTO body is an explicit `compatibility_lane=legacy_v1` only.
The target feedback endpoint likewise carries the Harness target `FeedbackEnvelope` outer contract;
the old DTO uses an explicit compatibility mapping. Legacy `FeedbackEnvelope` uses an explicit adapter mapping; missing
session/turn/request/digest/receipt correlation is `legacy_no_cognition_proof`. The default lane
must use the non-recursive V1 `request_digest` (outer context without its output digest or
`transport_attempt`) and the
shared provider invocation derivation. Any heuristic action recovery requires explicit
`compatibility_lane=legacy_heuristic_v1` and `legacy_heuristic_used`, and is excluded from target
parity/proven evidence and automatic memory/continuation semantics. Trace and feedback limits,
redaction and overflow behavior follow the Continuous Harness documents; raw HTTP bodies are not
authority or replay inputs. Target response memory candidates use `MemoryWriteIntentV1`; the existing
inner DTO is compatibility-lane only, and a committed Runtime outcome is still required.

## 5. 设计演进计划
- 先落地 launcher provider 配置、发现和 health-check。
- 再落地 `Local ProviderAdapter` 与 mock local HTTP contract tests。
- 再接 viewer 状态/trace 面板与低频 NPC 闭环。
- 最后基于单 NPC 试点结果决定是否扩展动作集、是否引入双向 callback 或更底层 IPC。
