# 链 HTTP 提交与 Explorer 查询一致性合同

- 配对设计：[`chain-http-submit-explorer-contract.design.md`](chain-http-submit-explorer-contract.design.md)
- 上位专业 authority：[`P2P PRD`](../prd.md)；执行、提交证明与恢复仍由 [`World Runtime PRD`](../../world-runtime/prd.md) 定义。
- 上位产品：[`世界基础设施成功标准`](../../product/world-infrastructure/prd.md#world-infrastructure-done) 的既有 SC-2 / SC-5；本文只承接请求关联与只读观察的专业组成部分。
- Owner role：`runtime_engineer`；验证由适用 QA 责任承接，Launcher 表现由其现有专业 authority 承接。
- 复核日期：2026-10-12；静态基线：`e01e29064517f4bf375767a19a09802335828249`。
- 合同性质：待实现与验收的目标要求；[#4450](https://github.com/nil-studio/oasis7/issues/4450#issuecomment-6126251006) 已在文档 PR #4637 承接后以 `not_planned` 关闭，仅为历史来源，不是当前执行台账。H1 共享身份与可靠处置、H2 状态单调、H3 P0 独立同步、H4 组合过滤仍由本合同保留；实际实现与验收由承接工作的 PR 和真实验证记录说明，关闭不表示完成。

## 1. Executive Summary

同一个 Node 接收多个 HTTP 提交入口的请求后，调用方需要凭返回身份分辨每个请求的后续结果。直接使用 Explorer 的测试、运维和机器客户端也需要独立读到本进程已提交的批次，而不依赖另一个用户先打开概览或查询状态。

本文定义 HTTP 接受责任、action 身份、tracker / Explorer 生命周期、P0 独立查询与组合过滤的稳定专业合同。范围为现有 chain HTTP submit → Node 队列 → tracker / Explorer 后端管线，以及这些结果经过现有 native / Web 代理后的语义保留。

上述固定基线的静态审读仍发现跨 HTTP 入口身份分配不统一、迟到接受登记可能覆盖确认、P0 查询依赖其他入口先同步、action ID 过滤可能跳过其余条件。它们是实现与验收缺口；静态分析不等于实际故障复现、交易执行失败或资金损失证据。本次文档收敛不把 H1–H4 标记为已修复，也不复制可变任务台账。

成功意味着：每个已接受请求保持可关联；批次失败不会静默吞掉请求；已知确认不被迟到登记或时间刷新覆盖；P0 单独使用时能看到本进程已提交批次；同时填写的交易过滤条件共同生效。五项要求分别验收，局部通过不能代替整个工作包完成。

本文不改变 HTTP `200 = accepted` 的含义。`accepted` 不表示已经 committed；Explorer `confirmed` 表示现行管线观察到入块，不自动证明执行成功、余额结算成功、正式最终性或网络就绪。

## 2. User Experience & Functionality

### 2.1 使用者与正常路径

| 使用者与场景 | 可观察结果 | 后续选择 |
| --- | --- | --- |
| 调用方在同一处理窗口提交不同类型的合法动作 | 每个请求获得可区分、能关联其后续处置的 action 身份 | 按原请求身份核对结果；不把其他请求的结果当成本次成功 |
| 测试或运维人员跟踪已接受请求 | 接受、等待确认、已观察入块、明确失败及观察超时的含义可区分 | 根据真实处置继续等待、核对或修复；未知结果不作为安全重提依据 |
| 机器客户端只查询 Explorer P0 | 无需先访问 overview / status / history，也能读取当前进程已提交批次 | 继续使用同一查询表面检查交易及区块关联 |
| native / Web 用户组合筛选交易 | 输入条件、命中条目、总数、排序与分页一致 | 调整或清空条件；无命中是可解释空结果 |

<a id="http-h1-identity"></a>
### 2.2 H1：共享 action 身份

- **Why：** 跨入口请求共用 Node 队列时，入口各自看似唯一的编号不足以支持逐请求关联。
- **What：** 同一 Node action 命名空间内，所有接入同一队列的 HTTP producer 必须遵循共享身份规则；transfer、main-token、gameplay / claim 等实际入口不得因各自分配编号而让不同合法请求取得冲突身份。
- 身份保证必须明示 Node / 进程范围、其他 producer 的相容边界，以及重启后与仍保留记录的关系。不得仅靠给两个 endpoint 错开常量宣称完成，也不承诺全网、全时段或跨重启永久唯一。
- consensus action ID、执行世界 action / job ID 与 `client_request_id` 是不同事实，不得互相替代；本要求不新增客户端重提的 consensus-level dedupe 保证。
- **Done：** 真实 HTTP handler 的隔离场景在同一 tick 前接受跨入口合法请求，身份不同且各自后续结果可关联；同入口连续请求仍成立。参与该命名空间的入口与重启 / 其他 producer 边界均有明确设计及对应验证。

<a id="http-h1-disposition"></a>
### 2.3 H1：已接受请求的可靠处置

- **Why：** HTTP 接受之后，调用方需要知道请求仍在处理，或已经获得可关联的明确处置；进程继续运行不能掩盖请求静默消失。
- **What：** 每个已接受请求必须保留处理责任。遇到身份冲突或批次错误时，仍可恢复处理的请求保持可恢复状态；不能继续的请求必须有能关联原 action 身份的明确失败处置。
- 同批一个冲突不能使无冲突请求失去可靠处置。处置结果应区分未被接受的提交拒绝、接受后的明确失败，以及仍待核对的结果；批次级错误日志不能替代逐请求结果。
- 可靠处置限于设计声明的运行与保留范围；本文不新增持久 outbox、跨重启完整请求恢复或无限历史保证。范围结束与记录不可得不得伪装为某笔请求已确认或确定失败。
- **Done：** 隔离场景注入冲突并加入无冲突第三请求，所有已接受请求均有明确归属，正常请求不被连带吞掉；队列条数 / 字节预算与实际保留或终止处置一致，后续 tick 可以继续处理正常请求。
- H1 身份唯一与可靠处置是两项独立义务：只消除常见身份碰撞，不能替代批次错误路径验收。

<a id="http-h2-lifecycle"></a>
### 2.4 H2：迟到登记幂等与确认不回退

- **Why：** 提交接受登记与异步提交确认存在先后交错时，同一请求的已确认事实必须稳定可读。
- **What：** tracker 与 Explorer 对同一 action 的重复或迟到 accepted 登记必须幂等；只能补齐缺失提交信息，不能覆盖已知 Confirmed 或制造重复顺序条目。Explorer 已有的区块高度 / 哈希关联必须保留。
- Confirmed 不得因后续时间刷新变成 Accepted、Pending 或 Timeout。两个 store 对同一已消费提交批次的确认状态必须一致，区块关联由实际拥有该字段的 Explorer 索引维护；本要求不新增 tracker 区块字段。仅修复其中一个 store 不构成完成。
- Timeout 表示观察窗口内尚未取得确认，不能单独证明未入块、未执行或已确定失败。迟到的有效提交确认可以纠正 Timeout；状态单调不能被实现为所有枚举值的简单大小比较或 Timeout 永久封闭。
- 对于结果未知的断连 / 超时，调用方保留原请求身份继续核对；不得把它当成全新动作自动重做。具体持久提交与恢复保证仍服从上位 runtime authority。
- **Done：** 分别覆盖“接受登记前已消费确认”和“tracker 已登记、Explorer 未登记时已消费确认”两种交错。恢复登记后，两侧仍为 Confirmed，记录和顺序索引唯一，Explorer 保留区块关联，超过 timeout 观察窗口也不回退。
- **Done：** 已保存的 Explorer 记录重载后仍保留原确认与区块关联；正常接受 / 等待 / 确认、确实尚未观察入块的 timeout、提交拒绝均保持其真实语义。该重载场景不要求补建原本不存在的跨重启全量历史。

<a id="http-h3-independent-query"></a>
### 2.5 H3：Explorer P0 独立查询

- **Why：** 直接 API 消费者没有义务复制 Launcher 或 Bridge 的默认调用顺序，也不应依赖其他消费者替它刷新索引。
- **What：** `/v1/chain/explorer/tx`、`txs`、`blocks`、`search` 必须能独立反映本进程已提交批次；不得以先访问 overview、transfer status、chain status 或 history 作为隐藏前置条件。
- 查询采用明确的提交观察边界；该边界之后并发发生的提交可以在后续读取体现。本文不要求覆盖响应生成期间所有新提交，也不引入独立端点的数值新鲜度 SLA。
- 同一批次的交易状态、区块关联和搜索结果必须一致；重复查询不得制造重复插入。已取得的提交证据不能因投影尚未同步，被时间刷新误报为 Timeout。
- 无法完成必要同步时，沿现有结构化错误 / 未就绪语义给出可诊断结果；不能把旧索引或空结果伪装成已完成本次同步的新鲜结果。
- **Done：** 合法请求完成正常接受登记并由隔离 Node 提交后，禁止调用 overview / status / history，仅用四类 P0 查询均能看到该批次；重复查询结果无重复且关联一致。
- **Done：** 真正未提交请求仍按既有规则呈现等待或观察超时；保留既有 status-first 回归，并独立验证 P0-only 路径。

<a id="http-h4-filters"></a>
### 2.6 H4：组合过滤一致性

- **Why：** 用户同时填写的筛选条件必须共同限定结果，否则回显条件会与实际命中相矛盾。
- **What：** `action_id`、账户（HTTP 参数 `account_id`）和 `status` 同时存在时采用 AND 语义；action ID 命中后仍必须满足其余条件，不存在隐式字段优先级。各字段已有匹配含义保持不变。
- 条件回显、`items`、`total`、排序与分页必须基于同一过滤结果集；native / Web 实际请求应保留同一组已填写条件，前端不得本地重算链查询结果。
- **Done：** action 匹配而账户不匹配、action 匹配而 status 不匹配时，分别得到 `items=[]` 与 `total=0`；全部匹配和只传 action ID 时正确返回。
- **Done：** native / Web 真实请求保留组合参数；排序、分页和回显与同一结果集一致。仅验证单个 status 条件不能替代组合场景。

## 3. AI System Requirements (If Applicable)

不适用。本文定义确定性的提交与查询合同，不引入模型、Agent 编排或外部 provider。机器客户端与 GUI-agent 消费这些接口时适用同一语义，不获得额外执行权。

## 4. Technical Specifications

### 4.1 Authority 与接口边界

| 事项 | 本文拥有的专业要求 | 相邻 authority |
| --- | --- | --- |
| HTTP 接受与身份 | 共享 Node 命名空间、接受后的可关联处置、producer 边界 | 入口接点、身份不变量与队列错误处理由[配对设计](chain-http-submit-explorer-contract.design.md)定义；重启接线和非 transfer 反馈的未决项由 runtime owner 在实施前明确 |
| tracker / Explorer | 迟到登记幂等、已确认事实不回退、独立读取与组合过滤 | store 更新、同步消费与并发约束由配对设计定义 |
| 身份与授权 | 既有签名、账户、余额和 nonce replay 保护必须保留 | [资产与密码学基线](p2p-mainnet-security-governance-readiness.prd.md#资产与密码学基线)；[signed metadata 合同](../token/mainchain-token-ideal-transaction.prd.md) |
| 执行与提交证明 | 不从接受回执或 Explorer 状态外推执行成功和正式最终性 | [World Runtime PRD](../../world-runtime/prd.md)及[P2P PRD](../prd.md) |
| native / Web 表现 | 原样保留请求参数与结果语义，空态 / 未就绪 / 错误可见 | [Explorer PRD](../../world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md)及[跨表面动作 PRD](../../world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md) |

本文覆盖现有 HTTP 提交入口与现有 P0 查询，不自动新增 endpoint、身份字段、钱包能力或写操作。需要改变既有返回结构时，必须在配对设计明确兼容性和消费者承接；不得让各 endpoint 私自约定不同含义。

### 4.2 状态词义与保证范围

| 观察或结果 | 允许的解释 | 不能据此推断 |
| --- | --- | --- |
| HTTP `200 / accepted` | 提交入口已经接受请求，后续结果按原身份关联 | 已 committed、已执行或已结算 |
| `pending` | 现有观察尚未确认请求入块 | 已产生世界效果，或一定永远不会入块 |
| `confirmed` | 现行管线已经观察到该 action 的入块及相关区块事实 | 执行 / 余额结算成功、正式最终性、公开服务 readiness |
| `timeout` | 在当前观察窗口内尚未取得确认，需要核对 | 确定失败、可安全创建全新请求重做、不可再被有效确认纠正 |
| 明确拒绝 / 失败处置 | 按实际处置阶段与原身份报告可诊断原因 | 所有失败都意味着同一执行阶段，或允许绕过身份 / nonce 检查 |
| 记录不可得 / 有限窗口 | 当前查询范围不足以返回该记录 | 请求不存在、已经成功或一定未执行 |

只保护设计明确承接的有限窗口、已保存记录和当前进程提交观察；不承诺 archive、跨重启永久交易历史或恢复所有未完成请求。重启 / 保留范围和其他 producer 的边界必须写清，不能用这种非保证掩盖保证范围内的身份碰撞、确认回退或静默丢失。

上位产品 SC-2 / SC-5 的完整验证还包括服务角色、权威证明、玩家状态和执行边界；本专题通过只能作为其中的组件证据。

## 5. Risks & Roadmap

- **语义升级风险：** 将 accepted 当 committed，或将 confirmed 当结算 / 最终性，会把本次修复扩大为当前没有证据的承诺。
- **局部修复风险：** H1 身份修复不能替代可靠处置或 H2 竞态；H2 不能替代 H3 独立同步；H4 仍需单独验证。
- **伪成功风险：** 默认 UI 调用顺序可能掩盖 P0 隐藏依赖；批次级日志、时间超时或有界记录裁剪不能替代逐请求真实处置。
- **范围扩张风险：** 不把 allocator 修复扩展为新的 fee / sponsor、共识算法、钱包、custody、安全治理或正式恢复系统；已有身份和 nonce 保护继续有效。

交付按“稳定合同 → 对应实现与定向验证 → 适用消费者回归”推进。各项可独立实现，但必须分别保留证据；Issue #4450 仅保留已关闭的历史来源，目标义务继续由本合同承接；实现决策、实际剩余项与完成证据记录在承接工作的后续 PR 和真实验证中，不重新激活 #4450，也不在本文维护复选框或候选提交台账。

## 6. Validation & Decision Record

### 6.1 独立验收追踪

| 要求 | 可独立判定的完成证据 | 设计承接 |
| --- | --- | --- |
| [H1 共享身份](#http-h1-identity) | 同 tick 跨 HTTP 入口身份不同且可关联；同入口连续请求；其他 producer / 重启范围明确 | [身份设计](chain-http-submit-explorer-contract.design.md#des-http-h1-identity) |
| [H1 可靠处置](#http-h1-disposition) | 冲突加正常第三请求不静默丢失；逐请求处置、队列条数 / 字节预算与后续 tick 一致 | [处置设计](chain-http-submit-explorer-contract.design.md#des-http-h1-disposition) |
| [H2 生命周期](#http-h2-lifecycle) | 两种迟到交错；两个 store 的确认与唯一索引、Explorer 的区块关联；时间推进与已保存记录重载不回退 | [生命周期设计](chain-http-submit-explorer-contract.design.md#des-http-h2-lifecycle) |
| [H3 独立查询](#http-h3-independent-query) | 禁止其他同步入口的 P0-only 查询、重复读取、真实未提交对照及 status-first 回归 | [查询设计](chain-http-submit-explorer-contract.design.md#des-http-h3-independent-query) |
| [H4 组合过滤](#http-h4-filters) | 两类不匹配为空、全部匹配、仅 action ID、native / Web 参数保留与排序分页一致 | [过滤设计](chain-http-submit-explorer-contract.design.md#des-http-h4-filters) |

配对设计逐项定义构造条件、并发控制、拒绝路径、验证入口和证据边界。行为验收使用隔离 runtime、测试账户和本地可控依赖；无需真实资产或付费 provider。

文档链接、结构和静态检查通过只证明文档可维护；不能替代上述行为验收。证据必须说明受测版本、入口、运行范围及适用窗口；没有真实命令 / 产物 / 结果时不得写成测试通过、部署完成或公开就绪。

### 6.2 已确定的范围取舍

| 决定 | 理由 |
| --- | --- |
| 用一份后端专业合同承接 H1–H4，保留五个独立验收入口 | 共用管线需要统一语义，不同根因仍必须分别判定 |
| Launcher 文档只承接用户可见状态、参数和结果 | 避免在表现层另立 runtime 索引、提交或状态演化真值 |
| HTTP 接受、入块观察、执行结果和正式最终性保持分离 | 不以修复查询正确性改变资产或网络保证 |
| Timeout 允许被迟到有效确认纠正；未知结果沿原身份核对 | 观察超时不足以证明请求未提交或已确定失败 |
| 不新增数值 SLA、全历史 archive 或客户端 consensus dedupe | 它们不属于本工作包已有义务，须有独立需求、设计及证据 |

固定基线与 Issue 中的历史源码说明仅用于追溯本次合同形成；实现合入后应按真实结果更新相关专业事实，实际任务状态持续以 GitHub Issue / PR 和验证记录为准。
