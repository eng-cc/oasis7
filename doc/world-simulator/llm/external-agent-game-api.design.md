# 外部 Agent Game API 系统设计

- 设计状态：target contract，待实现与独立评审；不是当前能力或发布报告。
- Canonical repository：`eng-cc/oasis7`；审读基线：`03fb58784a65de9f02a43dfeafc3e882bb04e5f2`（已合入产品 PR #4493）。
- Owner role：`agent_engineer`；最后审读：2026-10-10；新增合同 revision：`1`。
- 产品入口：[外部 Runtime 游玩](../../product/agents-world-simulation/external-agent-runtime-play.prd.md)；[玩家与 Runtime 协作](../../product/agents-world-simulation/player-runtime-collaboration.prd.md)。
- 系统总入口：[LLM / external runtime 系统入口](README.md)；写作规范：[系统设计规范](../../engineering/doc-governance/system-design-writing-standard.design.md)。

本文中的新组件、路由、记录和测试场景均为后续实施输入。`accepted` 只表示所属权威已受理，世界效果必须读取既有权威回执；文档合入不代表这些接口已经上线。

## 1. 问题、目标与非目标

把“游戏向 Provider 索取决策”与“外部 Runtime 主动玩游戏”分开。后者必须有可独立调用的观察、能力、行动、任务和结果接口，而不是把旧 `/v1/provider/*` 换个名字。首期复用既有 Runtime 行动校验与持久回执，不让 Viewer、模型或 API 服务器成为新的世界写入者。

目标：普通 HTTP 客户端可以完成授权、发现首局能力、提交一次有效意图并在响应丢失后找回同一结果；Web、桌面及外部 Runtime 共享业务语义，但不共享默认写权限。非目标：本 PR 实现服务、设计新共识、创建模型额度桥、立即发布任意 Runtime 适配或另建通用任务平台。

## 2. 上游约束与相关角色

Agent owner 负责协议与能力投影；Runtime owner 负责最终裁决、授权与持久化；Viewer owner 消费相同的只读状态；QA 负责普通客户端和真实 Runtime 的分层证据。具体协作状态、适配器与可视化分别消费配套系统设计，不能复制第二份状态机。

### 2.1 需求承接与分配表

