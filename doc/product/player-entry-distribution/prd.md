# 玩家接入与发行 PRD

## 文档身份

- 产品模块：玩家接入与发行
- 产品模块 slug：`player-entry-distribution`
- 产品层唯一 PRD：`doc/product/player-entry-distribution/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-004`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-08-06`
- 后继文档：`无`
- 下层专业域：[`README.md`](../../../README.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文只组合玩家从了解产品到进入、安装、验证受支持技术预览的路径。根 `README.md` 是公开当前状态与 claim envelope 的唯一权威；Viewer、Launcher、发行资产与访问模式的实现合同由 `world-simulator` 专业域拥有。

### 活跃产品专题

- [`访问模式与发行就绪`](access-modes-and-release-readiness.prd.md)：`viewer` / `pure_api` 模式边界、能力等价、失败恢复、统一候选门禁与公开 claim 升阶。
- [`本地启动会话连续性与恢复`](local-launch-session-continuity-and-recovery.prd.md)：受支持 Launcher 路径中本地会话的真实状态、停止、恢复与配置边界；不把本地运行事实升级为模式或发行 claim。
- [`发行沟通与公开口径`](release-communications-and-public-claims.prd.md)：内部证据到外部 claim 的状态隔离、审核、发布、纠正与撤回合同。
- [`参与和认可边界`](participation-and-recognition-boundaries.prd.md)：有限预览中的可审核贡献、非自动权益和治理执行边界。
- [`免费进入、世界内成长与有界认可`](free-entry-world-progression-and-recognition.prd.md)：免费基础进入、非权力型可选服务，以及对世界成长、情境认可和区域机会主责专题的入口/claim 组合边界；世界内规范性语义由世界规则与玩法专题承接。

## 1. 产品承诺

玩家能从一个不夸大的公开入口理解 oasis7 当前是什么、哪些模式已有证据、如何进入或安装，以及如何验证自己连接的是声明的产品路径。

## 2. 范围与玩家边界

覆盖产品发现、公开说明、Web 访问、平台安装、Launcher 转移、账号/模式入口和首次验证。玩家可以了解当前阶段并选择受支持入口；不应被未发布功能、内部测试模式、过期下载或历史 go 证据诱导。

### 安装与发行体验边界

每个平台应提供一个推荐下载包和主 CTA；下载前应清楚显示支持边界、版本与 checksum，使玩家无需研究 shell、bundle 内容或多个技术资产的差异即可完成受支持的安装或启动。

当前升级只支持重新下载最新主包并手动覆盖安装/替换；用户须先备份 `config.toml`、`.oasis7_launcher_ux_state.json` 与 `output/chain-runtime/<node_id>/reward-runtime-execution-world/`。这不是应用内更新、自动迁移或跨目录状态保留承诺。Windows codesigning trust chain 与 macOS 签名/notarization 未完成前，不得把技术预览资产、单主 CTA 或手动安装路径表述为普通用户广泛发行已就绪。

### 下载完整性失败边界

玩家可以在开始安装前核对推荐资产的版本、平台、发布来源与 checksum；若该核对缺失、无法完成或不匹配，产品只能明确停止该资产的安装/覆盖路径，并让玩家保留或重新取得与当前发布来源绑定的候选。玩家可以查看预期与实际核对结果、选择重新下载、改用同一候选中明确受支持的正确平台资产，或进入支持/恢复路径；不能把“文件已下载”“曾经可用的旧 checksum”“浏览器/操作系统未报警”或第三方镜像的自述当作验证通过，也不能被引导绕过核对继续覆盖既有状态。

完整性失败只裁定本地候选资产不可信，不自动撤销玩家 identity、session、Agent 授权、待决 intent 或已 committed receipt；同样，重新取得并通过本地核对的资产也不自动恢复这些权威能力。这样 world-first 的权威世界不由分发字节决定，persistent 的世界历史不会被本地安装诊断重写；emergence-first 的玩家、Agent 与组织行动不因下载渠道获得额外规则或经济权力；每次失败/恢复都应能以候选版本、平台、来源类别和结果追溯，而不要求公开本地路径、凭据或敏感诊断。未来新增平台、签名或其他校验机制可以扩展实现，但必须保留同一语义：未验证的资产不能被表达为受支持安装，验证通过也只恢复本地候选资格而不代签世界权威。

### 手动升级兼容与安全回退边界

新版本在首次不可逆处理旧配置、Launcher 状态或本地 runtime 数据前，必须形成绑定版本、平台、primary mode 与候选资产的兼容判定，并把结果表达为以下三类之一：

- **兼容并可继续**：新版本已经确认所需状态可读取或可安全迁移，玩家可以继续进入声明的模式；成功只覆盖本地升级结果，不自动证明远端世界状态、session authority 或待决行动已经恢复。
- **失败且确认未改写**：新版本在写入前停止，并能证明原状态未被改写；产品可以指引玩家退出新版本、恢复升级前备份并重新安装先前受支持资产。回退目标必须明确，不能让“下载旧包”代替版本、checksum 与状态来源核对。
- **未知或可能已改写**：无法证明写入边界，或已开始不可逆迁移；产品必须停止继续启动，保留备份与诊断材料，并提供重新安装当前版本、隔离新数据或进入支持/恢复流程的下一步。此时不得建议直接降级、反复覆盖或用旧程序打开可能已升级的状态。

本地软件回退不回滚权威世界、玩家身份、session authority、已 committed receipt 或已经被网络接受的行动；这些状态只能按各自专业域的恢复合同重新核验。备份也不是可移植存档或 authority 凭据。具体兼容版本、迁移格式、写入原子性、诊断字段和恢复命令由 Viewer/Launcher、runtime 与发行专业合同拥有，产品层只约束玩家可见结果、禁止误导的回退路径与跨域验收。

### 玩家访问模式与证据边界

- 正式玩家访问模式只有 `viewer` 与 `pure_api`：`viewer` 是 Web/UI surface，`pure_api` 是无 UI surface。兼容 alias、execution lane、provider、deployment 和 session context 都不会产生新的玩家访问模式。
- `hosted_public_join` 是 `viewer` 下的 deployment/session context，不是独立模式，也不表示更高的可用性或发布就绪。每份 claim 与 evidence 必须绑定一个 primary mode；`viewer` 与 `pure_api` 的证据不得互相代签。
- `pure_api` 只有在专业域规定的可玩性与 parity 前置条件成立且有对应证据时，才能支持正式可玩结论；observer 或 blocked 是该模式下的结果分类，不是第三种入口。具体 Viewer/provider 实现、玩法 parity、hosted session/custody 与测试证据分别由 [`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md) 与 [`doc/testing/prd.md`](../../testing/prd.md) 维护。

