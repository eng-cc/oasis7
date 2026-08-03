# 玩家入口与发行 PRD

## 文档身份

- 产品模块：玩家入口与发行
- 产品模块 slug：`player-entry-distribution`
- 产品层唯一 PRD：`doc/product/player-entry-distribution/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-004`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-08-03`
- 后继文档：`无`
- 下层专业域：[`README.md`](../../../README.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文只组合玩家从了解产品到进入、安装、验证受支持技术预览的路径。根 `README.md` 是公开当前状态与 claim envelope 的唯一权威；Viewer、Launcher、发行资产与访问模式的实现合同由 `world-simulator` 专业域拥有。

### 活跃产品专题

- [`访问模式与发行就绪`](access-modes-and-release-readiness.prd.md)：`viewer` / `pure_api` 模式边界、能力等价、失败恢复、统一候选门禁与公开 claim 升阶。
- [`本地启动会话连续性与恢复`](local-launch-session-continuity-and-recovery.prd.md)：受支持 Launcher 路径中本地会话的真实状态、停止、恢复与配置边界；不把本地运行事实升级为模式或发行 claim。
- [`发行沟通与公开口径`](release-communications-and-public-claims.prd.md)：内部证据到外部 claim 的状态隔离、审核、发布、纠正与撤回合同。
- [`参与和认可边界`](participation-and-recognition-boundaries.prd.md)：有限预览中的可审核贡献、非自动权益和治理执行边界。
- [`免费进入、世界内成长与有界认可`](free-entry-world-progression-and-recognition.prd.md)：免费基础进入、非权力型可选服务、世界内成长、区域互赖和不自动授予权力的认可边界。

## 1. 产品承诺

玩家能从一个不夸大的公开入口理解 oasis7 当前是什么、哪些模式已有证据、如何进入或安装，以及如何验证自己连接的是声明的产品路径。

## 2. 范围与玩家边界

覆盖产品发现、公开说明、Web 访问、平台安装、Launcher 转移、账号/模式入口和首次验证。玩家可以了解当前阶段并选择受支持入口；不应被未发布功能、内部测试模式、过期下载或历史 go 证据诱导。

### 安装与发行体验边界

每个平台应提供一个推荐下载包和主 CTA；下载前应清楚显示支持边界、版本与 checksum，使玩家无需研究 shell、bundle 内容或多个技术资产的差异即可完成受支持的安装或启动。

当前升级只支持重新下载最新主包并手动覆盖安装/替换；用户须先备份 `config.toml`、`.oasis7_launcher_ux_state.json` 与 `output/chain-runtime/<node_id>/reward-runtime-execution-world/`。这不是应用内更新、自动迁移或跨目录状态保留承诺。Windows codesigning trust chain 与 macOS 签名/notarization 未完成前，不得把技术预览资产、单主 CTA 或手动安装路径表述为普通用户广泛发行已就绪。

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

## 6. Non-Goals

- 不宣布任何未由当前根 README 与发布证据支持的玩家可用性。
- 不在产品层定义 Launcher、Viewer、登录、打包或渠道实现。
- 不用产品路线图代替发布任务、QA 放行、运营承诺或渠道 runbook。