| 上游条款 | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 排除与未覆盖范围 |
| --- | --- | --- | --- | --- |
| [AC-EXT-002](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-002) | 合法绑定、身份隔离与费用路由不能授予游戏权力 | [api-auth](#api-auth) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-003](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-003) | 首局能力目录、参数和实际执行一致 | [api-context](#api-context) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-005](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-005) | 成功、拒绝、未知结果可恢复且不重复世界效果 | [api-actions](#api-actions) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-010](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-010) | 两个真实 Runtime 分别完成首局与组合恢复 | [api-composition](#api-composition) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-012](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-012) | 普通客户端不依赖旧 Bridge 完成完整协议闭环 | [api-wire](#api-wire) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-013](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-013) | 鉴权、越权、限流与授权撤销在服务端生效 | [api-auth](#api-auth) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-014](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-014) | 关闭旧 Provider Bridge 后新主路径仍可运行 | [api-cutover](#api-cutover) | agent_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |

## 3. 当前状态、目标状态与差距

| 对象 | 基线事实及定位 | 本设计目标 / 差距 |
| --- | --- | --- |
| 共享 DTO | `crates/oasis7_agent_api/src/lib.rs` 已导出观察、能力、目标、反馈和 RuntimeBinding，且明确不执行世界 | 新增 `game` 命名空间的外部协议 DTO，复用已有强类型；不能把 crate 存在当网络服务存在 |
| 旧 Provider | `doc/world-simulator/llm/README.md` 记录 decision-context/feedback-context 回调方向 | 新 `/v1/game` 是主动调用入口，不接收旧回调 DTO 冒充新请求 |
| 世界运行 | Viewer 手册明确 `oasis7_chain_runtime` 管理 execution-world，Viewer 是 client/observer | API 经受信 RuntimePort 调用当前世界权威，不能写 snapshot/journal 文件来执行行动 |
| 认知 | `agent-cognition-lifecycle.design.md` 已定义 Harness identity、AgentDecisionEnvelope、MVCC、journal 目标 | API 只作外部适配；其生产恢复边界仍需实际配套实现，不宣称已完全具备 |

## 4. 边界与结构

<a id="api-composition"></a>
### 4.1 最小部署与模块分工

```text
玩家 Web / 桌面 ---- authenticated owner session ----┐
                                                   v
外部 Runtime + Skill -- outbound HTTPS --> GameApiIngress
                                            |     |     |
                              CollaborationPort  |  WorldReadPort
                                            |  DecisionAdmissionPort
                                            v     v     v
                              当前世界权威 Runtime + 持久 journal/checkpoint
```

箭头表示调用，不表示上游获得世界写权。`GameApiIngress` 是拟新增的路由组件，首期挂载于现有 `oasis7_viewer_live` 网络服务，并可在不启用 GUI/内置模型/旧 Provider 的情况下运行。渲染页面断开不销毁服务。它不内嵌 node 或共识；服务边界继续沿用当前 chain submit/status 与可信 Runtime adapter，必要的协议扩展须进入 Runtime 端。

`oasis7_agent_api` 拥有新 machine-facing DTO 与 schema；`oasis7_client_api` 消费公共标识/投影，禁止反向循环依赖。HTTP router、credential verifier、schema validation、限流和 serialization 位于 native host；World::step 不等待 HTTP 或模型。协作服务共用同一进程及 RuntimePort，不要求 Kafka、Redis 或另一套控制数据库。

- 协作存储与写入顺序：[协作权威设计](../../world-runtime/runtime/player-runtime-collaboration.design.md)。
- 实际 Runtime 生命周期与 Skill：[外部适配设计](external-agent-runtime-adapter.design.md)。
- 面向用户的世界投影：[Web/native 世界视图设计](../viewer/external-agent-world-view.design.md)。

### 4.2 发布基线补充：复用已合入的 World Service

产品审读基线保持 `03fb58784a65de9f02a43dfeafc3e882bb04e5f2`。发布前核对 `7039bc376882ab8f84f88ca014f6d7ce642539a8` 时，main 已增加独立、认证且不要求应用挂载世界目录的 World Service；以下是对本设计物理接入的收敛，不是再造一套基础设施。

- 当前 `crates/oasis7/src/world_service/wire.rs` 已定义 POST `/v1/world/describe`、`submit`、`lookup`、`view`、`changes`，并使用 `SignedReadRequest<T>`、`SignedServiceResponse<T>`、`WorldServicePayloadV1`。`client` / `verified_view` 是新 Game API 的底层调用与验证位置；不能改为读共享 snapshot 文件或由 Viewer 本地 World 代替。
- `GameApiIngress` 是面向玩家和执行器的 `/v1/game` 资源适配层，底层 WorldReadPort 使用 signed view/changes，DecisionAdmissionPort 使用 signed Cognition/Scheduler 的 submit/lookup。已有服务回复签名只证明受信服务断言，不等于共识最终性；继续核对 CommitRef、WorldBinding、receipt 与请求关联。
- 已有 `AgentSignerDelegationChangeV1` 的 owner 签名、Agent identity generation、delegation generation、nonce 和 delegate public key 不得被新 opaque token 取代。协作设计的 executor_epoch 是更窄的任务执行实例 fence，验证组合为 `(agent_identity_generation, delegation_generation, task_executor_epoch)`；任务 fence 只能收窄已有委托，不能自行授予或恢复 world signer 权限。
- 当前 payload 覆盖 GameplayJson、Cognition、FeedbackAck、Delegation、Scheduler，不包含本文的完整任务/目标/问答/审批写入。实施时为这些控制命令增加版本化的 typed payload 与世界侧校验，不把自由 JSON 当现成管理权限。服务器不支持所需变体时返回 capability_unavailable，而非绕过 World Service 直写文件。

**主体签名接续**：首期默认保留设备端的 owner/delegate 私钥，不把全局 owner 私钥交给 Game API。opaque token 只认证入口会话；需要 World Service 主体证明的读写由本机受限 signer（玩家端或 executor 工具代理）签署确切的现有 operation domain 和 canonical payload。Game API 不能替客户端改写已签 payload，也不能借服务响应签名密钥冒充 owner。

为使普通 HTTP 客户端与两个 Runtime 都可执行该步骤，增加目标 `POST /v1/game/proofs/prepare`：输入固定 allowlist 中的业务路由、原 client_operation_id、业务正文和已有 context；返回 `challenge_id / operation_domain / canonical_payload / payload_digest / signer_role / binding / expires_at`。它只准备提案、不提交行动或授予权限。调用方使用固定版本 codec 重算 digest，并检查原业务意图、world/Agent/task、目标、授权范围和 signer_role 后本地签名；在原业务路由附 `actor_proof={challenge_id,subject_public_key,signature_hex}` 继续。入口缺少证明时返回 `428 actor_proof_required`，不伪造成功。GameRequest 内部可以自动完成纯协议签名往返，但 owner 审批等高影响决定仍须既有明确确认，不让模型自行批准。

业务幂等 digest 不包含 token、transport_attempt 或 actor_proof；同一个操作在 prepare 时固定其待签 canonical payload 和底层 correlation，重试返回原 challenge/operation，不能通过重新签名重置 nonce、预算或重复 World Service submit。状态漂移导致原请求不可用时先 lookup 原结果，再明确生成新业务操作与 context。pairing 的 owner 批准仅是开始世界委托注册，只有对应 Delegation 提交和受权回读成立后才进入 execution_ready。设备配对通过、API token 有效或 proof prepare 成功都不代签此点。

复用验证入口为 `crates/oasis7/tests/world-service-conformance.manual.md` 与 `crates/oasis7/scripts/world-service-conformance.sh`。这些现有测试可证明其实际覆盖的基础服务合同；不代签新的 Game API、协作控制或真实外部 Runtime 体验。私有正文仍由协作设计所述 Runtime 受控存储保存，应用不获取世界存储挂载权。

## 5. 关键运行流程

<a id="api-auth"></a>
### 5.1 连接、配对和最小授权

玩家继续使用现有 hosted account / player session / device proof，控制权取自 Runtime 的实际 Agent 所有权与有效委托；不另发“公开用户 ID 即 bearer”凭证。

外部执行器首次连接采用显式设备配对：生成本地设备公钥，申请一次性 pairing；玩家在可信 Web/桌面会话核对该设备、世界、Agent、有效期与 requested scopes 并批准。配对码只是定位待批申请，不是最终权限。服务验证设备私钥持有证明后，交付短期 opaque access token 与轮换 refresh credential；refresh 权限始终不超过原配对范围。

默认 executor 只有 `world:observe / capability:read / context:read / action:submit / receipt:read / task:read / task:report / message:write / approval:request`；没有 `goal:write / approval:decide / executor:replace / grant:write`。owner 的任务写入和审批使用独立玩家会话及适用强认证。模型 API key、支付路由、Skill 文本和自报 Agent ID 均不能兑换游戏身份。

认证与世界授权分层：入口验证 token/device/session，Runtime 再验证 world、Agent、task、当前 executor epoch、goal revision、grant revision 与剩余额度。撤销或替换执行方不依赖 token 自然到期。无权读取返回不泄露对象存在性的错误；先校验权限，再返回版本冲突或最新值。

<a id="api-context"></a>
### 5.2 一次可执行的决策上下文

1. 执行器完成现有 World Service 委托的签名注册及回读后，读取任务 snapshot，取得当前 executor fence、有效目标和授权；陈旧版本先重新同步。
2. `POST .../agents/{agent}/contexts` 申请上下文，host-side Harness 分配已有 session/turn/request identity，记录同一已提交 WorldBinding、目标/授权版本及有效期，返回不可猜测 `context_id`。
3. 能力目录从 Runtime capability registry 和当前主体授权投影。每项包含稳定 action_ref、参数 schema、是否有副作用、前置与不可用原因；过滤目录不能取代提交时再校验。
4. 客户端可查询更多合法信息；副作用提案携带 context_id 和稳定 client_operation_id。服务端回读上下文，填充而非相信客户端提供的 authority/capability/MVCC 字段，转换为既有 AgentDecisionEnvelope。
5. 一个 Agent 至多一个活动 cognition turn；不同 Agent 可并行。新 context 不得在旧未知提交未核对时自动开启第二个副作用请求。只读 query 和结果查询不因等待模型而阻塞。

context 创建也必须携带 client_operation_id；超时重试返回原 context 或明确已过期，不能隐式再分配第二个 turn。除认证配对/兑换使用专用一次性 nonce 外，所有变更类 POST/PUT 都适用稳定操作身份和同键异内容拒绝。

只读查询后需要等待、提问或放弃轮次时，执行器用 outcomes 显式收口；成功 action 结果、有效 outcomes 或明确失效才释放活动 turn。wait 的唤醒提案仍经既有 Runtime continuation 校验，不能由 worker 私自推进世界；无回应的轮次到期按 Runtime 规则失效并保留原操作查询。

上下文过期、分支变化、授权撤销、目标版本不符时返回可区分的 `context_stale / authority_changed / goal_changed / recovery_required`。新 Context 不能仅改写旧提案上的版本号；必须使执行器重读并重新作出决策。

<a id="api-actions"></a>
### 5.3 提交、超时与唯一世界效果

幂等键作用域为 `(world_id, agent_id, authenticated_actor_id, operation_kind, client_operation_id)`，不包含可变化的 goal revision 或 executor epoch，以免重连后同一操作变成新键。服务器用版本化 canonical body 计算内容 digest；同键同内容返回既有 operation，同键异内容 `409 idempotency_conflict`，不覆盖历史。

先在权威 ingress journal 持久记录操作关联，再进入既有 admission/commit 流程。`202 pending` 只表示操作已登记；业务拒绝也生成可查询 disposition。仅匹配的 Runtime durable receipt 和有效 finality binding 允许 `committed`。投递超时返回原 operation_id 或由客户端按原 client_operation_id 查询，不能自动生成新键重做。

如果 admission 已成功而 HTTP 回复丢失，重试必须先找到原操作；授权改变后仍先验证当前主体是否允许读取该历史，不能借幂等回放绕过读权限。资源扣减、grant 消费及 effect/receipt 在 Runtime 的同一提交边界处理；入口缓存仅是加速，不能作为去重唯一来源。不同 client_operation_id 的重复业务意图仍需世界规则限制，本设计不声称任意语义操作全局 exactly-once。

## 6. 接口与数据合同

<a id="api-wire"></a>
### 6.1 统一路由与身份

以下是 revision 1 的**目标合同**，不是现有端点。路径前缀 `/v1/game`；`W=/worlds/{world_id}`、`T=W/tasks/{task_id}`。服务器发布同版本 schema / OpenAPI；所有 owner 与 executor 使用相同资源，但动作权限不同。

| 方法与路径 | 输入 / 输出 | 语义及主责 |
| --- | --- | --- |
| GET `/info` | api major、schema revision、受支持能力与 LimitsV1 | 只描述部署合同，不宣称真实 Runtime 组合已验收 |
| POST `/proofs/prepare` | 固定业务路由、operation ID、正文/context；待签 challenge | 主体证明准备；无世界副作用，签名后在原路由附 actor_proof 重试 |
| POST `/pairings`；POST `/pairings/{id}/approve`；POST `/pairings/{id}/exchange` | 设备公钥、一次性挑战；owner 决定；设备 proof | Auth 主责；批准与兑换分离，返回 token 只发给验证过的设备 |
| POST `/tokens/refresh`；POST `/tokens/revoke` | 轮换 refresh proof / token handle | 撤销可读回，旧 refresh 重放整族失效；不返回原始 secret |
| GET `W/agents`；GET `W/agents/{A}/observation` | 授权过滤的实体/观察及 WorldBinding | ReadPort；不允许 query 参数扩大 audience |
| GET `W/agents/{A}/capabilities`；POST `W/agents/{A}/queries` | 条件化能力目录 / schema-validated query | 只读；查询不产生 world effect |
| POST `W/agents/{A}/contexts` | client_operation_id、task_id、executor fence、expected goal revision | Harness 生成 ContextHandle；消费单 Agent 活动 turn 约束 |
| POST `W/agents/{A}/contexts/{C}/outcomes` | client_operation_id、kind=wait/blocked/abandoned、reason、wake_proposal? | 无动作轮次收口；校验当前 context 后关闭或走既有 Runtime continuation admission，不伪造世界效果 |
| POST `W/agents/{A}/actions`；GET `W/operations/{id}` | context_id、client_operation_id、action_ref、arguments；OperationView | AdmissionPort；受理、待决、拒绝、失败和结算分开 |
| GET `W/operations/by-client-id` | actor-scoped client_operation_id + operation_kind + agent_id | 不确定响应时找回原操作；不能读取他人操作 |
| POST `W/tasks`；GET `T/snapshot` | agent_id / task snapshot | 协作设计拥有任务唯一性与切点 |
| PUT `T/goal`；POST `T/goal-receipts` | expected revision / executor read-adopt report | 保存、激活、读取、采纳、用于决策五类证据分开 |
| POST `T/reports`；POST `T/questions`；POST `T/questions/{Q}/answers` | 稳定消息 ID、关联版本、source 与有界正文 | 协作主责；普通文本不授权 |
| POST `T/approval-requests`；POST `T/approval-requests/{P}/decisions` | 精确方案与范围 / owner 明确决定 | 只申请不自行批准；世界执行仍重验 |
| POST `T/executor-bindings`；POST `T/heartbeats`；POST `T/stop` | 当前绑定 / 诊断心跳 / 撤销后续执行 | 协作主责；状态不是取消既有世界效果 |
| GET `T/events` | cursor、limit、wait_ms | 至少支持有界增量轮询；过期游标必须显式重同步 |
| POST `W/views/query`；GET `W/views/entities/{E}`；GET `W/views/search`；GET `W/views/events` | 视点、尺度、搜索、授权投影与游标 | 世界视图设计主责，不能复用 executor 私有观察给普通 Viewer |

### 6.2 核心 DTO

```text
ContextHandleV1 = {api_revision, context_id, world_binding: RuntimeBindingV1,
  task_id, agent_id, executor_id, executor_epoch, goal_revision,
  grant_revision, capability_digest, request_digest, valid_until_tick,
  observation, capabilities}
ActionSubmissionV1 = {api_revision, client_operation_id, task_id,
  executor_epoch, context_id, action_ref, arguments, actor_proof}
OperationViewV1 = {api_revision, operation_id, client_operation_id,
  status: pending|committed|rejected|failed, disposition_reason,
  receipt_ref?, world_binding, observed_at, retry_advice}
ErrorV1 = {code, message, request_trace_id, operation_id?, retryable,
  retry_after_ms?, refresh: none|context|task_snapshot|reauth}
```

`operation_id`、world/Agent/task/executor ID 是不透明字符串；u64 revision/tick/epoch 在 JSON 使用十进制字符串，避免 JS 精度损失。严格校验请求的未知字段、类型、深度和重复 JSON key；响应可以增添可选字段，未知状态必须阻塞而非默认成功。HTTP 401/403 不作为业务失败的自动重放信号；409 表示版本/幂等冲突，410 表示已退出的协议版本，413/429/503 分别为大小、配额、服务不可用。

完整 WorldBinding/finality、ActionCatalogEntry、请求 digest、反馈和 receipt lineage 复用共享 crate 与 Runtime 合同，不在此另定哈希算法。客户端不能凭 receipt_ref 字符串自证提交成功，必须权威回读。

## 7. 状态、事务与持久化

API 入口无独立世界事务。token/配对的凭据记录由可信身份服务保存；task/goal/grant/fence 由协作权威保存；context/operation/dedup/disposition 由 Runtime/Harness 持久边界保存；渲染缓存随时可重建。各自的状态转换与崩溃恢复不可互相代签。

操作登记与 Runtime admission 跨进程时使用 durable ingress + outbox，不声称网络原子提交：outbox 可能重复，Runtime 按稳定 operation/request identity 去重。恢复扫描 registered/pending 操作，只重送同一请求；无法证明是否执行则保持 recovery_required。终态去重索引至少与权威操作历史同寿命；大正文可压缩，但删除键后不得把迟到请求作为新操作重新执行。

## 8. 部署、安全与运行约束

远程必须 HTTPS，CORS 使用部署级 allowlist，禁止携带凭据的 wildcard origin；Cookie 模式校验 CSRF/Origin。外部机器 token 不进入 URL、页面 JS、聊天或诊断包。循环地址也须认证，不能仅凭 loopback 或 `Origin:null` 授权；开发认证豁免不得进入默认发行配置。

入口与 Runtime 通道只接受固定配置的目标与受信调用者，拒绝客户端提供 file path、任意 callback URL 或费用路由作为后端目的地。授权/能力 registry 不可用时停止新写入，保留许可范围内的已确认结果读取。第三方 provider 网关的密钥仅在用户实际 runtime host，不上传到 Game API。

## 9. 质量与容量

以下为拟实施的可配置初始上限，不是当前测量值：JSON body 256 KiB、嵌套深度 16；goal 4 KiB、message/report 各 8 KiB；列表默认 100、最大 200；单响应 2 MiB；每身份并发读取 8、每 Agent 活动 cognition turn 1。`LimitsV1` 发布实际值，超限返回结构化错误，不截断目标、授权或回执。

HTTP 普通请求客户端 deadline 初值 10 秒；事件 long-poll 服务端最多 25 秒、客户端 30 秒。读轮询可退避，写重试必须先核对幂等操作；这些 wall-clock 预算不决定世界 logical tick/finality。容量验收测 P95、错误率、队列长度和 world tick 不被 I/O 阻塞，不为尚无基线的公网写入虚构 SLA。

## 10. 兼容、迁移与回滚

<a id="api-cutover"></a>
新路径只使用 Game API revision 1，不接收旧 Provider wire 作为 alias。实施顺序：接通权威读与鉴权 → context/catalog/operation → 协作与 worker → Viewer/native → 两 Runtime 首局。上线前先验证唯一 writer、持久 replay 与权限负例，随后把推荐 Skill 改指向新路径。

旧 Bridge 允许直接删除，不强制双写或永久兼容层；删除前列清仍在使用的 builtin、mock、运维与独立消费者并各自处置。撤回新入口是禁止新提交、继续查询既有 operation，不是把旧 bridge 静默接管当前 task；新旧存储不能无检查相互降级。schema 不兼容时旧服务拒绝打开新快照，恢复需采用明确的备份/转换方案，世界已提交历史不回滚。

## 11. 验证设计与可追溯性

普通 CI 补齐 parser、schema、授权、目录执行遍历、outbox/幂等及故障注入；只有真实模型行为、真实 Web/native 与实际部署恢复无法由普通 CI 证明的部分使用专项验证。文档检查不能代签功能完成。

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立 obligation 与适用条件 | 准确验证方法与场景 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-EXT-002](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-002) | [api-auth](#api-auth) | 合法绑定、身份隔离与费用路由不能授予游戏权力 | [api-auth](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-auth)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-003](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-003) | [api-context](#api-context) | 首局能力目录、参数和实际执行一致 | [api-catalog](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-catalog)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-005](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-005) | [api-actions](#api-actions) | 成功、拒绝、未知结果可恢复且不重复世界效果 | [api-idempotency](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-idempotency)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-010](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-010) | [api-composition](#api-composition) | 两个真实 Runtime 分别完成首局与组合恢复 | [api-first-session](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-first-session)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-012](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-012) | [api-wire](#api-wire) | 普通客户端不依赖旧 Bridge 完成完整协议闭环 | [api-wire](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-wire)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-013](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-013) | [api-auth](#api-auth) | 鉴权、越权、限流与授权撤销在服务端生效 | [api-safety](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-safety)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-014](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-014) | [api-cutover](#api-cutover) | 关闭旧 Provider Bridge 后新主路径仍可运行 | [api-cutover](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-cutover)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |

## 12. 决策、长期风险与未决问题

ADR-GAPI-1：复用既有服务进程和世界权威，增加主动游戏协议，不扩展旧 Provider callback 充当通用 API；代价是需要显式新 schema 与迁移验证，收益是消除调用方向和鉴权混淆。

ADR-GAPI-2：以 at-least-once 投递和持久幂等实现同一 operation 的唯一世界效果；不承诺网络 exactly-once。权威不可达时牺牲新写可用性，不能靠本地缓存放行。

剩余解除项：identity owner 必须证明当前 hosted 认证可签发最小执行凭据；Runtime owner 必须把新 context/operation 扩展接入实际持久 admission 而非仅内存 sidecar。没有这两项，部署保持 read-only/blocked。API major、上限与观测指标变更时由 agent/runtime/QA 复核相应 schema 和回归，不扩大本次产品范围。
