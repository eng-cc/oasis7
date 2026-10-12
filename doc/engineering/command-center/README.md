# oasis7 项目状态与指挥入口

本页帮助项目负责人在五分钟内看清长期方向、当前主线、已交付变化和需要关注的边界。
它是带来源和核验时间的滚动摘要；目标、规范、任务、部署和验收继续由原有主责文档及实际记录承载。

**当前判断：项目仍处于受限可玩技术预览。**
主线是同一持久世界中的首产物、阻塞恢复和回访接续；代码前置持续交付，正式开放与真人闭环尚未验收。
实际部署继续按已有用户决定暂缓。来源：[公开阶段](../../../README.md#项目状态)、[当前里程碑](../../core/prd.md#当前阶段收口优先级stage-closure-backlog)、[#4363 最新进展](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)。

| 想了解什么 | 入口 |
| --- | --- |
| 下次如何持续更新 | [持续更新流程](command-center.manual.md) |
| 整体分层、部署、存储、权限与恢复 | [顶层架构索引](architecture/README.md) |
| 指挥客户端的产品目标与技术方案 | [产品设计](command-center.prd.md) · [系统设计](command-center.design.md) |
| 如何运行和操作已有客户端 | [客户端使用说明](../../../tools/oasis7-command-center/README.md) |

<a id="cc-status-baseline"></a>

## 核验基线与覆盖范围

| 来源 | 本轮基线 |
| --- | --- |
| 仓库文档与代码 | main [`598b308a23f3f0189843618d14abb1aa31c0d2de`](https://github.com/eng-cc/oasis7/tree/598b308a23f3f0189843618d14abb1aa31c0d2de)，提交于 2026-10-11 18:40:57 +08:00 |
| GitHub 可变记录 | 2026-10-11 18:52–18:54 +08:00 读取；#4363 最新评论更新于 18:51:33；PR 状态与代码基线分开表达 |
| 最近变化窗口 | 上述 main 基准前的 30 条提交，事件时间覆盖 2026-10-10 04:18:00 至 2026-10-11 18:40:57 +08:00；选取下文 8 项有意义变化 |
| 本次局部补充 | 2026-10-12 仅核对 [PR #4624](https://github.com/eng-cc/oasis7/pull/4624) 的本机工具候选与 `affa7d1` arm64 同包 App 验证；PR 仍未合入，不前移上述 main、GitHub 或游戏/部署来源窗口 |
| 未覆盖 | 全部历史 PR、全部项目任务、当前线上拓扑/健康、真实故障域演练、同候选真人闭环；未重新运行上述 2026-10-11 窗口各 PR 记载的验证 |

本页的仓库相对链接用于继续阅读当前主责正文；需要复核本轮结论时，按上面的固定提交查看对应路径。
GitHub 记录可能晚于代码基准，未合入分支不计入 main 已实现能力。
最近 30 条提交是明确的取样窗口，不代表全量项目快照，也不是“自上次查看以来”的完整变化。
后续刷新沿[时间与缺失处理](command-center.manual.md#cc-maintenance-freshness)保留各来源自己的核验时间。

<a id="cc-status-goals"></a>

## 长期方向与当前阶段

长期方向是让玩家通过 Agent，在同一持续世界中形成有代价、可追溯的影响。
成长围绕“建立可恢复能力、服务区域需求、取得有限区域影响”展开；文明尺度共同项目是自愿扩展。
长期自治与文明涌现仍需验证，不能从代码规模或文档采纳推断已实现。来源：[产品承诺](../../product/world-rules-core-gameplay/prd.md#1-产品承诺)、[项目定位](../../../README.md#oasis7)。

| 产品域 | 长期用户结果 | 主责入口 |
| --- | --- | --- |
| 世界规则与玩法系统 | 玩家行动产生可归因成果，并形成新的能力、区域价值和下一阶段方向 | [产品承诺与路线图](../../product/world-rules-core-gameplay/prd.md#4-路线图) |
| 权威世界基础设施 | 同一世界保持唯一、确定、持久、可验证，并可沿原历史恢复 | [基础设施承诺](../../product/world-infrastructure/prd.md#1-产品承诺) |
| 智能体、世界模拟与交互 | Agent 理解并行动，玩家能观察、指导、纠正和接续其工作 | [智能体产品承诺](../../product/agents-world-simulation/prd.md#1-产品承诺) |
| 玩家接入与发行 | 用户能了解、进入并持续使用有证据支持的产品路径，清楚访问和发行边界 | [四域职责](../../product/README.md#四大产品模块)、[公开状态](../../../README.md#项目状态) |

**当前阶段目标**：受控真实玩家通过有界授权 Agent，在同一持久世界取得真实首产物，
理解并恢复生产阻塞、保留成果，并在再次进入后继续原目标。来源：[core 当前阶段](../../core/prd.md#当前阶段收口优先级stage-closure-backlog)。

代表性路径为 `starter-industrial-smelter-to-assembler-v1`：
Smelter 合法建成 → 铁锭生产 cycle → owner-bound output ledger 实际正量结算 → 开放 Assembler 候选。
首产物采用 `production_only`；持续生产能力、交付、真人理解和回访分别验收。
来源：[本阶段交付](../../../README.md#本阶段交付)、[生产与交付边界](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-004)。

<a id="cc-status-progress"></a>

## 当前主线与进展

以下主线消费既有里程碑和实际交付记录，不在这里重新排序或建立一套任务状态。
当前多里程碑剩余义务由 [#4363](https://github.com/eng-cc/oasis7/issues/4363) 承接；正文复选框与最新评论需一起读。
“已有代码”“适用验证通过”“正式开放”“真人接受”分别表达，不用 PR 数量换算项目完成率。

### 1. 持久单权威与同世界恢复

已有 manifest、journal/replica、离线 activation verifier、受保护本地工程执行等前置交付。
正式 initial activation 与普通消费者接线、不完整候选崩溃恢复、独立故障域保留/恢复、旧写者隔离和人工交接仍待完成。
下一步先在已有代码与验证范围内收口缺口；实际部署暂缓，不能把本地双目录当成独立硬件故障域。
来源：[基础设施当前阶段](../../product/world-infrastructure/prd.md#world-infrastructure-current-stage)、[#4363 当前剩余条件](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)。

### 2. 真实首产物、阻塞恢复与回访

starter 候选、owner 目标持久化等前置已有代码；Web guest 启动/只读世界推进和 Hosted 修复分别已合入。
同一候选、同一正式世界上的实际生产、授权纠偏、阻塞恢复与目标接续尚未组合验收。
下一步核对实际 owner/Agent 入口与首产物链路；guest 启动不能代替 owner onboarding 或游戏操作。
来源：[成果承接持续能力](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-008)、[#4363](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)、[#4566](https://github.com/eng-cc/oasis7/pull/4566)。

### 3. 当前验证与消费者收口

[#4568](https://github.com/eng-cc/oasis7/pull/4568) 已合入，自身最终 CI 的通过范围可追溯。
观测时 [#4571](https://github.com/eng-cc/oasis7/pull/4571) `b251e295` 未合入、新版本 CI 待完成；
依赖它的 [#4567](https://github.com/eng-cc/oasis7/pull/4567) `8ea69adb` 与 [#4569](https://github.com/eng-cc/oasis7/pull/4569) `4e94a2e6` 仍为草稿。
下一步由原负责人处理实际新源验证与合入；旧绿色结果不替代新版本，也不因此新增人工审批。

### 4. 受控真人两次会话

目标和观察口径已定义；持久世界与实际体验满足首次开放条件后，才开展目标玩家两次会话观察。
本轮未取得真人完成证据。届时需区分独立完成、提示、代操作、未完成和未覆盖，并区分安排的第二次会话与自发回访。
来源：[core 受控真人验证工作包](../../core/prd.md#当前阶段收口优先级stage-closure-backlog)、[#4363 验收义务](https://github.com/eng-cc/oasis7/issues/4363)。

### 5. BFT 与同世界交接

顶层合同允许 BFT 与受控单权威路径并行研发，先验证同一历史，再按 H/H+1 交接转移写权。
独立验证者协议、持久证书与同世界交接尚未验收；它不自动成为已满足条件的受控单权威试玩前置。
来源：[基础设施路线图](../../product/world-infrastructure/prd.md#4-路线图)、[#4363 当前记录](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)。

### 6. 外部游戏 Runtime 的 owner 目标控制前置

本项局部核验于 2026-10-12：[#4625](https://github.com/eng-cc/oasis7/pull/4625) 候选 `edae8d18` 已实现私有目标准备、单记录重试/重启、公开 canonical 发布和授权当前正文回读，尚在 PR 验证与合入流程，不计入 main 已交付能力。SDK 29、私有 library 22、任务链 8 项本地回归通过；journal tail 缺前驱的恢复缺口已观察失败并修复，修复后的原生回归与最终 CI 仍待完成。此段只刷新本项，不刷新前述全局或环境核验基线。

目标控制仍保持 suspended、未绑定 executor；完整 TaskSnapshot/Context、正式 Skill、实际 Runtime 目标应用与自主首局仍未验收。下一步连接这些消费者，并按真实游戏自测合同验收，不能用 TCP 脚本代签自主游玩。来源：[Game API 当前切片](../../world-simulator/llm/external-agent-game-api.design.md#已实施切片owner-目标准备发布与当前正文回读)、[协作与私有目标合同](../../world-runtime/runtime/player-runtime-collaboration.design.md#6-接口与数据合同)。

<a id="cc-status-changes"></a>

## 最近做了什么

下列 8 项均已由 GitHub 的实际合入状态与 merge commit 核对，且出现在固定 main 的最近 30 条提交中。
时间为北京时间（+08:00）；验证描述引用各 PR 记录，本次没有重新运行，也没有把合入解释为部署。

| 合入时间 | 变化及对当前工作的价值 | 事实边界与来源 |
| --- | --- | --- |
| 10-11 18:40:57 | 指挥面板读取文档 lifecycle/successor/reason，保留历史并定位明确继任条款 | 声明不自动取得规范权威；无法唯一、安全定位时仍 unknown。 [#4572](https://github.com/eng-cc/oasis7/pull/4572)，`598b308a` |
| 10-11 16:34:37 | 修复 Hosted 完成唤醒与已提交请求重放，避免重复执行和持久化 | 自身最终验证不代替后来 CLI/report 组合源；部署仍暂缓。 [#4568](https://github.com/eng-cc/oasis7/pull/4568)，`fa56ad2e` |
| 10-11 12:02:27 | 修复发行 Web 启动后端合同，验证实际 WASM 渲染与签名公开世界读取推进 | 覆盖隔离 QA guest/configuration 路径，不证明 owner onboarding、正式世界激活或真实游戏完成。 [#4566](https://github.com/eng-cc/oasis7/pull/4566)，`29f380b5` |
| 10-11 11:54:03 | 增加 owner-signed GameTaskCreate，经 World Service/consensus 持久化任务控制 | 初态 suspended，无目标或 executor；完整 context、action admission、配对未包含。 [#4565](https://github.com/eng-cc/oasis7/pull/4565)，`543120f8` |
| 10-11 11:46:12 | 指挥面板加入默认关闭的实验 Codex runtime，支持显式连接/输入/中断及原请求身份恢复 | 固定 CLI 0.137.0；真实握手/receipt 有记录，但模型 turn 返回 `other`，没有成功模型输出证明。 [#4563](https://github.com/eng-cc/oasis7/pull/4563)，`7828a2a0` |
| 10-11 10:28:14 | Game API 公布 OpenAPI 3.1 及已实现签名读取能力与限制 | 只读 bootstrap 不等于完整网络游戏行动闭环；后续 task-control 增量见 #4565。 [#4562](https://github.com/eng-cc/oasis7/pull/4562)，`679ba9ed` |
| 10-11 01:17:05 | 本地 signer 增加 Ed25519 生命周期、轮换、加密导入导出及恢复管理 | 代码与测试交付；没有真实密钥/策略操作或 host 升级。 [#4539](https://github.com/eng-cc/oasis7/pull/4539)，`9341adc8` |
| 10-11 00:04:47 | 指挥面板桌面基础实现：Tauri/Rust/SQLite/Solid，读取项目来源、有限 PR 窗口并保存阅读/决策 | 有 macOS arm64 本地打包和原生验证；公证/公开分发、旧系统/x86_64/VoiceOver 未验收。 [#4533](https://github.com/eng-cc/oasis7/pull/4533)，`2b334ed3` |

[#4518](https://github.com/eng-cc/oasis7/pull/4518) 是指挥客户端产品/系统设计来源；
后续 #4533、#4563、#4572 已有实现，不能继续把客户端整体描述为“仅方案”，也不能据此断言全部设计完成。
按事件时间阅读 PR：#4533 当时尚未实现的 runtime，后来由 #4563 增量交付。

### 本次候选增量（2026-10-12）

[PR #4624](https://github.com/eng-cc/oasis7/pull/4624) 仍是未合入候选，新增内置私有资料、同构建可选 CLI、兼容既有 signer 的文件签名接入，以及显式私有开发会话输入；普通本机工具边界见[当前设计](local-tools.design.md)。
本次在 `affa7d1` 的 macOS arm64 同包原生 App 核对了真实建库、条件保存冲突后的草稿保留和 CLI 安装。
这些结果只覆盖该候选与所列场景，不计入 main 已交付变化，也不证明部署、公共发行或全部本机工具验收。

管理员桥与 Apple 签名集成由用户明确暂停；本机用户尚未绑定 signer，真实签名提交未验收。
该候选不执行特权安装、custody 管理或生产变更，原有游戏、部署及其他 PR 的核验时点保持不变。
来源：[候选 PR 与验证范围](https://github.com/eng-cc/oasis7/pull/4624)、[暂停范围与验收边界](local-tools.design.md)。

<a id="cc-status-attention"></a>

## 需要人关注

### 需要决定

在本轮已读取的范围内，没有发现新增且已核实的即时待批准事项；当前 CI 等待和普通修复继续由原负责人推进。
后续若改变阶段目标、正式开放范围或部署安排，应回到对应主责条款和已有操作入口形成具体决定。
这里记录真实问题，不把正常 CI、文档刷新或已有授权工作包装成新的许可请求。来源：[开发流程边界](../workflow/source-of-truth.md#1-适用范围与规则边界)、[#4363 当前安排](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)。

### 待补事实

| 需要补齐的信息 | 为什么影响决策 | 处理入口 |
| --- | --- | --- |
| 正式部署候选、环境、故障域、实际保留/恢复与旧写者隔离结果 | 决定能否从代码前置进入正式持久世界开放；当前没有线上状态核验 | [架构索引](architecture/README.md)、[#4363](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860) |
| 同候选首产物、阻塞恢复、目标接续与真实玩家观察 | 决定邀请范围和是否继续、修复或收窄；局部实现与 guest 启动不足以回答 | [当前阶段](../../core/prd.md#当前阶段收口优先级stage-closure-backlog)、[首局验收](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-industrial-004) |
| #4571 及消费者的新源 CI/合入结果 | 更新代码进展，识别仍存在的具体回归；这属于后续取证，不是新增审批 | [#4571](https://github.com/eng-cc/oasis7/pull/4571)、[#4567](https://github.com/eng-cc/oasis7/pull/4567)、[#4569](https://github.com/eng-cc/oasis7/pull/4569) |

### 知悉变化

- **部署暂停已经决定。** 不创建正式世界、生产密钥或启动实际故障域演练；已有授权的代码、验证、评审及正常合入继续。来源：[已有用户限制](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6097891450)、[最新延续](https://github.com/eng-cc/oasis7/issues/4363#issuecomment-6107216860)。
- **游戏 Agent 与开发会话分开看。** Game API 的读取/发现/空任务切片不证明 OpenClaw/Codex 完成游戏自主首局；指挥面板实验 runtime 也不是游戏 Runtime 验收。来源：[外部 Runtime 产品目标](../../product/agents-world-simulation/prd.md#external-runtime-play)、[真实首期完成要求](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#req-ext-010)、[#4563](https://github.com/eng-cc/oasis7/pull/4563)。
- **架构方案与实际部署分开看。** 当前应持续查阅[架构索引](architecture/README.md)中的主责合同与部署核验入口；设计采纳、代码合入、环境验证和公开发行分别成立。来源：[基础设施事实边界](../../product/world-infrastructure/prd.md#53-事实边界与未决承接)。

下一次维护按[持续更新流程](command-center.manual.md#cc-maintenance-steps)增量核对上述变化与未知项。
已解决事项移出当前关注，历史通过 Git 和原 PR 追溯；没有实质变化时不制造新的提交。
