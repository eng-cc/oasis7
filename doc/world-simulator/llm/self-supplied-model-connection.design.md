# 网页自带模型连接：Agent 客户端侧系统设计

- 生命周期：`active`；状态：目标合同，未声明实现、部署、真实供应商兼容或安全验收完成。
- Owner role：`agent_engineer`；协作：`viewer_engineer`、`runtime_engineer`、`qa_engineer`、`producer_system_designer`。
- 固定审读基线：`nil-studio/oasis7@35961e32b7e7ad3469b7cd7db2e718fb1d6795b1`；2026-10-16。
- 产品主责：[网页自带模型连接 PRD](../../product/agents-world-simulation/self-supplied-model-connection.prd.md)；[配对产品设计](../../product/agents-world-simulation/self-supplied-model-connection.design.md)。
- 系统入口：[LLM/provider](README.md)；既有分层：[Agent Game Client](agent-game-client-boundary.design.md#des-agc-001)。

## 1. 问题、目标与非目标

目标是在受支持 Web 部署提供“选择 DeepSeek/OpenRouter、贴 Key、检查、确认开始”的第一方执行入口，复用既有 Harness、模型客户端与统一游戏合同。模型连接层拥有供应商配置、短期凭证、能力检查和模型调用准入；不拥有世界状态、玩家资产、长期 signer、游戏授权或任务最终性。

首期选择 **网页配置 + 可信第一方 Agent 执行端调用上游**。不把浏览器 CORS 支持、浏览器版 Harness 或玩家安装 loopback Bridge 作为前置。服务端转发是已告知的凭证处理，不得表述为端到端隐藏于服务端。执行端可以复用已有服务进程组和部署，不要求新建微服务平台。

非目标：长期 Key 托管、关闭网页后的无人值守云服务、OpenRouter OAuth、任意自定义网关、供应商管理 API、充值/额度兑换系统、通用公开推理代理、第二套 Game API、世界规则变化。LetAI 作为显式兼容组合保留，不成为新入口的必选项，也不默认删除其独立消费者。

## 2. 上游约束与相关角色

模型连接属于 `world-simulator/llm` 的第一方客户端侧；主 Web 是交互 surface，Launcher 组织启动/入口，工程指挥面板不持有玩家 Key。产品边界由配对 PRD 拥有；Runtime 继续拥有 task、grant、executor fence、operation 与世界回执。部署方负责受信执行端、TLS、隔离、日志和出站控制；部署未具备这些条件时不开放凭证接收。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-a95a56b1-22b6-4955-b152-0c672310ddb8；[REQ-BYOK-001](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-001) | 受支持 Web 完成预设选择、贴 Key、检查及开始；无服务时不先收 Key | [DES-BYOK-001](#des-byok-001) | viewer / agent；真实应用入口与受信执行端 | 静态站点独立运行、玩家安装 CLI、任意网关 |
| Trace refs: O7-REQ-26cefe7a-deb5-4b21-845a-a2d4556f7dca；[REQ-BYOK-002](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-002) | 接收方和费用知情、会话凭证隔离、断开忘记、检查/重试有界 | [DES-BYOK-002](#des-byok-002) | agent / 部署安全 owner；账号认证、供应商账户 | 长期保存、全账户账单硬封顶、替用户上游充值 |
| Trace refs: O7-REQ-84294c93-3275-4d19-8faf-389950b87464；[REQ-BYOK-003](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-003) | 模型就绪不授予游戏权限；会话内第一方执行复用统一游戏合同 | [DES-BYOK-003](#des-byok-003) | runtime / agent；有效任务、grant、fence 与客户端接缝 | 第一方特权、全量迁移完成、离线永久托管 |
| Trace refs: O7-REQ-d3d63078-ef30-45fb-8ae6-2f6db1f46c34；[REQ-BYOK-004](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-004) | 停用/切换隔离迟到结果、保留原操作、未知不重放 | [DES-BYOK-004](#des-byok-004) | runtime / agent；Provider Switch 与原操作恢复 | 取消已提交世界效果、保证退还上游调用费 |
| Trace refs: O7-REQ-e0ae0d80-31ca-488b-8ba2-a728bfb9ce8d；[REQ-BYOK-005](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-005) | 两家具体模型/协议分别验收、保留工具语义和可诊断失败 | [DES-BYOK-005](#des-byok-005) | agent / QA；供应商契约和实际测试配置 | 所有模型天然兼容、局部检查代签可玩或发行 |

### 2.2 既有权威的复用

[第一方客户端边界](agent-game-client-boundary.design.md#des-agc-001)拥有依赖方向；[Game API](external-agent-game-api.design.md#api-composition)拥有游戏接口；[Provider Switch](provider-switch-intent-continuity.design.md#des-psw-003)拥有在途意图连续性。这里的 Web 模型管理 API 既不是 Game API，也不是 operator/control API 的对外开放。它只能在用户自己的受限第一方执行上下文内管理模型连接。

<a id="current-state"></a>
## 3. 当前状态、目标状态与差距

| 对象 | 固定基线事实与源码入口 | 本设计增加的目标及尚未证明的部分 |
| --- | --- | --- |
| Harness | [`oasis7_agent_harness`](../../../crates/oasis7_agent_harness/src/lib.rs) 已导出 ModelProvider、TurnEngine 等，不依赖 World 实现 | 使用其 provider 边界，不把账号、Key 表单和 HTTP 服务塞进通用 Harness |
| 内置模型路径 | [`llm_agent.rs`](../../../crates/oasis7/src/simulator/llm_agent.rs) 与 [`openai_payload.rs`](../../../crates/oasis7/src/simulator/llm_agent/openai_payload.rs) 是现有 Responses 接线入口 | provider-neutral 配置、能力适配与请求级凭证注入；不能由类名推定协议 |
| 本地 Bridge | [`letai_direct.rs`](../../../crates/oasis7/src/bin/oasis7_provider_local_bridge/letai_direct.rs) 是 Chat Completions 与 LetAI 平台逻辑的既有入口 | 可复用传输/诊断；充值、平台凭证与 LetAI 标识必须留在专属适配器 |
| 游戏客户端 | [AGC 当前/目标差距](agent-game-client-boundary.design.md#des-agc-006)保留全量迁移与能力边界 | 本专题不证明 HTTP 写目录全部上线，也不以旧 callback 冒充新统一客户端 |
| Web 接入 | [执行方体验](../../product/agents-world-simulation/provider-agent-experience-continuity.design.md)已有配置、预算、委托分开的目标 | 本专题所需短期凭证会话、网页表单、连接检查和两家 E2E 尚无完整验收证据 |

以上源码入口是适配位置，不是本次修改清单或已交付声明。实施前核对当时 main 的移动/重命名，不为保持本文路径强行恢复旧实现。

## 4. 边界与结构

<a id="des-byok-001"></a>
### DES-BYOK-001：模型连接是第一方客户端的接入层

Trace refs: O7-REQ-a95a56b1-22b6-4955-b152-0c672310ddb8；[上游要求](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-001)。

```text
受支持主 Web / Launcher 复用入口
    | 认证会话 + CSRF 防护；Key 仅提交给明确受信的执行端
    v
第一方 Agent 执行端：连接管理 + 短期凭证 + 模型调用预算
    |                              |
    | 请求级 secret 注入            | 受限模型结果
    v                              v
DeepSeek / OpenRouter       Harness / 第一方 Game Client
                                   |
                              同一游戏交互合同
                                   v
                         World / Runtime 准入、效果与回执
```

凭证接收端由受支持应用部署配置绑定，不能从分享链接、玩家输入的 URL 或任意世界节点元数据自动发现并信任。界面在收 Key 前读取本部署能力、执行端身份、会话期限和可用预设；服务缺失、身份未建立或不可信时关闭提交入口。认证复用玩家系统，不用供应商 Key 充当 oasis7 登录凭证。

模型组件放在第一方客户端/组合服务侧；通用 Harness 只接收 ModelProvider 和预算结果，世界接口 DTO 不增加原始 Key、credential_ref 或模型账户字段。组合 binary 同进程也必须保持模块与数据隔离，不因同机部署让 World 读取凭证。不要为了接入新供应商再复制一套 Agent loop。

## 5. 数据与状态

以下为拟议客户端侧逻辑模型，不是现有 wire 类型，也不要求逐类型新建 crate 或数据库。

| 对象 | 内容与所有者 | 生命周期/限制 |
| --- | --- | --- |
| ProviderPreset | provider_id、固定 origin/path、protocol、模型/工具能力、路由与参数策略、preset_revision；发行配置拥有 | DeepSeek 与 OpenRouter 分开；模型目录只是输入，不自动成为已支持清单 |
| ModelConnection | 随机 connection_id、从认证派生的 owner/session、executor_instance、credential_version、preset/model/route revision、状态与到期时间 | 仅连接执行端可管理；ID 不具备 bearer 权限，每次访问重新校验主体与会话 |
| EphemeralCredential | secret bytes、owner/session、connection/version、允许的 provider origin | 仅执行端受保护内存；引用不出该服务信任域；不落配置、数据库、缓存快照或世界存储 |
| ProbeResult | connection/version、model/protocol/route、能力版本、检查时间/期限、结构化结果 | 任一绑定变化失效；不得自动改变执行配置；不含 Key 或真实私有目标 |
| ModelSessionLease | model connection 的有限使用期限、最后有效续期、绝对到期、调用/费用进度 | 是客户端模型使用租约，不是 Runtime cognition lease、任务授权或世界最终性 |
| ActiveBinding | connection/version 与当前 owner、world、agent、task、goal_version、executor fence 的绑定 | 游戏许可由现有 Runtime 权威授予；多个连接不能绕过单 Agent 有效执行方规则 |

连接状态可取 `checking / ready / blocked / revoked / expired`；执行运行状态单独投影，不用一个 `connected` 位替代。非敏感配置和已有游戏 operation refs 可按既有权限持久化；Key 不随恢复记录持久化。服务重启将内存凭证全部失效，不从全局环境变量恢复或切换为项目 Key。

## 6. 接口与协议

以下 `/v1/agent-client/*` 是拟议 Web 会话管理 namespace，部署上线前默认关闭；不能据此宣称当前已有端点。管理服务只组织模型连接与既有任务操作，不提供任意 prompt/URL 的开放代理。

| 操作 | 输入/输出合同 | 约束 |
| --- | --- | --- |
| GET `/v1/agent-client/capabilities` | 受信执行端身份、支持的预设/模型能力、期限/限制和不可用原因 | 可读不等于游戏授权；不输出 secret 或 operator 配置 |
| POST `/v1/agent-client/model-connections` | provider_id、model_id、raw_key、明确的本次使用同意；返回脱敏 connection/version | authenticated JSON body；不接受客户端指定 owner、任意 base_url/headers；新草稿不替换活跃连接 |
| POST `.../{id}/probe` | 当前 version、检查同意与客户端操作 ID；返回有界 ProbeResult | 有限、显式、可计费；测试工具不允许写世界；重复 ID 返回同一检查状态而非重复发上游请求 |
| POST `.../{id}/activate` | expected_version、目标任务/Agent 引用、用户确认的运行限制；返回绑定状态 | 复用当前 Runtime 授权；不上传长期 signer；无有效目标/委托不能自动补权 |
| POST `.../{id}/lease` | 当前 connection/version、有效应用会话；返回截止时间 | 有绝对期限；不延长游戏 grant、task 或 executor fence；过期不复活 |
| POST `.../{id}/stop` | 当前绑定与操作 ID；返回停止新推理的实际状态 | 与在途世界结果分开；无权限不伪报已停止其他执行方 |
| DELETE `.../{id}` | expected_version；返回停用/忘记状态 | 幂等停用凭证和续期；不调用上游管理/充值接口，不宣称已撤销供应商 Key |

所有写接口校验应用认证、精确 Origin 与 CSRF，响应 `Cache-Control: no-store`；不使用 URL/query/Referer 传 Key。跨账号读取返回不暴露对象是否存在的错误。并发更新用版本比较，冲突返回当前脱敏版本和刷新指引；禁止 last-write-wins 隐式替换。

<a id="des-byok-005"></a>
### DES-BYOK-005：按能力适配，而非只替换 base_url

Trace refs: O7-REQ-e0ae0d80-31ca-488b-8ba2-a728bfb9ce8d；[上游要求](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-005)。

预设区分 `provider_id` 与 `protocol`。保留既有 Responses 路径；Chat Completions 仅在其适配器完成工具往返、流式与用量契约后可启用，不能把 `letai_direct` 文本结果直接当作完整 Harness 工具闭环。DeepSeek 官方已提供 Chat Completions/工具调用说明；其文档也列有 Responses 入口，不能将“只支持 Chat”硬编码成永久产品前提。具体协议随候选配置验证，不做盲目协议探测或失败后隐式改协议。

传输允许域名仅为选定预设的 `api.deepseek.com` 或 `openrouter.ai`，实际 endpoint path 按已核实的官方协议固定。禁止跟随跨 origin 重定向、接受模型返回的调用 URL、用户自带代理地址或动态额外认证头；TLS 和出站策略必须阻止私网/metadata 访问、DNS rebinding 与 secret 转发到非批准目标。

模型适配器需保留工具名、schema、call ID、参数增量重组、工具结果顺序、终态及 usage。缺终态、格式损坏、工具未支持或必需输入无法容纳时返回明确阻塞，不把文本 JSON、截断结果或普通回答当动作。推理相关附加字段按具体协议保存必要历史，不能丢掉必需内容，也不能当作面向玩家或世界的事实。

OpenRouter Responses 按官方无状态合同携带必要历史，使用 `store: false` 且不设置非空 `previous_response_id`。Chat Completions 的 OpenRouter 路由默认锁在明确的允许集合，禁用集合外回退，并要求必要工具参数被支持；相应请求使用 `provider.only`、`allow_fallbacks: false`、`require_parameters: true`。其他协议只有在适配器能证明同等约束时才启用；字段不支持时拒绝该组合，不静默删除约束。默认不跨模型回退，实际模型/路由变化必须可诊断并重新检查适用性。

项目的 session、幂等和 provider invocation 标识保留在本地控制/诊断层，不无条件塞进供应商请求体；上游接受同名字段也不能证明免费重试或计费幂等。费用/用量缺失保留 unknown。

## 7. 关键流程

<a id="des-byok-002"></a>
### DES-BYOK-002：凭证、预算和数据流隔离

Trace refs: O7-REQ-26cefe7a-deb5-4b21-845a-a2d4556f7dca；[上游要求](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-002)。

Key 只从已认证网页提交到已明确展示的第一方执行端，随后按每次请求最小需要注入 HTTP 客户端。禁止写入进程级环境变量、命令参数、child-process 启动串、Debug/日志、APM 请求体、异常原文、session replay、浏览器 localStorage/sessionStorage/IndexedDB、Service Worker 缓存或分析事件。上游响应和错误同样按不可信数据脱敏，不能仅过滤请求 Authorization。

默认 EphemeralCredential 仅保存在受控内存，限制副本和可读主体，停用/过期后释放并尽力清零；禁止 core dump/请求体采样与 secret 备份。不能宣称能对不受控浏览器插件、主机管理员或内存取证提供绝对保密。客户端输入提交后清空，后续只持有短期应用会话及无独立授权能力的连接 ID。部署如无法满足内存和日志控制，不启用该功能。

每次上游发送前校验 owner/session、connection/version、固定 origin、有效租约和剩余模型预算；每个 probe、重试、推理调用均占用同一预算体系。并发调用在原子准入中预留次数、最大输入/输出 token 和可计算的费用上界，完成后以真实 usage 结算；usage/费用未知不返还为零，保守保留预留或暂停后续推理。429 按期限/Retry-After 有界重试，认证/余额/不兼容错误不自动重试、不自动充值。

固定调用次数、单请求 token、并发和时间上限是本执行端可强制的限制。只有已知定价、锁定路由、覆盖输入/推理/输出与上游附加收费的计算成立时，才能表述为本应用费用上限；否则展示估算及可执行调用上限，不能伪称能硬封顶整个供应商账户。Key 在其他应用的消费不受 oasis7 控制。

Prompt 只含当前授权的目标与世界投影，禁止注入凭证、其他用户目标/记忆和管理配置。新供应商/路由的数据去向重新确认；模型输出不能要求读取凭证、改支付来源或扩大允许的上游域名。

<a id="des-byok-003"></a>
### DES-BYOK-003：会话内执行与唯一世界权限

Trace refs: O7-REQ-84294c93-3275-4d19-8faf-389950b87464；[上游要求](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-003)。

激活只把已验证的模型连接接入第一方客户端。客户端用现有合法身份、任务/目标版本、授权观察、executor fence 和游戏预算向统一 Game Client 提交意图；模型 Key 不参与身份签名，不被转成世界权限。缺任务/委托、观察、报价或适用游戏接口就阻塞对应场景，不能回落到直接修改 kernel、特权私有工具或复制 World 状态。

首期 ModelSessionLease 使用有界滑动期限和不可续穿的绝对到期；部署配置必须在启用前提供明确值并由 capability 返回给页面。建议初始配置为续期间隔 30 秒、有效期 120 秒、绝对上限 2 小时，均是待验证的会话策略，不改变世界 tick 或 Runtime 授权期限。页面关闭不依赖 unload 可靠送达；执行端按本地可信时钟判定到期，并在每次发送前复核。浏览器后台节流可能导致到期，返回时如实提示，不无声续活已过期连接。

服务重启/分片失联不迁移原始 Key，也不从共享全局环境恢复；断开或租约到期阻止新模型请求并停止新决策，世界已经受理的长期作业仍按 Runtime 规则继续。模型租约、游戏 grant 和应用登录会话分别判定，任何一项恢复都不自动恢复另外两项。界面运行状态由实际执行端投影，不能由页面心跳倒推出模型正在工作。

## 8. 失败、恢复与一致性

<a id="des-byok-004"></a>
### DES-BYOK-004：停用、切换与迟到结果

Trace refs: O7-REQ-d3d63078-ef30-45fb-8ae6-2f6db1f46c34；[上游要求](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#req-byok-004)。

停用先原子标记 connection/version 不可再用并停止续期，再通知执行控制取消待发模型工作；已发请求尽力取消，但不能保证供应商不继续计费。停止状态必须回读确认，未确认不能只靠乐观 UI 宣称成功。执行端在模型返回后、产生新游戏提交前重查绑定、租约与 Runtime fence；已停用或过期结果只可成为受限诊断，不进入新决策。

切换以新连接完成检查为前提，版本 CAS 确认后执行既有[Provider Switch](provider-switch-intent-continuity.design.md#des-psw-003)停止/交接。新模型只读取授权的当前目标和世界结果，不自动复用旧模型私有推理；旧 provider 的未知世界 operation 必须保留原身份并先查询。读不到、超时、鉴权失败不能推定未发生；世界操作幂等性由现有 Game API/Runtime 合同保证，不由模型连接层重新实现。

多标签页共享已确认连接投影但不共享 raw Key；每次变更检查 version，旧页面不能用过期版本重新激活。停止/断开与并发 activate 的竞态以版本和撤销状态裁决；退出账号或 session 撤销使其连接停用。若已在原世界许可内完成提交，则保留原结果，不能因随后停用改写为取消。

## 9. 安全、隐私与可观测性

风险边界包含 XSS、CSRF、跨账号对象访问、SSRF/重定向泄漏、日志/采样泄漏、恶意模型内容、超量消费和迟到提交。含 Key 的表面采用严格 CSP、限制第三方脚本、禁用 session replay；执行端隔离凭证与 operator 路由，不把账户管理员权限交给模型 worker。安全评审覆盖实际部署，不只审核 DTO。

记录 connection 随机 ID、provider/model/protocol/preset revision、探测类别、尝试数、延迟、脱敏错误、已知用量及 unknown 标记；不记录 raw Key 或其可跨服务关联的公开指纹，不采集完整 prompt/response/供应商错误体。状态接口仅向当前有权主体暴露最小投影。

错误码按本地语义归一为 `authentication_failed`、`quota_unavailable`、`budget_exhausted`、`rate_limited`、`model_unavailable`、`capability_unsupported`、`transport_failed`、`session_expired`、`version_conflict` 等；HTTP 200 也需验证终态/工具结果。错误码不覆盖原世界 operation 的 pending/committed/rejected 等权威状态。

## 10. 兼容、迁移与部署

先复用第一方 Harness/Game Client 接缝，增加 provider-neutral 配置与短期凭证注入，再接 Web 组件。已有全局环境变量只允许显式单用户 legacy 路径继续使用；多用户 BYOK 路径中 credential_ref 缺失必须失败，禁止继承宿主 LetAI/OpenAI/任意全局 Key。

旧 `rust_direct_letai` 的平台账户、自动充值、平台 user/project 标识与 quota 换算留在 LetAI 专属模块；通用模型 transport 不依赖它们。禁用 LetAI/NewAPI bridge 时，新 BYOK 组合仍须独立通过验收；不因本次目标而删除已有支付/旧 Provider callback 的真实消费者。

单个受控执行端可承载首期会话，绑定实例内存，不为 Key 引入跨节点复制。无实例亲和或实例失效返回 session_expired 并重新连接，不把无 Key 节点自动接管为公共付费节点。分布式凭证托管、跨设备续接、长期无人值守另行设计；不能用部署可扩展性需要暗中改变默认保存边界。

功能默认关闭；按实际部署/协议/模型组合启用。未迁移场景保持明确不支持，而非等待全仓全量迁移或以 legacy 世界写入绕过目标边界。回滚停止新建/激活与新推理、清除有效凭证，保留可授权读取的历史世界 operation；不回滚世界事实。

## 11. 验证与验收

<a id="verification-mapping"></a>
### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 验证项 / 场景 | 方法与测试层级 | 证据与判定边界 | 状态 / 未覆盖范围 |
| --- | --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-a95a56b1-22b6-4955-b152-0c672310ddb8；[AC-BYOK-001](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-001) | [DES-BYOK-001](#des-byok-001) | BYOK-01 无 LetAI/CLI 的网页主流程；静态/缺服务入口 | Web E2E + 部署能力负例 | 两家逐一从表单至明确开始；未支持入口不收 Key | 计划验证；未运行真实网页 |
| Trace refs: O7-REQ-26cefe7a-deb5-4b21-845a-a2d4556f7dca；[AC-BYOK-002](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-002) | [DES-BYOK-002](#des-byok-002) | BYOK-02 租户隔离、CSRF、SSRF、重定向、日志与内存生命周期 | 单元/HTTP 集成 + 部署安全评审 | canary secret 不出授权端；不读全局 Key；销毁/脱敏覆盖异常路径 | 计划验证；文档存在不等于安全批准 |
| Trace refs: O7-REQ-26cefe7a-deb5-4b21-845a-a2d4556f7dca；[AC-BYOK-002](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-002) | [DES-BYOK-002](#des-byok-002) | BYOK-03 并发预留、probe/重试计费、未知 usage、预算耗尽 | 假时钟/计费 stub/故障注入 | 无超出本地许可的新请求；不把 unknown 当零；不承诺外部账户封顶 | 计划验证；真实费用需供应商记录 |
| Trace refs: O7-REQ-84294c93-3275-4d19-8faf-389950b87464；[AC-BYOK-003](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-003) | [DES-BYOK-003](#des-byok-003) | BYOK-04 观察→工具→合法意图→回执，越权/过期目标 | Game Client 集成 + prepared-world 闭环 | 同一 Runtime 准入路径；模型成功不能越权；实际世界结果另核验 | 计划验证；不代签全量游戏能力 |
| Trace refs: O7-REQ-84294c93-3275-4d19-8faf-389950b87464；[AC-BYOK-003](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-003) | [DES-BYOK-003](#des-byok-003) | BYOK-05 浏览器关闭/节流、租约到期、服务重启 | 假时钟 + Web/服务故障注入 | 到期后新上游调用为零；世界作业照原规则；恢复需确认 | 计划验证；不是长期托管测试 |
| Trace refs: O7-REQ-d3d63078-ef30-45fb-8ae6-2f6db1f46c34；[AC-BYOK-004](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-004) | [DES-BYOK-004](#des-byok-004) | BYOK-06 换 Key/供应商、停用、双标签页 CAS、迟到回复与原结果未知 | 并发/中断恢复集成 + Web E2E | 旧版本不发新请求、不产生新提交；原 operation 不重放且可追读 | 计划验证；上游取消/退费不作保证 |
| Trace refs: O7-REQ-e0ae0d80-31ca-488b-8ba2-a728bfb9ce8d；[AC-BYOK-005](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-005) | [DES-BYOK-005](#des-byok-005) | BYOK-07 Responses/Chat 工具往返、流式终态、路由/字段约束 | 固定响应 fixture + adapter 合同测试 | 协议可配不是已支持；不丢 tools/安全路由条件 | 计划验证；不硬编码全部模型支持 |
| Trace refs: O7-REQ-e0ae0d80-31ca-488b-8ba2-a728bfb9ce8d；[AC-BYOK-005](../../product/agents-world-simulation/self-supplied-model-connection.prd.md#ac-byok-005) | [DES-BYOK-005](#des-byok-005) | BYOK-08 两家独立真实模型 + BYOK-01/04 场景组合 | 授权真实 Key、有限预算、固定构建和部署 | 分供应商/模型/协议/路由/浏览器证据；无 LetAI/NewAPI 依赖 | 计划验证；未调用真实模型，未升级公开 claim |

本表都是待执行场景，不是新增 CI 状态或流程材料。实现 PR 记录实际测试命令、版本和结果；费用调用另获真实凭证/预算授权。fixture 不使用真实 Key；截图、HAR、日志和回执证据必须先脱敏。普通 CI 可验证确定性安全与协议合同，真实付费模型和真实玩家体验不能由其代签。

## 12. 设计决定、风险与外部依据

决定：沿用第一方执行端而不是先实现浏览器 Harness；提供两家预设而不是增加任意代理；默认短期内存凭证而不是自动永久保存；显式协议能力和共享 Game Client 而不是复制 LetAI 充值与世界写入代码。风险是执行端成为凭证处理方、网页后台续期会失败、供应商能力/价格会变化；分别通过知情同意与安全部署、明确到期恢复、固定版本能力/预算核验处理。部署方安全责任不可由 UI 声明转移给玩家。

官方资料核对日期：2026-10-16。下面仅支持上游接口事实；不证明 oasis7 的实现或实际部署，模型/价格不冻结在本设计。

- [DeepSeek API 入口](https://api-docs.deepseek.com/)与[工具调用](https://api-docs.deepseek.com/guides/tool_calls/)：官方地址、Key 与工具往返；不由服务端接口推断浏览器 CORS。
- [OpenRouter Responses](https://openrouter.ai/docs/api/reference/responses/overview)：无状态历史及 `store` / `previous_response_id` 限制。
- [OpenRouter 路由](https://openrouter.ai/docs/guides/routing/provider-selection)：允许提供商集合、fallback 和必要参数策略。
- [OpenRouter OAuth PKCE](https://openrouter.ai/docs/guides/overview/auth/oauth)：可作为后续免复制 Key 的连接方式，不是首期前置，也不自动提供安全托管。
- [OWASP HTML5 Security](https://cheatsheetseries.owasp.org/cheatsheets/HTML5_Security_Cheat_Sheet.html)：浏览器敏感数据存储和跨域安全风险。