### Hosted 玩家进入与会话边界

- Hosted Web 保持 `viewer` 模式；共享 URL 只提供玩家加入或观察入口，不提供 operator/control 能力。guest observation 不等于可玩身份；进入可玩状态还需要有效的玩家 identity/session，并成功绑定到 runtime。
- 会话过期、撤销或服务重启后，产品必须明确引导玩家 reconnect、re-register 或 re-auth，不得静默恢复旧 authority。玩家不管理长期 signer material；登录恢复的是 identity/session，不是 private key，也不会仅凭登录解锁敏感操作。
- 当前状态是 limited playable technical preview，不代表 universal sharing、production custody 或 broad readiness。Hosted entry、session、authentication 与 signer custody 的专业合同由 [`doc/p2p/prd.md`](../../p2p/prd.md) 维护。

<a id="player-session-authorization-continuity"></a>

### 会话失效、待决 intent 与 Agent 授权连续性

会话过期、撤销、断开或重新认证只改变玩家当前进入与提交新动作的能力；它本身既不撤销、不确认，也不重放已由权威系统接受的 intent。客户端尚未收到 committed receipt，不等于权威系统确认请求尚未生效：超时、断连或回复丢失可能发生在实际提交之后。

| 请求结果的玩家语义 | 判定依据与恢复边界 |
| --- | --- |
| 已确认尚未生效的待决请求 | 必须有适用的权威状态证明，不能从缺少回执推导。尚未决定的请求按当前有效权限、资源、期限及规则重审；已固定执行决定的原请求按专业恢复合同续接，不因重新登录而重新决定或换成新动作。 |
| 提交结果未知、待核对 | 无法证明原请求是否提交时保留该状态和原请求身份。恢复读取资格后先核对原结果；超时、单端未找到、陈旧缓存、查询不可用或无读取权限都不能被解释成确定失败、已取消或可放心重新执行。 |
| 已确认提交及其结果 | 以绑定目标世界与原请求的可验证结果为准。重连、重复查询或专业合同允许的原请求幂等重试，只返回或续接原结果，不再次扣减资源、结算或授予权限。动作提交不自动等于生产、交付等后续业务完成。 |

