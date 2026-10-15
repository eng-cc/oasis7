# Agent Harness 与统一游戏客户端边界

- 状态：目标合同；本设计不声明代码迁移、真实 Runtime 支持或部署验收已完成。
- Owner role：`agent_engineer`；协作：`runtime_engineer`、`viewer_engineer`、`qa_engineer`、`producer_system_designer`。
- 审读基线：`eng-cc/oasis7@d6f6ff3d10f0f2990bf17a111d55c7b36ebf0a5d`；2026-10-15。当前/目标/差距见 §3；后续候选另记录实际源码与环境。
- 产品主责：[Agent/provider 体验连续性](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#first-party-agent-client)；[配对体验设计](../../product/agents-world-simulation/provider-agent-experience-continuity.design.md)。
- 系统入口：[LLM / external runtime](README.md)。实际实施与验证：[Issue #4726](https://github.com/eng-cc/oasis7/issues/4726)。

## 1. 问题、目标与非目标

当前已抽出通用 Harness primitives，但内置模型和 simulator/Viewer 的接线、外部签名客户端仍依赖主应用。仅移动文件不能保证内外 Agent 在观察、权限、重试和世界效果上的同一路径。

目标是把官方内置 Harness 作为第一方游戏客户端：内置决策与外部 Runtime 消费同一游戏交互合同，项目维护的 Rust/CLI 客户端复用可靠性实现，服务端统一准入和权威执行。World 不知道模型是谁，Harness 不取得 World 实现。

非目标：重写 Harness、创造第二套 Game API/任务数据库/共识；要求所有外部 Runtime 使用官方 Harness/SDK；强制同进程调用绕 HTTP；独立仓库或微服务平台；改变玩法、世界节奏、默认入口、支持矩阵和部署许可；无损迁移 Runtime 私有记忆。文档合入不启用路由、不松授权、不关闭旧代码。

## 2. 上游约束与相关角色

产品 owner 拥有可理解的选择与结果；agent owner 拥有客户端和决策编排；runtime owner 拥有世界权威、任务/执行许可和恢复；Viewer/launcher owner 消费状态并组织启动，不成为世界写者；QA 按实际候选分层取证。

现有 [Game API](external-agent-game-api.design.md#api-composition)、[外部 Runtime adapter](external-agent-runtime-adapter.design.md#adapter-loop)、[Runtime cognition](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md)、[协作权威](../../world-runtime/runtime/player-runtime-collaboration.design.md)、[Provider Switch](provider-switch-intent-continuity.design.md#des-psw-003)各自继续拥有协议、控制与恢复细节。本文收敛它们的调用与依赖关系，不复制其状态机。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | 已声明能力内保持单向依赖和唯一世界权威，官方客户端没有内部特权捷径 | [DES-AGC-001](#des-agc-001) | agent / runtime；现有共享合同与 World Service | 不承诺模型策略、费用/延迟或所有平台已支持 |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | 相同主体、授权、任务版本、世界切点与观察范围下复用编码、签名、准入和结果恢复合同 | [DES-AGC-002](#des-agc-002) | agent / runtime；Game API、普通客户端与 Runtime 验证 | 随机 ID 不要求字节相同；合同一致不证明真实模型 parity |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | 有效完整目标和授权观察驱动主动决策；不以 shadow World、截断或过期 Context 补能力缺口 | [DES-AGC-003](#des-agc-003) | agent / runtime；输入完整性、Context 与目标控制合同 | 不扩大 lane 观察范围，不放宽 TTL/tick/deadline |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | 关闭已迁移场景的旧通道后仍可完成目标与恢复，全部保留能力逐项收口 | [DES-AGC-006](#des-agc-006) | agent / runtime / QA；能力映射与旧消费者 | 首样例不证明全量生产能力、冷进入或完整 starter chain |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | 控制循环独立处理停止、许可和预算变化，模型阻塞不阻塞世界且不静默替代 | [DES-AGC-003](#des-agc-003) | agent / runtime；执行许可、模型预算与世界推进 | 不承诺未测规模、云托管或固定性能收益 |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | 玩家可确认实际执行方、运行位置、费用/预算、委托与停止状态，不误报持续托管 | [DES-AGC-005](#des-agc-005) | agent / launcher / Viewer / QA；实际进程和玩家入口主责 | 不新增云托管、免费模型、默认执行方或其它入口支持 |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | 迁移和升级保留便捷启动、执行方状态与阻塞解释，不通过旧通道静默兜底 | [DES-AGC-006](#des-agc-006) | agent / launcher / Viewer / QA；实际发行组合与兼容性 | 不把单平台或局部安装成功升级为全部组合已支持 |
| Trace refs: O7-REQ-70188c23-ddbf-49ca-8a70-3fa05c152ba5；[REQ-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-002) | 新旧执行方切换保留原未知请求，隔离迟到结果且不重放 | [DES-AGC-004](#des-agc-004) | runtime / agent；Provider Switch 与 task fence 合同 | 不授予无权读取或自动撤回已提交作业的能力 |
| Trace refs: O7-REQ-70188c23-ddbf-49ca-8a70-3fa05c152ba5；[REQ-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-002) | 迁移和回滚保留旧缓存/历史读取，不复活旧执行资格或丢弃未知结果 | [DES-AGC-006](#des-agc-006) | runtime / agent；历史格式、Provider Switch 与支持构建 | 保留历史读不代表旧执行权限或所有 binary 可降级 |
| Trace refs: O7-REQ-21f1c7b1-8be0-438e-8f92-d4d1b34be0e6；[REQ-AGENT-PARITY-001](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-001) | 分能力、平台和执行方独立取证，局部共享不冒充全量支持或 parity | [DES-AGC-006](#des-agc-006) | 产品 / agent / QA；支持矩阵与实际证据 | 不代签未运行Runtime、冷进入、完整首局、公开发行或部署 |

## 3. 当前状态、目标状态与差距

本表是固定基线源码事实与目标差距，不是执行台账。

| 对象 | 固定基线事实及定位 | 目标与差距 |
| --- | --- | --- |
| Harness | [`oasis7_agent_harness`](../../../crates/oasis7_agent_harness/src/lib.rs) 已导出 actor、TurnEngine、记忆/continuation 与 ModelProvider；[`Cargo.toml`](../../../crates/oasis7_agent_harness/Cargo.toml) 不依赖主应用 | 保留算法，把仍在应用内的模型/prompt/策略接线移为客户端实现；crate 存在不等于路径统一 |
| 内置接线 | [`agent_harness_adapter`](../../../crates/oasis7/src/simulator/agent_harness_adapter.rs)、[`llm_agent`](../../../crates/oasis7/src/simulator/llm_agent.rs)、[`llm_sidecar`](../../../crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs) 仍使用 simulator/runtime 类型 | 按权威拆分，客户端只消费合法投影；不能整体搬走 World 权威或以 shadow kernel 补观察 |
| 普通客户端 | [`client.rs`](../../../crates/oasis7/src/bin/oasis7_game_client/client.rs)、[`client_build.rs`](../../../crates/oasis7/src/bin/oasis7_game_client/client_build.rs) 已实现签名、报价和原操作恢复，但 import 主 crate 的 runtime/world_service/private_goal_store | 抽轻量客户端与必要公共合同，CLI 变薄；仅换目录而保留反向依赖不算完成 |
| 外部能力 | [`oasis7-play`](../../../skills/oasis7-play/SKILL.md) 是 prepared-world 普通签名客户端 profile，覆盖移动、转运、内置建厂/配方和原结果查询 | 不等于完整 Game HTTP 写目录、冷进入、通用模块或两个真实 Runtime 全首局已交付 |
| 权威接缝 | [`RuntimeAuthority`](../../../crates/oasis7_agent_harness/src/authority.rs) 要求宿主验证 lease/receipt/continuation；[`共享 API`](../../../crates/oasis7_agent_api/src/lib.rs) 只承载提案和投影 | 以受信客户端适配供给已验证读模型，不能让客户端自签世界最终性或创建执行许可 |

现有 Game HTTP bootstrap 的只读/默认关闭边界以及原生 World Service 任务写入切片按 [Game API 当前实施范围](external-agent-game-api.design.md)保持。这里的统一业务合同不是“所有 `/v1/game/*` 已上线”的别名，也不是旧 `/v1/provider/*` 的重命名。

## 4. 边界与结构

<a id="des-agc-001"></a>
### DES-AGC-001：职责、依赖与复用边界

```text
外部 Runtime + Skill            官方内置 Harness
     |                              |
CLI / 工具适配                     Rust 调用
     +---------------+--------------+
                     v
             共享 Game Client 库
       受权读 / Context / 报价 / 签名 / 原操作恢复
                     v
            同一游戏交互合同与 transport
                     v
        服务端接入 -> 统一应用准入 -> World/Runtime
         身份和限制     task/grant/fence     效果和回执
```

外部独立客户端也可直接实现公共协议；官方 SDK 是复用选项，不是授权前提。Skill 是说明资产，不是 daemon、安全边界或世界写权。CLI/工具包装只转换输入输出，不能重实现游戏授权/资源规则。

**依赖方向：** Harness/第一方执行器 → 客户端端口与轻量合同；SDK/CLI → 轻量合同、签名/验证和 transport；服务端 → 同一合同 + Runtime。复用现有 `oasis7_agent_api` 与 `oasis7_client_api` 分工，必要公共 wire 类型或纯 codec 向合同层移动，保留唯一声明和兼容再导出，禁止复制 Action/schema。

SDK 和 Harness 的受支持 feature/target 依赖闭包不得反向包含主 `oasis7`、World/kernel 或 Viewer 实现。单独构建 CLI/SDK 不应拉入模型 SDK 或模拟器。组合 binary 可组装两侧，但组合入口不能成为反向依赖漏洞；世界执行核心关闭 Agent 集成后仍能独立构建和推进。crate 名称按实际依赖拆分决定，不按图的层数机械建包。

| 职责 | 权威/实现归属 | 不可越界 |
| --- | --- | --- |
| 模型、prompt、规划、工具选择、私有记忆、模型预算 | Harness / 各外部 Runtime | 只能形成候选；不持有 World，不从服务器文件补信息 |
| 读、签名/服务验证、报价、发送前缓存、提交/查询、客户端恢复 | 共享 Game Client | 缓存不成为任务权威，不自行授予 grant 或修改旧请求 |
| 任务/目标权威版本、owner/executor、活动 turn/lease、Stop/撤权 | Runtime / 协作服务 | 控制授权与持久性；不是可整体外移的“思考状态” |
| 准入、资源、世界状态/作业、最终执行结果、回放 | World / Runtime | 不因执行方或传输变更产生第二套实现 |
| 展示、启动与进程诊断 | Viewer / launcher | 不以在线/模型结果代签效果或获得隐式 owner 权力 |

首轮复用现有远程 World Service 客户端。以后加入同进程 transport，只能省序列化/网络；仍经过同一认证上下文、能力、fence、提交与回执处理。不允许以 process-local 指针或 `trusted=true` 绕过普通 executor 的规则。

## 5. 关键运行流程

<a id="des-agc-003"></a>
### DES-AGC-003：主动决策与独立控制

控制循环独立于模型调用：读取有效任务/委托与执行方状态，处理目标变化、Stop、撤权、超时和预算；不能等待模型返回才停止新的出站工具。世界侧当前 fence 是最终防线，本地 cancel 只表示请求中断。

决策循环使用：当前 task/完整有效目标 → 同切点受权 Context 与能力 → 模型/策略按需只读查询 → 候选行动/报价 → SDK 保存原请求并提交 → 验证原权威结果 → 在仍有许可时开始新轮。模型会话 ID 与世界请求 ID 分开，不能混作幂等键或新权限。

完整目标、软约束和必需工具结果必须进入获授权模型上下文；不能为接旧 Harness 摘要而静默截断为旧长度。原始 `world-agent-state/v1` 不能强转成旧 ProviderObservation；缺少公开投影时先补主责接口或声明 unsupported，不读取 shadow World 的隐藏字段。不同 lane 的观察限界原样保留。

当前 native Context 租期消费既有上限及 grant 到期限制，现有 profile 最多 16 ticks；客户端不续租、不改变世界时钟，也不自动用新 base 重签旧请求。长推理返回时先验证可用性；已过期则按 §7 保留原结果和终结/到期条件，再取得新 Context 重新规划。若持续世界推进使合法决策不能完成，应测量并由 Runtime/agent owner 另审活性方案，不松 TTL、tick 或 deadline 掩盖缺陷。

World::step 不等待 HTTP 或模型。已提交的生产、转运和确定性 WASM 世界作业留在 World 执行，即使没有 Harness 也按规则推进；只有后续新的候选由执行方发起。若存在确定性内置测试/脚本消费者，明确其测试或系统角色，不把它冒充普通玩家的统一路径。

## 6. 接口与数据合同

<a id="des-agc-002"></a>
### DES-AGC-002：共享客户端合同与唯一行为

下表是客户端职责映射，**不是新 endpoint、RPC、wire schema 或已经实现的方法名**。实现优先复用既有 Game API/World Service 数据和签名 domain，不把服务端实现类型全部公开。

| 客户端职责 | 输入/约束 | 输出、错误与唯一主责 |
| --- | --- | --- |
| 发现/授权观察 | 世界/Agent/主体/实际版本与 scope | 实际授权目录与投影、截断/新鲜度；目录不代替最终 admission |
| 当前任务与 Context | 当前 owner/executor fence、稳定原 operation、完整目标 | 同切点、有效期、已验证服务关联；prepared 不等于可行动 |
| 查询与报价 | 当前 Context、业务选择、预期 base | 由服务端重用真实准入生成规范计划；报价不预留、不扣费、不保证稍后通过 |
| 提交行动/终结 | 稳定原请求及当前授权；同 Context/操作种类只保留原内容 | 受理/待决/权威终态的既有语义；同键异内容冲突，不生成替代操作 |
| 查原结果/读变化 | 原请求身份、当前读取资格、已支持的游标/版本 | 已验证关联和权威结果；断连或无权查询保持未知，不返回旧缓存冒充当前授权 |

SDK 统一严格编码、签名、世界/主体/服务绑定验证、有限响应解析、错误分类和有界 transport retry。模型失败重试属于 Harness；游戏操作恢复属于 SDK；资源准入与业务重试条件属于 Runtime。不得三处各建一套结果状态机。

权限、fence、能力、任务版本、基准状态和资源均由服务端在适用的提交与实际执行边界重验。内置、普通 CLI 与自行实现的网络客户端即便绕过 SDK，也不能绕过服务器校验。共享错误/行动 registry 有唯一来源；如果迁移 DTO，保留原 wire 和数字/签名编码，不能以“重构”修改历史验证语义。

`RuntimeAuthority` 继续消费宿主已完整验证的 lease、持久结果和 continuation 投影。客户端实现可从已验证服务响应构造只读视图，但关联 ID、lineage、模型自报成功或普通 DTO 本身不是证明。受信服务签名证明服务交付内容，不代替当前权威 profile 要求的共识/持久化最终性证据。

## 7. 状态、一致性与恢复

<a id="des-agc-004"></a>
### DES-AGC-004：唯一世界事实与原操作恢复

| 状态/材料 | owner 与提交点 | 恢复/并发约束 |
| --- | --- | --- |
| 世界任务、目标激活、executor epoch、活动许可 | Runtime canonical receipt 与现有 checkpoint/journal | 不增加网关内存表作为第二权威；当前控制优先于旧本地缓存 |
| 原签名准备、prefix、action、outcome 及其关联 | SDK 本地加密恢复存储，按既有格式发送前持久化 | 同键同内容恢复、异内容冲突；损坏/缺失且无法确认原意图时 fail closed，不悄悄换 nonce |
| 私有会话、工作记忆、模型调用进度 | Harness / 外部 Runtime | 不能由本地完成推出世界完成；跨 Runtime 私有记忆迁移不在本次保证 |
| 已受理的世界作业与效果 | World / Runtime | 已提交效果不因 Stop/换执行器撤销；尚需执行准入的旧请求仍重验 fence |
| 心跳、日志、重试和用量 | 执行方诊断投影 | 有时效、可缺失；不是执行许可、活动 turn 锁或世界事实 |

原 action 结果未知时只查原结果或按合同重投完全相同请求，不重新报价、换 action 或捕获新身份来替代该副作用。明确拒绝不必然释放活动 Context；原 outcome 也可能拒绝，按既有终结/租期条件收口后才取新 Context。查询权限被撤销时保持未知，不擅自恢复旧授权。

只读恢复保留原身份与权限边界：原结果轮询只查询缓存请求；准备阶段只可重投已持久化的同一签名。传输失败、HTTP 503 和已分类的有界 cooldown 可在调用窗口内恢复，普通 403/409/429、签名或主体验证失败不自动重试。context read 仅在服务端重验当前权限、fence 与 TTL 后确认交付切点变化时返回 `task_context_read_delivery_drift`；factory/recipe quote 仅用 `task_quote_base_drift` / `task_quote_delivery_drift` 区分已知只读基准竞态。SDK 对这些固定路由/状态/代码的有限重读不续期、不换 Context/nonce、不改变业务选择；已有 action/outcome 时优先恢复原件，不再次报价。此机制不能消除响应返回到行动准入之间的世界变化，权威 `stale_base` 拒绝仍按原结果处理，后续新决策由执行方策略负责。

切换先经现有 task executor 控制使旧 epoch 失效，并围栏旧入口；当前授权下最多一个活动 turn。两个 Harness 的进程锁、不同 session ID 或启动顺序不能取代世界侧互斥。目标变化后旧计算可被丢弃，但新世界效果只接收当前许可；已提交原结果按原关联保留。原结果查询与新行动资格分别校验。

反馈按世界/Agent/task/fence/请求关联隔离，重复与乱序不重复应用记忆投影或世界效果；客户端恢复不是一套第二世界回放引擎。缓存格式、密钥文件与历史目标的兼容必须沿用既有保护；抽 `PrivateGoalStore` 的纯本地能力不能让客户端读取服务器私有存储。

## 8. 部署、安全与运行约束

<a id="des-agc-005"></a>
### DES-AGC-005：独立执行、身份与费用隔离

同仓库模块边界优先于进程/仓库拆分。第一方执行器可由现有 launcher 组织启动；模型并发 worker 不逐 tick 启 CLI，不以创建大量微服务取得逻辑解耦。后续独立进程由实际隔离/容量收益决定，不改变公共合同。

界面明确实际执行方、运行位置、启动/阻塞/停止、模型预算来源与当前合法 Agent。关闭 Viewer 不应销毁已独立运行的执行器；执行器退出不能伪报持续思考。没有独立托管能力就不声称云端仍在执行。进程/心跳与权威任务状态分别读取，不能用健康接口代签目标完成。

游戏 owner/设备签名、executor delegation、模型 API key、operator 权限相互独立。官方客户端默认使用普通受限 executor 凭据，不取得 owner、世界目录或管理员权限；角色差异必须通过明确授权表达。不会因换模型自动创建/认领另一个游戏 Agent。

私有目标/观察只交给已授权模型上下文，不写入公开 reason、日志或模型思考转录。凭据与客户端缓存使用原安全机制，不随 issue、PR、截图或日志发布；Skill 只是说明，不能声称其限制已提供 OS 级隔离。HTTP/TLS、loopback、多用户主机和重定向限制沿用原 World Service 合同，不通过同进程适配降低安全要求。

预算分层：模型调用预算在执行方，世界动作/资源预算在 Runtime；一项获准不意味着另一项获准。连接失败、超时或用量未知保持真实状态，不静默调用付费替代模型、切换 Runtime 或以默认 Wait 伪装有效决策。耗尽时停止新调用并执行合法的原轮次收口，未知费用不记零。

## 9. 质量与容量

新增 transport 和执行循环的收益须测量，不预先宣称降低构建时间或延迟。质量目标消费现有适用 profile；无现成性能阈值时，由实现 PR 固定原基线/候选/资源并报告测量与 owner 判断，不捏造默认数字。

| 场景 | 环境、规模与资源 | 预期响应 | 指标/阈值来源 | 验证入口 | 当前范围 |
| --- | --- | --- | --- | --- | --- |
| 模型阻塞/并发执行 | 同硬件、世界规模、tick、并发 Agent 和预算，固定基线与候选 | 世界不等待模型；队列、重试和轮询有界 | 既有 profile 限制；tick 延迟/积压、CPU/内存与队列深度 | AGC-06、AGC-10 | 未验证 |
| 普通操作/原结果恢复 | 相同合法/非法意图与故障时点，分别 SDK/CLI | 准入/后果一致，故障不重复效果 | 世界语义与至多一次效果是硬约束；P50/P95 分离模型/客户端/服务耗时 | AGC-02..05、AGC-10 | 未验证 |
| 客户端构建和平台 | 实际声明 target/features，含默认与隔离构建 | 不反向依赖主应用、无隐藏模型依赖 | DES-AGC-001；实际依赖闭包而非包名 | AGC-01、AGC-09 | 未验证 |

优先沿用已有读变化或受支持通知；只有轮询时使用明确有界退避和生命周期关闭。通知只是唤醒提示，错过通知仍按原权威状态恢复；不新增消息基础设施作为首轮依赖。批量/增量观察和同进程优化不得泄露更宽 scope，未经测量不改变传输默认值。

## 10. 兼容、迁移与回滚

<a id="des-agc-006"></a>
### DES-AGC-006：按能力收口与旧路径退役

迁移顺序是：盘点消费者与能力 → 抽 SDK/合同依赖且 CLI 行为不变 → 内置首个完整闭环走普通客户端 → 逐项迁移保留能力 → 关闭已迁移入口的旧 runner/callback → 删除无独立消费者的接线。首个闭环不是全量迁移的验收替代。

| 迁移对象 | 保留或迁出规则 | 关闭条件 |
| --- | --- | --- |
| 模型/prompt/私有记忆/策略接线 | 迁到客户端执行方，复用现有 Harness | 不接 World/simulator 私有输入，同候选输入完整性及行为回归 |
| 原 CLI 与 RemoteWorldServiceClient | 变薄并共用 SDK；wire/签名/原操作保持 | 单独构建与普通客户端协议、缓存恢复回归 |
| 查询/模块动作/等待/聊天/目标/continuation | 逐项映射公共能力；未覆盖明确受限或保留旧 profile | 有授权投影与主责测试；不以少数 native 动作代表全能力 |
| Runtime lease/任务/回执/世界作业 | 留在权威端，不按 Harness 文件名搬走 | 迁移前后执行、权限与历史恢复相同 |
| 旧 Provider/runner 与 WASM/测试消费者 | 按实际独立消费者保留窄兼容 | 未迁移消费者有去向，已迁移 Agent 无双写，历史读取仍可用 |

每项能力映射至少说明原调用点、公开合同、目标路径、观察/权限范围、验证与保留/退役理由；可变台账在实现 Issue/PR，长期文档只保留支持边界。纯 DTO 共享、仅跑 mock、仅移文件或仅通过首样例都不满足全量收口。

没有与全部保留能力对应的证据，不把新路径设为全量默认。新旧 profile 可以显式共存，但同一 Agent 不允许两个写入者；不能在运行失败后偷偷转旧通道。外部标准 HTTP 首期目标及两个真实 Runtime 的独立验收继续有效，Rust SDK 的成功不把公共 API 降级成 CLI-only 产品。

客户端格式/类型迁移优先 wire 不变；如必须改变 schema 或历史变体，先明确读/写版本和部署兼容，再同步所有消费者。保留旧历史解码不等于允许旧身份发新请求。回滚只回到能读当前历史且符合当前授权的已知构建；不能撤销已提交成果、复活旧 epoch 或重发未知行动。不满足兼容时停用执行器并前向修复，不恢复特权路径。

## 11. 验证设计与可追溯性

AGC 是本设计的**计划场景 ID**，并非已有测试名称或通过报告。实现复用真实测试入口，补齐能检出反例的回归；具体候选、命令、非零执行数量、环境与原始结果放在 [#4726](https://github.com/eng-cc/oasis7/issues/4726)及实现 PR。规范检查不证明运行正确。

验证资料按职责引用：[总测试手册](../../../testing-manual.md)约束固定候选、环境与分层执行；[外部合同手册](../../testing/manual/external-agent-runtime-contract-validation.manual.md)拥有下表具名的 API、任务、围栏与恢复场景；[独立实玩手册](../../testing/manual/agent-runtime-self-playtest.manual.md)拥有真实试玩、预算与隔离方法。每行只链接一个主验证入口；同手册中的补充场景和跨手册方法仍按正文明确组合执行，不减少验证义务。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | [DES-AGC-001](#des-agc-001) | 已声明能力内保持单向依赖和唯一世界权威，官方客户端没有内部特权捷径 | AGC-01：依[测试手册](../../../testing-manual.md)固定实际 manifest/target/features，分别构建 SDK/Harness/CLI，检查 cargo metadata 依赖闭包与唯一 wire 声明；补充架构回归，证明无主应用/World/Viewer或隐藏模型依赖。该回归是实现后的计划，不是现有通过记录 | 实际构建命令、非零执行数量、依赖输出与架构回归结果 | 不承诺模型策略、费用/延迟或所有平台已支持 |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | [DES-AGC-002](#des-agc-002) | 相同主体、授权、任务版本、世界切点与观察范围下复用编码、签名、准入和结果恢复合同 | AGC-02、05：按[api-wire](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-wire)与同手册 `api-safety` 场景的隔离和对账方法，固定初始世界/主体/fence/意图，分别经 SDK 与独立 CLI/网络入口；注入错签名/scope/越权/伪报价/私有日志。原生已实施切片与完整 HTTP 目标分别记录，不互相代签 | 规范化准入/拒绝、世界效果与回执对账、负例和脱敏结果 | 随机 ID 不要求字节相同；合同一致不证明真实模型 parity |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | [DES-AGC-003](#des-agc-003) | 有效完整目标和授权观察驱动主动决策；不以 shadow World、截断或过期 Context 补能力缺口 | AGC-02、04、05：扩展[能力目录场景](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-catalog)与同手册目标版本场景 `collab-goals`，比较同观察范围固定决策序列；核对完整目标/必需工具输入，并注入长推理、Context临界过期、quote后世界变化和隐藏状态访问；先恢复原结果，再合法重取 | 输入/授权范围对账、原请求及权威拒绝、合法后续轮次结果 | 不扩大 lane 观察范围，不放宽 TTL/tick/deadline |
| Trace refs: O7-REQ-ad3ea59c-596c-4c50-8da8-fe47608c6f01；[REQ-AGENT-PARITY-003](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-003) | [DES-AGC-006](#des-agc-006) | 关闭已迁移场景的旧通道后仍可完成目标与恢复，全部保留能力逐项收口 | AGC-07、08：复用[旧路径停用场景](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-cutover)与同手册能力目录场景 `api-catalog`，在真节点 prepared-world完成移动/建厂/转运/排产；逐项覆盖查询/模块/等待/目标/聊天/记忆/continuation及两lane，不把未实现扩展当已运行 | FactoryBuilt/RecipeCompleted/到账等实际结果、能力映射及逐项回归 | 首样例不证明全量生产能力、冷进入或完整 starter chain |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | [DES-AGC-003](#des-agc-003) | 控制循环独立处理停止、许可和预算变化，模型阻塞不阻塞世界且不静默替代 | AGC-06、10：按[执行方替换场景](../../testing/manual/external-agent-runtime-contract-validation.manual.md#collab-bind)的身份对账方法，补充卡模型、双执行器、断网和预算耗尽；在总测试手册规定的固定候选环境中对比同硬件/规模/tick/profile的P50/P95、CPU/内存/队列、tick延迟和积压 | 无替代、单活动执行者和世界独立推进的记录，性能原始样本 | 不承诺未测规模、云托管或固定性能收益 |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | [DES-AGC-005](#des-agc-005) | 玩家可确认实际执行方、运行位置、费用/预算、委托与停止状态，不误报持续托管 | AGC-06、11：按[实玩手册](../../testing/manual/agent-runtime-self-playtest.manual.md)与外部合同手册的结果展示场景 `collab-report`，对获准组合检查选择/开始/关闭Viewer/回来/停worker/预算失败；核对进程、世界作业和费用未知分别表达，各玩家入口单独取证 | 平台/入口、实际执行方和预算绑定，状态与世界结果对账及玩家操作证据 | 不新增云托管、免费模型、默认执行方或其它入口支持 |
| Trace refs: O7-REQ-d452396d-291d-41a5-a912-6d615b58c29e；[REQ-AGENT-PARITY-004](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-004) | [DES-AGC-006](#des-agc-006) | 迁移和升级保留便捷启动、执行方状态与阻塞解释，不通过旧通道静默兜底 | AGC-09、11：依[切换验证](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-cutover)与独立实玩方法，对声明平台的旧新客户端检查启动/恢复/停止、未知费用和缺组件状态；正常启动不要求玩家发现内部进程，失败不得自动切换模型或Runtime | 实际构建/平台/安装组合、正常与失败操作结果、无静默回退证据 | 不把单平台或局部安装成功升级为全部组合已支持 |
| Trace refs: O7-REQ-70188c23-ddbf-49ca-8a70-3fa05c152ba5；[REQ-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-002) | [DES-AGC-004](#des-agc-004) | 新旧执行方切换保留原未知请求，隔离迟到结果且不重放 | AGC-03、04：扩展[原操作幂等场景](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-idempotency)与同手册执行方围栏场景 `collab-bind`，覆盖丢响应/重启/同键异内容/损坏缓存/反馈乱序、临界过期/换目标/Stop/撤权/替换/迟到结果；同键原内容恢复，未知不换请求 | 原签名/关联、canonical结果、单活动turn与至多一次效果证据 | 不授予无权读取或自动撤回已提交作业的能力 |
| Trace refs: O7-REQ-70188c23-ddbf-49ca-8a70-3fa05c152ba5；[REQ-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-002) | [DES-AGC-006](#des-agc-006) | 迁移和回滚保留旧缓存/历史读取，不复活旧执行资格或丢弃未知结果 | AGC-09：按[切换验证](../../testing/manual/external-agent-runtime-contract-validation.manual.md#api-cutover)与同手册持久恢复场景 `collab-recovery`，运行实际旧新客户端/声明平台；验证旧缓存、snapshot/journal/receipt读取，围栏后恢复与不兼容拒绝；只允许能读当前历史且权限兼容的回滚 | 固定版本兼容矩阵、原历史/结果和拒绝不兼容降级的证据 | 保留历史读不代表旧执行权限或所有 binary 可降级 |
| Trace refs: O7-REQ-21f1c7b1-8be0-438e-8f92-d4d1b34be0e6；[REQ-AGENT-PARITY-001](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#req-agent-parity-001) | [DES-AGC-006](#des-agc-006) | 分能力、平台和执行方独立取证，局部共享不冒充全量支持或 parity | AGC-08、11：按[实玩手册](../../testing/manual/agent-runtime-self-playtest.manual.md)与外部合同手册的分 Runtime 首局场景 `api-first-session`，获授权独立试玩者仅用实际Skill/普通凭据；分别记录真实内置模型、各外部Runtime及玩家入口。缺前置明确未验证，不用固定决策替代模型支持 | source/build/Skill/runtime/profile/权限/预算/环境绑定及目标、Stop/重启/回访材料 | 不代签未运行Runtime、冷进入、完整首局、公开发行或部署 |

验证顺序为静态依赖→确定性/故障注入→真节点普通客户端→获授权真实模型/玩家表面；模型费用和外部环境必须实际获准。缺前置记录未验证/阻塞，不用 mock、空测试选择或普通 CI 替代实玩。独立安全/恢复/兼容审查按现有流程组织，不新增审批协议或门禁身份。

## 12. 决策、长期风险与未决问题

| 决策 | 被排除的方案与理由 | 风险、owner 与复核触发 |
| --- | --- | --- |
| 第一方客户端 + 共享 SDK，复用现有 Harness | 仅搬文件仍保留 World 耦合；新建另一套 Harness/协议增加状态和恢复分叉 | agent/runtime owner 在能力映射或依赖抽取无法保持唯一合同、普通权限时复核 |
| 统一游戏语义，不强制外部 Harness/SDK | 强制嵌套官方 turn loop 损害外部 Runtime 自主会话；统一 DTO 本身不能保证统一准入 | agent owner 以独立普通客户端反例验证接口，不以官方包装自证可用 |
| 首轮远程 transport，逻辑拆分优先 | 立即微服务化增加运维；先做两套 local/remote 快路径放大校验差异 | 性能 owner 在实际测量表明 transport 为瓶颈时评估同进程/批处理，仍受同一校验 |
| 不迁出世界许可和确定性世界作业 | 全部“agent/cognition”文件外移会丢权威、恢复或世界自治 | runtime owner 复核 lease、memory receipt、continuation、历史解码归属 |

尚需实测的是具体 crate 切口、保留能力覆盖、受支持平台安装体验、长推理与有界 Context 的活性，以及 transport 的真实成本。实现 PR 必须给出对应结果和范围；它们不是放宽权限、改默认值或自动发布的授权。当前部署暂停/公开支持限制仍由原发行与部署 authority 持有，本文不解除。
