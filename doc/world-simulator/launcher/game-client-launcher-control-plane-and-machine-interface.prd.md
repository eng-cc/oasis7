# 游戏客户端启动器控制面与机器接口

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.design.md`
- 历史迁移、验证与 task 状态：GitHub task issue evidence。

## 目标与权威边界

本专题是 `oasis7_web_launcher` 的稳定 launcher control-plane authority，收敛已完成的 native/Web UI 同层复用、共享 UI schema、静态资源服务、控制台、控制面以及 GUI-agent 机器接口历史专题。它描述 launcher 如何呈现和受控地调用既有进程控制能力；不定义 runtime 世界状态演化、链执行规则或 Agent 决策语义。

native 与 Web 客户端消费同一份状态和动作合同。Web 入口可托管 launcher 静态资源，控制面提供状态读取及游戏、链相关的既有受控操作；实际可用状态、配置约束和错误均以当前响应为准。

## 范围

覆盖 launcher 的共享表现、控制状态和 operator HTTP-JSON 适配面；不覆盖 runtime 世界规则、执行状态演化、transfer 可用性或 Agent 决策。

## 人员与使用场景

- 桌面用户和 operator：通过 native 或 Web launcher 观察相同的控制状态，并使用相同语义的受控启动、停止和诊断入口。
- 自动化 operator：通过 HTTP JSON 机器接口发现当前能力、读取状态、提交一个已声明动作并读取统一结果；它不是世界内 autonomous Agent，也不是 `DecisionProvider`。
- launcher 开发者：维护共享 UI schema、客户端表现和控制面映射，不复制进程编排或 runtime 规则。

## 接口 / 数据

### 客户端与控制面

- `GET /api/state` 是 launcher 控制状态读取入口；native/Web 均以服务端快照而非客户端推断来展示状态。
- 既有控制路由包含 `POST /api/start`、`POST /api/stop`、`POST /api/chain/start` 和 `POST /api/chain/stop`；它们的可执行性、参数和错误以运行中服务的当前合同为准。
- launcher 静态资源和 API 可并存；静态目录缺失或资源不可用不得改变既有 API 路由语义。
- 共享 UI schema 负责字段分组、文案和可见性的一致映射。它是表现层合同，不是 runtime 配置或世界规则的第二真源。

### 受控停止与 Web 诊断

- `POST /api/stop` 的 launcher 表现必须将用户请求、当前结果和随后状态快照如实呈现；它不是对 chain、所有子进程或 runtime 最终清理完成的承诺。native launcher 对其直接管理的子进程先请求优雅停止，在有界等待后才使用终止回退；用户选择停止和关闭窗口走同一停止生命周期，失败必须保留可诊断结果。
- Web 表面不得因初始化、浏览器环境、状态读取或控制面请求失败而把 launcher 显示为 ready。它应保留可见的 blocked/error 诊断及可恢复下一步，并继续以服务端状态快照为准；这条表现边界不声明浏览器 clock、轮询、WASM lifecycle 或 runtime session 的具体实现。
- chain runtime、execution-world 输出、stale-session/recovery，以及浏览器/WASM 的运行时兼容细节，统一转至 `game-client-launcher-runtime-session-continuity.prd.md` 的 runtime 专业 authority；本控制面只保留请求、结果和状态的呈现合同。

### 机器接口

- `GET /api/gui-agent/capabilities` 是机器客户端的 capability discovery 入口；响应中的版本、`actions[]` 与 `query_targets` 是当前动作清单和查询目标的真值，文档不维护永久且穷尽的动作清单。payload 约束不由该响应发现，而是在提交 action 时按当前实现校验。
- `GET /api/gui-agent/state` 提供与 `/api/state` 对齐的机器可读状态别名。
- `POST /api/gui-agent/action` 接收严格的枚举 action 与 JSON payload；不支持的 action、缺失/非法 payload 和被当前状态拒绝的请求必须返回可分类结果。
- 动作响应保持统一的 `ok`、`action`、可选 `error_code`/`error`/`data` 以及最新 `state` 结构，方便调用方在不解析 Web UI 的情况下继续诊断或恢复。
- 查询代理只可经服务端已声明的目标和参数映射；机器接口不提供任意 URL 或任意命令穿透。

## 部署与安全非主张

`gui-agent` 是 operator-plane HTTP JSON 接口，不是玩家世界操作面，也不授予 runtime 直接写入权。`hosted_public_join` 对已枚举的 operator 路由使用 peer-IP gate；该事实不表示所有 API 都是私有的、服务仅绑定 loopback，或已经具备认证、RBAC、TLS、rate limit、公开可用性或任何 network/readiness/SLA 结论。wildcard bind 和公开路由仍按各自当前实现及部署配置处理。

本专题同样不承诺 transfer 可用性、永久动作覆盖，或将 GUI-agent 与 `DecisionProvider`、自治 Agent 等同。涉及 runtime 执行、身份、安全部署或世界状态语义的变更，必须回到相应专业 authority 和当前任务证据。

## 里程碑

- 已完成：共享 schema、Web 静态资源与 control-plane 并存、机器能力发现/状态/统一 action 合同。
- 维护：后续实现或安全边界变更以新任务、代码和当前 capability discovery 响应重新验证。

## 风险

- 动作清单、查询目标和 action payload 约束可能随实现演进；调用方必须发现当前能力并处理执行期校验结果，不得把本文档当成永久 action catalog 或 payload schema。
- operator-plane 路由边界不能替代 runtime 权威、安全部署或公开 readiness 的专业结论。
- 停止请求被接受、Web 未崩溃或诊断可见，都不等于 runtime 已收敛、所有进程已退出，或浏览器/WASM 环境已经具备通用兼容性。

## 验收与追溯

- 文档入口与链接：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`。
- 机器接口合同：`oasis7_web_launcher` 的定向测试覆盖 capabilities、状态别名、严格 JSON/action 解析、统一响应以及 hosted operator-path gate。
- 停止与诊断表现：实现变更时验证有界停止/失败回退、停止与窗口关闭的一致入口，以及 Web 对不可用状态的非崩溃诊断呈现；runtime/session/WASM 正确性由其 successor authority 复核。
- 本稳定专题吸收的历史完成证据保留在 Git history 和 GitHub task issue evidence comments；不再把已删除的日期化专题作为当前入口。