已结算 receipt 不因会话失效消失，重新认证也不得静默重提旧 intent、复制资源 sink，或把旧会话、界面缓存和 Agent 计划当成新的提交授权。结果仍未知时，安全停止或重新定目标只约束后续决策，不代表已经取消原请求、返还其投入或消除其可能已提交的效果。

Agent 离线工作是否继续，只由其独立且仍有效的授权范围、到期条件和专业合同决定。玩家会话恢复不得延长、扩大或复活已失效的 Agent 授权；授权失效后，Agent 必须停止产生新的越权世界动作，或按专业合同把尚未生效的动作明确置为待决、暂停或拒绝，不能伪装成已完成。若 intent 已由权威系统接受，玩家登录中断也不能把它误报为已取消或从历史中隐藏。

`viewer` 与 `pure_api` 的正式 surface 至少要区分“会话阻塞”“已确认尚未生效的待决请求”“提交结果未知、待核对”“Agent 授权失效或暂停”和“已确认提交及其结果”。会话、读取资格和 Agent 授权是与请求结果正交的状态；登录失败或授权失效不能覆盖已知结果，也不能将未知结果改写为未生效。Viewer 必须以玩家可读形式呈现原请求、当前因果状态与安全下一步；无 UI 的 `pure_api` 必须返回语义等价、可由客户端消费的结构化状态与下一步类别，不要求内建玩家界面。

下一步可以是 reconnect/re-auth、按原请求核对、等待明确的服务恢复、查看 receipt、重新规划，或专业域明确允许的撤回/替代。查询和原请求幂等重试不得换成新的意图；另行发起新动作必须有新的显式授权，并保留与原请求的关系，不能作为恢复流程的隐含副作用。查询仍按当前读取权限与世界/主体绑定验证；重新认证或持有旧请求引用不自动授予读取他人私有结果的权限。

