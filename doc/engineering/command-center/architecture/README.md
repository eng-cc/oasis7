# 指挥面板架构索引

这里集中列出做项目决策时需要记住的架构边界、实现入口和部署核验缺口。它是主责文档与交付事实的派生导航；具体约束、变更理由和验收仍在链接的原文中维护。

- 代码与文档核对基线：`main@598b308a23f3f0189843618d14abb1aa31c0d2de`，摘要核对日期为 2026-10-11。
- 本轮没有实际环境采样，没有部署、重启、恢复演练或模型调用；下文 PR 中记载的验证不是本轮重新运行的结果。
- 历史环境表 Last Verified 为 **2026-07-06**，节点表为 **2026-06-23**。读取摘要的日期不能代替部署、健康或可用性的核验日期。
- 项目进度与待处理事项见[当前状态](../README.md#cc-status-attention)；本页的更新方法见[架构索引维护流程](../command-center.manual.md#cc-maintenance-architecture)。

## 从问题进入

Cargo 边界局部增量（2026-10-12）：[#4626](https://github.com/eng-cc/oasis7/pull/4626) / `133d70e45b71bd062032a734d68be1e290692b2c` 将 native 生产 cognition 状态转换转接至 Harness，World readback 与 signed-service 校验仍由 Runtime 适配器承担，旧 DTO/checkpoint 兼容保留；不新增 Harness→Oasis 依赖。支持包 profile 隔离尚在分支验证，直接 Oasis wasm32 目标及部署不因本次局部摘要变为已验证。测试入口见[测试手册](../../../../testing-manual.md)。

L4 工具链局部文档更新（2026-10-12，核对基线 `27db6f894bd106e9a4fee9ee3062cd8fd86e1056`）：[testing 合同](../../../testing/prd.md#l4-build-driver)明确默认 release/正式 UI、同一实际 Hosted 执行会话、逐次 RunResult 与评审归属；[入口兼容设计](../../../world-simulator/viewer/viewer-web-entry-compatibility.design.md)保留 alias，并将测试 API 限于显式测试产物。[#4438](https://github.com/eng-cc/oasis7/issues/4438) 仍承担旧 runner/scaffold 的实现和运行验收；此处只更新设计导航，不表示新部署或试玩通过。

- 要开新环境、搬服务或调整节点：[环境与部署](#cc-architecture-deployment)。
- 要决定首次开放、扩成多节点权威或改变恢复承诺：[世界权威与状态](#cc-architecture-authority)。
- 要拆进程、调整执行边界：[服务与执行](#cc-architecture-runtime)。
- 要接入玩家、外部 Runtime 或远程控制：[身份与 Agent](#cc-architecture-agents)。
- 要升级、回退或发布：[发布与运维](#cc-architecture-release)。
- 要调整指挥面板技术路线、数据或执行能力：[指挥客户端](#cc-architecture-command-center)。

## 怎么读状态

| 层面 | 本页表达什么 | 不由它推导什么 |
| --- | --- | --- |
| 设计 | 当前主责文档采用的边界、目标和未决事项 | 文档存在不证明实现完成 |
| 实现 | 固定基线中的代码切片、已合入 PR 及其验证范围 | 测试入口或局部通过不证明完整产品验收 |
| 实际部署 | 指定环境、版本、节点/世界身份和窗口下的运行事实 | 合入、旧清单和本页刷新不证明当前上线或健康 |

无法核实时保留“未核对”及最近来源，不把信息缺失写成正常、失败或没有部署。下面的“需人关注时机”是阅读提示，普通已授权实施仍沿用[开发流程](../../workflow/source-of-truth.md)。

<a id="cc-architecture-deployment"></a>

## 环境与部署

先确认当前承诺和环境，再看机器与服务。项目环境、网络 tier、provider lane、玩家接入模式分别回答不同问题。

| 主题 | 关键边界 | 需人关注时机 | 权威入口 |
| --- | --- | --- | --- |
| ARCH-01 · 公开阶段 | 当前仍为受限可玩技术预览；构建包和局部测试不自动代表正式发布或赛季上线 | 邀请新用户、修改下载入口、发布公告或改变成果保留承诺 | [公开状态](../../../../README.md#项目状态)、[安装与发行](../../../product/player-entry-distribution/prd.md#安装与发行体验边界) |
| ARCH-02 · 环境分层 | local / test / production 与 network tier、Hosted 登录、provider lane 分开；测试与正式数据/凭据隔离 | 新增端点、搬机器、共用存储或凭据、调整环境成本和开放范围 | [三套环境](../../governance/environment-lanes-and-inventory-2026-05-29.md#2-三套环境总览)、[子系统映射](../../governance/environment-lanes-and-inventory-2026-05-29.md#4-子系统映射表) |
| ARCH-03 · 拓扑与角色 | 清单标签、实际进程角色和世界身份分别核对；observer 追到高 head 不等于完整执行恢复或取得写权 | 增删节点、改变 validator/observer、调整故障域或发现节点分歧 | [部署清单](../../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#22-current-operator-inventory)、[采样合同](../../../p2p/node/node-triad-operations-observability.prd.md#inventory-与采样合同) |

**设计状态：** 三项均有当前主责入口。环境分类和角色约束持续有效，表中的机器、服务和就绪结果受其实际证据窗口限制。

**已有实现：**

- ARCH-01：仓库已提供技术预览、构建与验证入口；发行支持范围继续由根 README 和玩家接入产品域拥有。
- ARCH-02：环境表记录了 Hosted 登录及网络/provider lane 的映射和历史部署；内部 production lane 不等于项目已向普通用户正式开放。
- ARCH-03：已有运维采样、部署更新与恢复脚本。runbook 的五节点 inventory 是重新核对前的执行目标，不能代替实时发现。

**实际部署核验：** 本组三项均未取得当日运行窗口。[历史云上清单](../../governance/environment-lanes-and-inventory-2026-05-29.md#7-当前云上清单)只用于定位；当前节点数、版本、实际角色、健康和外部可达性保持未核对。

<a id="cc-architecture-authority"></a>

## 世界权威与状态

持久单权威和 BFT 是同一世界的不同权威阶段。单权威不等于仅靠一台主机或一个存储目录保存所有已确认结果。

<a id="cc-architecture-runtime"></a>

### 服务与执行

| 主题 | 关键边界 | 需人关注时机 | 权威入口 |
| --- | --- | --- | --- |
| ARCH-04 · 单权威到 BFT | 首次开放采用受控单权威持久世界；BFT 并行研发，后续在同世界 H/H+1 交接激活写权 | 决定首次开放、切换 signer/epoch、扩展权威集合或改变保留承诺 | [当前阶段](../../../product/world-infrastructure/prd.md#world-infrastructure-current-stage)、[权威 profiles](../../../p2p/design.md#des-p2p-authority-profiles) |
| ARCH-05 · 服务分工 | P2P 维护合法提交证明，runtime 确定性执行，game/Agent/Viewer 提交意图和消费已提交状态；Web Viewer 不承担完整节点职责 | 拆服务/进程、加网关、改模块归属或引入第二条执行路径 | [执行分层](../../../world-runtime/design.md#3-目标设计结构)、[P2P 边界](../../../p2p/design.md#4-边界与结构)、[高层架构](../../../../README.md#高层架构) |
| ARCH-06 · 写入与持久 | accepted、prepare、committed 分开；正式确认依赖完整记录和决定跨独立故障域持久，未知提交先查原请求 | 改存储、超时重试、写者隔离，或决定故障期间是否继续写入 | [双端持久前置](../../../p2p/design.md#des-p2p-replicated-durability-prerequisite)、[runtime 提交与恢复](../../../world-runtime/design.md#runtime-authority-commit-recovery-target) |
| ARCH-07 · 保留与恢复 | replay log、checkpoint、snapshot 分工；GC 不破坏保留范围的重建，同世界只读恢复不自动恢复写服务 | 调保留/磁盘预算、清理迁移数据、选择归档方案或灾备切换 | [存储层次](../../../world-runtime/runtime/runtime-storage-footprint-governance.prd.md#4-technical-specifications)、[重新服务边界](../../../product/world-infrastructure/prd.md#恢复后的重新服务闸门) |

**设计状态：** 本组是当前主责文档声明的路线与目标合同；正式 profile、完整事务和灾备目标不能因为已有前置切片而标为已验收。

2026-10-12 局部设计增量（实现边界核对至 `main@e01e29064517f4bf375767a19a09802335828249`）：[原候选恢复 v1](../../../p2p/design.md#des-p2p-original-candidate-recovery-v1) 要求执行前完整材料持久化、先核对原决定再隔离重放；[正式受控权威 v1](../../../p2p/design.md#des-p2p-formal-controlled-authority-v1) 与 [epoch 接管](../../../p2p/design.md#des-p2p-controlled-epoch-takeover-v1) 集中定义激活、普通消费者和存储/签名隔离。均为待实现目标，现有工程 scope 不因本次文档收敛取得正式权威；实际部署和故障域证据未刷新。

**已有实现：**

- ARCH-04：本地 journal、双端持久协议、受控历史重执行、activation/bootstrap 等前置持续落地；完整正式激活、消费者接线和 BFT 验收尚未闭合。
- ARCH-05：已有有序动作、roots、execution records、checkpoint/replay 及多种 prepare/publish 路径；[当前实现边界](../../../world-runtime/design.md#5-current-implementation-boundary-and-target-gap)仍明确整体能力为 partial。
- ARCH-06：局部双端持久证明不等于正式 CommitRef，若干原子发布切片不等于整个 World 的根事务；同机双目录不能证明独立故障域。
- ARCH-07：已有 execution bridge v3、CAS、sidecar、retention/GC/replay 的实现合同；[本地恢复边界](../../../world-runtime/runtime/runtime-storage-footprint-governance.prd.md#storage-accept-service-boundary)不证明分布式可得性或完整灾备。

**实际部署核验：** 未验证正式 profile 激活、实际存储同步、故障域隔离、旧写者 fencing、备份可用性或恢复后的写服务。这些结论需要各自候选与环境证据，不能由代码数量汇总为完成率。

[当前主线记录](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)仍保留正式激活、独立故障域恢复和人工交接等未完成义务；本页只链接，不新增一套验收台账。

<a id="cc-architecture-agents"></a>

## 身份与 Agent

玩家进入、游戏行动委托、operator 控制和代码开发会话分别成立。相同工具名称不能合并它们的权限与完成证据。

| 主题 | 关键边界 | 需人关注时机 | 权威入口 |
| --- | --- | --- | --- |
| ARCH-08 · 网络与权限 | URL/登录只提供接入；玩家 session、Agent 委托和 operator 权限分开，恢复不能复活旧授权 | 暴露公网 API、接账号/模型端点、改认证或委托、增加远程控制 | [Hosted 进入](../../../product/player-entry-distribution/prd.md#hosted-玩家进入与会话边界)、[会话与授权连续性](../../../product/player-entry-distribution/prd.md#player-session-authorization-continuity)、[headless 鉴权](../../../headless-runtime/design.md#6-鉴权与恢复边界) |
| ARCH-09 · 两类 Runtime | 游戏侧目标是 OpenClaw/Codex 通过官方游戏 Skill 和受授权游戏 API 持续游玩；开发指挥客户端管理代码开发会话 | 声明 Runtime 支持、管理多 Agent、改预算/停止行为或连接游戏 Viewer 与开发看板 | [游戏角色](../../../product/agents-world-simulation/external-agent-runtime-play.prd.md#12-角色与职责)、[首批 Runtime 目标](../../../product/agents-world-simulation/external-agent-runtime-play.prd.md#21-首批-runtime-与支持单位)、[开发会话](../command-center.design.md#1101-可选实验执行实现) |

**设计状态：** 接入、委托和角色边界由各自主责合同拥有。外部 Runtime 的首期功能清单是承诺范围，阶段标签不等于实现状态。

2026-10-12 局部设计增量（同上固定基线）：[首次绑定目标](../../../game/gameplay/gameplay-agent-claim-economy-contract.design.md#des-claim-entry-001) 按账号在同一世界的权威状态处理注册、返回和幂等；[首聊 service 目标](../../../game/gameplay/gameplay-agent-claim-economy-contract.design.md#des-claim-chat-005) 使用 owner 最小权威投影区分可通过、受阻和未知，并在提交时重验。注册/绑定、starter OC 领取与经济 ClaimAgent 分别成立；现有固定 starter 和不完整余额投影不能证明这些后继目标已实现。

**已有实现：**

- ARCH-08：headless ingress 已定义签名 payload、ed25519、单调 nonce 与恢复防重放；它不自动证明任意新 API 的完整授权和恢复行为。
- ARCH-09：[PR #4562](https://github.com/eng-cc/oasis7/pull/4562)交付 Game API 的发现合同和已实现签名读取；范围仍是显式开启的 loopback bootstrap。
- [PR #4565](https://github.com/eng-cc/oasis7/pull/4565)增加持久、owner-signed task control；初态 suspended、没有 executor。配对、私有目标、完整 context 和 action admission 不在该切片内。
- 2026-10-12 局部核验：[#4625](https://github.com/eng-cc/oasis7/pull/4625) 已合入 `19e4309b`，私有 owner proof/盐独立密文保存、公共 canonical 目标发布与 owner/history 当前正文回读已交付；journal-tail 控制链恢复回归和最终 CI 已通过。当前下一候选接通 executor 控制、私有 Context 与 task-fenced 行动，仍待完整验证和独立试玩。[当前实施候选](../../../world-simulator/llm/external-agent-game-api.design.md#当前实施候选原生任务执行链)不表示完整 TaskSnapshot、两 Runtime 首局或部署已完成；其他主题基线不变。
- 两个真实游戏 Runtime 的自主首局、多轮目标和恢复仍需分别证明；开发 Codex adapter 的合入不能替代这些游戏结果。

**实际部署核验：** 本轮未核对端点暴露、TLS/网关、账号存储、凭据 lane 或真实 Runtime 支持组合，也没有启动游戏/模型任务。

<a id="cc-architecture-release"></a>

## 发布与运维

| 主题 | 关键边界 | 需人关注时机 | 权威入口 |
| --- | --- | --- | --- |
| ARCH-10 · 发布与恢复 | 节点软件、世界 runtime manifest、客户端、不兼容基础协议分属不同发布 lane；换 binary 不自动激活世界规则，软件回退不回滚已提交世界 | 升级 ABI/协议、发布候选、补更 fleet、重置/回滚/恢复或重新开放写服务 | [四类发布路径](../../../world-runtime/design.md#4-runtime-activation-deployment-and-recovery-target)、[运维事实边界](../../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#stable-authority-and-evidence-boundary)、[失败与回滚](../../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#14-phase-h---failure-handling-and-rollback) |

**设计状态：** 升级、激活和恢复分别由世界与环境合同约束；技术恢复步骤不新增普通 PR 的流程门禁。

**已有实现：** 已有部署更新、有界 proof/identity 及回滚入口。runbook 当前明记 [identity-v2 与 managed peer registry provisioning 缺口](../../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#221-bounded-204-proof-and-identity-v2-contract)，只影响依赖它的具体部署/破坏性操作，不能扩成阅读、看板刷新或所有开发任务的前置条件。

**实际部署核验：** 没有本次发布或恢复结果；当前主线的[实际部署暂缓约束](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)仍需遵守。重启、旧窗口通过和源码合入均不证明当日恢复完成。

<a id="cc-architecture-command-center"></a>

## 指挥客户端

| 主题 | 关键边界 | 需人关注时机 | 权威入口 |
| --- | --- | --- | --- |
| ARCH-11 · 客户端自身 | 独立 macOS Tauri 2 + Rust + Solid/TypeScript + SQLite；文档/Git/PR 保留事实所有权，Codex 默认关闭且属可选实验接入 | 改技术栈、引入云端/常驻 supervisor、共享团队状态、公开分发、扩版本或自动执行 | [技术栈](../command-center.design.md#43-具体技术栈)、[基础切片](../command-center.design.md#110-首个基础实现切片)、[实验接入](../command-center.design.md#DES-CC-012)、[操作说明](../../../../tools/oasis7-command-center/README.md#verification) |

**设计状态：** 配对产品/系统设计仍含目标能力，完整产品验收未完成。系统设计 §3 固定于早期设计基线，当前交付要同时读取 §11 及后续 PR，不能继续把客户端写成“只有方案”。

**已有实现与验证：**

- [PR #4533](https://github.com/eng-cc/oasis7/pull/4533)：桌面基础、四视图、SQLite、真实 worktree 和有限 GitHub 窗口；记录了 macOS arm64 本地打包与原生交互验证。
- [PR #4563](https://github.com/eng-cc/oasis7/pull/4563)：固定 Codex CLI 0.137.0 的可选实验执行、原请求身份、未知结果与退出管理；真实握手/receipt 有记录，但没有成功模型输出证明。
- [PR #4572](https://github.com/eng-cc/oasis7/pull/4572)：读取显式规范后继、保留历史版本和来源理由；记录了原生界面与重启恢复验证。
- 个人决定、阅读记录和未决操作与可重建缓存分开；启用连接不代表授权任务，恢复不自动重发未知副作用。

**实际部署核验：** 上述原生验证是 PR 中特定版本和机器的结果；本轮未核对用户当前安装。公共分发、公证、其他系统/CLI 组合及全部产品验收仍不能由这些切片推出。

## 下次刷新时

- 先核对主责入口和显式后继，再核对影响本主题的实际合入、验收或部署结果；没有变化不为整张表重写状态。
- 保留来源版本和验证窗口；设计改变、代码合入、实际环境变化分别更新，旧成功不自动扩到新范围。
- 对确需决策的承诺、成本、权限、不可逆数据或真实不确定结果，在[待人关注](../README.md#cc-status-attention)说明影响与下一步。
- 决定落实回相应主责产品/系统文档，本页随源刷新；具体维护步骤沿用[持续更新流程](../command-center.manual.md#cc-maintenance-architecture)。