[Runtime 提交与恢复设计](../../world-runtime/design.md#runtime-authority-commit-recovery-target)和[P2P 原候选恢复设计](../../p2p/design.md#des-p2p-original-candidate-recovery-v1)拥有原请求查询、持久去重、既定执行续接与未知结果核对的技术合同。本节组合它们的玩家语义，不新增 wire 状态枚举、查询授权或第二套恢复协议；具体 session token、签名、Agent 授权字段、API schema、transport 和 UI 表达仍由对应专业域拥有。

## 3. 权威与冲突处理

| 产品层拥有 | 公开/专业域权威 |
| --- | --- |
| 发现、访问、安装、验证与发行路径的组合体验 | `README.md` 拥有当前公开状态与 claim；`doc/world-simulator/prd.md` 拥有 Viewer、Launcher、provider、资产与访问实现；`doc/world-runtime/prd.md` 拥有持久 runtime 状态兼容、写入原子性与恢复合同；`doc/game/prd.md` 拥有 pure API 玩法与 parity；`doc/p2p/prd.md` 拥有 hosted entry/session/authentication/signer custody；`doc/testing/prd.md` 拥有证据与门禁合同 |

本 PRD 不得独立宣布新版本、新渠道、新下载或发布就绪。公开 claim 变更必须先有专业验证与 QA 结论，再由 `liveops_community` 同步根 README/公开渠道。

## 4. 路线图

1. 唯一发现路径：根 README 说明当前阶段、受支持入口与公开边界。
2. 可验证进入：Web、Launcher 或平台资产带玩家到达声明的真实模式。
3. 发行一致性：版本、资产、说明、可用性与回退路径经同一 release gate 核验。

## 5. Done：成功标准与验收

- SC-1：根 README 只声明当前受证据支持的产品状态、访问方式与公开边界。
- SC-2：每个公开入口都能指向对应的模式、版本、平台和可重复验证路径。
- SC-3：发行资产、Launcher 转移与主 Web 入口不会把内部、回退或假模式宣称为真实发布体验。
- SC-4：公开 claim 变更可追踪到专业 PRD-ID、QA 结论与 LiveOps 同步 owner。
- SC-5：访问结论只绑定 `viewer` 或 `pure_api` 之一，execution/provider/deployment/session context 不被升格为新模式；两种模式的可玩性、parity、observer 或 blocked 结论均有各自证据且不得互相代签。
- SC-6：至少一条发行证据链以同一版本和 primary mode 贯通公开发现、正确的平台/模式选择、下载安装或 Web/pure API 进入、玩家核对版本/backend/mode，以及 unsupported、失败或手动升级时的真实恢复说明；仅对适用的 Viewer/Launcher 路径验证 Launcher 到达声明的真实后端。每个受支持的平台/入口组合分别验证，`viewer` 与 `pure_api` 仍不得互相代签。
- SC-7：产品样例证明免费客户端、账户和基础进入与可选的 hosting/storage/support 便利服务相分离，后者不授予世界权力；成长、认可和区域协作仍以世界内有代价、可审计的资产、行动和有界资格为基础，且长期目标不被误报为当前技术预览可用性或发行就绪。
- SC-8：每个支持手动覆盖升级的平台资产都在不可逆处理前给出兼容判定；兼容时可继续，只有能证明原状态未改写时才提供绑定明确旧版本与备份来源的回退，未知或可能已改写时停止并禁止盲目降级。本地回退不会被表达为权威世界、identity/session authority、committed receipt 或网络已接受行动的回退。
- SC-9：代表性 hosted 与恢复样例证明，会话失效不会把已接受的 intent 或已结算 receipt 静默撤销、隐藏、重放或二次结算；客户端缺少回执不能被当作未生效。玩家能区分已确认未生效、提交结果未知及已确认提交，并在当前读取资格有效后按原请求核对。重新认证不恢复旧 authority，也不扩展 Agent 委托；尚未决定的请求与既定执行的续接分别遵循专业合同。提交成功但回复丢失的样例必须查回原结果且只有一次效果，查询受阻时保持真实未知与安全下一步，见[组合验收](#player-session-unknown-outcome-acceptance)。
- SC-10：每个受支持平台至少有一条推荐资产的完整性失败样例：缺失、无法完成或不匹配的核对会在写入既有状态前停止安装/覆盖，并给出预期/实际结果和重新下载、正确平台资产或支持/恢复中的适用下一步；通过重新取得资产的本地核对不被表述为 identity/session/Agent authority 恢复、pending intent 结算或 world receipt。新增校验机制或平台必须保留这组语义边界。

<a id="player-session-unknown-outcome-acceptance"></a>

### 会话恢复与未知提交的组合验收

本节细化 SC-9，复用上文系统设计与现有候选证据，不新建测试平台或并行任务台账。每个实际声明支持的 primary mode 分别成案；同一案例绑定同一候选、世界、合法主体和原请求，不能拼接不同任务或模式的局部成功。以下为验收设计，不是已执行结果。

| 样例 | 操作与故障窗口 | 必须观察到的结果 |
| --- | --- | --- |
| SC-9-U1：提交后回复丢失 | 对合法且有资源后果的原请求，在权威提交后阻断返回，随后让会话失效；重新认证后按原请求查询，并重复查询或执行专业合同允许的同身份幂等重试。 | 客户端先保持结果未知，查明后显示原世界结果及其回执关联；资源效果、里程碑及适用外部副作用不重复，不能生成第二个意图来消除未知。 |
| SC-9-U2：已知无效果与既定续接 | 分别准备权威确认尚未决定的待决请求，以及已有固定执行决定但尚未发布的原请求；恢复前改变适用权限或资源条件。 | 前者按当前条件重审且无旧权限透传；后者核对并续办原持久决定，不重新 admission、重选结果或重复执行已保存的完整 candidate。两者都不能仅凭客户端缺少 receipt 判定阶段。 |
| SC-9-U3：原结果暂时不可查询 | 在 U1 恢复查询时分别注入超时、服务不可用、单端未找到或无当前读取资格。 | 如实呈现服务/会话/读取阻塞，原请求结果保持未知；给出重新认证、原请求复核、等待服务恢复或安全停止中的适用下一步，不将错误、空列表或旧缓存变成未提交证明。 |
| SC-9-U4：Agent 授权与会话分离 | 在提交及重新登录前后使 Agent 委托到期或撤销；分别核对原已提交结果与尚未决定的新动作。 | 已合法提交历史保留；在当前读取权限允许时仍能查明原结果，否则如实限制读取。重新登录不复活委托，不授权新动作，也不把授权错误当作原请求取消。 |
| SC-9-U5：错误归属或结果绑定 | 用其他世界、其他主体或不匹配原请求的结果替代恢复材料。 | 不采纳为原请求结果，不泄露无权读取的私有内容；保持原请求的已知事实与待核对边界，不转投其他世界或借新身份重放。 |

QA 关联会话变化、原请求、实际查询返回、权威结果及其资源变化；Viewer 另保留玩家可见状态与恢复动作，pure API 保留结构化同义结果。只证明提交受理或排程的回执不能被记为铁锭完成；声称首产物完成时仍须按[首局合同](../world-rules-core-gameplay/first-session-and-continuation.prd.md)核对匹配的正量 output ledger 与持久里程碑。

候选记录区分 fixture、真实 Runtime/入口和真人观察，标明实际执行、未执行与不适用的范围；受控故障样例不代签独立故障域容灾或完整邀测放行。公开材料只保留脱敏关联与必要结果，不包含会话令牌、签名密钥或私有请求正文。具体故障注入、协议字段和当前结果仍由专业测试及 PR 证据承接。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | liveops_community | PRD-WORLD_SIMULATOR-042/043 | `README.md`; `doc/world-simulator/prd.md` | 公开 claim 与当前发布证据对齐审计 | test_tier_required |
| SC-2 | viewer_engineer | PRD-WORLD_SIMULATOR-020-031 | `doc/world-simulator/prd.md` | Web、Launcher、平台入口到声明模式的 smoke | test_tier_required |
| SC-3 | viewer_engineer | PRD-WORLD_SIMULATOR-039/041/046 | `doc/world-simulator/prd.md` | 真实后端/模式与回退边界回归 | test_tier_required |
| SC-4 | qa_engineer | PRD-WORLD_SIMULATOR-042/043 | `doc/world-simulator/prd.md` | release gate、公开文案与 LiveOps owner 记录 | test_tier_required |
| SC-5 | qa_engineer | PRD-WORLD_SIMULATOR-039/041/046 / PRD-GAME-008 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/game/prd.md`; `doc/testing/prd.md` | primary mode、可玩性/parity 分类与非替代证据审计 | test_tier_required |
| SC-6 | viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-020 / PRD-WORLD_SIMULATOR-042 / PRD-WORLD_SIMULATOR-045 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同版本、平台和 primary mode 的发现、进入、核验、失败与升级恢复端到端证据，包含适用平台真实资产与完整 release gate | test_tier_full |
| SC-7 | producer_system_designer / gameplay_designer / viewer_engineer / qa_engineer / liveops_community | PRD-GAME-015 / PRD-WORLD_SIMULATOR-042/043/045 / PRD-TESTING-003 | `doc/product/player-entry-distribution/free-entry-world-progression-and-recognition.prd.md`; `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 免费基础进入、非权力型可选服务、世界内成长/认可/区域互赖及当前 claim 分离的组合审计 | test_tier_required |
| SC-8 | producer_system_designer / viewer_engineer / runtime_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-020 / PRD-WORLD_SIMULATOR-042 / PRD-WORLD_RUNTIME-014 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 真实发行资产覆盖兼容继续、写入前失败且可验证回退、未知或可能写入时阻止降级三类样例，并核对版本、平台、primary mode、候选资产、备份来源与权威状态非回退边界 | test_tier_full |
| SC-9 | producer_system_designer / runtime_engineer / agent_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | PRD-P2P-004 / PRD-P2P-023 / PRD-P2P-029 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-003 / PRD-WORLD_SIMULATOR-039 / PRD-WORLD_SIMULATOR-041 / PRD-WORLD_SIMULATOR-046 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md`; `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md`; `doc/product/world-infrastructure/prd.md`; `doc/product/world-rules-core-gameplay/prd.md` | [SC-9-U1–U5 组合样例](#player-session-unknown-outcome-acceptance)：提交后丢回复、重新认证、原请求查询/幂等重试、查询受阻及错误归属；区分尚未决定与既定执行续接，核对 Agent 授权先于或晚于会话失效时的单次效果、无越权新提交、无自动续权，以及 Viewer 玩家可读状态、`pure_api` 结构化状态与原 receipt 的同义结果 | test_tier_full |
| SC-10 | viewer_engineer / runtime_engineer / qa_engineer | PRD-WORLD_SIMULATOR-039 / PRD-WORLD_RUNTIME-003 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 每个受支持平台上的错误 checksum、缺失/不可完成核对与重新取得正确候选的安装前负例；核对结果、可用恢复路径，以及不改写本地状态、不恢复世界 authority/receipt 的组合核对 | test_tier_required |

## 6. Non-Goals

- 不宣布任何未由当前根 README 与发布证据支持的玩家可用性。
- 不在产品层定义 Launcher、Viewer、登录、打包或渠道实现。
- 不用产品路线图代替发布任务、QA 放行、运营承诺或渠道 runbook。
